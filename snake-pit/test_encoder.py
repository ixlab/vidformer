from fractions import Fraction
import os
import json
import subprocess as sp

import pytest
import vidformer as vf
import cv2


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


def test_output_codec_default():
    """Make sure we properly default to h264."""

    server = vf.YrdenServer()
    tos = vf.Source(server, "tos_720p", "tos_720p.mp4", 0)

    domain = tos.ts()[:50]

    def render(t, i):
        return tos.iloc[300 + i]

    fmt = tos.fmt()

    spec = vf.Spec(domain, render, fmt)

    spec.save(server, "enc.mp4")
    assert get_codec("enc.mp4") == "h264"
    os.remove("enc.mp4")


def test_output_raw():
    server = vf.YrdenServer()
    tos = vf.Source(server, "tos_720p", "tos_720p.mp4", 0)

    domain = tos.ts()[:50]

    def render(t, i):
        return tos.iloc[300 + i]

    fmt = tos.fmt()

    spec = vf.Spec(domain, render, fmt)

    spec.save(server, "enc.raw", encoder="rawvideo", format="rawvideo")
    assert os.path.exists("enc.raw")
    os.remove("enc.raw")


@pytest.mark.parametrize(
    "codec,encoder,pix_fmt,container,opts",
    [
        ("h264", "libx264", "yuv420p", "mp4", {"preset": "ultrafast", "crf": "18"}),
        ("ffv1", "ffv1", "yuv420p", "mov", {}),
        ("prores", "prores", "yuv422p10le", "mov", {}),
    ],
)
def test_output_codec(codec, encoder, pix_fmt, container, opts):
    server = vf.YrdenServer()
    tos = vf.Source(server, "tos_720p", "tos_720p.mp4", 0)

    scale = vf.Filter("Scale")

    domain = tos.ts()[:50]

    def render(t, i):
        return scale(tos.iloc[300 + i], pix_fmt=pix_fmt)

    fmt = tos.fmt()
    fmt["pix_fmt"] = pix_fmt

    spec = vf.Spec(domain, render, fmt)

    pth = "enc." + container
    spec.save(server, pth, encoder=encoder, encoder_opts=opts)
    assert get_codec(pth) == codec

    os.remove(pth)
