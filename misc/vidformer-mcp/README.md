# vidformer-mcp

[MCP](https://modelcontextprotocol.io) server that lets an LLM edit video by
writing `cv2` code, backed by
[vidformer](https://github.com/ixlab/vidformer).

```bash
pip install .
python serve.py
```

Listens on `http://127.0.0.1:8765/mcp`. Override with `HOST`, `PORT`,
`VIDFORMER_API_URL` (default `api.vidformer.org`), and `VIDFORMER_API_KEY`
(default `VF_GUEST`).

Tools: `create_video(code)` returns a playable video URL — use this one;
`run_code(code)` runs Python with no video output, for exploring data first.

**Submitted code is `exec`'d, so run this only where you trust the inputs.**