//! Media file

use std::ffi::CString;
use std::sync::Once;

use ffmpeg_next::format::{self, context};

use crate::Result;

/// `ffmpeg-next` requires a one-off global init (error tables, network stack).
/// Doing it lazily here keeps it invisible to callers.
fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = ffmpeg_next::init();
    });
}

/// AV file trait.
pub trait AVFile {
    /// The file url.
    fn path_url(&self) -> &str;

    /// The c-style file url.
    fn c_path_url(&self) -> Result<CString> {
        CString::new(self.path_url()).map_err(Into::into)
    }

    /// Open the file as format context input.
    fn ifmt_ctx(&self) -> Result<context::Input> {
        init();
        // `ffmpeg-next` panics on an interior NUL; reject it as an error first.
        self.c_path_url()?;
        format::input(self.path_url()).map_err(Into::into)
    }

    /// Open the file as format context output.
    fn ofmt_ctx(&self) -> Result<context::Output> {
        init();
        self.c_path_url()?;
        format::output(self.path_url()).map_err(Into::into)
    }
}

/// Represents an input file with a URL, which provide video stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VFile {
    /// The URL of the file.
    pub path_url: String,
}

impl VFile {
    /// Creates a new `Video File` from a url or path.
    pub fn new(path_url: impl AsRef<str>) -> Self {
        Self {
            path_url: path_url.as_ref().to_owned(),
        }
    }
}

impl AVFile for VFile {
    fn path_url(&self) -> &str {
        self.path_url.as_str()
    }
}

/// Represents an input file with a URL, which provide audio stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AFile {
    /// The URL of the file.
    pub path_url: String,
}

impl AFile {
    /// Creates a new `Audio File` from a url or path.
    pub fn new(path_url: impl AsRef<str>) -> Self {
        Self {
            path_url: path_url.as_ref().to_owned(),
        }
    }
}

impl AVFile for AFile {
    fn path_url(&self) -> &str {
        self.path_url.as_str()
    }
}
