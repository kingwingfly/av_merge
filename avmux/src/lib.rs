//! A crate to merge video and audio based on rsmpeg (dynamic link with ffmpeg lib).
//!
//! More information can be found in the [`document`].
#![deny(
    missing_docs,
    rustdoc::broken_intra_doc_links,
    elided_lifetimes_in_paths
)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod document;
pub mod error;

use std::collections::HashMap;
use std::ffi::{CString, NulError};

use error::AVMuxError;
use rsmpeg::avformat::{AVFormatContextInput, AVFormatContextOutput};

/// Represents an input file with a URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AVFile {
    /// The URL of the file.
    pub path_url: String,
}

impl AVFile {
    /// Creates a new `File` from a url or path.
    pub fn new(path_url: impl AsRef<str>) -> Self {
        Self {
            path_url: path_url.as_ref().to_owned(),
        }
    }

    fn url_path(&self) -> Result<CString, NulError> {
        CString::new(self.path_url.as_str())
    }

    /// Open the file as format context input.
    fn ifmt_ctx(&self) -> Result<AVFormatContextInput, AVMuxError> {
        let c_path = self.url_path()?;
        AVFormatContextInput::open(c_path.as_c_str(), None, &mut None).map_err(Into::into)
    }

    /// Open the file as format context output.
    fn ofmt_ctx(&self) -> Result<AVFormatContextOutput, AVMuxError> {
        AVFormatContextOutput::create(&CString::new(self.path_url.as_str())?, None)
            .map_err(Into::into)
    }
}

/// Trait for merging multiple media files into one.
pub trait Mux {
    /// Merges multiple media files into a single output file.
    fn mux(self, output: AVFile) -> Result<(), AVMuxError>;
}

impl<FS> Mux for FS
where
    FS: IntoIterator<Item = AVFile>,
{
    fn mux(self, output: AVFile) -> Result<(), AVMuxError> {
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
        ofmt_ctx.write_header(&mut None)?;
        for (mut ifmt_ctx, map) in stream_maps.into_iter() {
            loop {
                match ifmt_ctx.read_packet() {
                    Ok(Some(mut packet)) => {
                        let Some(index) = map.get(&packet.stream_index) else {
                            continue;
                        };
                        let old_ts = ifmt_ctx.streams()[packet.stream_index as usize].time_base;
                        let new_ts = ofmt_ctx.streams()[*index as usize].time_base;
                        packet.rescale_ts(old_ts, new_ts);
                        packet.set_stream_index(*index);
                        packet.set_pos(-1);
                        ofmt_ctx.interleaved_write_frame(&mut packet)?;
                    }
                    Ok(None) => break,
                    e => {
                        e?;
                    }
                }
            }
        }
        ofmt_ctx.write_trailer()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_av_file_mux() {
        let file1 = AVFile::new("testa.mp3");
        let file2 = AVFile::new("testv.mp4");
        let output = AVFile::new("output.mp4");
        let files = vec![file1, file2];
        files.mux(output).unwrap();
    }
}
