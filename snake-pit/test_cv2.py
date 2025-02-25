import os
import re

import cv2 as ocv_cv2
import numpy as np
import pytest
import subprocess


import vidformer.cv2 as vf_cv2

TEST_VID_PATH = "../tos_720p.mp4"
TEST_IMG_PATH = "apollo.jpg"
TMP_VID_PATH = "tmp.mp4"
FFPROBE_PATH = "../ffmpeg/build/bin/ffprobe"


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

    out = cv2.VideoWriter(
        TMP_VID_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
    )

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

    assert os.path.exists(TMP_VID_PATH)
    assert ffprobe_count_frames(TMP_VID_PATH) == count
    fmt = ffprobe_fmt(TMP_VID_PATH)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(TMP_VID_PATH)


def test_rw_ocv():
    rw(ocv_cv2)


def test_rw_vf():
    rw(vf_cv2)


def videowriter_numpy(cv2):
    # make up random numpy frames, write them to a video
    width, height = 300, 200

    out = cv2.VideoWriter(
        TMP_VID_PATH, cv2.VideoWriter_fourcc(*"mp4v"), 30, (width, height)
    )

    for i in range(3):
        frame = np.random.randint(0, 255, (height, width, 3), dtype=np.uint8)
        out.write(frame)
    out.release()

    assert os.path.exists(TMP_VID_PATH)
    os.remove(TMP_VID_PATH)


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
    assert re.match(r"http://localhost:\d+/\w+-\w+-\w+-\w+-\w+/stream.m3u8", link)

    # test vidplay on a VideoWriter
    link = cv2.vidplay(out, method="link")
    assert re.match(r"http://localhost:\d+/\w+-\w+-\w+-\w+-\w+/stream.m3u8", link)


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

    vf_cv2.imwrite("apollo_resized.png", frame_resized)
    assert os.path.exists("apollo_resized.png")
    fmt = ffprobe_fmt("apollo_resized.png")
    assert fmt["width"] == 300
    assert fmt["height"] == 250
    assert fmt["pix_fmt"] == "rgb24"
    os.remove("apollo_resized.png")


def test_resize_numpy():
    frame = np.random.randint(0, 255, (100, 200, 3), dtype=np.uint8)
    frame = vf_cv2.resize(frame, (300, 250))
    assert frame.shape[0] == 250
    assert frame.shape[1] == 300
    assert frame.shape[2] == 3

    vf_cv2.imwrite("random_resized.png", frame)
    assert os.path.exists("random_resized.png")
    os.remove("random_resized.png")


def rectangle(cv2):
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        TMP_VID_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
    )

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

    assert os.path.exists(TMP_VID_PATH)
    assert ffprobe_count_frames(TMP_VID_PATH) == count
    fmt = ffprobe_fmt(TMP_VID_PATH)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(TMP_VID_PATH)


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

    vf_cv2.imwrite("rectangle.png", frame)
    assert os.path.exists("rectangle.png")
    os.remove("rectangle.png")


def putText(cv2):
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        TMP_VID_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
    )

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

    assert os.path.exists(TMP_VID_PATH)
    assert ffprobe_count_frames(TMP_VID_PATH) == count
    fmt = ffprobe_fmt(TMP_VID_PATH)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(TMP_VID_PATH)


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

    vf_cv2.imwrite("text.png", frame)
    assert os.path.exists("text.png")
    os.remove("text.png")


def arrowedLine(cv2):
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        TMP_VID_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
    )

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

    assert os.path.exists(TMP_VID_PATH)
    os.path.exists(TMP_VID_PATH)
    fmt = ffprobe_fmt(TMP_VID_PATH)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(TMP_VID_PATH)


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

    vf_cv2.imwrite("arrowedLine.png", frame)
    assert os.path.exists("arrowedLine.png")
    os.remove("arrowedLine.png")


def line(cv2):
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        TMP_VID_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
    )

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

    assert os.path.exists(TMP_VID_PATH)
    os.path.exists(TMP_VID_PATH)
    fmt = ffprobe_fmt(TMP_VID_PATH)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(TMP_VID_PATH)


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

    vf_cv2.imwrite("line.png", frame)
    assert os.path.exists("line.png")
    os.remove("line.png")


def circle(cv2):
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        TMP_VID_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
    )

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

    assert os.path.exists(TMP_VID_PATH)
    os.path.exists(TMP_VID_PATH)
    fmt = ffprobe_fmt(TMP_VID_PATH)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(TMP_VID_PATH)


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

    out = cv2.VideoWriter(
        TMP_VID_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
    )

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

    assert os.path.exists(TMP_VID_PATH)
    os.path.exists(TMP_VID_PATH)
    fmt = ffprobe_fmt(TMP_VID_PATH)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(TMP_VID_PATH)


def test_ellipse_ocv():
    ellipse(ocv_cv2)


def test_ellipse_vf():
    ellipse(vf_cv2)


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

    vf_cv2.imwrite("circle.png", frame)
    assert os.path.exists("circle.png")
    os.remove("circle.png")


def seek(cv2):
    # seek to 4 different places, two of which with msec and two with frames; read 3 seconds at each place

    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        TMP_VID_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
    )

    count = 0
    while True:
        if count == 0:
            cap.set(cv2.CAP_PROP_POS_MSEC, 1000)
        elif count == 25:
            cap.set(cv2.CAP_PROP_POS_FRAMES, 1000)
        elif count == 50:
            cap.set(cv2.CAP_PROP_POS_MSEC, 20000)
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

    assert os.path.exists(TMP_VID_PATH)
    os.path.exists(TMP_VID_PATH)
    fmt = ffprobe_fmt(TMP_VID_PATH)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(TMP_VID_PATH)


def test_seek_ocv():
    seek(ocv_cv2)


def test_seek_vf():
    seek(vf_cv2)


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

    out = cv2.VideoWriter(
        TMP_VID_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
    )

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

    assert os.path.exists(TMP_VID_PATH)
    os.path.exists(TMP_VID_PATH)
    fmt = ffprobe_fmt(TMP_VID_PATH)
    assert fmt["width"] == width
    assert fmt["height"] == height
    assert fmt["pix_fmt"] == "yuv420p"
    os.remove(TMP_VID_PATH)


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

    vf_cv2.imwrite("addWeighted.png", frame)
    assert os.path.exists("addWeighted.png")
    os.remove("addWeighted.png")


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
    cv2.imwrite("apollo2.jpg", img)
    assert os.path.exists("apollo2.jpg")
    os.remove("apollo2.jpg")

    # jpeg
    cv2.imwrite("apollo2.jpeg", img)
    assert os.path.exists("apollo2.jpeg")
    os.remove("apollo2.jpeg")

    # png
    cv2.imwrite("apollo2.png", img)
    assert os.path.exists("apollo2.png")
    os.remove("apollo2.png")

    # from 1000th frame of tos_720p.mp4
    cap = cv2.VideoCapture(TEST_VID_PATH)
    assert cap.isOpened()

    cap.set(cv2.CAP_PROP_POS_FRAMES, 1000)
    ret, frame = cap.read()
    assert ret

    # jpg
    cv2.imwrite("tos_720p_1000.jpg", frame)
    assert os.path.exists("tos_720p_1000.jpg")
    os.remove("tos_720p_1000.jpg")

    # jpeg
    cv2.imwrite("tos_720p_1000.jpeg", frame)
    assert os.path.exists("tos_720p_1000.jpeg")
    os.remove("tos_720p_1000.jpeg")

    # png
    cv2.imwrite("tos_720p_1000.png", frame)
    assert os.path.exists("tos_720p_1000.png")
    os.remove("tos_720p_1000.png")


def test_imwrite_ocv():
    imwrite(ocv_cv2)


def test_imwrite_vf():
    imwrite(vf_cv2)


def imwrite_numpy(cv2):
    width, height = 300, 200

    red_image = np.zeros((height, width, 3), dtype=np.uint8)
    red_image[:, :] = (0, 0, 255)

    output_filename = "red_image.png"
    cv2.imwrite(output_filename, red_image)

    assert os.path.exists(output_filename)

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
    # use cv2 to write "apollo.png", because jpeg decoding can be lossy
    img = ocv_cv2.imread(TEST_IMG_PATH)
    ocv_cv2.imwrite("apollo.png", img)

    img1 = ocv_cv2.imread("apollo.png")
    img2 = vf_cv2.imread("apollo.png").numpy()

    assert img1.dtype == img2.dtype
    assert img1.shape == img2.shape
    assert np.all(img1 == img2)
    os.remove("apollo.png")


def test_frameify():
    # write a video with all white frames
    width, height = 300, 200
    out = vf_cv2.VideoWriter(
        TMP_VID_PATH, vf_cv2.VideoWriter_fourcc(*"mp4v"), 30, (width, height)
    )

    for i in range(3):
        frame = np.full((height, width, 3), 255, dtype=np.uint8)
        frame_shape = frame.shape

        frame = vf_cv2.frameify(frame)
        assert isinstance(frame, vf_cv2.Frame)
        assert frame.shape == frame_shape

        out.write(frame)

    out.release()
    assert os.path.exists(TMP_VID_PATH)
    os.path.exists(TMP_VID_PATH)
    os.remove(TMP_VID_PATH)


def test_frame_array_slicing_appolo():
    frame_orig = ocv_cv2.imread(TEST_IMG_PATH)[:1000, :1000]
    ocv_cv2.imwrite("apollo.png", frame_orig)
    assert os.path.exists("apollo.png")
    frame = vf_cv2.imread("apollo.png")

    assert frame.shape == frame_orig.shape

    frame = frame[500:600, 500:650]
    frame_orig = frame_orig[500:600, 500:650]

    assert frame.shape == frame_orig.shape
    assert np.all(frame.numpy() == frame_orig)
    os.remove("apollo.png")


def test_write_slice_apollo():
    frame_orig = ocv_cv2.imread(TEST_IMG_PATH)[:1000, :1000]
    ocv_cv2.imwrite("apollo.png", frame_orig)
    assert os.path.exists("apollo.png")
    frame = vf_cv2.imread("apollo.png")

    assert frame.shape == frame_orig.shape

    write_array = np.random.randint(0, 255, (100, 150, 3), dtype=np.uint8)

    frame[500:600, 500:650] = write_array
    frame_orig[500:600, 500:650] = write_array

    assert frame.shape == frame_orig.shape
    assert np.all(frame.numpy() == frame_orig)
    os.remove("apollo.png")


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
