# vidformer-slack

Slack bot that turns a natural-language request into a video: an LLM writes
[vidformer](https://github.com/ixlab/vidformer) `cv2` code, and the bot posts
back the rendered video's URL.

## Setup

Create a Slack app from `manifest.json`, install it, and enable Socket Mode.
Then:

```bash
pip install .
cp .env.example .env   # fill in your tokens
source .env
python main.py
```

Mention the bot or DM it: *"Show me a 10 second clip of Tears of Steel with
'Hello World' text overlay"*.

Generated code is `exec`'d, so run this only where you trust the inputs.
