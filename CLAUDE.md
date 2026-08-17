# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A Cargo workspace whose single member is the `avmux` library crate (`avmux/`, edition 2024). It muxes one video source and one audio source into a single output file via [ffmpeg-next](https://crates.io/crates/ffmpeg-next) (FFmpeg bindings, dynamically linked against system FFmpeg).

## Commands

Everything runs from the workspace root.

```sh
cargo build
cargo clippy --all-targets
cargo fmt
cargo doc --open
```

Verification that needs **no GPU and no media fixtures**:

```sh
cargo build
cargo clippy --all-targets
cargo test --doc        # README example is `no_run`: compiled, not executed
```

The one integration test **does** run FFmpeg end to end:

```sh
cargo test test_av_file_mux
```

It requires an NVIDIA GPU plus an nvenc-enabled FFmpeg, and it reads `testa.mp3` / `testv.mp4` from the package root — i.e. the nested `avmux/avmux/` directory, since `cargo test` runs with CWD at the package root and the test uses bare relative paths. Those fixtures are gitignored (`*.mp3`, `*.mp4`) and are not in the repo, so a fresh checkout cannot run this test. A failure here usually means a missing fixture or missing nvenc — check both before changing code.

### FFmpeg version and linking

There is nothing to select. `ffmpeg-sys-next`'s build script probes the installed
libraries (pkg-config, or vcpkg on MSVC) and emits `ffmpeg_X_Y` cfgs itself, which
`ffmpeg-next` compiles against. The crate is developed against FFmpeg 9.0.

Two passthrough features exist for unusual setups: `build` (compile FFmpeg from
source) and `static` (link statically). Neither is on by default.

## Architecture

Public surface is re-exported from `lib.rs`: `Mux`, `VFile`/`AFile`/`AVFile`, and the `CodecConfig`/`VConf`/`AConf` builders.

**Inputs are typed by role, not content.** `VFile` and `AFile` are both just a URL string; the type says which stream role that file contributes. `Mux` is implemented on the tuple `(VFile, AFile)`; the `(AFile, VFile)` impl merely swaps and delegates, so all real logic lives in one place in `mux.rs`.

**Two very different paths:**

- `simple_mux` — stream copy, no decode. Carries video, audio *and* subtitle streams straight through with `rescale_ts`.
- `mux` — full decode → convert → re-encode. Handles video and audio only; subtitles are dropped.

**The closure-per-stream pipeline** is the central idea in `mux()`. For each input stream it builds a `convert` closure that owns everything that stream needs — its decoder, encoder, and (for audio) the `AudioFifo` — and stores it in a `HashMap<stream_index, closure>`. The read loop then just dispatches each packet to `convert(Some(&packet), ofmt_ctx)` by index. Calling `convert(None, …)` is the flush signal: it drains the decoder, flushes the encoder, and writes the trailing packets. Adding a codec or stream kind means fitting into this closure shape.

Scalers and resamplers are deliberately built *per frame* rather than stored: `software::scaling::Context` is not `Send`, and these closures are moved onto worker threads.

**Phase ordering is a hard constraint.** All output streams must be created on `ofmt_ctx` and all encoders opened *before* `write_header`, which must happen before any worker thread spawns. New stream types have to be set up in that same setup phase.

**Threading.** Both entry points spawn one thread per *input file* (video, audio) sharing an `Arc<Mutex<format::context::Output>>`; the mutex serializes `write_interleaved`, which is what keeps the interleaving correct. Threads are joined before `write_trailer`.

**Audio re-encode buffers through a FIFO.** Decoded frames are resampled into an `AVAudioFifo`, then drained in exact `encoder.frame_size` chunks with a manually maintained `a_pts` counter — encoders reject partial frames, so this indirection is required, not incidental. `ffmpeg-next` has no binding for `AVAudioFifo`, so `src/fifo.rs` (a private module) wraps the raw FFI. Because `a_pts` counts samples, the audio encoder and output stream are given a `1/sample_rate` time base rather than the demuxer's — reusing the demuxer's (as 0.2.x did) makes the output audio report a wrong duration.

**Where the safe API runs out.** `ffmpeg-next` exposes no hardware device/frames contexts, no audio FIFO, and no setter for `AVFrame::time_base`, `AVCodecContext::gop_size` as `-1`, or `AVCodecContext::framerate`. Those go through `ffmpeg_next::ffi` (a re-export of `ffmpeg-sys-next`) in `mux.rs` and `fifo.rs`. `ChannelLayout` also holds raw pointers and is not `Send`, hence the `SendChannelLayout` wrapper — reach its inner value through `.layout()`, since edition-2021 capture rules would otherwise make a closure capture the non-`Send` field directly.

### NVENC is hard-coded

The video encode path in `mux()` always selects `h264_nvenc` / `hevc_nvenc` / `av1_nvenc`, with `AV_PIX_FMT_CUDA` and a CUDA hwframes context. Two consequences:

- `CodecConfig::default()` means "match the input codec", so a source that is not H264/HEVC/AV1 fails with `CodecNotFound`. The caller must set `VConf.format` explicitly in that case.
- nvenc is non-free under FFmpeg's license; the README warns that an nvenc-enabled FFmpeg build cannot be redistributed commercially. Keep that caveat in mind for anything touching encoder selection.

`VFormat` (H264/HEVC/AV1) and `AFormat` (AAC/FLAC) are the only selectable output formats.

## Conventions

- `lib.rs` has `#![deny(missing_docs, rustdoc::broken_intra_doc_links, elided_lifetimes_in_paths)]`. A new public item without a doc comment **fails the build**.
- Config struct fields (`VConf`, `AConf`, `CodecConfig`) are `pub(crate)`, so the `bon`-generated builders are the only way external callers can set them. Adding an option means adding a field and documenting it there.
- `document.rs` contains no logic — it is a single `#![doc = include_str!("../README.md")]` that renders the README into rustdoc. The README's usage block is fenced ` ```rust no_run `, so **editing that example can break `cargo test`**; it must stay compilable against the current API.
- Errors funnel through `AVMuxError` in `error.rs` and the crate-local `Result<T>` alias.
- `silent_log()` / `resume_log()` in `lib.rs` toggle FFmpeg's global log callback via raw FFI.
