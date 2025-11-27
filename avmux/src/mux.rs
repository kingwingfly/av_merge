//! Mux trait

use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext, AVPacket},
    avformat::AVFormatContextOutput,
    avutil::{AVAudioFifo, AVChannelLayout, AVFrame, AVHWDeviceContext},
    error::RsmpegError,
    ffi::{
        AV_CODEC_ID_AAC, AV_CODEC_ID_AV1, AV_CODEC_ID_FLAC, AV_CODEC_ID_H264, AV_CODEC_ID_HEVC,
        AV_HWDEVICE_TYPE_CUDA, AV_PIX_FMT_CUDA, AV_PIX_FMT_YUV420P, SWS_FAST_BILINEAR,
    },
    swresample::SwrContext,
    swscale::SwsContext,
};

use crate::{
    Result,
    codec::{AFormat, CodecConfig, VFormat},
    error::AVMuxError,
    file::{AFile, AVFile as _, VFile},
};

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// Trait for merging multiple media files into one.
pub trait Mux {
    /// Simply mux two media files into a single output file.
    fn simple_mux(self, output: VFile) -> Result<VFile>;
    /// Encode and mux two media files into a single output file.
    fn mux(self, output: VFile, conf: CodecConfig) -> Result<VFile>;
}

impl Mux for (VFile, AFile) {
    fn simple_mux(self, output: VFile) -> Result<VFile> {
        let mut ofmt_ctx = output.ofmt_ctx()?;
        let mut stream_maps = vec![];

        {
            let ifmt_ctx = self.0.ifmt_ctx()?;
            let mut map = HashMap::new();
            for stream in ifmt_ctx.streams() {
                let codec_type = stream.codecpar().codec_type();
                if !codec_type.is_video() && !codec_type.is_audio() && !codec_type.is_subtitle() {
                    continue;
                }
                let mut new_stream = ofmt_ctx.new_stream();
                new_stream.set_codecpar(stream.codecpar().clone());
                map.insert(stream.index, new_stream.index);
            }
            stream_maps.push((ifmt_ctx, map));
        }
        {
            let ifmt_ctx = self.1.ifmt_ctx()?;
            let mut map = HashMap::new();
            for stream in ifmt_ctx.streams() {
                let codec_type = stream.codecpar().codec_type();
                if !codec_type.is_video() && !codec_type.is_audio() && !codec_type.is_subtitle() {
                    continue;
                }
                let mut new_stream = ofmt_ctx.new_stream();
                new_stream.set_codecpar(stream.codecpar().clone());
                map.insert(stream.index, new_stream.index);
            }
            stream_maps.push((ifmt_ctx, map));
        }

        ofmt_ctx.write_header(&mut None)?;

        let ofmt_ctx = Arc::new(Mutex::new(ofmt_ctx));

        let jhs = stream_maps
            .into_iter()
            .map(|(mut ifmt_ctx, map)| {
                let ofmt_ctx = ofmt_ctx.clone();
                std::thread::spawn(move || {
                    loop {
                        match ifmt_ctx.read_packet() {
                            Ok(Some(mut packet)) => {
                                let Some(index) = map.get(&packet.stream_index) else {
                                    continue;
                                };
                                let old_ts =
                                    ifmt_ctx.streams()[packet.stream_index as usize].time_base;
                                let new_ts =
                                    ofmt_ctx.lock().unwrap().streams()[*index as usize].time_base;
                                packet.rescale_ts(old_ts, new_ts);
                                packet.set_stream_index(*index);
                                packet.set_pos(-1);
                                ofmt_ctx
                                    .lock()
                                    .unwrap()
                                    .interleaved_write_frame(&mut packet)?;
                            }
                            Ok(None) => break,
                            e => {
                                e?;
                            }
                        }
                    }
                    Ok::<_, RsmpegError>(())
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
            let codec_type = stream.codecpar().codec_type();
            if !codec_type.is_video() {
                continue;
            }

            let codec_id = stream.codecpar().codec_id;
            let decoder =
                AVCodec::find_decoder(codec_id).ok_or(AVMuxError::CodecNotFound(format!(
                    "Decoder id {codec_id} not found for video {}",
                    vfile.path_url
                )))?;
            let mut decoder = AVCodecContext::new(&decoder);
            decoder.apply_codecpar(&stream.codecpar())?;
            let time_base = stream.time_base;
            decoder.set_time_base(time_base);
            decoder.open(None)?;

            let codec_name = vconf
                .format
                .map(|format| match format {
                    VFormat::H264 => c"h264_nvenc",
                    VFormat::HEVC => c"hevc_nvenc",
                    VFormat::AV1 => c"av1_nvenc",
                })
                .unwrap_or(match stream.codecpar().codec_id {
                    AV_CODEC_ID_H264 => c"h264_nvenc",
                    AV_CODEC_ID_HEVC => c"hevc_nvenc",
                    AV_CODEC_ID_AV1 => c"av1_nvenc",
                    id => {
                        return Err(AVMuxError::CodecNotFound(format!(
                            "Encoder codec id {id} of video ({}) is not supported by NVIDIA\n
                            The default output codec is the same as the source video, please set output format in CodecConf",
                            vfile.path_url
                        )));
                    }
                });
            let encoder = AVCodec::find_encoder_by_name(codec_name).ok_or(
                AVMuxError::CodecNotFound(format!(
                    "Encoder {} not found for video {}",
                    codec_name.to_str().unwrap(),
                    vfile.path_url
                )),
            )?;
            let mut encoder = AVCodecContext::new(&encoder);
            let bitrate = vconf.bitrate.unwrap_or(stream.codecpar().bit_rate);
            encoder.set_bit_rate(bitrate);
            encoder.set_framerate(stream.codecpar().framerate);
            let (width, height) = vconf
                .size
                .unwrap_or((stream.codecpar().width, stream.codecpar().height));
            encoder.set_width(width);
            encoder.set_height(height);
            encoder.set_time_base(time_base);
            encoder.set_pix_fmt(AV_PIX_FMT_CUDA);
            encoder.set_gop_size(vconf.gop.unwrap_or(-1));

            let hw_device_ctx = AVHWDeviceContext::create(AV_HWDEVICE_TYPE_CUDA, None, None, 0)?;
            let mut hw_frames_ctx = hw_device_ctx.hwframe_ctx_alloc();
            hw_frames_ctx.data().format = AV_PIX_FMT_CUDA;
            hw_frames_ctx.data().sw_format = AV_PIX_FMT_YUV420P;
            hw_frames_ctx.data().width = encoder.width;
            hw_frames_ctx.data().height = encoder.height;
            hw_frames_ctx.data().initial_pool_size = 16;
            hw_frames_ctx.init()?;
            encoder.set_hw_frames_ctx(hw_frames_ctx);
            encoder.open(None)?;

            let mut v_frame = AVFrame::new();
            v_frame.set_width(encoder.width);
            v_frame.set_height(encoder.height);
            v_frame.set_format(AV_PIX_FMT_YUV420P);
            v_frame.set_time_base(time_base);
            v_frame.alloc_buffer()?;

            let mut new_stream = ofmt_ctx.new_stream();
            new_stream.set_codecpar(encoder.extract_codecpar());
            new_stream.set_time_base(time_base);

            fn recv_frame(decoder: &mut AVCodecContext, v_frame: &mut AVFrame) -> Result<()> {
                let frame = decoder.receive_frame()?;

                let mut sw_scaler = SwsContext::get_context(
                    frame.width,
                    frame.height,
                    frame.format,
                    v_frame.width,
                    v_frame.height,
                    v_frame.format,
                    SWS_FAST_BILINEAR,
                    None,
                    None,
                    None,
                )
                .ok_or(AVMuxError::Invalid)?;
                sw_scaler.scale_frame(&frame, 0, frame.height, v_frame)?;
                v_frame.set_pts(frame.pts);

                Ok(())
            }

            let send_frame =
                move |encoder: &mut AVCodecContext, frame: Option<&AVFrame>| -> Result<()> {
                    let Some(frame) = frame else {
                        encoder.send_frame(None)?;
                        return Ok(());
                    };

                    let mut hw_v_frame = AVFrame::new();
                    encoder
                        .hw_frames_ctx_mut()
                        .expect("Encoder should have hw ctx")
                        .get_buffer(&mut hw_v_frame)?;
                    hw_v_frame.hwframe_transfer_data(frame)?;
                    hw_v_frame.set_time_base(time_base);
                    hw_v_frame.set_pts(frame.pts);
                    encoder.send_frame(Some(&hw_v_frame))?;
                    Ok(())
                };

            let index = new_stream.index;
            let recv_packets = move |encoder: &mut AVCodecContext,
                                     ofmt_ctx: &mut AVFormatContextOutput|
                  -> Result<()> {
                loop {
                    let mut packet = match encoder.receive_packet() {
                        Err(RsmpegError::EncoderDrainError | RsmpegError::EncoderFlushedError) => {
                            break;
                        }
                        res => res?,
                    };
                    packet.set_stream_index(index);
                    ofmt_ctx.interleaved_write_frame(&mut packet)?;
                }
                Ok(())
            };

            let convert = move |packet: Option<&AVPacket>,
                                ofmt_ctx: &mut AVFormatContextOutput|
                  -> Result<()> {
                decoder.send_packet(packet)?;
                loop {
                    match recv_frame(&mut decoder, &mut v_frame) {
                        Err(AVMuxError::Rsmpeg(
                            RsmpegError::DecoderDrainError | RsmpegError::DecoderFlushedError,
                        )) => break,
                        res => res?,
                    }
                    recv_packets(&mut encoder, ofmt_ctx)?;
                    send_frame(&mut encoder, Some(&v_frame))?;
                }
                if packet.is_none() {
                    recv_packets(&mut encoder, ofmt_ctx)?;
                    send_frame(&mut encoder, None)?;
                    recv_packets(&mut encoder, ofmt_ctx)?;
                }
                Ok(())
            };

            v_map.insert(stream.index, convert);
        }
        let mut a_ifmt_ctx = afile.ifmt_ctx()?;
        let mut a_map = HashMap::new();
        for stream in a_ifmt_ctx.streams() {
            let codec_type = stream.codecpar().codec_type();
            if !codec_type.is_audio() {
                continue;
            }

            let mut codec_id = stream.codecpar().codec_id;
            let decoder =
                AVCodec::find_decoder(codec_id).ok_or(AVMuxError::CodecNotFound(format!(
                    "Decoder id {codec_id} not found for audio {}",
                    afile.path_url
                )))?;
            let mut decoder = AVCodecContext::new(&decoder);
            decoder.apply_codecpar(&stream.codecpar())?;
            let time_base = stream.time_base;
            decoder.set_time_base(time_base);
            decoder.open(None)?;

            if let Some(format) = aconf.format {
                match format {
                    AFormat::AAC => codec_id = AV_CODEC_ID_AAC,
                    AFormat::FLAC => codec_id = AV_CODEC_ID_FLAC,
                }
            }
            let encoder =
                AVCodec::find_encoder(codec_id).ok_or(AVMuxError::CodecNotFound(format!(
                    "Encoder id {codec_id} not found for audio {}",
                    afile.path_url
                )))?;
            let sample_fmt = encoder.sample_fmts().unwrap()[0];
            let mut encoder = AVCodecContext::new(&encoder);
            let sample_rate = aconf.sample_rate.unwrap_or(stream.codecpar().sample_rate);
            encoder.set_sample_rate(sample_rate);
            encoder.set_sample_fmt(sample_fmt);
            let ch_layout = aconf
                .nb_channels
                .map_or(stream.codecpar().ch_layout().clone(), |num| {
                    AVChannelLayout::from_nb_channels(num)
                });
            encoder.set_ch_layout(*ch_layout);
            encoder.set_time_base(time_base);
            encoder.open(None)?;

            let mut a_fifo = AVAudioFifo::new(sample_fmt, ch_layout.nb_channels, 1);

            let mut new_stream = ofmt_ctx.new_stream();
            new_stream.set_codecpar(encoder.extract_codecpar());
            new_stream.set_time_base(time_base);

            let recv_frame = {
                let ch_layout = ch_layout.clone();
                move |decoder: &mut AVCodecContext, a_fifo: &mut AVAudioFifo| -> Result<()> {
                    let frame = decoder.receive_frame()?;

                    let mut sw_resampler = SwrContext::new(
                        &ch_layout,
                        sample_fmt,
                        sample_rate,
                        &frame.ch_layout,
                        frame.format,
                        frame.sample_rate,
                    )
                    .map_err(|_| AVMuxError::Invalid)?;
                    sw_resampler.init()?;

                    let mut a_frame = AVFrame::new();
                    a_frame.set_ch_layout(*ch_layout);
                    a_frame.set_sample_rate(sample_rate);
                    a_frame.set_nb_samples(frame.nb_samples);
                    a_frame.set_format(sample_fmt);
                    a_frame.get_buffer(0)?;

                    sw_resampler.convert_frame(Some(&frame), &mut a_frame)?;

                    unsafe {
                        a_fifo.write(a_frame.data_mut().as_mut_ptr(), a_frame.nb_samples)?;
                    }

                    Ok(())
                }
            };

            let index = new_stream.index;
            let recv_packets = move |encoder: &mut AVCodecContext,
                                     ofmt_ctx: &mut AVFormatContextOutput|
                  -> Result<()> {
                loop {
                    let mut packet = match encoder.receive_packet() {
                        Err(RsmpegError::EncoderDrainError | RsmpegError::EncoderFlushedError) => {
                            break;
                        }
                        res => res?,
                    };
                    packet.set_stream_index(index);
                    ofmt_ctx.interleaved_write_frame(&mut packet)?;
                }
                Ok(())
            };

            let mut a_pts = 0;
            let convert = move |packet: Option<&AVPacket>,
                                ofmt_ctx: &mut AVFormatContextOutput|
                  -> Result<()> {
                decoder.send_packet(packet)?;
                loop {
                    match recv_frame(&mut decoder, &mut a_fifo) {
                        Err(AVMuxError::Rsmpeg(
                            RsmpegError::DecoderDrainError | RsmpegError::DecoderFlushedError,
                        )) => break,
                        res => res?,
                    }
                    while a_fifo.size() >= encoder.frame_size {
                        let mut a_frame = AVFrame::new();
                        a_frame.set_ch_layout(*ch_layout);
                        a_frame.set_sample_rate(sample_rate);
                        a_frame.set_format(sample_fmt);
                        a_frame.set_nb_samples(encoder.frame_size);
                        a_frame.get_buffer(0)?;
                        a_frame.set_time_base(time_base);
                        a_frame.set_pts(a_pts);
                        a_pts += a_frame.nb_samples as i64;
                        unsafe {
                            a_fifo.read(a_frame.data_mut().as_mut_ptr(), a_frame.nb_samples)?;
                        }
                        recv_packets(&mut encoder, ofmt_ctx)?;
                        encoder.send_frame(Some(&a_frame))?;
                    }
                }
                if packet.is_none() {
                    if a_fifo.size() > 0 {
                        let mut a_frame = AVFrame::new();
                        a_frame.set_ch_layout(*ch_layout);
                        a_frame.set_sample_rate(sample_rate);
                        a_frame.set_format(sample_fmt);
                        a_frame.set_nb_samples(a_fifo.size());
                        a_frame.get_buffer(0).unwrap();
                        a_frame.set_time_base(time_base);
                        a_frame.set_pts(a_pts);
                        a_pts += a_frame.nb_samples as i64;
                        unsafe {
                            a_fifo.read(a_frame.data_mut().as_mut_ptr(), a_frame.nb_samples)?;
                        }
                        recv_packets(&mut encoder, ofmt_ctx)?;
                        encoder.send_frame(Some(&a_frame))?;
                    }
                    recv_packets(&mut encoder, ofmt_ctx)?;
                    encoder.send_frame(None)?;
                    recv_packets(&mut encoder, ofmt_ctx)?;
                }
                Ok(())
            };

            a_map.insert(stream.index, convert);
        }

        ofmt_ctx.write_header(&mut None)?;

        let ofmt_ctx = Arc::new(Mutex::new(ofmt_ctx));

        let v_jh = std::thread::spawn({
            let ofmt_ctx = ofmt_ctx.clone();
            move || -> Result<()> {
                while let Some(packet) = v_ifmt_ctx.read_packet()? {
                    let Some(convert) = v_map.get_mut(&packet.stream_index) else {
                        continue;
                    };
                    convert(Some(&packet), &mut ofmt_ctx.lock().unwrap())?;
                }
                for (_, mut convert) in v_map.drain() {
                    convert(None, &mut ofmt_ctx.lock().unwrap())?;
                }
                Ok(())
            }
        });

        let a_jh = std::thread::spawn({
            let ofmt_ctx = ofmt_ctx.clone();

            move || -> Result<()> {
                while let Some(packet) = a_ifmt_ctx.read_packet()? {
                    let Some(convert) = a_map.get_mut(&packet.stream_index) else {
                        continue;
                    };
                    convert(Some(&packet), &mut ofmt_ctx.lock().unwrap())?;
                }
                for (_, mut convert) in a_map.drain() {
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
