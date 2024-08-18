# libvidformer

libvidformer is our core video synthesis/transformation library.
It handles the movement, control flow, and processing of video and conventional (non-video) data.

Here are the [source code](https://github.com/ixlab/vidformer/tree/main/vidformer) and [docs](https://ixlab.github.io/vidformer/vidformer/).

* It's written in Rust 🦀
    * So it does some fancy parallel processing and does so safely
* Uses the [FFmpeg libav libraries](https://www.ffmpeg.org/documentation.html) for multimedia stuff
    * So it should work with nearly every video file ever made
* Uses [Apache OpenDAL](https://opendal.apache.org/) for I/O
    * So it can access videos in a bunch of storage services
* Implements some filters using [OpenCV](https://opencv.org/)
