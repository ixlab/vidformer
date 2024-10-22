# vidformer - Video Data Transformation

[![Test](https://github.com/ixlab/vidformer/actions/workflows/test.yml/badge.svg)](https://github.com/ixlab/vidformer/actions/workflows/test.yml)
[![PyPI version](https://img.shields.io/pypi/v/vidformer.svg)](https://pypi.org/project/vidformer/)
[![Crates.io Version](https://img.shields.io/crates/v/vidformer)](https://crates.io/crates/vidformer)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/ixlab/vidformer/blob/main/LICENSE)


A research project providing infrastructure for video interfaces and pipelines.
Developed by the OSU Interactive Data Systems Lab.

## 🎯 Why vidformer

Vidformer efficiently transforms video data, enabling faster annotation, editing, and processing of video data—without having to focus on performance.

It uses a declarative specification format to represent transformations. This enables:

* **⚡ Transparent Optimization:** Vidformer optimizes the execution of declarative specifications just like a relational database optimizes relational queries.
  
* **⏳ Lazy/Deferred Execution:** Video results can be retrieved on-demand, allowing for practically instantaneous playback of video results.

* **🔄 Transpilation:** Vidformer specifications can be created from existing code (like `cv2`).

## 🚀 Quick Start

The easiest way to get started is using vidformer's `cv2` frontend, which allows most Python OpenCV visualization scripts to replace `import cv2` with `import vidformer.cv2 as cv2`:

```python
import vidformer.cv2 as cv2

cap = cv2.VideoCapture("my_input.mp4")
fps = cap.get(cv2.CAP_PROP_FPS)
width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

out = cv2.VideoWriter("my_output.mp4", cv2.VideoWriter_fourcc(*"mp4v"),
                        fps, (width, height))
while True:
    ret, frame = cap.read()
    if not ret:
      break

    cv2.putText(frame, "Hello, World!", (100, 100), cv2.FONT_HERSHEY_SIMPLEX,
                1, (255, 0, 0), 1)
    out.write(frame)

cap.release()
out.release()
```

You can find details on this in our [Getting Started Guide](https://ixlab.github.io/vidformer/getting-started.html).

## 📘 Documentation

* [🌐 Website](https://ixlab.github.io/vidformer/)
* [🚦 Getting Started](https://ixlab.github.io/vidformer/getting-started.html)
* [🐍 vidformer-py](https://ixlab.github.io/vidformer/vidformer-py/)
* [🛠️ vidformer core](https://ixlab.github.io/vidformer/vidformer/)

## 🔍 About the project

Vidformer is a highly modular suite of tools that work together; these are detailed [here](https://ixlab.github.io/vidformer/tools.html).

❌ vidformer is ***NOT***:
* A conventional video editor (like Premiere Pro or Final Cut)
* A video database/VDBMS
* A natural language query interface for video
* A computer vision library (like OpenCV)
* A computer vision AI model (like CLIP or Yolo)

However, vidformer is highly complementary to each of these.
If you're working on any of the later four, vidformer may be for you.

**License:** Vidformer is open source under [Apache-2.0](./LICENSE).
Contributions welcome.

**File Layout**:
- [*./vidformer*](./vidformer/): The core transformation library
- [*./vidformer-py*](./vidformer-py/): A Python video editing client
- [*./vidformer-cli*](./vidformer-cli/): A command-line interface for vidformer servers
- [*./snake-pit*](./snake-pit/): The main vidformer test suite
- [*./docs*](./docs/): The [vidformer website](https://ixlab.github.io/vidformer/)
