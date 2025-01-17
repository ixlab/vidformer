# Install

Using vidformer requires the Python client library, vidformer-py, and a yrden server which is distributed through `vidformer-cli`.

## **vidformer-py**

```bash
pip install vidformer
```

## **vidformer-cli**

### 🐳 Docker:
```bash
docker pull dominikwinecki/vidformer:latest
docker run --rm -it -p 8000:8000 dominikwinecki/vidformer:latest yrden --print-url
```

This launches a vidformer yrden server, which is our reference server implementation for local usage, on port 8000.
If you want to read or save video files locally add `-v /my/local/dir:/data` and then reference them as `/data` in the code.

To use:
```python
import vidformer as vf
server = vf.YrdenServer(domain="localhost", port=8000)

# or for cv2
import vidformer.cv2 as cv2
cv2.set_server(server)
```

### Precompiled binary:

Precompiled binaries are available for [vidformer releases](https://github.com/ixlab/vidformer/releases).

For example:
```bash
wget https://github.com/ixlab/vidformer/releases/download/<version>/vidformer-cli-ubuntu22.04-amd64
sudo mv  vidformer-cli-ubuntu22.04-amd64 /usr/local/bin/vidformer-cli
sudo chmod +x /usr/local/bin/vidformer-cli
sudo apt install -y libopencv-dev libfdk-aac-dev
```

To use:
```python
import vidformer as vf
server = vf.YrdenServer(bin="vidformer-cli")
```

or
```bash
export VIDFORMER_BIN='vidformer-cli'
```

```python
import vidformer as vf
server = vf.YrdenServer()
```

### Build from Sources

`vidformer-cli` can be compiled from our [git repo](https://github.com/ixlab/vidformer) with a standard `cargo build`.

This depends on the core `vidformer` library which itself requires linking to FFmpeg and OpenCV.
Details are available [here](https://github.com/ixlab/vidformer/tree/main/vidformer).