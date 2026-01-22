# Local Install

You can deploy the server locally with docker:
```bash
git clone https://github.com/ixlab/vidformer
cd vidformer
docker build -t igni -f Dockerfile .
docker-compose -f vidformer-igni/deploy/docker-compose.local.yaml up
```

Vidformer-py can be installed with pip:
```bash
pip3 install vidformer
```

There are two ways to connect the client to the server.
Either use the environment variables printed out by the server or set it manually:
```python
import vidformer as vf
import vidformer.cv2 as cv2

cv2.set_server(vf.Server("<ENDPOINT>", "<API_KEY>"))
```

## Run admin commands

Admin commands can be run from inside the server container:

```bash
docker-compose -f vidformer-igni/deploy/docker-compose.local.yaml exec igni bash
vidformer-igni user ls
```

Run `vidformer-igni --help` for other commands.