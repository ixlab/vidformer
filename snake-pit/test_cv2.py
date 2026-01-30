import os
import re

import cv2 as ocv_cv2
import numpy as np
import pytest
import subprocess
import random


import vidformer.cv2 as vf_cv2

TEST_VID_PATH = "../tos_720p.mp4"
TEST_IMG_PATH = "../apollo.jpg"
FFPROBE_PATH = "../ffmpeg/build/bin/ffprobe"


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


def ffprobe_count_frames(path):
    cmd = [
        FFPROBE_PATH,
        "-v",
        "error",
        "-count_frames",
        "-select_streams",
        "v:0",
        "-show_entries",
        "stream=nb_read_frames",
        "-of",
        "default=nokey=1:noprint_wrappers=1",
        path,
    ]
    result = subprocess.run(
        cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, text=True
    )
    return int(result.stdout)


def ffprobe_fmt(path):
    cmd = [
        FFPROBE_PATH,
        "-v",
        "error",
        "-select_streams",
        "v:0",
        "-show_entries",
        "stream=width,height,pix_fmt",
        "-of",
        "default=noprint_wrappers=1:nokey=1",
        path,
    ]
    result = subprocess.run(
        cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, text=True
    )
    width, height, pix_fmt = result.stdout.strip().split("\n")
    return {"width": int(width), "height": int(height), "pix_fmt": pix_fmt}


def test_constants():
    assert ocv_cv2.CAP_PROP_POS_MSEC == vf_cv2.CAP_PROP_POS_MSEC
    assert ocv_cv2.CAP_PROP_POS_FRAMES == vf_cv2.CAP_PROP_POS_FRAMES
    assert ocv_cv2.CAP_PROP_FRAME_WIDTH == vf_cv2.CAP_PROP_FRAME_WIDTH
    assert ocv_cv2.CAP_PROP_FRAME_HEIGHT == vf_cv2.CAP_PROP_FRAME_HEIGHT
    assert ocv_cv2.CAP_PROP_FPS == vf_cv2.CAP_PROP_FPS
    assert ocv_cv2.CAP_PROP_FRAME_COUNT == vf_cv2.CAP_PROP_FRAME_COUNT

    assert ocv_cv2.FONT_HERSHEY_SIMPLEX == vf_cv2.FONT_HERSHEY_SIMPLEX
    assert ocv_cv2.FONT_HERSHEY_PLAIN == vf_cv2.FONT_HERSHEY_PLAIN
    assert ocv_cv2.FONT_HERSHEY_DUPLEX == vf_cv2.FONT_HERSHEY_DUPLEX
    assert ocv_cv2.FONT_HERSHEY_COMPLEX == vf_cv2.FONT_HERSHEY_COMPLEX
    assert ocv_cv2.FONT_HERSHEY_TRIPLEX == vf_cv2.FONT_HERSHEY_TRIPLEX
    assert ocv_cv2.FONT_HERSHEY_COMPLEX_SMALL == vf_cv2.FONT_HERSHEY_COMPLEX_SMALL
    assert ocv_cv2.FONT_HERSHEY_SCRIPT_SIMPLEX == vf_cv2.FONT_HERSHEY_SCRIPT_SIMPLEX
    assert ocv_cv2.FONT_HERSHEY_SCRIPT_COMPLEX == vf_cv2.FONT_HERSHEY_SCRIPT_COMPLEX
    assert ocv_cv2.FONT_ITALIC == vf_cv2.FONT_ITALIC

    assert ocv_cv2.FILLED == vf_cv2.FILLED
    assert ocv_cv2.LINE_4 == vf_cv2.LINE_4
    assert ocv_cv2.LINE_8 == vf_cv2.LINE_8
    assert ocv_cv2.LINE_AA == vf_cv2.LINE_AA

    assert ocv_cv2.MARKER_CROSS == vf_cv2.MARKER_CROSS
    assert ocv_cv2.MARKER_TILTED_CROSS == vf_cv2.MARKER_TILTED_CROSS
    assert ocv_cv2.MARKER_STAR == vf_cv2.MARKER_STAR
    assert ocv_cv2.MARKER_DIAMOND == vf_cv2.MARKER_DIAMOND
    assert ocv_cv2.MARKER_SQUARE == vf_cv2.MARKER_SQUARE
    assert ocv_cv2.MARKER_TRIANGLE_UP == vf_cv2.MARKER_TRIANGLE_UP
    assert ocv_cv2.MARKER_TRIANGLE_DOWN == vf_cv2.MARKER_TRIANGLE_DOWN

    assert ocv_cv2.INTER_NEAREST == vf_cv2.INTER_NEAREST
    assert ocv_cv2.INTER_LINEAR == vf_cv2.INTER_LINEAR
    assert ocv_cv2.INTER_CUBIC == vf_cv2.INTER_CUBIC
    assert ocv_cv2.INTER_AREA == vf_cv2.INTER_AREA
    if hasattr(ocv_cv2, "INTER_LANCOZOS4"):  # I guess some versions don't have it?
        assert ocv_cv2.INTER_LANCOZOS4 == vf_cv2.INTER_LANCOZOS4
    assert ocv_cv2.INTER_LINEAR_EXACT == vf_cv2.INTER_LINEAR_EXACT
    assert ocv_cv2.INTER_NEAREST_EXACT == vf_cv2.INTER_NEAREST_EXACT


def test_cap_all_frames():
    """Make sure VideoCapture can read all frames of a video correctly."""
    import vidformer.cv2 as cv2

    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    while True:
        ret, frame = cap.read()
        if not ret:
            break
        assert frame.shape[0] == height
        assert frame.shape[1] == width
        assert frame.shape[2] == 3

    cap.release()


def rw(cv2):
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    assert width == 1280
    assert height == 720

    path = tmp_path("mp4")
    out = cv2.VideoWriter(path, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height))
    assert out.isOpened()

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret or count > 100:
            break

        assert frame.shape[0] == height
        assert frame.shape[1] == width
        assert frame.shape[2] == 3

        out.write(frame)
        count += 1

    cap.release()
    out.release()

    assert os.path.exists(path)
    assert ffprobe_count_frames(path) == count
    fmt = ffprobe_fmt(path)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(path)


def test_rw_ocv():
    rw(ocv_cv2)


def test_rw_vf():
    rw(vf_cv2)


def videowriter_numpy(cv2):
    # make up random numpy frames, write them to a video
    width, height = 300, 200

    path = tmp_path("mp4")
    out = cv2.VideoWriter(path, cv2.VideoWriter_fourcc(*"mp4v"), 30, (width, height))

    for i in range(3):
        frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)
        out.write(frame)
    out.release()

    assert os.path.exists(path)
    os.remove(path)


def test_videowriter_numpy_ocv():
    videowriter_numpy(ocv_cv2)


def test_videowriter_numpy_vf():
    videowriter_numpy(vf_cv2)


def test_numpy():
    import numpy as np

    import vidformer.cv2 as vf_cv2

    img = vf_cv2.imread(TEST_IMG_PATH)
    assert isinstance(img, vf_cv2.Frame)
    assert img.shape[0] == 3936
    assert img.shape[1] == 3912
    assert img.shape[2] == 3

    img_np = img.numpy()
    assert isinstance(img_np, np.ndarray)
    assert img_np.shape[0] == 3936
    assert img_np.shape[1] == 3912
    assert img_np.shape[2] == 3

    # the 1000th frame of tos_720p.mp4
    cap = vf_cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    cap.set(vf_cv2.CAP_PROP_POS_FRAMES, 1000)
    ret, frame = cap.read()

    frame_np = frame.numpy()
    assert isinstance(frame_np, np.ndarray)
    assert frame_np.shape[0] == 720
    assert frame_np.shape[1] == 1280
    assert frame_np.shape[2] == 3


def test_vidplay():
    import vidformer.cv2 as cv2

    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    assert width == 1280
    assert height == 720

    out = cv2.VideoWriter(None, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height))

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret or count > 100:
            break

        out.write(frame)
        count += 1

    cap.release()
    out.release()

    # test vidplay on a Spec
    spec = out.spec()
    link = cv2.vidplay(spec, method="link")
    assert re.match(r"http://localhost:\d+/vod/\w+-\w+-\w+-\w+-\w+/playlist.m3u8", link)

    # test vidplay on a VideoWriter
    link = cv2.vidplay(out, method="link")
    assert re.match(r"http://localhost:\d+/vod/\w+-\w+-\w+-\w+-\w+/playlist.m3u8", link)


def test_zeros():
    frame = vf_cv2.zeros((100, 200, 3), dtype=np.uint8)
    assert type(frame) is vf_cv2.Frame
    assert frame.shape[0] == 100
    assert frame.shape[1] == 200
    assert frame.shape[2] == 3

    frame_np = frame.numpy()
    assert isinstance(frame_np, np.ndarray)
    assert frame_np.shape[0] == 100
    assert frame_np.shape[1] == 200
    assert frame_np.shape[2] == 3
    assert np.all(frame_np == 0)


def test_resize():
    frame = vf_cv2.imread(TEST_IMG_PATH)
    assert type(frame) is vf_cv2.Frame

    frame_resized = vf_cv2.resize(frame, (300, 250))
    assert type(frame_resized) is vf_cv2.Frame
    assert frame_resized.shape[0] == 250
    assert frame_resized.shape[1] == 300
    assert frame_resized.shape[2] == 3

    frame_resized_np = frame_resized.numpy()
    assert isinstance(frame_resized_np, np.ndarray)
    assert frame_resized_np.shape[0] == 250
    assert frame_resized_np.shape[1] == 300
    assert frame_resized_np.shape[2] == 3

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame_resized)
    assert os.path.exists(path)
    fmt = ffprobe_fmt(path)
    assert fmt["width"] == 300
    assert fmt["height"] == 250
    assert fmt["pix_fmt"] == "rgb24"
    os.remove(path)

    # Test that adding an interpolation argument works, but make sure it doesn't do anything
    for interpolation in [
        vf_cv2.INTER_NEAREST,
        vf_cv2.INTER_LINEAR,
        vf_cv2.INTER_CUBIC,
        vf_cv2.INTER_AREA,
        vf_cv2.INTER_LANCOZOS4,
        vf_cv2.INTER_LINEAR_EXACT,
        vf_cv2.INTER_NEAREST_EXACT,
        vf_cv2.INTER_MAX,
    ]:
        frame_resized = vf_cv2.resize(frame, (300, 250), interpolation=interpolation)
        assert type(frame_resized) is vf_cv2.Frame
        assert frame_resized.shape[0] == 250
        assert frame_resized.shape[1] == 300
        assert frame_resized.shape[2] == 3


def test_resize_numpy():
    frame = np.random.randint(0, 255, (100, 200, 3), dtype=np.uint8)
    frame = vf_cv2.resize(frame, (300, 250))
    assert frame.shape[0] == 250
    assert frame.shape[1] == 300
    assert frame.shape[2] == 3

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def rectangle(cv2):
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    path = tmp_path("mp4")
    out = cv2.VideoWriter(path, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height))

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret or count > 100:
            break

        cv2.rectangle(frame, (100, 100), (200, 200), (0, 255, 0, 255), 3)

        out.write(frame)
        count += 1

    cap.release()
    out.release()

    assert os.path.exists(path)
    assert ffprobe_count_frames(path) == count
    fmt = ffprobe_fmt(path)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(path)


def test_rectangle_ocv():
    rectangle(ocv_cv2)


def test_rectangle_vf():
    rectangle(vf_cv2)


def test_rectangle_numpy():
    width, height = 300, 200

    frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)
    vf_cv2.rectangle(frame, (100, 100), (200, 200), (0, 255, 0, 255), 3)

    assert frame.shape[0] == height
    assert frame.shape[1] == width
    assert frame.shape[2] == 3

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def putText(cv2):
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    path = tmp_path("mp4")
    out = cv2.VideoWriter(path, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height))

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret or count > 100:
            break

        cv2.putText(
            frame,
            "Hello, World!",
            (100, 100),
            cv2.FONT_HERSHEY_SIMPLEX,
            1,
            (255, 0, 0),
            1,
        )

        out.write(frame)
        count += 1

    cap.release()
    out.release()

    assert os.path.exists(path)
    assert ffprobe_count_frames(path) == count
    fmt = ffprobe_fmt(path)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(path)


def test_text_ocv():
    putText(ocv_cv2)


def test_text_vf():
    putText(vf_cv2)


def test_text_numpy():
    width, height = 300, 200

    frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)
    vf_cv2.putText(
        frame,
        "Hello, World!",
        (100, 100),
        vf_cv2.FONT_HERSHEY_SIMPLEX,
        1,
        (255, 0, 0),
        1,
    )

    assert frame.shape[0] == height
    assert frame.shape[1] == width
    assert frame.shape[2] == 3

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def arrowedLine(cv2):
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    path = tmp_path("mp4")
    out = cv2.VideoWriter(path, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height))

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret or count > 100:
            break

        cv2.arrowedLine(
            frame,
            (100, 100),
            (200, 200),
            (0, 255, 0, 255),
            3,
        )

        out.write(frame)
        count += 1

    cap.release()
    out.release()

    assert os.path.exists(path)
    os.path.exists(path)
    fmt = ffprobe_fmt(path)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(path)


def test_arrowedLine_ocv():
    arrowedLine(ocv_cv2)


def test_arrowedLine_vf():
    arrowedLine(vf_cv2)


def test_arrowedLine_numpy():
    width, height = 300, 200

    frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)
    vf_cv2.arrowedLine(
        frame,
        (100, 100),
        (200, 200),
        (0, 255, 0, 255),
        3,
    )

    assert frame.shape[0] == height
    assert frame.shape[1] == width
    assert frame.shape[2] == 3

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def test_arrowedLine_tipLength_without_shift():
    width, height = 100, 100
    color = (0, 255, 0)

    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.arrowedLine(
        frame_ocv, (10, 50), (90, 50), color, thickness=2, tipLength=0.3
    )

    frame_vf = vf_cv2.arrowedLine(
        np.zeros((height, width, 3), dtype=np.uint8),
        (10, 50),
        (90, 50),
        color,
        thickness=2,
        tipLength=0.3,
    ).numpy()

    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), "arrowedLine with tipLength but no shift should match OpenCV"


def test_arrowedLine_shift_without_line_type():
    width, height = 100, 100
    color = (255, 0, 0)

    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.arrowedLine(frame_ocv, (10, 50), (90, 50), color, thickness=2, shift=0)

    frame_vf = vf_cv2.arrowedLine(
        np.zeros((height, width, 3), dtype=np.uint8),
        (10, 50),
        (90, 50),
        color,
        thickness=2,
        shift=0,
    ).numpy()

    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), "arrowedLine with shift but no line_type should match OpenCV"


def test_arrowedLine_only_tipLength():
    width, height = 100, 100
    color = (0, 0, 255)

    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.arrowedLine(frame_ocv, (10, 50), (90, 50), color, tipLength=0.5)

    frame_vf = vf_cv2.arrowedLine(
        np.zeros((height, width, 3), dtype=np.uint8),
        (10, 50),
        (90, 50),
        color,
        tipLength=0.5,
    ).numpy()

    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), "arrowedLine with only tipLength should match OpenCV"


def line(cv2):
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    path = tmp_path("mp4")
    out = cv2.VideoWriter(path, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height))

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret or count > 100:
            break

        cv2.line(
            frame,
            (100, 100),
            (200, 200),
            (0, 255, 0, 255),
            3,
        )

        out.write(frame)
        count += 1

    cap.release()
    out.release()

    assert os.path.exists(path)
    os.path.exists(path)
    fmt = ffprobe_fmt(path)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(path)


def test_line_ocv():
    line(ocv_cv2)


def test_line_vf():
    line(vf_cv2)


def test_line_numpy():
    width, height = 300, 200

    frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)
    vf_cv2.line(
        frame,
        (100, 100),
        (200, 200),
        (0, 255, 0, 255),
        3,
    )

    assert frame.shape[0] == height
    assert frame.shape[1] == width
    assert frame.shape[2] == 3

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def circle(cv2):
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    path = tmp_path("mp4")
    out = cv2.VideoWriter(path, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height))

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret or count > 100:
            break

        cv2.circle(
            frame,
            (150, 150),
            50,
            (0, 255, 0, 255),
            3,
        )

        out.write(frame)
        count += 1

    cap.release()
    out.release()

    assert os.path.exists(path)
    os.path.exists(path)
    fmt = ffprobe_fmt(path)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(path)


def test_circle_ocv():
    circle(ocv_cv2)


def test_circle_vf():
    circle(vf_cv2)


def ellipse(cv2):
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    path = tmp_path("mp4")
    out = cv2.VideoWriter(path, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height))

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret or count > 100:
            break

        cv2.ellipse(
            frame,
            (150, 150),
            (50, 50),
            0,
            0,
            360,
            (0, 255, 0, 255),
            3,
        )

        out.write(frame)
        count += 1

    cap.release()
    out.release()

    assert os.path.exists(path)
    os.path.exists(path)
    fmt = ffprobe_fmt(path)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(path)


def test_ellipse_ocv():
    ellipse(ocv_cv2)


def test_ellipse_vf():
    ellipse(vf_cv2)


def polylines(cv2):
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    path = tmp_path("mp4")
    out = cv2.VideoWriter(path, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height))

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret or count > 100:
            break

        # Draw a triangle
        pts = np.array([[100, 50], [50, 150], [150, 150]], np.int32)
        pts = pts.reshape((-1, 1, 2))
        cv2.polylines(frame, [pts], True, (0, 255, 0, 255), 3)

        out.write(frame)
        count += 1

    cap.release()
    out.release()

    assert os.path.exists(path)
    os.path.exists(path)
    fmt = ffprobe_fmt(path)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(path)


def test_polylines_ocv():
    polylines(ocv_cv2)


def test_polylines_vf():
    polylines(vf_cv2)


def test_polylines_numpy():
    width, height = 300, 200

    frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)

    # Draw a triangle
    pts = np.array([[100, 50], [50, 150], [150, 150]], np.int32)
    pts = pts.reshape((-1, 1, 2))
    vf_cv2.polylines(frame, [pts], True, (0, 255, 0, 255), 3)

    assert frame.shape[0] == height
    assert frame.shape[1] == width
    assert frame.shape[2] == 3

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def test_polylines_open():
    width, height = 300, 200

    frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)

    # Draw an open polyline
    pts = np.array([[50, 50], [100, 100], [150, 50], [200, 100]], np.int32)
    pts = pts.reshape((-1, 1, 2))
    vf_cv2.polylines(frame, [pts], False, (255, 0, 0, 255), 2)

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def test_polylines_multiple():
    width, height = 300, 200

    frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)

    # Draw two separate polygons
    pts1 = np.array([[50, 50], [100, 50], [75, 100]], np.int32).reshape((-1, 1, 2))
    pts2 = np.array([[150, 50], [200, 50], [175, 100]], np.int32).reshape((-1, 1, 2))
    vf_cv2.polylines(frame, [pts1, pts2], True, (0, 0, 255, 255), 2)

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def test_fillPoly():
    width, height = 300, 200

    frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)

    # Draw a filled triangle
    pts = np.array([[100, 50], [50, 150], [150, 150]], np.int32).reshape((-1, 1, 2))
    vf_cv2.fillPoly(frame, [pts], (0, 255, 0, 255))

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def test_fillPoly_multiple():
    width, height = 300, 200

    frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)

    # Draw two filled polygons
    pts1 = np.array([[50, 50], [100, 50], [75, 100]], np.int32).reshape((-1, 1, 2))
    pts2 = np.array([[150, 50], [200, 50], [175, 100]], np.int32).reshape((-1, 1, 2))
    vf_cv2.fillPoly(frame, [pts1, pts2], (0, 0, 255, 255))

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def test_fillPoly_color_match():
    width, height = 100, 100
    color = (255, 0, 0)  # BGR blue

    pts = np.array([[20, 20], [80, 20], [50, 80]], np.int32).reshape((-1, 1, 2))

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.fillPoly(frame_ocv, [pts], color)

    # Vidformer
    frame_vf = vf_cv2.fillPoly(
        np.zeros((height, width, 3), dtype=np.uint8), [pts], color
    ).numpy()

    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), "Blue fillPoly mismatch: OpenCV vs vidformer"


def test_fillConvexPoly():
    width, height = 300, 200

    frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)

    # Draw a filled convex quadrilateral
    pts = np.array([[100, 50], [150, 100], [100, 150], [50, 100]], np.int32).reshape(
        (-1, 1, 2)
    )
    vf_cv2.fillConvexPoly(frame, pts, (255, 0, 0, 255))

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def test_fillConvexPoly_color_match():
    width, height = 100, 100
    color = (255, 255, 0)  # BGR cyan (B+G)

    pts = np.array([[20, 20], [80, 20], [80, 80], [20, 80]], np.int32).reshape(
        (-1, 1, 2)
    )

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.fillConvexPoly(frame_ocv, pts, color)

    # Vidformer
    frame_vf = vf_cv2.fillConvexPoly(
        np.zeros((height, width, 3), dtype=np.uint8), pts, color
    ).numpy()

    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), "Cyan fillConvexPoly mismatch: OpenCV vs vidformer"


def test_drawContours():
    width, height = 300, 200

    frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)

    # Create some contours (list of point arrays)
    contour1 = np.array([[50, 50], [100, 50], [75, 100]], np.int32).reshape((-1, 1, 2))
    contour2 = np.array([[150, 50], [200, 50], [175, 100]], np.int32).reshape(
        (-1, 1, 2)
    )

    vf_cv2.drawContours(frame, [contour1, contour2], -1, (0, 255, 0, 255), 2)

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def test_drawContours_color_match():
    width, height = 100, 100
    color = (0, 0, 255)  # BGR red

    contour = np.array([[20, 20], [80, 20], [80, 80], [20, 80]], np.int32).reshape(
        (-1, 1, 2)
    )

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.drawContours(frame_ocv, [contour], -1, color, 2)

    # Vidformer
    frame_vf = vf_cv2.drawContours(
        np.zeros((height, width, 3), dtype=np.uint8), [contour], -1, color, 2
    ).numpy()

    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), "Red drawContours mismatch: OpenCV vs vidformer"


def test_drawContours_filled():
    width, height = 100, 100
    color = (255, 255, 0)  # BGR cyan

    contour = np.array([[20, 20], [80, 20], [50, 80]], np.int32).reshape((-1, 1, 2))

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.drawContours(frame_ocv, [contour], 0, color, -1)

    # Vidformer
    frame_vf = vf_cv2.drawContours(
        np.zeros((height, width, 3), dtype=np.uint8), [contour], 0, color, -1
    ).numpy()

    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), "Cyan filled drawContours mismatch: OpenCV vs vidformer"


def test_drawMarker():
    width, height = 300, 200

    frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)

    vf_cv2.drawMarker(frame, (150, 100), (0, 255, 0, 255))

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def test_drawMarker_color_match():
    width, height = 100, 100
    color = (255, 0, 0)  # BGR blue
    position = (50, 50)

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.drawMarker(frame_ocv, position, color)

    # Vidformer
    frame_vf = vf_cv2.drawMarker(
        np.zeros((height, width, 3), dtype=np.uint8), position, color
    ).numpy()

    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), "Blue drawMarker mismatch: OpenCV vs vidformer"


@pytest.mark.parametrize(
    "marker_type",
    [
        ocv_cv2.MARKER_CROSS,
        ocv_cv2.MARKER_TILTED_CROSS,
        ocv_cv2.MARKER_STAR,
        ocv_cv2.MARKER_DIAMOND,
        ocv_cv2.MARKER_SQUARE,
        ocv_cv2.MARKER_TRIANGLE_UP,
        ocv_cv2.MARKER_TRIANGLE_DOWN,
    ],
    ids=lambda m: f"MARKER_{m}",
)
def test_drawMarker_types(marker_type):
    width, height = 100, 100
    color = (0, 255, 255)  # BGR yellow
    position = (50, 50)

    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.drawMarker(frame_ocv, position, color, marker_type)

    frame_vf = vf_cv2.drawMarker(
        np.zeros((height, width, 3), dtype=np.uint8), position, color, marker_type
    ).numpy()

    assert np.allclose(frame_ocv, frame_vf, atol=1)


def test_clipLine():
    # Test clipping a line that crosses the image boundary
    imgRect = (0, 0, 100, 100)
    pt1 = (-10, 50)
    pt2 = (110, 50)

    retval_ocv, pt1_ocv, pt2_ocv = ocv_cv2.clipLine(imgRect, pt1, pt2)
    retval_vf, pt1_vf, pt2_vf = vf_cv2.clipLine(imgRect, pt1, pt2)

    assert retval_ocv == retval_vf
    assert pt1_ocv == pt1_vf
    assert pt2_ocv == pt2_vf


def test_ellipse2Poly():
    center = (50, 50)
    axes = (30, 20)
    angle = 0
    arcStart = 0
    arcEnd = 360
    delta = 10

    pts_ocv = ocv_cv2.ellipse2Poly(center, axes, angle, arcStart, arcEnd, delta)
    pts_vf = vf_cv2.ellipse2Poly(center, axes, angle, arcStart, arcEnd, delta)

    assert np.array_equal(pts_ocv, pts_vf)


def test_circle_numpy():
    width, height = 300, 200

    frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)
    vf_cv2.circle(
        frame,
        (150, 150),
        50,
        (0, 255, 0, 255),
        3,
    )

    assert frame.shape[0] == height
    assert frame.shape[1] == width
    assert frame.shape[2] == 3

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def seek(cv2):
    # seek to 4 different places, two of which with msec and two with frames; read 3 seconds at each place

    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    path = tmp_path("mp4")
    out = cv2.VideoWriter(path, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height))

    count = 0
    while True:
        if count == 0:
            cap.set(cv2.CAP_PROP_POS_MSEC, 1000)
        elif count == 25:
            cap.set(cv2.CAP_PROP_POS_FRAMES, 1000)
        elif count == 50:
            cap.set(cv2.CAP_PROP_POS_MSEC, 20000.0)
        elif count == 75:
            cap.set(cv2.CAP_PROP_POS_FRAMES, 2000)
        elif count == 100:
            break

        ret, frame = cap.read()
        if not ret:
            break

        out.write(frame)
        count += 1

    cap.release()
    out.release()

    assert os.path.exists(path)
    os.path.exists(path)
    fmt = ffprobe_fmt(path)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(path)


def test_seek_ocv():
    seek(ocv_cv2)


def test_seek_vf():
    seek(vf_cv2)


def get_pos_msec(cv2):
    # test getting position in milliseconds
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)

    # At start, should be at position 0
    pos_msec = cap.get(cv2.CAP_PROP_POS_MSEC)
    assert pos_msec == 0.0

    # Seek to frame 24 (1 second at 24fps)
    cap.set(cv2.CAP_PROP_POS_FRAMES, 24)
    pos_msec = cap.get(cv2.CAP_PROP_POS_MSEC)
    expected_msec = 24 * (1000 / fps)
    assert abs(pos_msec - expected_msec) < 1.0  # within 1ms

    # Seek to 5000ms using set, then verify with get
    cap.set(cv2.CAP_PROP_POS_MSEC, 5000)
    pos_msec = cap.get(cv2.CAP_PROP_POS_MSEC)
    assert abs(pos_msec - 5000) < 2 * (1000 / fps)  # within two frame durations

    # Seek to frame 1000
    cap.set(cv2.CAP_PROP_POS_FRAMES, 1000)
    pos_msec = cap.get(cv2.CAP_PROP_POS_MSEC)
    expected_msec = 1000 * (1000 / fps)
    assert abs(pos_msec - expected_msec) < 2 * (
        1000 / fps
    )  # within two frame durations

    # Read a frame and verify position advances
    pos_before = cap.get(cv2.CAP_PROP_POS_MSEC)
    ret, frame = cap.read()
    assert ret
    pos_after = cap.get(cv2.CAP_PROP_POS_MSEC)
    assert pos_after > pos_before

    cap.release()


# We can not run this with ocv_cv2 since OpenCV's get CAP_PROP_POS_MSEC is quite inaccurate
def test_get_pos_msec_vf():
    get_pos_msec(vf_cv2)


def test_getFontScaleFromHeight():
    import cv2 as ocv_cv2

    import vidformer.cv2 as vf_cv2

    fonts = [
        ocv_cv2.FONT_HERSHEY_SIMPLEX,
        ocv_cv2.FONT_HERSHEY_PLAIN,
        ocv_cv2.FONT_HERSHEY_DUPLEX,
        ocv_cv2.FONT_HERSHEY_COMPLEX,
        ocv_cv2.FONT_HERSHEY_TRIPLEX,
        ocv_cv2.FONT_HERSHEY_COMPLEX_SMALL,
        ocv_cv2.FONT_HERSHEY_SCRIPT_SIMPLEX,
        ocv_cv2.FONT_HERSHEY_SCRIPT_COMPLEX,
    ]
    font_sizes = list(range(50))
    font_thicknesses = list(range(1, 10))

    for font in fonts:
        for size in font_sizes:
            for thickness in font_thicknesses:
                assert ocv_cv2.getFontScaleFromHeight(
                    font, size, thickness
                ) == vf_cv2.getFontScaleFromHeight(font, size, thickness)


def test_getTextSize():
    import cv2 as ocv_cv2

    import vidformer.cv2 as vf_cv2

    texts = ["", "hello", "hello, world!", "123456890+-3", "a" * 1000]
    fonts = [
        ocv_cv2.FONT_HERSHEY_SIMPLEX,
        ocv_cv2.FONT_HERSHEY_PLAIN,
        ocv_cv2.FONT_HERSHEY_DUPLEX,
        ocv_cv2.FONT_HERSHEY_COMPLEX,
        ocv_cv2.FONT_HERSHEY_TRIPLEX,
        ocv_cv2.FONT_HERSHEY_COMPLEX_SMALL,
        ocv_cv2.FONT_HERSHEY_SCRIPT_SIMPLEX,
        ocv_cv2.FONT_HERSHEY_SCRIPT_COMPLEX,
    ]
    font_sizes = list(range(50))
    font_thicknesses = list(range(1, 10))

    for text in texts:
        for font in fonts:
            for size in font_sizes:
                for thickness in font_thicknesses:
                    assert ocv_cv2.getTextSize(
                        text, font, size, thickness
                    ) == vf_cv2.getTextSize(text, font, size, thickness)


def addWeighted(cv2):
    # blend two videos, one second apart

    cap1 = cv2.VideoCapture(TEST_VID_PATH)
    assert cap1.isOpened()

    cap2 = cv2.VideoCapture(TEST_VID_PATH)
    assert cap2.isOpened()

    fps = cap1.get(cv2.CAP_PROP_FPS)
    width = int(cap1.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap1.get(cv2.CAP_PROP_FRAME_HEIGHT))
    cap2.set(cv2.CAP_PROP_POS_MSEC, 1000)

    path = tmp_path("mp4")
    out = cv2.VideoWriter(path, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height))

    count = 0
    while True:
        ret1, frame1 = cap1.read()
        ret2, frame2 = cap2.read()
        if not ret1 or not ret2 or count > 100:
            break

        frame1 = cv2.addWeighted(frame1, 0.5, frame2, 0.5, 0)

        out.write(frame1)
        count += 1

    cap1.release()
    cap2.release()
    out.release()

    assert os.path.exists(path)
    os.path.exists(path)
    fmt = ffprobe_fmt(path)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(path)


def test_addWeighted_ocv():
    addWeighted(ocv_cv2)


def test_addWeighted_vf():
    addWeighted(vf_cv2)


def test_addWeighted_numpy():
    width, height = 300, 200

    frame1 = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)
    frame2 = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)

    frame = vf_cv2.addWeighted(frame1, 0.5, frame2, 0.5, 0)

    assert frame.shape[0] == height
    assert frame.shape[1] == width
    assert frame.shape[2] == 3

    path = tmp_path("png")
    vf_cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def test_imread():
    import vidformer.cv2 as vf_cv2

    img = vf_cv2.imread(TEST_IMG_PATH)

    assert img._fmt["width"] == 3912
    assert img._fmt["height"] == 3936
    assert img._fmt["pix_fmt"] == "yuvj444p"


def imread(cv2):
    img = cv2.imread(TEST_IMG_PATH)

    assert img.shape[0] == 3936
    assert img.shape[1] == 3912
    assert img.shape[2] == 3


def test_imread_ocv():
    imread(ocv_cv2)


def test_imread_vf():
    imread(vf_cv2)


def imwrite(cv2):
    # from apollo.jpg
    img = cv2.imread(TEST_IMG_PATH)

    # jpg
    path = tmp_path("jpg")
    cv2.imwrite(path, img)
    assert os.path.exists(path)
    os.remove(path)

    # jpeg
    path = tmp_path("jpeg")
    cv2.imwrite(path, img)
    assert os.path.exists(path)
    os.remove(path)

    # png
    path = tmp_path("png")
    cv2.imwrite(path, img)
    assert os.path.exists(path)
    os.remove(path)

    # from 1000th frame of tos_720p.mp4
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    cap.set(cv2.CAP_PROP_POS_FRAMES, 1000)
    ret, frame = cap.read()
    assert ret

    # jpg
    path = tmp_path("jpg")
    cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)

    # jpeg
    path = tmp_path("jpeg")
    cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)

    # png
    path = tmp_path("png")
    cv2.imwrite(path, frame)
    assert os.path.exists(path)
    os.remove(path)


def test_imwrite_ocv():
    imwrite(ocv_cv2)


def test_imwrite_vf():
    imwrite(vf_cv2)


def imwrite_numpy(cv2):
    width, height = 300, 200

    red_image = np.zeros((height, width, 3), dtype=np.uint8)
    red_image[:, :] = (0, 0, 255)

    output_filename = tmp_path("png")
    cv2.imwrite(output_filename, red_image)

    assert os.path.exists(output_filename)
    file_size = os.path.getsize(output_filename)
    print(f"File {output_filename} size: {file_size} bytes")

    red_image2 = cv2.imread(output_filename)
    if isinstance(red_image2, vf_cv2.Frame):
        red_image2 = red_image2.numpy()

    assert red_image2.dtype == np.uint8
    assert red_image2.shape[0] == height
    assert red_image2.shape[1] == width
    assert red_image2.shape[2] == 3
    os.remove(output_filename)


def test_imwrite_numpy_ocv():
    imwrite_numpy(ocv_cv2)


def test_imwrite_numpy_vf():
    imwrite_numpy(vf_cv2)


def test_imread_numpy_match_content():
    # use cv2 to write TMP_PNG_PATH, because jpeg decoding can be lossy
    img = ocv_cv2.imread(TEST_IMG_PATH)
    assert img.shape == (3936, 3912, 3)
    path = tmp_path("png")
    ocv_cv2.imwrite(path, img)

    img1 = ocv_cv2.imread(path)
    assert img1.shape == (3936, 3912, 3)
    img2 = vf_cv2.imread(path).numpy()
    assert img2.shape == (3936, 3912, 3)

    assert img1.dtype == img2.dtype
    assert img1.shape == img2.shape
    assert np.all(img1 == img2)
    os.remove(path)


def test_frameify():
    # write a video with all white frames
    width, height = 300, 200
    path = tmp_path("mp4")
    out = vf_cv2.VideoWriter(
        path, vf_cv2.VideoWriter_fourcc(*"mp4v"), 30, (width, height)
    )

    for i in range(3):
        frame = np.full((height, width, 3), 255, dtype=np.uint8)
        frame_shape = frame.shape

        frame = vf_cv2.frameify(frame)
        assert isinstance(frame, vf_cv2.Frame)
        assert frame.shape == frame_shape

        out.write(frame)

    out.release()
    assert os.path.exists(path)
    os.path.exists(path)
    os.remove(path)


def test_frame_array_slicing_appolo():
    frame_orig = ocv_cv2.imread(TEST_IMG_PATH)[:1000, :1000]
    path = tmp_path("png")
    ocv_cv2.imwrite(path, frame_orig)
    assert os.path.exists(path)
    frame = vf_cv2.imread(path)

    assert frame.shape == frame_orig.shape

    frame = frame[500:600, 500:650]
    frame_orig = frame_orig[500:600, 500:650]

    assert frame.shape == frame_orig.shape
    assert np.all(frame.numpy() == frame_orig)
    os.remove(path)


def test_write_slice_apollo():
    frame_orig = ocv_cv2.imread(TEST_IMG_PATH)[:1000, :1000]
    path = tmp_path("png")
    ocv_cv2.imwrite(path, frame_orig)
    assert os.path.exists(path)
    frame = vf_cv2.imread(path)

    assert frame.shape == frame_orig.shape

    write_array = np.random.randint(0, 255, (100, 150, 3), dtype=np.uint8)

    frame[500:600, 500:650] = write_array
    frame_orig[500:600, 500:650] = write_array

    assert frame.shape == frame_orig.shape
    assert np.all(frame.numpy() == frame_orig)
    os.remove(path)


class Slicer:
    def __getitem__(self, key):
        return key


@pytest.mark.parametrize(
    "s",
    [
        Slicer()[:, :],
        Slicer()[:100, :],
        Slicer()[:100, 150:],
        Slicer()[100:, :],
        Slicer()[100:200, 150:250],
        Slicer()[100:200, :250],
        Slicer()[:200, 150:250],
        Slicer()[-100:, :],
        Slicer()[:-100, :],
        Slicer()[-100:-50, :],
        Slicer()[:, :-150],
        Slicer()[:, -150:],
        Slicer()[:-100, :-150],
        Slicer()[-100:, -150:],
    ],
)
def test_frame_array_slicing(s):
    frame_orig = ocv_cv2.imread(TEST_IMG_PATH)[1000:1500, 1000:1512]

    vf_frame = vf_cv2.frameify(frame_orig)
    assert isinstance(vf_frame, vf_cv2.Frame)

    frame_ocv = frame_orig[s]
    assert isinstance(frame_ocv, np.ndarray)

    frame_vf = vf_frame[s]
    assert isinstance(vf_frame, vf_cv2.Frame)

    assert frame_ocv.shape == frame_vf.shape
    frame_vf = frame_vf.numpy()
    assert frame_ocv.shape == frame_vf.shape
    assert np.all(frame_ocv == frame_vf)


@pytest.mark.parametrize(
    "s",
    [
        Slicer()[:, :],
        Slicer()[:100, :],
        Slicer()[:100, 150:],
        Slicer()[100:, :],
        Slicer()[100:200, 150:250],
        Slicer()[100:200, :250],
        Slicer()[:200, 150:250],
        Slicer()[-100:, :],
        Slicer()[:-100, :],
        Slicer()[-100:-50, :],
        Slicer()[:, :-150],
        Slicer()[:, -150:],
        Slicer()[:-100, :-150],
        Slicer()[-100:, -150:],
    ],
)
def test_frame_array_slice_write(s):
    frame_orig = ocv_cv2.imread(TEST_IMG_PATH)[1000:1500, 1000:1512]
    random_array = np.random.randint(0, 255, frame_orig.shape, dtype=np.uint8)

    vf_frame = vf_cv2.frameify(frame_orig)
    assert isinstance(vf_frame, vf_cv2.Frame)

    write_array = random_array[s]

    vf_frame[s] = write_array
    frame_orig[s] = write_array

    assert isinstance(vf_frame, vf_cv2.Frame)
    assert vf_frame.shape == frame_orig.shape
    assert np.all(vf_frame.numpy() == frame_orig)


# =============================================================================
# Color matching tests - verify vidformer produces same colors as OpenCV
# =============================================================================


def test_color_rectangle_blue():
    """Test that BGR blue (255, 0, 0) draws as blue in both OpenCV and vidformer"""
    width, height = 100, 100
    color = (255, 0, 0)  # BGR blue

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.rectangle(frame_ocv, (10, 10), (90, 90), color, -1)

    # Vidformer - capture return value for numpy array input
    frame_vf = vf_cv2.rectangle(
        np.zeros((height, width, 3), dtype=np.uint8), (10, 10), (90, 90), color, -1
    ).numpy()

    # Compare - they should match
    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), f"Blue rectangle mismatch: OpenCV vs vidformer"

    # Verify the drawn region is actually blue (B=255, G=0, R=0 in BGR)
    drawn_region_ocv = frame_ocv[50, 50]
    assert (
        drawn_region_ocv[0] == 255
    ), f"OpenCV blue channel should be 255, got {drawn_region_ocv[0]}"
    assert (
        drawn_region_ocv[1] == 0
    ), f"OpenCV green channel should be 0, got {drawn_region_ocv[1]}"
    assert (
        drawn_region_ocv[2] == 0
    ), f"OpenCV red channel should be 0, got {drawn_region_ocv[2]}"


def test_color_rectangle_green():
    """Test that BGR green (0, 255, 0) draws as green in both OpenCV and vidformer"""
    width, height = 100, 100
    color = (0, 255, 0)  # BGR green

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.rectangle(frame_ocv, (10, 10), (90, 90), color, -1)

    # Vidformer - capture return value for numpy array input
    frame_vf = vf_cv2.rectangle(
        np.zeros((height, width, 3), dtype=np.uint8), (10, 10), (90, 90), color, -1
    ).numpy()

    # Compare - they should match
    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), f"Green rectangle mismatch: OpenCV vs vidformer"

    # Verify the drawn region is actually green (B=0, G=255, R=0 in BGR)
    drawn_region_ocv = frame_ocv[50, 50]
    assert (
        drawn_region_ocv[0] == 0
    ), f"OpenCV blue channel should be 0, got {drawn_region_ocv[0]}"
    assert (
        drawn_region_ocv[1] == 255
    ), f"OpenCV green channel should be 255, got {drawn_region_ocv[1]}"
    assert (
        drawn_region_ocv[2] == 0
    ), f"OpenCV red channel should be 0, got {drawn_region_ocv[2]}"


def test_color_rectangle_red():
    """Test that BGR red (0, 0, 255) draws as red in both OpenCV and vidformer"""
    width, height = 100, 100
    color = (0, 0, 255)  # BGR red

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.rectangle(frame_ocv, (10, 10), (90, 90), color, -1)

    # Vidformer - capture return value for numpy array input
    frame_vf = vf_cv2.rectangle(
        np.zeros((height, width, 3), dtype=np.uint8), (10, 10), (90, 90), color, -1
    ).numpy()

    # Compare - they should match
    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), f"Red rectangle mismatch: OpenCV vs vidformer"

    # Verify the drawn region is actually red (B=0, G=0, R=255 in BGR)
    drawn_region_ocv = frame_ocv[50, 50]
    assert (
        drawn_region_ocv[0] == 0
    ), f"OpenCV blue channel should be 0, got {drawn_region_ocv[0]}"
    assert (
        drawn_region_ocv[1] == 0
    ), f"OpenCV green channel should be 0, got {drawn_region_ocv[1]}"
    assert (
        drawn_region_ocv[2] == 255
    ), f"OpenCV red channel should be 255, got {drawn_region_ocv[2]}"


def test_color_circle_blue():
    """Test that BGR blue (255, 0, 0) draws as blue circle"""
    width, height = 100, 100
    color = (255, 0, 0)  # BGR blue

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.circle(frame_ocv, (50, 50), 30, color, -1)

    # Vidformer - capture return value for numpy array input
    frame_vf = vf_cv2.circle(
        np.zeros((height, width, 3), dtype=np.uint8), (50, 50), 30, color, -1
    ).numpy()

    # Compare - they should match
    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), f"Blue circle mismatch: OpenCV vs vidformer"


def test_color_line_red():
    """Test that BGR red (0, 0, 255) draws as red line"""
    width, height = 100, 100
    color = (0, 0, 255)  # BGR red

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.line(frame_ocv, (10, 50), (90, 50), color, 5)

    # Vidformer - capture return value for numpy array input
    frame_vf = vf_cv2.line(
        np.zeros((height, width, 3), dtype=np.uint8), (10, 50), (90, 50), color, 5
    ).numpy()

    # Compare - they should match
    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), f"Red line mismatch: OpenCV vs vidformer"


def test_color_putText_green():
    """Test that BGR green (0, 255, 0) draws as green text"""
    width, height = 200, 100
    color = (0, 255, 0)  # BGR green

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.putText(
        frame_ocv, "Test", (10, 50), ocv_cv2.FONT_HERSHEY_SIMPLEX, 1, color, 2
    )

    # Vidformer - capture return value for numpy array input
    frame_vf = vf_cv2.putText(
        np.zeros((height, width, 3), dtype=np.uint8),
        "Test",
        (10, 50),
        vf_cv2.FONT_HERSHEY_SIMPLEX,
        1,
        color,
        2,
    ).numpy()

    # Compare - they should match
    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), f"Green text mismatch: OpenCV vs vidformer"


def test_color_ellipse_yellow():
    """Test that BGR yellow (0, 255, 255) draws as yellow ellipse"""
    width, height = 100, 100
    color = (0, 255, 255)  # BGR yellow (G+R)

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.ellipse(frame_ocv, (50, 50), (30, 20), 0, 0, 360, color, -1)

    # Vidformer - capture return value for numpy array input
    frame_vf = vf_cv2.ellipse(
        np.zeros((height, width, 3), dtype=np.uint8),
        (50, 50),
        (30, 20),
        0,
        0,
        360,
        color,
        -1,
    ).numpy()

    # Compare - they should match
    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), f"Yellow ellipse mismatch: OpenCV vs vidformer"


def test_color_polylines_cyan():
    """Test that BGR cyan (255, 255, 0) draws as cyan polyline"""
    width, height = 100, 100
    color = (255, 255, 0)  # BGR cyan (B+G)

    pts = np.array([[20, 20], [80, 20], [80, 80], [20, 80]], np.int32).reshape(
        (-1, 1, 2)
    )

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.polylines(frame_ocv, [pts], True, color, 3)

    # Vidformer - capture return value for numpy array input
    frame_vf = vf_cv2.polylines(
        np.zeros((height, width, 3), dtype=np.uint8), [pts], True, color, 3
    ).numpy()

    # Compare - they should match
    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), f"Cyan polyline mismatch: OpenCV vs vidformer"


def test_color_arrowedLine_magenta():
    """Test that BGR magenta (255, 0, 255) draws as magenta arrowed line"""
    width, height = 100, 100
    color = (255, 0, 255)  # BGR magenta (B+R)

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.arrowedLine(frame_ocv, (10, 50), (90, 50), color, 3)

    # Vidformer - capture return value for numpy array input
    frame_vf = vf_cv2.arrowedLine(
        np.zeros((height, width, 3), dtype=np.uint8), (10, 50), (90, 50), color, 3
    ).numpy()

    # Compare - they should match
    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), f"Magenta arrowed line mismatch: OpenCV vs vidformer"


def test_color_rectangle_with_alpha():
    """Test that 4-channel BGR+alpha color works correctly"""
    width, height = 100, 100
    color = (255, 0, 0, 255)  # BGR blue with alpha

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.rectangle(frame_ocv, (10, 10), (90, 90), color, -1)

    # Vidformer - capture return value for numpy array input
    frame_vf = vf_cv2.rectangle(
        np.zeros((height, width, 3), dtype=np.uint8), (10, 10), (90, 90), color, -1
    ).numpy()

    # Compare - they should match
    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), f"Blue rectangle with alpha mismatch"


@pytest.mark.parametrize(
    "color,name",
    [
        ((255, 0, 0), "blue"),
        ((0, 255, 0), "green"),
        ((0, 0, 255), "red"),
        ((255, 255, 0), "cyan"),
        ((255, 0, 255), "magenta"),
        ((0, 255, 255), "yellow"),
        ((255, 255, 255), "white"),
        ((128, 128, 128), "gray"),
    ],
)
def test_color_parametrized_rectangle(color, name):
    """Parametrized test for various BGR colors"""
    width, height = 100, 100

    # OpenCV
    frame_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    ocv_cv2.rectangle(frame_ocv, (10, 10), (90, 90), color, -1)

    # Vidformer - capture return value for numpy array input
    frame_vf = vf_cv2.rectangle(
        np.zeros((height, width, 3), dtype=np.uint8), (10, 10), (90, 90), color, -1
    ).numpy()

    # Compare - they should match
    assert np.allclose(
        frame_ocv, frame_vf, atol=1
    ), f"{name} rectangle mismatch: OpenCV vs vidformer"


# =============================================================================
# Slice write-back tests - verify modifications to slices propagate to parent
# =============================================================================


def test_slice_writeback_putText():
    """Test that drawing on a slice propagates back to the parent frame"""
    width, height = 200, 100
    color = (255, 255, 255)  # white

    # OpenCV
    canvas_ocv = np.zeros((height, width * 2, 3), dtype=np.uint8)
    ocv_cv2.putText(
        canvas_ocv[0:height, 0:width],
        "Left",
        (10, 50),
        ocv_cv2.FONT_HERSHEY_SIMPLEX,
        1,
        color,
        2,
    )
    ocv_cv2.putText(
        canvas_ocv[0:height, width : width * 2],
        "Right",
        (10, 50),
        ocv_cv2.FONT_HERSHEY_SIMPLEX,
        1,
        color,
        2,
    )

    # Vidformer
    canvas_vf = vf_cv2.zeros((height, width * 2, 3))
    vf_cv2.putText(
        canvas_vf[0:height, 0:width],
        "Left",
        (10, 50),
        vf_cv2.FONT_HERSHEY_SIMPLEX,
        1,
        color,
        2,
    )
    vf_cv2.putText(
        canvas_vf[0:height, width : width * 2],
        "Right",
        (10, 50),
        vf_cv2.FONT_HERSHEY_SIMPLEX,
        1,
        color,
        2,
    )
    canvas_vf = canvas_vf.numpy()

    # Compare - they should match
    assert np.allclose(
        canvas_ocv, canvas_vf, atol=1
    ), "Slice writeback putText mismatch"


def test_slice_writeback_rectangle():
    """Test that drawing rectangles on slices propagates back to parent"""
    width, height = 100, 100
    color = (0, 255, 0)  # green

    # OpenCV
    canvas_ocv = np.zeros((height, width * 2, 3), dtype=np.uint8)
    ocv_cv2.rectangle(canvas_ocv[0:height, 0:width], (10, 10), (90, 90), color, -1)
    ocv_cv2.rectangle(
        canvas_ocv[0:height, width : width * 2], (10, 10), (90, 90), color, 3
    )

    # Vidformer
    canvas_vf = vf_cv2.zeros((height, width * 2, 3))
    vf_cv2.rectangle(canvas_vf[0:height, 0:width], (10, 10), (90, 90), color, -1)
    vf_cv2.rectangle(
        canvas_vf[0:height, width : width * 2], (10, 10), (90, 90), color, 3
    )
    canvas_vf = canvas_vf.numpy()

    # Compare - they should match
    assert np.allclose(
        canvas_ocv, canvas_vf, atol=1
    ), "Slice writeback rectangle mismatch"


def test_slice_writeback_circle():
    """Test that drawing circles on slices propagates back to parent"""
    width, height = 100, 100
    color = (255, 0, 0)  # blue

    # OpenCV
    canvas_ocv = np.zeros((height, width * 2, 3), dtype=np.uint8)
    ocv_cv2.circle(canvas_ocv[0:height, 0:width], (50, 50), 30, color, -1)
    ocv_cv2.circle(canvas_ocv[0:height, width : width * 2], (50, 50), 30, color, 2)

    # Vidformer
    canvas_vf = vf_cv2.zeros((height, width * 2, 3))
    vf_cv2.circle(canvas_vf[0:height, 0:width], (50, 50), 30, color, -1)
    vf_cv2.circle(canvas_vf[0:height, width : width * 2], (50, 50), 30, color, 2)
    canvas_vf = canvas_vf.numpy()

    # Compare - they should match
    assert np.allclose(canvas_ocv, canvas_vf, atol=1), "Slice writeback circle mismatch"


def test_slice_writeback_combined():
    """Test multiple different drawing operations on slices of the same canvas"""
    width, height = 100, 100

    # OpenCV - multiple drawing operations on slices
    canvas_ocv = np.zeros((height, width * 2, 3), dtype=np.uint8)
    # First operation on left half
    ocv_cv2.rectangle(
        canvas_ocv[0:height, 0:width], (10, 10), (90, 90), (0, 255, 0), -1
    )
    # Second operation on right half
    ocv_cv2.circle(
        canvas_ocv[0:height, width : width * 2], (50, 50), 30, (255, 0, 0), -1
    )
    # Third operation on left half again (should layer on top)
    ocv_cv2.circle(canvas_ocv[0:height, 0:width], (50, 50), 20, (0, 0, 255), -1)
    # Fourth operation on right half again
    ocv_cv2.rectangle(
        canvas_ocv[0:height, width : width * 2], (20, 20), (80, 80), (255, 255, 0), 2
    )

    # Vidformer - same operations
    canvas_vf = vf_cv2.zeros((height, width * 2, 3))
    vf_cv2.rectangle(canvas_vf[0:height, 0:width], (10, 10), (90, 90), (0, 255, 0), -1)
    vf_cv2.circle(canvas_vf[0:height, width : width * 2], (50, 50), 30, (255, 0, 0), -1)
    vf_cv2.circle(canvas_vf[0:height, 0:width], (50, 50), 20, (0, 0, 255), -1)
    vf_cv2.rectangle(
        canvas_vf[0:height, width : width * 2], (20, 20), (80, 80), (255, 255, 0), 2
    )
    canvas_vf = canvas_vf.numpy()

    # Compare - they should match
    assert np.allclose(
        canvas_ocv, canvas_vf, atol=1
    ), "Slice writeback combined use case mismatch"

    # Verify that content is actually drawn (not all black)
    assert np.sum(canvas_ocv) > 0, "OpenCV canvas should have content"
    assert np.sum(canvas_vf) > 0, "Vidformer canvas should have content"


def test_slice_writeback_line():
    """Test that drawing lines on slices propagates back to parent"""
    width, height = 100, 100
    color = (0, 0, 255)  # red

    # OpenCV
    canvas_ocv = np.zeros((height, width * 2, 3), dtype=np.uint8)
    ocv_cv2.line(canvas_ocv[0:height, 0:width], (10, 10), (90, 90), color, 3)
    ocv_cv2.line(canvas_ocv[0:height, width : width * 2], (90, 10), (10, 90), color, 3)

    # Vidformer
    canvas_vf = vf_cv2.zeros((height, width * 2, 3))
    vf_cv2.line(canvas_vf[0:height, 0:width], (10, 10), (90, 90), color, 3)
    vf_cv2.line(canvas_vf[0:height, width : width * 2], (90, 10), (10, 90), color, 3)
    canvas_vf = canvas_vf.numpy()

    # Compare - they should match
    assert np.allclose(canvas_ocv, canvas_vf, atol=1), "Slice writeback line mismatch"


def test_slice_writeback_nested():
    """Test nested slices (slice of a slice)"""
    width, height = 200, 200
    color = (0, 255, 0)  # green

    # OpenCV
    canvas_ocv = np.zeros((height, width, 3), dtype=np.uint8)
    # Get a slice, then a slice of that slice
    quadrant = canvas_ocv[0:100, 0:100]
    sub_quadrant = quadrant[25:75, 25:75]
    ocv_cv2.rectangle(sub_quadrant, (10, 10), (40, 40), color, -1)

    # Vidformer
    canvas_vf = vf_cv2.zeros((height, width, 3))
    quadrant_vf = canvas_vf[0:100, 0:100]
    sub_quadrant_vf = quadrant_vf[25:75, 25:75]
    vf_cv2.rectangle(sub_quadrant_vf, (10, 10), (40, 40), color, -1)
    canvas_vf = canvas_vf.numpy()

    # Compare - they should match
    assert np.allclose(canvas_ocv, canvas_vf, atol=1), "Nested slice writeback mismatch"
