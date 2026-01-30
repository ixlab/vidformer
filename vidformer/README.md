# vidformer - Video Data Transformation Library

[![Crates.io Version](https://img.shields.io/crates/v/vidformer)](https://crates.io/crates/vidformer)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/ixlab/vidformer/blob/main/LICENSE)

(lib)vidformer is a video transformation library.
It handles the movement, control flow, and processing of video and conventional (non-video) data.

**Quick links:**
* [📦 Crates.io](https://crates.io/crates/vidformer)
* [📘 Documentation](https://ixlab.github.io/vidformer/vidformer/)
* [🧑‍💻 Source Code](https://github.com/ixlab/vidformer/tree/main/vidformer/)

## About

* It's written in Rust 🦀
    * So it does some fancy parallel processing and does so safely
* Uses the [FFmpeg libav libraries](https://www.ffmpeg.org/documentation.html) for multimedia stuff
    * So it should work with nearly every video file ever made
* Uses [Apache OpenDAL](https://opendal.apache.org/) for I/O
    * So it can access videos in a bunch of storage services
* Implements filters using [OpenCV](https://opencv.org/)


## Building

This crate requires linking with FFmpeg, as detailed in the `rusty_ffmpeg` crate.
We currently target FFmpeg 7.0.
