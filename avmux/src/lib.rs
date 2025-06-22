//! A library for merging multiple media files into one using the ffmpeg library.
//!
//! More information can be found in the [`document`].
#![deny(
    missing_docs,
    rustdoc::broken_intra_doc_links,
    elided_lifetimes_in_paths
)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod document;

use std::collections::HashMap;
use std::ffi::{CString, NulError};
use std::path::Path;

use rsmpeg::{
    avformat::{AVFormatContextInput, AVFormatContextOutput},
    error::RsmpegError,
};
use terrors::OneOf;
use url::Url;

/// Represents a file with a URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AVFile {
    /// The URL of the file.
    pub url: Url,
}

impl AVFile {
    /// Creates a new `File` from a URL.
    pub fn new(url: impl AsRef<str>) -> Result<Self, url::ParseError> {
        let url = Url::parse(url.as_ref())?;
        Ok(Self { url })
    }

    /// Creates a new `File` from a file path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        if !path.as_ref().exists() {
            return Err(std::io::ErrorKind::NotFound)?;
        }
        let url = Url::from_file_path(path).map_err(|_| std::io::ErrorKind::InvalidInput)?;
        Ok(Self { url })
    }

    fn url_path(&self) -> Result<CString, NulError> {
        CString::new(self.url.as_str())
    }

    /// Open the file as format context input.
    fn ifmt_ctx(&self) -> Result<AVFormatContextInput, OneOf<(RsmpegError, NulError)>> {
        let c_path = self.url_path().map_err(OneOf::new)?;
        AVFormatContextInput::open(c_path.as_c_str(), None, &mut None).map_err(OneOf::new)
    }

    /// Open the file as format context output.
    fn ofmt_ctx(&self) -> Result<AVFormatContextOutput, OneOf<(RsmpegError, NulError)>> {
        let c_path = self.url_path().map_err(OneOf::new)?;
        AVFormatContextOutput::create(c_path.as_c_str(), None).map_err(OneOf::new)
    }
}

/// Trait for merging multiple media files into one.
pub trait Mux {
    /// Merges multiple media files into a single output file.
    fn mux(self, output: AVFile) -> Result<(), OneOf<(RsmpegError, NulError)>>;
}

impl<FS> Mux for FS
where
    FS: IntoIterator<Item = AVFile>,
{
    fn mux(self, output: AVFile) -> Result<(), OneOf<(RsmpegError, NulError)>> {
        let mut ofmt_ctx = output.ofmt_ctx()?;
        let mut stream_maps = vec![];
        for file in self.into_iter() {
            let ifmt_ctx = file.ifmt_ctx()?;
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
        ofmt_ctx.write_header(&mut None).map_err(OneOf::new)?;
        for (mut ifmt_ctx, map) in stream_maps.into_iter() {
            loop {
                match ifmt_ctx.read_packet() {
                    Ok(Some(mut packet)) => {
                        let Some(index) = map.get(&packet.stream_index) else {
                            continue;
                        };
                        packet.set_stream_index(*index);
                        packet.set_pos(-1);
                        ofmt_ctx
                            .interleaved_write_frame(&mut packet)
                            .map_err(OneOf::new)?;
                    }
                    Ok(None) => break,
                    e => {
                        e.map_err(OneOf::new)?;
                    }
                }
            }
        }
        ofmt_ctx.write_trailer().map_err(OneOf::new)?;
        Ok(())
    }
}
