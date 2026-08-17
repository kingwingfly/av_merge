//! Mux trait

use ffmpeg_next::{
    self as ffmpeg, ChannelLayout, Packet, codec, decoder, encoder,
    ffi::{
        AVBufferRef, AVHWDeviceType, AVHWFramesContext, AVPixelFormat, EAGAIN, av_buffer_unref,
        av_hwdevice_ctx_create, av_hwframe_ctx_alloc, av_hwframe_ctx_init, av_hwframe_get_buffer,
        av_hwframe_transfer_data,
    },
    format::{self, context::Output},
    frame, media,
    software::{resampling, scaling},
};

use crate::{
    Result,
    codec::{AFormat, CodecConfig, VFormat},
    error::AVMuxError,
    fifo::AudioFifo,
    file::{AFile, AVFile as _, VFile},
};

use std::{
    collections::HashMap,
    ptr,
    sync::{Arc, Mutex},
};

/// Trait for merging multiple media files into one.
pub trait Mux {
    /// Simply mux two media files into a single output file.
    fn simple_mux(self, output: VFile) -> Result<VFile>;
    /// Encode and mux two media files into a single output file.
    fn mux(self, output: VFile, conf: CodecConfig) -> Result<VFile>;
}

/// `ChannelLayout` carries raw pointers for custom channel orders and so is not
/// `Send`. Every layout used here is either pointer-free (`ChannelLayout::default`)
/// or copied from an input context that moves onto the same worker thread as the
/// closure holding it, so the transfer never outlives the memory it borrows.
#[derive(Clone, Copy)]
struct SendChannelLayout(ChannelLayout);

// SAFETY: see the type's documentation.
unsafe impl Send for SendChannelLayout {}

impl SendChannelLayout {
    /// Unwrap the layout. Closures must reach the layout through this method:
    /// reading the field directly makes them capture the inner, non-`Send`
    /// `ChannelLayout` instead of this wrapper.
    fn layout(self) -> ChannelLayout {
        self.0
    }
}

/// `true` when the codec only reported "nothing to hand back yet" or "fully
/// flushed", the two conditions that terminate a drain loop.
fn drained(e: &AVMuxError) -> bool {
    match e {
        AVMuxError::Ffmpeg(ffmpeg::Error::Eof) => true,
        AVMuxError::Ffmpeg(ffmpeg::Error::Other { errno }) => *errno == EAGAIN,
        _ => false,
    }
}

/// Allocate and initialise a CUDA `AVHWFramesContext`; the caller owns the
/// returned reference.
fn cuda_frames_ctx(width: i32, height: i32) -> Result<*mut AVBufferRef> {
    unsafe {
        let mut device = ptr::null_mut();
        match av_hwdevice_ctx_create(
            &mut device,
            AVHWDeviceType::AV_HWDEVICE_TYPE_CUDA,
            ptr::null(),
            ptr::null_mut(),
            0,
        ) {
            e if e < 0 => return Err(ffmpeg::Error::from(e).into()),
            _ => {}
        }

        // `av_hwframe_ctx_alloc` takes its own reference on the device.
        let mut frames = av_hwframe_ctx_alloc(device);
        av_buffer_unref(&mut device);
        if frames.is_null() {
            return Err(AVMuxError::Invalid);
        }

        let ctx = (*frames).data as *mut AVHWFramesContext;
        (*ctx).format = AVPixelFormat::AV_PIX_FMT_CUDA;
        (*ctx).sw_format = AVPixelFormat::AV_PIX_FMT_YUV420P;
        (*ctx).width = width;
        (*ctx).height = height;
        (*ctx).initial_pool_size = 16;

        match av_hwframe_ctx_init(frames) {
            e if e < 0 => {
                av_buffer_unref(&mut frames);
                Err(ffmpeg::Error::from(e).into())
            }
            _ => Ok(frames),
        }
    }
}

/// Pull one decoded frame and scale it into `v_frame`.
fn recv_video_frame(decoder: &mut decoder::Video, v_frame: &mut frame::Video) -> Result<()> {
    let mut frame = frame::Video::empty();
    decoder.receive_frame(&mut frame)?;

    let mut sw_scaler = scaling::Context::get(
        frame.format(),
        frame.width(),
        frame.height(),
        v_frame.format(),
        v_frame.width(),
        v_frame.height(),
        scaling::Flags::FAST_BILINEAR,
    )?;
    sw_scaler.run(&frame, v_frame)?;
    v_frame.set_pts(frame.pts());

    Ok(())
}

/// Upload `frame` into the encoder's CUDA frame pool and submit it, or flush
/// the encoder when `frame` is `None`.
fn send_video_frame(
    encoder: &mut encoder::video::Encoder,
    frame: Option<&frame::Video>,
) -> Result<()> {
    let Some(frame) = frame else {
        encoder.send_eof()?;
        return Ok(());
    };

    let mut hw_v_frame = frame::Video::empty();
    unsafe {
        let hw_frames_ctx = (*encoder.as_mut_ptr()).hw_frames_ctx;
        match av_hwframe_get_buffer(hw_frames_ctx, hw_v_frame.as_mut_ptr(), 0) {
            e if e < 0 => return Err(ffmpeg::Error::from(e).into()),
            _ => {}
        }
        match av_hwframe_transfer_data(hw_v_frame.as_mut_ptr(), frame.as_ptr(), 0) {
            e if e < 0 => return Err(ffmpeg::Error::from(e).into()),
            _ => {}
        }
        (*hw_v_frame.as_mut_ptr()).time_base = (*frame.as_ptr()).time_base;
    }
    hw_v_frame.set_pts(frame.pts());
    encoder.send_frame(&hw_v_frame)?;
    Ok(())
}

impl Mux for (VFile, AFile) {
    fn simple_mux(self, output: VFile) -> Result<VFile> {
        let mut ofmt_ctx = output.ofmt_ctx()?;
        let mut stream_maps = vec![];

        {
            let ifmt_ctx = self.0.ifmt_ctx()?;
            let mut map = HashMap::new();
            for stream in ifmt_ctx.streams() {
                let medium = stream.parameters().medium();
                if medium != media::Type::Video
                    && medium != media::Type::Audio
                    && medium != media::Type::Subtitle
                {
                    continue;
                }
                let mut new_stream = ofmt_ctx.add_stream(encoder::find(codec::Id::None))?;
                new_stream.set_parameters(stream.parameters());
                map.insert(stream.index(), new_stream.index());
            }
            stream_maps.push((ifmt_ctx, map));
        }
        {
            let ifmt_ctx = self.1.ifmt_ctx()?;
            let mut map = HashMap::new();
            for stream in ifmt_ctx.streams() {
                let medium = stream.parameters().medium();
                if medium != media::Type::Video
                    && medium != media::Type::Audio
                    && medium != media::Type::Subtitle
                {
                    continue;
                }
                let mut new_stream = ofmt_ctx.add_stream(encoder::find(codec::Id::None))?;
                new_stream.set_parameters(stream.parameters());
                map.insert(stream.index(), new_stream.index());
            }
            stream_maps.push((ifmt_ctx, map));
        }

        ofmt_ctx.write_header()?;

        let ofmt_ctx = Arc::new(Mutex::new(ofmt_ctx));

        let jhs = stream_maps
            .into_iter()
            .map(|(mut ifmt_ctx, map)| {
                let ofmt_ctx = ofmt_ctx.clone();
                std::thread::spawn(move || -> Result<()> {
                    loop {
                        // `Packet::read` does not unref first, so it needs a
                        // fresh packet on every iteration.
                        let mut packet = Packet::empty();
                        match packet.read(&mut ifmt_ctx) {
                            Ok(()) => {}
                            Err(ffmpeg::Error::Eof) => break,
                            Err(e) => return Err(e.into()),
                        }
                        let Some(&index) = map.get(&packet.stream()) else {
                            continue;
                        };
                        let old_ts = ifmt_ctx
                            .stream(packet.stream())
                            .expect("Packet should come from a known stream")
                            .time_base();
                        let mut ofmt_ctx = ofmt_ctx.lock().unwrap();
                        let new_ts = ofmt_ctx
                            .stream(index)
                            .expect("Output stream should exist")
                            .time_base();
                        packet.rescale_ts(old_ts, new_ts);
                        packet.set_stream(index);
                        packet.set_position(-1);
                        packet.write_interleaved(&mut ofmt_ctx)?;
                    }
                    Ok(())
                })
            })
            .collect::<Vec<_>>();
        for jh in jhs {
            jh.join().expect("JoinHandle should be able to be joined")?;
        }

        ofmt_ctx.lock().unwrap().write_trailer()?;
        Ok(output)
    }

    fn mux(self, output: VFile, conf: CodecConfig) -> Result<VFile> {
        let CodecConfig { vconf, aconf } = conf;

        let mut ofmt_ctx = output.ofmt_ctx()?;

        let (vfile, afile) = self;

        let mut v_ifmt_ctx = vfile.ifmt_ctx()?;
        let mut v_map = HashMap::new();
        for stream in v_ifmt_ctx.streams() {
            let params = stream.parameters();
            if params.medium() != media::Type::Video {
                continue;
            }

            let codec_id = params.id();
            let dec_codec = decoder::find(codec_id).ok_or(AVMuxError::CodecNotFound(format!(
                "Decoder id {codec_id:?} not found for video {}",
                vfile.path_url
            )))?;
            let time_base = stream.time_base();
            let mut dec_ctx = codec::context::Context::from_parameters(stream.parameters())?;
            dec_ctx.set_time_base(time_base);
            let mut decoder = dec_ctx.decoder().open_as(dec_codec)?.video()?;

            let codec_name = match vconf.format {
                Some(VFormat::H264) => "h264_nvenc",
                Some(VFormat::HEVC) => "hevc_nvenc",
                Some(VFormat::AV1) => "av1_nvenc",
                None => match codec_id {
                    codec::Id::H264 => "h264_nvenc",
                    codec::Id::HEVC => "hevc_nvenc",
                    codec::Id::AV1 => "av1_nvenc",
                    id => {
                        return Err(AVMuxError::CodecNotFound(format!(
                            "Encoder codec id {id:?} of video ({}) is not supported by NVIDIA\n
                            The default output codec is the same as the source video, please set output format in CodecConf",
                            vfile.path_url
                        )));
                    }
                },
            };
            let enc_codec =
                encoder::find_by_name(codec_name).ok_or(AVMuxError::CodecNotFound(format!(
                    "Encoder {codec_name} not found for video {}",
                    vfile.path_url
                )))?;
            let mut encoder = codec::context::Context::new_with_codec(enc_codec)
                .encoder()
                .video()?;
            let bitrate = vconf.bitrate.unwrap_or(params.bit_rate());
            encoder.set_bit_rate(bitrate as usize);
            // Neither the source frame rate nor a `-1` gop size is reachable
            // through `ffmpeg-next`'s typed setters.
            unsafe {
                (*encoder.as_mut_ptr()).framerate = (*params.as_ptr()).framerate;
                (*encoder.as_mut_ptr()).gop_size = vconf.gop.unwrap_or(-1);
            }
            let (width, height) = vconf
                .size
                .unwrap_or(unsafe { ((*params.as_ptr()).width, (*params.as_ptr()).height) });
            encoder.set_width(width as u32);
            encoder.set_height(height as u32);
            encoder.set_time_base(time_base);
            encoder.set_format(format::Pixel::CUDA);

            // The encoder takes ownership of the frames context reference and
            // frees it even if `open` fails.
            let hw_frames_ctx = cuda_frames_ctx(width, height)?;
            unsafe { (*encoder.as_mut_ptr()).hw_frames_ctx = hw_frames_ctx };
            let mut encoder = encoder.open()?;

            let mut v_frame =
                frame::Video::new(format::Pixel::YUV420P, width as u32, height as u32);
            // `ffmpeg-next` exposes no typed setter for `AVFrame::time_base`.
            unsafe { (*v_frame.as_mut_ptr()).time_base = time_base.into() };

            let mut new_stream = ofmt_ctx.add_stream(enc_codec)?;
            new_stream.set_parameters(&encoder);
            new_stream.set_time_base(time_base);

            let index = new_stream.index();
            let recv_packets =
                move |encoder: &mut encoder::video::Encoder, ofmt_ctx: &mut Output| -> Result<()> {
                    loop {
                        let mut packet = Packet::empty();
                        match encoder.receive_packet(&mut packet) {
                            Err(ffmpeg::Error::Eof) => break,
                            Err(ffmpeg::Error::Other { errno }) if errno == EAGAIN => break,
                            res => res?,
                        }
                        packet.set_stream(index);
                        packet.write_interleaved(ofmt_ctx)?;
                    }
                    Ok(())
                };

            let convert = move |packet: Option<&Packet>, ofmt_ctx: &mut Output| -> Result<()> {
                match packet {
                    Some(packet) => decoder.send_packet(packet)?,
                    None => decoder.send_eof()?,
                }
                loop {
                    match recv_video_frame(&mut decoder, &mut v_frame) {
                        Err(e) if drained(&e) => break,
                        res => res?,
                    }
                    recv_packets(&mut encoder, ofmt_ctx)?;
                    send_video_frame(&mut encoder, Some(&v_frame))?;
                }
                if packet.is_none() {
                    recv_packets(&mut encoder, ofmt_ctx)?;
                    send_video_frame(&mut encoder, None)?;
                    recv_packets(&mut encoder, ofmt_ctx)?;
                }
                Ok(())
            };

            v_map.insert(stream.index(), convert);
        }

        let mut a_ifmt_ctx = afile.ifmt_ctx()?;
        let mut a_map = HashMap::new();
        for stream in a_ifmt_ctx.streams() {
            let params = stream.parameters();
            if params.medium() != media::Type::Audio {
                continue;
            }

            let mut codec_id = params.id();
            let dec_codec = decoder::find(codec_id).ok_or(AVMuxError::CodecNotFound(format!(
                "Decoder id {codec_id:?} not found for audio {}",
                afile.path_url
            )))?;
            let time_base = stream.time_base();
            let mut dec_ctx = codec::context::Context::from_parameters(stream.parameters())?;
            dec_ctx.set_time_base(time_base);
            let mut decoder = dec_ctx.decoder().open_as(dec_codec)?.audio()?;

            if let Some(format) = aconf.format {
                match format {
                    AFormat::AAC => codec_id = codec::Id::AAC,
                    AFormat::FLAC => codec_id = codec::Id::FLAC,
                }
            }
            let enc_codec = encoder::find(codec_id).ok_or(AVMuxError::CodecNotFound(format!(
                "Encoder id {codec_id:?} not found for audio {}",
                afile.path_url
            )))?;
            let sample_fmt = enc_codec
                .audio()?
                .formats()
                .and_then(|mut formats| formats.next())
                .ok_or(AVMuxError::Invalid)?;
            let mut encoder = codec::context::Context::new_with_codec(enc_codec)
                .encoder()
                .audio()?;
            let sample_rate = aconf
                .sample_rate
                .unwrap_or(unsafe { (*params.as_ptr()).sample_rate });
            encoder.set_rate(sample_rate);
            encoder.set_format(sample_fmt);
            let ch_layout = match aconf.nb_channels {
                Some(num) => ChannelLayout::default(num),
                None => ChannelLayout(unsafe { (*params.as_ptr()).ch_layout }),
            };
            encoder.set_channel_layout(ch_layout);
            // The re-encoded frames are stamped with a running sample counter,
            // so the encoder and the output stream must count in samples too.
            // Reusing the demuxer's time base here (as versions <= 0.2 did)
            // stretches the output audio: an MP3 input reports 1/14112000.
            let time_base = ffmpeg::Rational(1, sample_rate);
            encoder.set_time_base(time_base);
            let mut encoder = encoder.open()?;

            let mut a_fifo = AudioFifo::new(sample_fmt, ch_layout.channels())?;
            let ch_layout = SendChannelLayout(ch_layout);

            let mut new_stream = ofmt_ctx.add_stream(enc_codec)?;
            new_stream.set_parameters(&encoder);
            new_stream.set_time_base(time_base);

            let recv_frame =
                move |decoder: &mut decoder::Audio, a_fifo: &mut AudioFifo| -> Result<()> {
                    let mut frame = frame::Audio::empty();
                    decoder.receive_frame(&mut frame)?;

                    let mut sw_resampler = resampling::Context::get(
                        frame.format(),
                        frame.channel_layout(),
                        frame.rate(),
                        sample_fmt,
                        ch_layout.layout(),
                        sample_rate as u32,
                    )?;

                    let mut a_frame =
                        frame::Audio::new(sample_fmt, frame.samples(), ch_layout.layout());
                    a_frame.set_rate(sample_rate as u32);
                    sw_resampler.run(&frame, &mut a_frame)?;

                    a_fifo.write(&mut a_frame)?;

                    Ok(())
                };

            let index = new_stream.index();
            let recv_packets =
                move |encoder: &mut encoder::audio::Encoder, ofmt_ctx: &mut Output| -> Result<()> {
                    loop {
                        let mut packet = Packet::empty();
                        match encoder.receive_packet(&mut packet) {
                            Err(ffmpeg::Error::Eof) => break,
                            Err(ffmpeg::Error::Other { errno }) if errno == EAGAIN => break,
                            res => res?,
                        }
                        packet.set_stream(index);
                        packet.write_interleaved(ofmt_ctx)?;
                    }
                    Ok(())
                };

            let mut a_pts = 0;
            let convert = move |packet: Option<&Packet>, ofmt_ctx: &mut Output| -> Result<()> {
                let frame_size = encoder.frame_size() as i32;
                match packet {
                    Some(packet) => decoder.send_packet(packet)?,
                    None => decoder.send_eof()?,
                }
                loop {
                    match recv_frame(&mut decoder, &mut a_fifo) {
                        Err(e) if drained(&e) => break,
                        res => res?,
                    }
                    while a_fifo.size() >= frame_size {
                        let mut a_frame =
                            frame::Audio::new(sample_fmt, frame_size as usize, ch_layout.layout());
                        a_frame.set_rate(sample_rate as u32);
                        unsafe { (*a_frame.as_mut_ptr()).time_base = time_base.into() };
                        a_frame.set_pts(Some(a_pts));
                        a_pts += a_frame.samples() as i64;
                        a_fifo.read(&mut a_frame)?;
                        recv_packets(&mut encoder, ofmt_ctx)?;
                        encoder.send_frame(&a_frame)?;
                    }
                }
                if packet.is_none() {
                    if a_fifo.size() > 0 {
                        let mut a_frame = frame::Audio::new(
                            sample_fmt,
                            a_fifo.size() as usize,
                            ch_layout.layout(),
                        );
                        a_frame.set_rate(sample_rate as u32);
                        unsafe { (*a_frame.as_mut_ptr()).time_base = time_base.into() };
                        a_frame.set_pts(Some(a_pts));
                        a_pts += a_frame.samples() as i64;
                        a_fifo.read(&mut a_frame)?;
                        recv_packets(&mut encoder, ofmt_ctx)?;
                        encoder.send_frame(&a_frame)?;
                    }
                    recv_packets(&mut encoder, ofmt_ctx)?;
                    encoder.send_eof()?;
                    recv_packets(&mut encoder, ofmt_ctx)?;
                }
                Ok(())
            };

            a_map.insert(stream.index(), convert);
        }

        ofmt_ctx.write_header()?;

        let ofmt_ctx = Arc::new(Mutex::new(ofmt_ctx));

        let v_jh = std::thread::spawn({
            let ofmt_ctx = ofmt_ctx.clone();
            move || -> Result<()> {
                loop {
                    let mut packet = Packet::empty();
                    match packet.read(&mut v_ifmt_ctx) {
                        Ok(()) => {}
                        Err(ffmpeg::Error::Eof) => break,
                        Err(e) => return Err(e.into()),
                    }
                    let Some(convert) = v_map.get_mut(&packet.stream()) else {
                        continue;
                    };
                    convert(Some(&packet), &mut ofmt_ctx.lock().unwrap())?;
                }
                for convert in v_map.values_mut() {
                    convert(None, &mut ofmt_ctx.lock().unwrap())?;
                }
                Ok(())
            }
        });

        let a_jh = std::thread::spawn({
            let ofmt_ctx = ofmt_ctx.clone();

            move || -> Result<()> {
                loop {
                    let mut packet = Packet::empty();
                    match packet.read(&mut a_ifmt_ctx) {
                        Ok(()) => {}
                        Err(ffmpeg::Error::Eof) => break,
                        Err(e) => return Err(e.into()),
                    }
                    let Some(convert) = a_map.get_mut(&packet.stream()) else {
                        continue;
                    };
                    convert(Some(&packet), &mut ofmt_ctx.lock().unwrap())?;
                }
                for convert in a_map.values_mut() {
                    convert(None, &mut ofmt_ctx.lock().unwrap())?;
                }
                Ok(())
            }
        });

        v_jh.join().unwrap()?;
        a_jh.join().unwrap()?;

        ofmt_ctx.lock().unwrap().write_trailer()?;
        Ok(output)
    }
}

impl Mux for (AFile, VFile) {
    #[inline(always)]
    fn simple_mux(self, output: VFile) -> Result<VFile> {
        (self.1, self.0).simple_mux(output)
    }

    #[inline(always)]
    fn mux(self, output: VFile, conf: CodecConfig) -> Result<VFile> {
        (self.1, self.0).mux(output, conf)
    }
}
