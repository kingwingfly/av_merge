//! Media file

use std::ffi::CString;

use rsmpeg::avformat::{AVFormatContextInput, AVFormatContextOutput};

use crate::Result;

/// AV file trait.
pub trait AVFile {
    /// The file url.
    fn path_url(&self) -> &str;

    /// The c-style file url.
    fn c_path_url(&self) -> Result<CString> {
        CString::new(self.path_url()).map_err(Into::into)
    }

    /// Open the file as format context input.
    fn ifmt_ctx(&self) -> Result<AVFormatContextInput> {
        AVFormatContextInput::open(self.c_path_url()?.as_c_str()).map_err(Into::into)
    }

    /// Open the file as format context output.
    fn ofmt_ctx(&self) -> Result<AVFormatContextOutput> {
        AVFormatContextOutput::create(self.c_path_url()?.as_c_str()).map_err(Into::into)
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
