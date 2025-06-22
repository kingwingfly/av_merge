//! Error handling module for the library.

use rsmpeg::error::RsmpegError;
use std::{ffi::NulError, ops::Deref};

type AVMuxErrorInner = terrors::OneOf<(RsmpegError, NulError)>;

/// Type alias for errors that can occur in the library.
#[derive(Debug)]
pub struct AVMuxError(AVMuxErrorInner);

impl Deref for AVMuxError {
    type Target = AVMuxErrorInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<RsmpegError> for AVMuxError {
    fn from(err: RsmpegError) -> Self {
        Self(terrors::OneOf::new(err))
    }
}

impl From<NulError> for AVMuxError {
    fn from(err: NulError) -> Self {
        Self(terrors::OneOf::new(err))
    }
}
