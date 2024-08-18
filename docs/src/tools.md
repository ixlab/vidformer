# The vidformer Tools

vidformer is a highly modular suite of tools that work together:

- [*vidformer-py*](./vidformer-py.md): A Python 🐍 client for declarative video synthesis
  - Provides an easy-to-use library for symbolically representing transformed videos
  - Acts as a client for a VoD server (i.e., for yrden)
  - **Using vidformer-py is the best place to get started**

- [*libvidformer*](./libvidformer.md): The core data-oriented declarative video editing library
  - An embedded video processing execution engine with low-level interfaces
  - Systems code, written in Rust 🦀
  - **You should use if:** You are building a VDBMS or other multimodal data-system infrastructure.
  - **You should *not* use if:** You just want to use vidformer in your workflows or projects.

- *yrden*: A vidformer Video-on-Demand server
  - Provides vidformer services over a REST-style API
  - Allows for client libraries to be written in any language
  - Serves video results via [HLS](https://en.wikipedia.org/wiki/HTTP_Live_Streaming) streams
  - Designed for local single-tenant use
  - **You should use if:** You want to create faster video results in your workflows or projects.
  - Note that yrden servers may be spun up transparently by client libraries, so you might use yrden without realizing it.

**Client libraries in other languages:**
Writing a vidformer client library for other languages is simple.
It's a few hundred lines of code, and you just have to construct some JSON.
Contributions or suggestions for other languages are welcome.

**Other VoD servers:**
We provide yrden as a simple reference VoD server implementation.
If you want to scale-out deployments, multi-tenant deployments, or deep integration with a specific system, writing another VoD server is needed. (In progress work)
