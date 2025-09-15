//! Error types

use thiserror::Error;

#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum AVMuxError {
    #[error("{0}")]
    Rsmpeg(#[from] rsmpeg::error::RsmpegError),
    #[error("{0}")]
    Ffi(#[from] std::ffi::NulError),
    #[error("{0}")]
    CodecNotFound(String),
    #[error("Source media parameters invalid")]
    Invalid,
}
