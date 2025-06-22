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
  <a href="https://github.com/kingwingfly/avmux">
    <img src="images/logo.png" alt="Logo" width="80" height="80">
  </a>

<h3 align="center">AV Merge</h3>

  <p align="center">
    A crate to merge video and audio based on rsmpeg (dynamic link with ffmpeg lib)
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

[![Product Name Screen Shot][product-screenshot]](https://github.com/kingwingfly/avmux)

This crate provides a simple way to merge video and audio files into a single output file. It uses the rsmpeg library, which is a Rust binding for FFmpeg, to handle the underlying media processing.

<p align="right">(<a href="#readme-top">back to top</a>)</p>



### Built With

* [![Rust][Rust]][Rust-url]

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- GETTING STARTED -->
## Getting Started

### Prerequisites

* Install FFmpeg and its development libraries
    - **Linux**: Use your package manager to install FFmpeg, e.g., `sudo apt install ffmpeg` or `sudo pacman -S ffmpeg`
    - **macOS**: Use Homebrew to install FFmpeg, e.g., `brew install ffmpeg`
    - **Windows**: I don't know

* install pkg-config
    - **Linux**: Use your package manager to install pkg-config, e.g., `sudo pacman -S pkgconf`
    - **macOS**: Use Homebrew to install pkg-config, e.g., `brew install pkgconf`
    - **Windows**: I don't know

### Importing the Crate

To use this crate in your Rust project, add the following to your `Cargo.toml`:

```toml
[dependencies]
avmux = { version = "0.1"}
```

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- USAGE EXAMPLES -->
## Usage

```rust no_run
use avmux::{AVFile, Mux as _};
use std::path::PathBuf;

let video_file = AVFile::from_path(PathBuf::from("input_video.mp4"));
let audio_file = AVFile::new("https://music.com/input_audio.mp3").unwrap();
let output_file = AVFile::new("output.mp4").unwrap();
[video_file, audio_file].mux(output_file).unwrap();
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
[product-screenshot]: images/screenshot.png
[Rust]: https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=Rust&logoColor=orange
[Rust-url]: https://www.rust-lang.org
