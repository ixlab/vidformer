# FAQ

### What is vidformer NOT?

vidformer is strongly complementary to many tools, but is not:
* A conventional video editor (like Premiere Pro or Final Cut)
* A video database/VDBMS
* A natural language query interface for video
* A computer vision library (like OpenCV)
* A computer vision AI model (like CLIP or YOLO)

If you're working on any of the latter four, vidformer may be useful for accelerating your visualization workflows.

### What video formats does vidformer support?

In short, essentially everything.
vidformer uses the [FFmpeg/libav*](https://ffmpeg.org/) libraries internally, so any media FFmpeg works with should work in vidformer as well.
We support many container formats (e.g., mp4, mov) and codecs (e.g., H.264, VP8).

A full list of supported codecs enabled in a vidformer build can be found by running:
```bash
vidformer-cli codecs
```

### Can I access remote videos on the internet?

Yes, vidformer uses [Apache OpenDAL](https://opendal.apache.org/) for I/O, so most common data/storage access protocols are supported.
However, not all storage services are enabled in distributed binaries.
We guarantee that HTTP, S3, and the local filesystem are always available.

### How does vidformer compare to FFmpeg?

vidformer is far more expressive than the FFmpeg filter interface.
Mainly, vidformer is designed for data-oriented work, so edits are created programmatically and can reference data.
Also, vidformer enables serving result videos on demand.

vidformer uses the [FFmpeg/libav*](https://ffmpeg.org/) libraries internally, so any media FFmpeg works with should also work in vidformer.

### How does vidformer compare to OpenCV/cv2?

vidformer orchestrates data movement, but does not implement image processing directly.
Most use cases will still use OpenCV for this.
