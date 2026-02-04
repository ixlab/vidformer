import pytest
import numpy as np
import os

import vidformer as vf
import vidformer.cv2 as vf_cv2

import cv2 as ocv_cv2

CWD_PREFIX = "../snake-pit/"

TEST_VID_PATH = "../tos_720p.mp4"
TEST_IMG_PATH = "apollo.jpg"
TMP_VID_PATH = "tmp.mp4"
FFPROBE_PATH = "../ffmpeg/build/bin/ffprobe"


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


def test_connect():
    cap = vf_cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()
    count = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break
        count += 1
    cap.release()

    assert count == 17616


def test_access_video_by_http_url():
    cap = vf_cv2.VideoCapture("https://f.dominik.win/data/dve2/tos_720p.mp4")
    assert cap.isOpened()
    count = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break
        count += 1
    assert count == 17616


def test_access_video_by_igni_source():
    server = tos = vf_cv2.get_server()
    assert type(server) is vf.Server
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    cap = vf_cv2.VideoCapture(tos)
    assert cap.isOpened()
    count = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break
        count += 1
    assert count == 17616


@pytest.mark.parametrize("ttl", [None, 10, 10**7])
def test_write_video(ttl):
    cap = vf_cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(vf_cv2.CAP_PROP_FPS)
    width = int(cap.get(vf_cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(vf_cv2.CAP_PROP_FRAME_HEIGHT))

    out = vf_cv2.VideoWriter(
        None,
        vf_cv2.VideoWriter_fourcc(*"mp4v"),
        fps,
        (width, height),
        batch_size=50,
        ttl=ttl,
    )
    video_url = vf_cv2.vidplay(out, method="link")
    assert type(video_url) is str

    video_url = vf_cv2.vidplay(out.spec(), method="link")
    assert type(video_url) is str

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break
        out.write(frame)
        count += 1
        if count == 500:
            break
    cap.release()
    out.release()


def test_imread():
    img = vf_cv2.imread("https://f.dominik.win/data/dve2/apollo.jpg")
    assert type(img) is vf_cv2.Frame
    assert img._fmt["width"] == 3912
    assert img._fmt["height"] == 3936
    assert img._fmt["pix_fmt"] == "yuvj444p"


def test_write_video_with_text():
    cap = vf_cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(vf_cv2.CAP_PROP_FPS)
    width = int(cap.get(vf_cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(vf_cv2.CAP_PROP_FRAME_HEIGHT))

    out = vf_cv2.VideoWriter(
        None, vf_cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height), batch_size=101
    )

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break
        vf_cv2.putText(
            frame,
            "Hello, world!",
            (50, 50),
            vf_cv2.FONT_HERSHEY_SIMPLEX,
            1,
            (255, 255, 255),
            2,
        )
        out.write(frame)
        count += 1
        if count == 500:
            break
    cap.release()
    out.release()


def test_deeply_nested_filters():
    cap = vf_cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(vf_cv2.CAP_PROP_FPS)
    width = int(cap.get(vf_cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(vf_cv2.CAP_PROP_FRAME_HEIGHT))

    out = vf_cv2.VideoWriter(
        None, vf_cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height), batch_size=101
    )

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break

        # Draw 25 rectangles
        for i in range(50):
            vf_cv2.rectangle(
                frame,
                (i * 10, i * 10),
                (i * 10 + 100, i * 10 + 100),
                (255, 0, 0),
                2,
            )

        # Draw 25 circles
        for i in range(50):
            vf_cv2.circle(
                frame,
                (i * 10, i * 10),
                50,
                (0, 255, 0),
                2,
            )

        # Draw 25 lines
        for i in range(50):
            vf_cv2.line(
                frame,
                (i * 10, i * 10),
                (i * 10 + 100, i * 10 + 100),
                (0, 0, 255),
                2,
            )

        # Draw 25 texts
        for i in range(50):
            vf_cv2.putText(
                frame,
                "Hello, world!",
                (i * 10, i * 10),
                vf_cv2.FONT_HERSHEY_SIMPLEX,
                1,
                (255, 255, 255),
                2,
            )

        out.write(frame)
        count += 1
        if count == 5:
            break

    cap.release()
    out.release()


@pytest.mark.parametrize("compression", [None, "gzip"])
def test_block_compression(compression):
    cap = vf_cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(vf_cv2.CAP_PROP_FPS)
    width = int(cap.get(vf_cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(vf_cv2.CAP_PROP_FRAME_HEIGHT))

    out = vf_cv2.VideoWriter(
        None,
        vf_cv2.VideoWriter_fourcc(*"mp4v"),
        fps,
        (width, height),
        batch_size=50,
        compression=compression,
    )
    video_url = vf_cv2.vidplay(out, method="link")
    assert type(video_url) is str

    video_url = vf_cv2.vidplay(out.spec(), method="link")
    assert type(video_url) is str

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break
        out.write(frame)
        count += 1
        if count == 500:
            break
    cap.release()
    out.release()


def test_numpy():
    cap = vf_cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    width = int(cap.get(vf_cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(vf_cv2.CAP_PROP_FRAME_HEIGHT))

    cap.set(vf_cv2.CAP_PROP_POS_FRAMES, 100)

    i = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break

        frame_numpy = frame.numpy()
        assert type(frame_numpy) is np.ndarray
        assert frame_numpy.shape == (height, width, 3)
        assert frame_numpy.dtype == np.uint8

        i += 1
        if i == 10:
            break


def test_numpy_gray():
    img = vf_cv2.zeros((100, 150, 1), dtype=np.uint8)
    assert type(img) is vf_cv2.Frame
    assert img._fmt["width"] == 150
    assert img._fmt["height"] == 100
    assert img._fmt["pix_fmt"] == "gray"
    assert img.shape == (100, 150, 1)

    img_numpy = img.numpy()
    assert type(img_numpy) is np.ndarray
    assert img_numpy.dtype == np.uint8
    assert img_numpy.shape == (100, 150, 1)

    assert np.all(img_numpy == 0)


def test_set_color_with_mask():
    # TODO: Eventually test this with a non-zero mask
    mymask = vf_cv2.zeros((100, 150, 1), dtype=np.uint8)
    assert type(mymask) is vf_cv2.Frame
    assert mymask._fmt["width"] == 150
    assert mymask._fmt["height"] == 100
    assert mymask._fmt["pix_fmt"] == "gray"

    myframe = vf_cv2.zeros((100, 150, 3), dtype=np.uint8)
    assert type(myframe) is vf_cv2.Frame
    assert myframe._fmt["width"] == 150
    assert myframe._fmt["height"] == 100
    assert myframe._fmt["pix_fmt"] == "rgb24"

    myframe[mymask] = [255, 0, 0]  # Blue
    myframe_np = myframe.numpy()
    assert type(myframe_np) is np.ndarray
    assert myframe_np.dtype == np.uint8
    assert myframe_np.shape == (100, 150, 3)


def test_initial_batch_size():
    """Test that initial_batch_size parameter works for faster first push."""
    out = vf_cv2.VideoWriter(
        None,
        vf_cv2.VideoWriter_fourcc(*"mp4v"),
        30,
        (100, 100),
        batch_size=1024,
        initial_batch_size=10,
    )

    for i in range(50):
        frame = vf_cv2.zeros((100, 100, 3), dtype=np.uint8)
        vf_cv2.putText(
            frame, str(i), (10, 50), vf_cv2.FONT_HERSHEY_SIMPLEX, 1, (255, 255, 255), 2
        )
        out.write(frame)

    out.release()
