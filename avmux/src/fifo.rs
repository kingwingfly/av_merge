//! Owned wrapper around FFmpeg's `AVAudioFifo`.
//!
//! `ffmpeg-next` has no safe binding for the audio FIFO, so the audio
//! re-encode path in [`crate::mux`] drives it through raw FFI.

use ffmpeg_next::{
    ffi::{
        AVAudioFifo, av_audio_fifo_alloc, av_audio_fifo_free, av_audio_fifo_read,
        av_audio_fifo_size, av_audio_fifo_write,
    },
    format::Sample,
    frame,
};

use crate::{Result, error::AVMuxError};

/// A growable FIFO of decoded audio samples, used to re-chunk decoded frames
/// into the exact `frame_size` the encoder demands.
pub(crate) struct AudioFifo {
    ptr: *mut AVAudioFifo,
}

// SAFETY: the FIFO owns its buffer and is only ever reachable through `&self` /
// `&mut self`, so moving it to another thread cannot alias it.
unsafe impl Send for AudioFifo {}

impl AudioFifo {
    pub(crate) fn new(format: Sample, nb_channels: i32) -> Result<Self> {
        let ptr = unsafe { av_audio_fifo_alloc(format.into(), nb_channels, 1) };
        if ptr.is_null() {
            return Err(AVMuxError::Invalid);
        }
        Ok(Self { ptr })
    }

    /// Number of samples currently buffered.
    pub(crate) fn size(&self) -> i32 {
        unsafe { av_audio_fifo_size(self.ptr) }
    }

    /// Append every sample of `frame`; the FIFO grows as needed.
    pub(crate) fn write(&mut self, frame: &mut frame::Audio) -> Result<()> {
        let nb_samples = frame.samples() as i32;
        let written = unsafe {
            av_audio_fifo_write(
                self.ptr,
                (*frame.as_mut_ptr()).data.as_mut_ptr().cast(),
                nb_samples,
            )
        };
        match written {
            e if e < 0 => Err(ffmpeg_next::Error::from(e).into()),
            n if n < nb_samples => Err(AVMuxError::Invalid),
            _ => Ok(()),
        }
    }

    /// Fill `frame` with exactly `frame.samples()` buffered samples.
    pub(crate) fn read(&mut self, frame: &mut frame::Audio) -> Result<()> {
        let nb_samples = frame.samples() as i32;
        let read = unsafe {
            av_audio_fifo_read(
                self.ptr,
                (*frame.as_mut_ptr()).data.as_mut_ptr().cast(),
                nb_samples,
            )
        };
        match read {
            e if e < 0 => Err(ffmpeg_next::Error::from(e).into()),
            n if n < nb_samples => Err(AVMuxError::Invalid),
            _ => Ok(()),
        }
    }
}

impl Drop for AudioFifo {
    fn drop(&mut self) {
        unsafe { av_audio_fifo_free(self.ptr) }
    }
}
