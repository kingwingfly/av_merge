//! A crate to merge video and audio based on rsmpeg (dynamic link with ffmpeg lib).
//!
//! More information can be found in the [`document`].
#![deny(
    missing_docs,
    rustdoc::broken_intra_doc_links,
    elided_lifetimes_in_paths
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod codec;
pub mod document;
pub mod error;
pub mod file;
pub mod mux;

pub use codec::{AConf, AFormat, CodecConfig, VConf, VFormat};
pub use file::{AFile, AVFile, VFile};
pub use mux::Mux;

#[allow(missing_docs)]
pub type Result<T> = std::result::Result<T, error::AVMuxError>;

/// silent ffmpeg logs
pub fn silent_log() {
    unsafe {
        rsmpeg::ffi::av_log_set_callback(None);
    }
}

/// resume ffmpeg logs
pub fn resume_log() {
    unsafe {
        rsmpeg::ffi::av_log_set_callback(Some(rsmpeg::ffi::av_log_default_callback));
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_av_file_mux() {
        let file1 = AFile::new("testa.mp3");
        let file2 = VFile::new("testv.mp4");
        let output = VFile::new("output1.mp4");
        (file1, file2).mux(output, CodecConfig::default()).unwrap();

        let file1 = AFile::new("testa.mp3");
        let file2 = VFile::new("testv.mp4");
        let output = VFile::new("output2.mp4");
        (file1, file2)
            .mux(
                output,
                CodecConfig::builder()
                    .vconf(VConf::builder().format(VFormat::H264).build())
                    .build(),
            )
            .unwrap();
    }
}
