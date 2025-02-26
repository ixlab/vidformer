import os
import subprocess as sp
import random

import pytest

import vidformer as vf
import vidformer.cv2 as vf_cv2


def tmp_path(extension: str):
    pytest_name = (
        os.environ.get("PYTEST_CURRENT_TEST", "test")
        .replace("::", "_")
        .replace(" ", "_")
        .replace("_(call)", "")
    )
    random_8_alnum_chars = "".join(
        random.choices("abcdefghijklmnopqrstuvwxyz0123456789", k=8)
    )
    return f"../snake-pit/tmp_{pytest_name}_{random_8_alnum_chars}.{extension}"


def get_codec(pth):
    args = [
        "../ffmpeg/build/bin/ffprobe",
        "-v",
        "error",
        "-select_streams",
        "v:0",
        "-show_entries",
        "stream=codec_name",
        "-of",
        "default=noprint_wrappers=1:nokey=1",
        pth,
    ]
    ret = sp.run(args, capture_output=True, text=True)
    assert ret.returncode == 0
    return ret.stdout.strip()


def tmp_spec(pix_fmt="yuv420p"):
    cap = vf_cv2.VideoCapture("../tos_720p.mp4")
    width = int(cap.get(vf_cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(vf_cv2.CAP_PROP_FRAME_HEIGHT))

    out = vf_cv2.VideoWriter(
        None,
        None,
        30,
        (width, height),
        pix_fmt=pix_fmt,
    )

    i = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break
        if pix_fmt is not None:
            f = frame._f
            f = vf.Filter("Scale")(f, pix_fmt=pix_fmt)
            frame = vf_cv2.Frame(
                f,
                {
                    "width": width,
                    "height": height,
                    "pix_fmt": pix_fmt,
                },
            )
        out.write(frame)
        if i == 50:
            break
        i += 1

    cap.release()
    out.release()
    return out.spec()


def test_output_codec_default():
    """Make sure we properly default to h264."""
    spec = tmp_spec()
    path = tmp_path("mp4")
    vf_cv2.get_server().export_spec(spec.id(), path)
    assert get_codec(path) == "h264"
    os.remove(path)


def test_output_raw():
    spec = tmp_spec()
    path = tmp_path("raw")
    vf_cv2.get_server().export_spec(
        spec.id(), path, encoder="rawvideo", format="rawvideo"
    )
    assert os.path.exists(path)
    assert os.path.getsize(path) > 0
    os.remove(path)


@pytest.mark.parametrize(
    "codec,encoder,pix_fmt,container,opts",
    [
        ("h264", "libx264", "yuv420p", "mp4", {"preset": "ultrafast", "crf": "18"}),
        ("ffv1", "ffv1", "yuv420p", "mov", {}),
        ("prores", "prores", "yuv422p10le", "mov", {}),
    ],
)
def test_output_codec(codec, encoder, pix_fmt, container, opts):
    spec = tmp_spec(pix_fmt=pix_fmt)
    path = tmp_path(container)
    vf_cv2.get_server().export_spec(spec.id(), path, encoder=encoder, encoder_opts=opts)
    assert get_codec(path) == codec
    os.remove(path)
