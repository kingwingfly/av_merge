<a name="readme-top"></a>

<!--
*** This README is modified from https://github.com/othneildrew/Best-README-Template
-->

<!-- PROJECT SHIELDS -->
[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]


<!-- PROJECT LOGO -->
<br />
<div align="center">
<h3 align="center">AV Merge</h3>
  <p align="center">
    A crate to merge video and audio based on ffmpeg-next (dynamic link with ffmpeg lib)
    <br />
    <a href="https://github.com/kingwingfly/avmux"><strong>Explore the docs »</strong></a>
    <br />
    <br />
    <a href="https://github.com/kingwingfly/avmux">View Demo</a>
    ·
    <a href="https://github.com/kingwingfly/avmux/issues/new?labels=bug&template=bug-report---.md">Report Bug</a>
    ·
    <a href="https://github.com/kingwingfly/avmux/issues/new?labels=enhancement&template=feature-request---.md">Request Feature</a>
  </p>
</div>



<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
    <li><a href="#acknowledgments">Acknowledgments</a></li>
  </ol>
</details>



<!-- ABOUT THE PROJECT -->
## About The Project

This crate provides a simple way to merge video and audio files into a single output file. It uses the [ffmpeg-next](https://crates.io/crates/ffmpeg-next) library, which is a Rust binding for FFmpeg, to handle the underlying media processing.

<p align="right">(<a href="#readme-top">back to top</a>)</p>


### Built With

* [![Rust][Rust]][Rust-url]
* [![ffmpeg-next][ffmpeg-next]][ffmpeg-next-url]

<p align="right">(<a href="#readme-top">back to top</a>)</p>


<!-- GETTING STARTED -->
## Getting Started

### Prerequisites

* Install FFmpeg (9.0 is what this crate is developed against) and its development libraries
* install pkg-config

See an [example GitHub workflow](https://github.com/kingwingfly/fav/blob/dev/.github/workflows/release.yaml).

### Importing the Crate

To use this crate in your Rust project, add the following to your `Cargo.toml`:

```toml
[dependencies]
# 0.3 is built on `ffmpeg-next` and supports FFmpeg 9
avmux = { version = "0.3" }
# 0.2 is built on `rsmpeg` and supports FFmpeg <= 8
avmux = { version = "0.2" }
# if you just want to mux audio and video without re-encode, try version 0.1
avmux = { version = "0.1" }
```

NOTICE: versions `0.2` and `0.3` hard-code `nvenc` hwaccel as the encoder, which is non-free in FFmpeg's LICENSE.
(You **cannot** distribute the FFmpeg binary or library with nvenc enabled for commertial usage. If you need, feel free to fork and modify this crate.)

### Features

No feature has to be selected to pick an FFmpeg version or a linking mode: `ffmpeg-next`
probes the installed libraries itself (pkg-config, or vcpkg on MSVC).

- **build**: build FFmpeg from source instead of linking the system libraries
- **static**: link FFmpeg statically

<p align="right">(<a href="#readme-top">back to top</a>)</p>


<!-- USAGE EXAMPLES -->
## Usage

```rust no_run
use avmux::*;

let video_file = VFile::new("input_video.mp4");
let audio_file = AFile::new("https://music.com/input_audio.mp3");
let output_file = VFile::new("output1.mp4");
(video_file, audio_file).mux(output_file, CodecConfig::default()).unwrap();

let file1 = AFile::new("testa.mp3");
let file2 = VFile::new("testv.mp4");
let output = VFile::new("output2.mp4");
(file1, file2)
    .mux(
        output,
        CodecConfig::builder()
            .vconf(VConf::builder().format(VFormat::H264).build())
            .aconf(AConf::builder().format(AFormat::AAC).build())
            .build(),
    )
    .unwrap();
```

_For more examples, please refer to the [Documentation](https://crates.io/avmux)_

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- ROADMAP -->
## Roadmap

- [ ] Feature

See the [open issues](https://github.com/kingwingfly/avmux/issues) for a full list of proposed features (and known issues).

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- CONTRIBUTING -->
## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE.txt` for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- CONTACT -->
## Contact

Louis - 836250617@qq.com

Project Link: [https://github.com/kingwingfly/avmux](https://github.com/kingwingfly/avmux)

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- ACKNOWLEDGMENTS -->
## Acknowledgments

* []()

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[contributors-shield]: https://img.shields.io/github/contributors/kingwingfly/avmux.svg?style=for-the-badge
[contributors-url]: https://github.com/kingwingfly/avmux/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/kingwingfly/avmux.svg?style=for-the-badge
[forks-url]: https://github.com/kingwingfly/avmux/network/members
[stars-shield]: https://img.shields.io/github/stars/kingwingfly/avmux.svg?style=for-the-badge
[stars-url]: https://github.com/kingwingfly/avmux/stargazers
[issues-shield]: https://img.shields.io/github/issues/kingwingfly/avmux.svg?style=for-the-badge
[issues-url]: https://github.com/kingwingfly/avmux/issues
[license-shield]: https://img.shields.io/github/license/kingwingfly/avmux.svg?style=for-the-badge
[license-url]: https://github.com/kingwingfly/avmux/blob/master/LICENSE.txt
[Rust]: https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=Rust&logoColor=orange
[Rust-url]: https://www.rust-lang.org
[ffmpeg-next]: https://img.shields.io/badge/ffmpeg--next-000000?style=for-the-badge&logo=ffmpeg&logoColor=white
[ffmpeg-next-url]: https://crates.io/crates/ffmpeg-next
