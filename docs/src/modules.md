# The vidformer modules

vidformer is a highly modular suite of tools that work together:

- [*vidformer-py*](./vidformer-py.md): A Python 🐍 client for declarative video synthesis
  - Provides an easy-to-use library for symbolically representing transformed videos
  - Acts as a client for a vidformer server
  - **Using vidformer-py is the best place to get started**

- [*libvidformer*](./libvidformer.md): The core data-oriented declarative video editing library
  - An embedded video processing execution engine with low-level interfaces
  - Systems code, written in Rust 🦀
  - **You should use this if:** You are building a VDBMS or other multimodal data-system *infrastructure*.
  - **You should *not* use this if:** You just want to use vidformer in your workflows or projects.

- [*vidformer-igni*](./vidformer-igni.md): The vidformer server
  - A multi-tenant scale-out server
  - Designed for Video on Demand *only*
    - Does not support full-video exports
    - All video sources must be over the network, not local
  - Enables live streaming and waiting on external dependencies for even lower time-to-playback latency

**Client libraries in other languages:**
Writing a vidformer client library for other languages is simple.
It's a few hundred lines of code, and you just have to construct some JSON.
Contributions or suggestions for other languages are welcome.
