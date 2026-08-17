# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

<!--
Note: In this file, do not use the hard wrap in the middle of a sentence for compatibility with GitHub comment style markdown rendering.
-->

## [Unreleased]
### [0.3.0] - 2026-08-17

- Replace `rsmpeg` with `ffmpeg-next` 9.0, which supports FFmpeg 9.0. `rsmpeg` 0.18 does not build against FFmpeg 9.
- **Breaking**: the `ffmpeg8`/`ffmpeg7_1`/`ffmpeg7`/`ffmpeg6`/`link_system_ffmpeg`/`link_vcpkg_ffmpeg` features are gone. `ffmpeg-next` detects the installed FFmpeg version and linking mode itself, so nothing has to be selected. New passthrough features `build` and `static` replace them.
- **Breaking**: `AVMuxError::Rsmpeg` is now `AVMuxError::Ffmpeg`, wrapping `ffmpeg_next::Error`.
- **Breaking**: `AVFile::ifmt_ctx`/`ofmt_ctx` now return `ffmpeg_next::format::context::Input`/`Output`.
- Fix: the audio encoder and output stream now count time in samples (`1/sample_rate`) instead of reusing the demuxer's time base, which the sample-counting `pts` do not match. Muxing a 3-second MP3 (demuxer time base `1/14112000`) previously produced an audio stream reporting 11.65 seconds; it now reports 3.000.

### [0.0.1] - 2025-XX-XX

- MVP
