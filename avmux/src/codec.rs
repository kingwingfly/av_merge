//! Codec

use bon::Builder;

/// Codec Config
#[derive(Debug, Default, Builder)]
pub struct CodecConfig {
    #[builder(default)]
    pub(crate) vconf: VConf,
    #[builder(default)]
    pub(crate) aconf: AConf,
}

/// Video Codec Config
#[derive(Debug, Default, Builder)]
pub struct VConf {
    /// output bitrate, the same as input if None
    pub(crate) bitrate: Option<i64>,
    /// output (width, height), the same as input if None
    ///
    /// # Notice
    /// the `width:height` should equel to the source `width:height`,
    /// or the output will be buggy since no padding is done.
    pub(crate) size: Option<(i32, i32)>,
    /// output gop, the default value is decided by FFmpeg, depending on the encoder.
    pub(crate) gop: Option<i32>,
    /// output video format
    pub(crate) format: Option<VFormat>,
}

/// Audio Codec Config
#[derive(Debug, Default, Builder)]
pub struct AConf {
    /// output number of channels, the same as input if None
    pub(crate) nb_channels: Option<i32>,
    /// output sample rate, the same as input if None
    pub(crate) sample_rate: Option<i32>,
    /// output audio format
    pub(crate) format: Option<AFormat>,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VFormat {
    H264,
    HEVC,
    AV1,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AFormat {
    AAC,
    FLAC,
}
