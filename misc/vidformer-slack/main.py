import os
import re
import logging
import json
import sys
import pickle
import traceback
from io import StringIO

from slack_bolt import App
from slack_bolt.adapter.socket_mode import SocketModeHandler
from slack_sdk.errors import SlackApiError
from dotenv import load_dotenv
from cerebras.cloud.sdk import Cerebras

logging.basicConfig(level=logging.INFO)
load_dotenv()

APP_TOKEN = os.environ["SLACK_APP_TOKEN"]
BOT_TOKEN = os.environ["SLACK_BOT_TOKEN"]
CEREBRAS_API_KEY = os.environ["CEREBRAS_API_KEY"]
VIDFORMER_API_URL = os.environ.get("VIDFORMER_API_URL", "api.vidformer.org")
VIDFORMER_API_KEY = os.environ.get("VIDFORMER_API_KEY", "VF_GUEST")

app = App(token=BOT_TOKEN)
cerebras_client = Cerebras(api_key=CEREBRAS_API_KEY)

auth = app.client.auth_test()
BOT_USER_ID = auth["user_id"]
logging.info(f"Connected as bot user: {BOT_USER_ID}")
MENTION_PATTERN = re.compile(rf"<@{re.escape(BOT_USER_ID)}>\s*")

# System prompt for GPT to generate vidformer cv2 code
SYSTEM_PROMPT = """You are a Python code generator for video processing using cv2.

You will be given a natural language description of a video to create. Generate Python code that uses cv2 to create the video.

IMPORTANT RULES:
1. DO NOT include any import statements - they are already provided
2. DO NOT download any files - open videos directly with URLs, read text files with requests.get(url).text
3. For video URLs with spaces, do NOT escape the spaces when using cv2.VideoCapture
4. For other URLs (like text files), you MAY escape spaces with %20
5. The following are already imported and available:
   - cv2
   - supervision
   - requests
   - re
   - math
   - pickle
   - json
   - All standard library modules
6. Use the video's native resolution unless otherwise specified

If using supervision:
7. You have access to supervision annotators, including supervision.LabelAnnotator, BoxAnnotator, etc., which you can use to directly draw supervision.Detection objects on frames.
8. Supervision default are sane, so you do NOT need to pass default values for parameters like color, thickness, etc., unless specified in the user query.
9. BoxAnnotator does NOT draw text, do NOT pass text_thickness or text_color to it. Use LabelAnnotator for text.

An example of using both BoxAnnotator and LabelAnnotator together:
```
box_anot = supervision.BoxAnnotator()
label_anot = supervision.LabelAnnotator()

labels = [
    f"{class_name} {{confidence:.2f}}"
    for class_name, confidence in zip(det["class_name"], det.confidence)
]
frame = box_anot.annotate(frame.copy(), det)
frame = label_anot.annotate(frame.copy(), det, labels)
```

When you create a VideoWriter, the video will be automatically posted to Slack, so do NOT include any code to post messages to Slack.
Output ONLY the Python code, no explanations or markdown. The code should be directly executable."""


def build_user_prompt(user_query: str) -> str:
    """Wrap the user's simple query with standard instructions."""
    return f"""{user_query}

Additional instructions:
- Use the video's native resolution
- DO NOT download any files - open videos directly with URLs
- After creating the video writer and writing frames, call writer.release()"""


def generate_video_code(user_query: str) -> str:
    """Use Cerebras to generate vidformer cv2 code from natural language."""
    full_prompt = build_user_prompt(user_query)
    response = cerebras_client.chat.completions.create(
        model="gpt-oss-120b",
        messages=[
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": full_prompt},
        ],
    )

    code = response.choices[0].message.content

    # Strip markdown code blocks if present
    if code.startswith("```python"):
        code = code[9:]
    elif code.startswith("```"):
        code = code[3:]
    if code.endswith("```"):
        code = code[:-3]

    # Remove imports for modules that are already provided
    provided_modules = {
        "cv2",
        "supervision",
        "requests",
        "re",
        "math",
        "pickle",
        "json",
    }
    lines = code.strip().split("\n")
    filtered_lines = []
    for line in lines:
        stripped = line.strip()
        # Check "import X, Y, Z" style (comma-separated)
        if stripped.startswith("import "):
            modules = [m.strip().split(".")[0] for m in stripped[7:].split(",")]
            # Filter out provided modules, keep the rest
            remaining = [
                m.strip()
                for m in stripped[7:].split(",")
                if m.strip().split(".")[0] not in provided_modules
            ]
            if not remaining:
                continue
            elif len(remaining) < len(modules):
                filtered_lines.append(
                    line[: len(line) - len(stripped)] + "import " + ", ".join(remaining)
                )
                continue
        # Check "from X import" style
        elif stripped.startswith("from "):
            module = stripped[5:].split()[0].split(".")[0]
            if module in provided_modules:
                continue
        filtered_lines.append(line)

    return "\n".join(filtered_lines).strip()


def execute_video_code(code: str, say, logger, title: str) -> str | None:
    """
    Execute the generated video code.
    The video is posted to Slack as soon as the VideoWriter is created.
    Returns error_message if an error occurred, None otherwise.
    """
    import vidformer as vf
    import vidformer.cv2 as cv2
    import vidformer.supervision as sv
    import requests
    import math

    # Callback that posts the video to Slack when a VideoWriter is created
    def on_writer_init(writer):
        spec = writer.spec()
        vod_endpoint = spec._vod_endpoint
        video_url = vod_endpoint.rstrip("/")
        logger.info(f"VideoWriter created, posting video URL: {video_url}")
        post_video_message(say, logger, video_url, title)

    # Set up the vidformer server with the callback
    api_url = (
        f"https://{VIDFORMER_API_URL}"
        if not VIDFORMER_API_URL.startswith("http")
        else VIDFORMER_API_URL
    )
    server = vf.Server(
        api_url,
        api_key=VIDFORMER_API_KEY,
        vod_only=True,
        cv2_writer_init_callback=on_writer_init,
    )
    cv2.set_server(server)

    # Create execution namespace
    exec_globals = {
        "cv2": cv2,
        "supervision": sv,
        "vf": vf,
        "requests": requests,
        "re": re,
        "math": math,
        "json": json,
        "pickle": pickle,
        "__builtins__": __builtins__,
    }

    # Capture stdout/stderr
    old_stdout = sys.stdout
    old_stderr = sys.stderr
    sys.stdout = StringIO()
    sys.stderr = StringIO()

    try:
        logger.info(f"Executing code:\n{code}")
        exec(code, exec_globals)

        stdout_output = sys.stdout.getvalue()
        stderr_output = sys.stderr.getvalue()

        if stdout_output:
            logger.info(f"Code stdout: {stdout_output}")
        if stderr_output:
            logger.warning(f"Code stderr: {stderr_output}")

        return None

    except Exception as e:
        error_msg = f"Error executing code: {str(e)}\n{traceback.format_exc()}"
        logger.error(error_msg)
        return error_msg
    finally:
        sys.stdout = old_stdout
        sys.stderr = old_stderr


def post_video_message(say, logger, video_url: str, title: str = "Generated Video"):
    """Post a video message to Slack."""
    video_block = {
        "blocks": [
            {
                "type": "video",
                "title": {
                    "type": "plain_text",
                    "text": "Video rendered with Vidformer",  # Slack limits title length
                    "emoji": True,
                },
                "title_url": video_url,
                "video_url": f"{video_url}/embedded-player",
                "alt_text": title,
                "thumbnail_url": "https://f.dominik.win/data/dve2/vidformer-thumbnail-1080p.jpg",
            }
        ]
    }

    try:
        say(text=f"Here's your video: {title}", blocks=video_block["blocks"])
        logger.info("Posted video block successfully.")
    except SlackApiError as e:
        err = e.response.get("error")
        needed = e.response.get("needed")
        provided = e.response.get("provided")
        logger.error(
            f"Failed to post video block: {err} (needed={needed}, provided={provided})"
        )
        # Fall back to just posting the URL
        say(text=f"Here's your video: {video_url}")


def process_query(query: str, say, logger):
    """Process a natural language query and generate a video."""
    logger.info(f"Processing query: {query}")

    # Send initial acknowledgment
    say(text="Generating Python code with an LLM -- This may take a moment.")

    try:
        # Generate the code using Cerebras
        logger.info("Generating code with Cerebras...")
        code = generate_video_code(query)
        logger.info(f"Generated code:\n{code}")

        # Send the code to the user
        # code_preview = code[:2900] if len(code) > 2900 else code  # Slack message limit
        # say(text=f"Generated code:\n```python\n{code_preview}\n```")
        say(text="Code generated! Rendering video with Vidformer...")

        # Execute the code - video is posted automatically when VideoWriter is created
        logger.info("Executing generated code...")
        title = query[:100] if len(query) <= 100 else query[:97] + "..."
        error = execute_video_code(code, say, logger, title)

        if error:
            say(
                text=f"Sorry, I encountered an error generating your video:\n```\n{error[:500]}\n```"
            )

    except Exception as e:
        logger.exception("Error processing query")
        say(text=f"Sorry, something went wrong: {str(e)[:200]}")


# Log every incoming request as middleware
@app.middleware
def log_all(body, logger, next):
    try:
        evt = body.get("event") or {}
        logger.info(
            "EVENT IN: %s",
            json.dumps(
                {
                    "type": evt.get("type"),
                    "subtype": evt.get("subtype"),
                    "channel": evt.get("channel"),
                    "text": evt.get("text"),
                },
                indent=2,
            ),
        )
    except Exception:
        logger.exception("logging middleware failed")
    return next()


# Subscribed so Slack delivers the event; unfurling is handled by the manifest's
# unfurl_domains rather than by this handler.
@app.event("link_shared")
def handle_link_shared(body, event, logger):
    logger.debug("link_shared: %s", json.dumps(event))


# Handle mentions: process the query
@app.event("app_mention")
def handle_app_mention(event, say, logger):
    text = event.get("text") or ""
    clean = MENTION_PATTERN.sub("", text).strip()

    if not clean:
        say(
            text="Hi! Tell me what kind of video you'd like to create. For example: 'Show me a compilation of Apollo 11 footage with subtitles'"
        )
        return

    process_query(clean, say, logger)


# Handle DMs: process the query
@app.event("message")
def handle_dm(event, say, logger):
    if event.get("channel_type") == "im" and not event.get("bot_id"):
        text = (event.get("text") or "").strip()

        if not text:
            say(text="Hi! Tell me what kind of video you'd like to create.")
            return

        process_query(text, say, logger)


if __name__ == "__main__":
    logging.info("Starting Slack bot in Socket Mode...")
    SocketModeHandler(app, APP_TOKEN).start()
