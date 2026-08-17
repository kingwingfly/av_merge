//! Error types

use thiserror::Error;

#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum AVMuxError {
    #[error("{0}")]
    Ffmpeg(#[from] ffmpeg_next::Error),
    #[error("{0}")]
    Ffi(#[from] std::ffi::NulError),
    #[error("{0}")]
    CodecNotFound(String),
    #[error("Source media parameters invalid")]
    Invalid,
}
