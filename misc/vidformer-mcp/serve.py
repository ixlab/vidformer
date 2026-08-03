#!/usr/bin/env python3
"""Vidformer MCP Server - Video processing with cv2/OpenCV via MCP."""

import os
import sys
import math
import json
import re
import traceback
import logging
import threading
import uuid
from datetime import datetime, timezone
from io import StringIO

import uvicorn
import vidformer as vf
import vidformer.cv2 as cv2
import vidformer.numpy as np
from mcp.server.fastmcp import FastMCP
from mcp import types
from mcp.server.transport_security import TransportSecuritySettings
from starlette.middleware.cors import CORSMiddleware

# Set up logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s [%(levelname)s] %(name)s: %(message)s"
)
logger = logging.getLogger(__name__)

# Configuration
# Loopback by default: this server execs submitted code.
HOST = os.environ.get("HOST", "127.0.0.1")
PORT = int(os.environ.get("PORT", "8765"))

_api_url_raw = os.environ.get("VIDFORMER_API_URL", "api.vidformer.org")
VIDFORMER_API_URL = (
    f"https://{_api_url_raw}" if not _api_url_raw.startswith("http") else _api_url_raw
)
VIDFORMER_API_KEY = os.environ.get("VIDFORMER_API_KEY", "VF_GUEST")
ERROR_LOG_DIR = os.environ.get("ERROR_LOG_DIR", "/tmp/vidformer-mcp-errors")


def write_error_log(code: str, error: str, mode: str, stdout: str = "") -> str | None:
    """Write error details to a JSON file. Returns the filepath or None on failure."""
    try:
        os.makedirs(ERROR_LOG_DIR, exist_ok=True)
        error_id = uuid.uuid4().hex[:8].upper()
        filename = f"error_{error_id}.json"
        filepath = os.path.join(ERROR_LOG_DIR, filename)

        error_data = {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "error_id": error_id,
            "mode": mode,
            "code": code,
            "error": error,
            "stdout": stdout,
        }

        with open(filepath, "w") as f:
            json.dump(error_data, f, indent=2)

        logger.info(f"Error log written to {filepath}")
        return filepath
    except Exception as e:
        logger.warning(f"Failed to write error log: {e}")
        return None


# Create MCP server
mcp = FastMCP(
    "vidformer-mcp",
    stateless_http=True,
    transport_security=TransportSecuritySettings(enable_dns_rebinding_protection=False),
)

# Lazy initialization of vidformer server
_server = None


def get_server() -> vf.Server:
    """Get or create the vidformer server instance."""
    global _server
    if _server is None:
        _server = vf.Server(VIDFORMER_API_URL, api_key=VIDFORMER_API_KEY, vod_only=True)
        cv2.set_server(_server)
    return _server


# Modules pre-provided in the execution namespace
PROVIDED_MODULES = {"cv2", "math", "json", "re", "vidformer", "vf", "np", "numpy"}

# Modules that are never allowed
BLOCKED_MODULES = {"importlib"}

# Valid cv2 attributes (functions, classes, constants)
CV2_VALID_ATTRS = {x for x in dir(cv2) if not x.startswith("_")}


def validate_code(code: str) -> str | None:
    """Check code for blocked modules and invalid cv2 attributes. Returns error message if invalid, None if OK."""
    # Check for blocked modules
    for module in BLOCKED_MODULES:
        if re.search(rf"\b{module}\b", code):
            return f"Module '{module}' is not allowed"

    # Check for invalid cv2.xyz references. Yes, strings and comments could be false positives, but ¯\_(ツ)_/¯
    cv2_refs = re.findall(r"\bcv2\.([a-zA-Z_][a-zA-Z0-9_]*)", code)
    invalid_refs = [ref for ref in cv2_refs if ref not in CV2_VALID_ATTRS]
    if invalid_refs:
        unique_invalid = sorted(set(invalid_refs))
        return (
            f"Invalid cv2 attribute(s): {', '.join(f'cv2.{r}' for r in unique_invalid)}"
        )

    return None


def filter_imports(code: str) -> str:
    """Remove import statements for modules that are already provided."""
    lines = code.split("\n")
    filtered_lines = []

    for line in lines:
        stripped = line.strip()

        if not stripped or stripped.startswith("#"):
            filtered_lines.append(line)
            continue

        if stripped.startswith("import "):
            import_part = stripped[7:].strip()
            remaining = [
                m.strip()
                for m in import_part.split(",")
                if m.strip().split(" as ")[0].split(".")[0] not in PROVIDED_MODULES
            ]
            if remaining:
                filtered_lines.append(f"import {', '.join(remaining)}")
            continue

        if stripped.startswith("from "):
            parts = stripped.split(" import ")
            if len(parts) >= 2:
                module = parts[0][5:].strip().split(".")[0]
                if module in PROVIDED_MODULES:
                    continue

        filtered_lines.append(line)

    return "\n".join(filtered_lines)


def execute_code(
    code: str, server: vf.Server, expect_video: bool
) -> tuple[str | None, str | None, str]:
    """
    Execute cv2 code in a sandboxed environment.

    If expect_video is True, returns after the first frame is successfully pushed
    (for live streaming). Otherwise waits for execution to complete.
    """
    original_code = code
    mode = "VIDEO" if expect_video else "EXEC"

    # Check for blocked modules
    if error := validate_code(code):
        write_error_log(original_code, error, mode)
        return None, error, ""

    code = filter_imports(code)

    logger.info(f"{'='*60}")
    logger.info(f"[{mode}] Executing code:")
    logger.info(f"{'-'*60}")
    for i, line in enumerate(code.split("\n"), 1):
        logger.info(f"  {i:3d} | {line}")
    logger.info(f"{'-'*60}")

    video_url = None
    early_error = None
    video_ready = threading.Event()
    execution_done = threading.Event()
    captured_stdout = StringIO()
    captured_stderr = StringIO()

    def on_writer_init(writer):
        nonlocal video_url
        spec = writer.spec()
        video_url = spec._vod_endpoint
        logger.info(f"[{mode}] VideoWriter created -> {video_url}")

    def on_first_push(writer):
        logger.info(f"[{mode}] First frame pushed successfully")
        video_ready.set()

    endpoint = server._endpoint
    api_key = server._api_key

    if not endpoint or not endpoint.startswith(("http://", "https://")):
        error = f"Invalid server endpoint: {endpoint!r}"
        write_error_log(original_code, error, mode)
        return None, error, ""

    try:
        cv2.set_server(
            vf.Server(
                endpoint,
                api_key=api_key,
                vod_only=True,
                cv2_writer_init_callback=on_writer_init if expect_video else None,
                cv2_writer_first_push_callback=on_first_push if expect_video else None,
            )
        )
    except Exception as e:
        error = f"Failed to set up cv2 server: {e}"
        write_error_log(original_code, error, mode)
        return None, error, ""

    # Fresh execution namespace each time
    exec_globals = {
        "cv2": cv2,
        "np": np,
        "numpy": np,
        "math": math,
        "json": json,
        "re": re,
        "__builtins__": __builtins__,
    }

    def run_code():
        nonlocal early_error
        old_stdout, old_stderr = sys.stdout, sys.stderr
        sys.stdout = captured_stdout
        sys.stderr = captured_stderr
        try:
            exec(code, exec_globals)
        except Exception as e:
            early_error = f"{type(e).__name__}: {e}\n\n{traceback.format_exc()}"
            logger.error(f"[{mode}] ERROR: {early_error}")
            video_ready.set()
        finally:
            sys.stdout, sys.stderr = old_stdout, old_stderr
            stdout_content = captured_stdout.getvalue()
            stderr_content = captured_stderr.getvalue()
            if stdout_content:
                logger.info(f"[{mode}] STDOUT:")
                for line in stdout_content.rstrip().split("\n"):
                    logger.info(f"  > {line}")
            if stderr_content:
                logger.warning(f"[{mode}] STDERR:")
                for line in stderr_content.rstrip().split("\n"):
                    logger.warning(f"  ! {line}")
            logger.info(f"[{mode}] Execution finished")
            execution_done.set()
            video_ready.set()

    thread = threading.Thread(target=run_code, daemon=True)
    thread.start()

    if expect_video:
        # Wait for video URL or completion
        video_ready.wait()

        if early_error:
            # Error occurred - report it even if VideoWriter was created
            write_error_log(original_code, early_error, mode, captured_stdout.getvalue())
            return None, early_error, captured_stdout.getvalue()

        if not video_url:
            error = "Script completed without creating a VideoWriter"
            write_error_log(original_code, error, mode, captured_stdout.getvalue())
            return None, error, captured_stdout.getvalue()

        return video_url, None, captured_stdout.getvalue()
    else:
        # Wait for completion and return output
        execution_done.wait()

        if early_error:
            write_error_log(original_code, early_error, mode, captured_stdout.getvalue())
            return None, early_error, captured_stdout.getvalue()

        return None, None, captured_stdout.getvalue()


# Video player HTML template
# Two-phase UI:
#   Phase 1: "Generating code..." while LLM writes code (before ontoolresult)
#   Phase 2: Clean video player when ready (after ontoolresult with URL)
VIDEO_PLAYER_HTML = """<!DOCTYPE html>
<html>
<head>
  <meta name="color-scheme" content="light dark">
  <style>
    html, body { margin: 0; padding: 0; background: transparent; }
    video { display: block; max-width: 100%; }
    .generating {
      color: #888;
      font-family: system-ui, -apple-system, sans-serif;
      font-size: 14px;
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 8px 0;
    }
    .generating::before {
      content: '';
      width: 12px;
      height: 12px;
      border: 2px solid #666;
      border-top-color: transparent;
      border-radius: 50%;
      animation: spin 1s linear infinite;
    }
    @keyframes spin { to { transform: rotate(360deg); } }
    .error {
      color: #f66;
      font-family: system-ui, -apple-system, sans-serif;
      padding: 12px;
      white-space: pre-wrap;
    }
  </style>
</head>
<body>
  <div id="player"><span class="generating">Generating code...</span></div>
  <script src="https://cdn.jsdelivr.net/npm/hls.js@1"></script>
  <script type="module">
    import { App } from "https://unpkg.com/@modelcontextprotocol/ext-apps@0.4.0/app-with-deps";
    const app = new App({ name: "Vidformer Player", version: "1.0.0" });
    app.ontoolresult = ({ content }) => {
      const playerDiv = document.getElementById('player');
      const textContent = content?.find(c => c.type === 'text');
      if (textContent?.text) {
        if (textContent.text.startsWith('Error:')) {
          playerDiv.innerHTML = `<span class="error">${textContent.text}</span>`;
          return;
        }
        const match = textContent.text.match(/https:\\/\\/[^\\s]+/);
        if (match) {
          const hlsUrl = match[0].replace(/\\/embedded-player$/, '/stream.m3u8');
          playerDiv.innerHTML = '<video id="video" controls autoplay></video>';
          const video = document.getElementById('video');
          if (Hls.isSupported()) {
            const hls = new Hls();
            hls.loadSource(hlsUrl);
            hls.attachMedia(video);
          } else if (video.canPlayType('application/vnd.apple.mpegurl')) {
            video.src = hlsUrl;
          }
          return;
        }
      }
      playerDiv.innerHTML = '<span class="error">No video URL found</span>';
    };
    await app.connect();
  </script>
</body>
</html>"""

VIDEO_PLAYER_URI = "ui://vidformer/player"


@mcp.resource(
    VIDEO_PLAYER_URI,
    name="video_player",
    description="Video player UI for rendered videos",
    mime_type="text/html;profile=mcp-app",
    meta={
        "ui": {
            "csp": {
                "resourceDomains": [
                    "https://unpkg.com",
                    "https://cdn.jsdelivr.net",
                    "https://cdn.vidformer.org",
                ],
                "connectDomains": ["https://cdn.vidformer.org"],
            }
        }
    },
)
def video_player_resource() -> str:
    return VIDEO_PLAYER_HTML


@mcp.tool()
def run_code(code: str) -> list[types.TextContent]:
    """
    Execute Python code for data exploration (NO video output).

    AVOID using this tool unless absolutely necessary! Your goal is create_video.

    Only use run_code when you genuinely cannot proceed without first exploring
    unknown data structures or schemas (e.g., an unfamiliar API response format,
    unknown subtitle structure, etc.). If you already know the format or can
    reasonably assume it, go straight to create_video.

    BAD: Using run_code to "check" standard formats like SRT subtitles
    GOOD: Using run_code when facing a truly unknown JSON API schema

    Available: cv2, np/numpy, math, json, re, plus any imports you need (requests, etc.)
    """
    _, error, stdout = execute_code(code, get_server(), expect_video=False)

    if error:
        text = f"Error: {error}"
        if stdout:
            text += f"\n\nOutput:\n{stdout}"
        return [types.TextContent(type="text", text=text)]

    if stdout:
        result = stdout.rstrip()
    else:
        result = "(No output)"
    return [types.TextContent(type="text", text=result)]


@mcp.tool(meta={"ui": {"resourceUri": VIDEO_PLAYER_URI}})
def create_video(code: str) -> list[types.TextContent]:
    """
    Create a video using cv2. Returns playable video URL via live streaming.

    Use this for ANY video task: showing, editing, compiling, adding effects, etc.
    Even just "show me this video" should use this tool to display it.

    This tool RUNS code on the server. Do NOT create artifacts or React components.

    IMPORTANT: Do everything in ONE call when possible! Each tool call adds latency.
    Fetch data, parse it, AND create the video all in one script.

    The video URL returns IMMEDIATELY when VideoWriter is created - you can watch
    while frames are still being generated (live streaming).

    FRAME COUNT: Always process ALL frames unless the user asks for a subset.
    Never artificially limit frame count - the system streams efficiently.

    RESPONSE GUIDELINE: When the video is created successfully, keep your response
    very brief (1-2 sentences max). The video player shows automatically - let the
    video speak for itself. Don't summarize what's in the video.

    Available: cv2, np/numpy, math, json, re, plus any imports you need (requests, etc.)

    Video operations:
    - cv2.VideoCapture(url) - Open video from URL
    - cv2.VideoWriter(None, 0, fps, (width, height)) - Create output (REQUIRED!)
    - cap[i] - Get frame i (0-indexed)
    - cap.get(cv2.CAP_PROP_FPS) for fps
    - cap.get(cv2.CAP_PROP_FRAME_WIDTH), cap.get(cv2.CAP_PROP_FRAME_HEIGHT) for dimensions
    - cap.get(cv2.CAP_PROP_FRAME_COUNT) for frame count
    - cv2.rectangle, cv2.putText, cv2.circle, cv2.line - Drawing
    - out.release() - Finalize video (REQUIRED!)

    Example - reverse video with frame counter:
    ```python
    cap = cv2.VideoCapture("https://f.dominik.win/data/dve2/tos_720p.mp4")
    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))
    frame_count = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))

    out = cv2.VideoWriter(None, 0, fps, (width, height))

    for i in range(frame_count):  # Process ALL frames
        frame = cap[frame_count - 1 - i]  # Reverse order
        cv2.putText(frame, f"Frame {i}", (10, 50), cv2.FONT_HERSHEY_SIMPLEX, 1, (255, 255, 255), 2)
        out.write(frame)

    out.release()
    ```
    """
    video_url, error, stdout = execute_code(code, get_server(), expect_video=True)

    if error:
        text = f"Error: {error}"
        if stdout:
            text += f"\n\nOutput:\n{stdout}"
        return [types.TextContent(type="text", text=text)]

    if video_url:
        result = f"Video created: {video_url.rstrip('/')}/embedded-player"
        if stdout:
            result += f"\n\nOutput:\n{stdout}"
        return [types.TextContent(type="text", text=result)]

    # No video created
    if stdout:
        result = f"No video created. Output:\n{stdout.rstrip()}"
    else:
        result = "Error: No video created. Code must use cv2.VideoWriter."
    return [types.TextContent(type="text", text=result)]


if __name__ == "__main__":
    app = mcp.streamable_http_app()
    app.add_middleware(
        CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"]
    )
    print(f"Vidformer MCP Server: http://{HOST}:{PORT}/mcp")
    uvicorn.run(app, host=HOST, port=PORT)
