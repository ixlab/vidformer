# vidformer

[![Test](https://github.com/ixlab/vidformer/actions/workflows/test.yml/badge.svg)](https://github.com/ixlab/vidformer/actions/workflows/test.yml)
[![PyPI version](https://img.shields.io/pypi/v/vidformer.svg)](https://pypi.org/project/vidformer/)
[![Crates.io Version](https://img.shields.io/crates/v/vidformer)](https://crates.io/crates/vidformer)
[![Open In Colab](https://colab.research.google.com/assets/colab-badge.svg)](https://colab.research.google.com/github/ixlab/vidformer/blob/main/misc/Colab_Vidformer.ipynb)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/ixlab/vidformer/blob/main/LICENSE)

A research project for accelerating video/data visualization.
See the [preprint on arXiv](https://arxiv.org/abs/2601.17221) for details.

Developed by the OSU Interactive Data Systems Lab.

## Quick Start

To quickly try out Vidformer you can:

*  [![Open In Colab](https://colab.research.google.com/assets/colab-badge.svg)](https://colab.research.google.com/github/ixlab/vidformer/blob/main/misc/Colab_Vidformer.ipynb)
* try the online [Vidformer Playground](https://ixlab.github.io/vidformer/playground/)

Or, you can deploy it yourself:

```bash
git clone https://github.com/ixlab/vidformer
cd vidformer
docker build -t igni -f Dockerfile .
docker-compose -f vidformer-igni/docker-compose-local.yaml up
```

You can find details on this in our [Getting Started Guide](https://ixlab.github.io/vidformer/docs/getting-started.html).

## Why vidformer

Vidformer efficiently transforms videos, enabling faster annotation, editing, and processing of video data—without having to focus on performance. Just swap `import cv2` with `import vidformer.cv2 as cv2` to see video outputs instantly.

Vidformer uses a declarative specification format to represent transformations. This enables:

* **Transparent Optimization:** Vidformer optimizes the execution of declarative specifications just like a relational database optimizes relational queries.
  
* **Lazy/Deferred Renderjng:** Video results can be retrieved on-demand, allowing for practically instantaneous playback of video results.

Vidformer usually renders videos 2-3x faster than cv2, and hundreds of times faster (*practically instantly*) when serving videos on-demand.

Vidformer builds on open technologies you may already use:
  * **OpenCV:** A `cv2`-compatible interface ensures both you (and LLMs) can use  existing knowlege and code.
  * **Supervision:** [Supervision](https://supervision.roboflow.com/latest/)-compatible annotators make visualizing computer vision models trivial.
  * **FFmpeg:** Built on the same libraries, codecs, and formats that run the world.
  * **Jupyter:** View transformed videos instantly right in your notebook.
  * **HTTP Live Streaming (HLS):** Serve transformed videos over a network directly into any media player.
  * **Apache OpenDAL:** Access source videos no matter where they are stored.

## Documentation

* [🌐 Website](https://ixlab.github.io/vidformer/)
* [🌐 Docs](https://ixlab.github.io/vidformer/docs/)
* [🚀 Getting Started](https://ixlab.github.io/vidformer/docs/getting-started.html)
* [🐍 vidformer-py](https://ixlab.github.io/vidformer/docs/vidformer-py.html)
* [🛠️ vidformer core](https://ixlab.github.io/vidformer/vidformer/)

## About the project

**Cite:**
```
@misc{winecki2026_vidformer,
      title={Vidformer: Drop-in Declarative Optimization for Rendering Video-Native Query Results},
      author={Dominik Winecki and Arnab Nandi},
      year={2026},
      eprint={2601.17221},
      archivePrefix={arXiv},
      primaryClass={cs.DB},
      url={https://arxiv.org/abs/2601.17221},
}
```

**File Layout**:
- [*./vidformer*](./vidformer/): The core rendering library
- [*./vidformer-py*](./vidformer-py/): The python frontend
- [*./vidformer-igni*](./vidformer-igni/): The vidformer server
- [*./snake-pit*](./snake-pit/): The main vidformer test suite
- [*./vidformer-cli*](./vidformer-cli/): Vidformer binary for testing
- [*./docs*](./docs/): The [project docs](https://ixlab.github.io/vidformer/docs/)
- [*./misc/landing-page*](./misc/landing-page/): The [main page](https://ixlab.github.io/vidformer/)
- [*./misc/playground*](./misc/playground/): The [online playground](https://ixlab.github.io/vidformer/playground/)

Vidformer components are detailed [here](https://ixlab.github.io/vidformer/docs/modules.html).

❌ vidformer is ***NOT***:
* A conventional video editor (like Premiere Pro or Final Cut)
* A video database/VDBMS
* A natural language query interface for video
* A computer vision library (like OpenCV)
* A computer vision AI model (like CLIP or Yolo)

However, vidformer is strongly complementary to each of these.
If you're working on any of the later four, vidformer may be for you.

**License:** Vidformer is open source under [Apache-2.0](./LICENSE).
Contributions are welcome.

**Acknowledgements:** Vidformer is based upon work supported by the National Science Foundation under Awards #2118240 and #1910356.
