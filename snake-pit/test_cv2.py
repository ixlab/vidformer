import os
import re

import cv2 as ocv_cv2
import vidformer.cv2 as vf_cv2

VID_PATH = "../tos_720p.mp4"
TMP_PATH = "tmp.mp4"


def test_constants():
    assert ocv_cv2.CAP_PROP_POS_MSEC == vf_cv2.CAP_PROP_POS_MSEC
    assert ocv_cv2.CAP_PROP_POS_FRAMES == vf_cv2.CAP_PROP_POS_FRAMES
    assert ocv_cv2.CAP_PROP_FRAME_WIDTH == vf_cv2.CAP_PROP_FRAME_WIDTH
    assert ocv_cv2.CAP_PROP_FRAME_HEIGHT == vf_cv2.CAP_PROP_FRAME_HEIGHT
    assert ocv_cv2.CAP_PROP_FPS == vf_cv2.CAP_PROP_FPS

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

    cap = cv2.VideoCapture(VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
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
    cap = cv2.VideoCapture(VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    assert width == 1280
    assert height == 720

    out = cv2.VideoWriter(
        TMP_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
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

    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)


def test_rw_ocv():
    rw(ocv_cv2)


def test_rw_vf():
    rw(vf_cv2)


def test_numpy():
    import vidformer.cv2 as vf_cv2
    import numpy as np

    img = vf_cv2.imread("apollo.jpg")
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
    cap = vf_cv2.VideoCapture(VID_PATH)
    assert cap.isOpened()

    cap.set(vf_cv2.CAP_PROP_POS_FRAMES, 1000)
    ret, frame = cap.read()

    frame_np = frame.numpy()
    assert isinstance(frame_np, np.ndarray)
    assert frame_np.shape[0] == 720
    assert frame_np.shape[1] == 1280
    assert frame_np.shape[2] == 3


def test_vidplay():
    import vidformer as vf
    import vidformer.cv2 as cv2

    cap = cv2.VideoCapture(VID_PATH)
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


def rectangle(cv2):
    cap = cv2.VideoCapture(VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        TMP_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
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

    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)


def test_rectangle_ocv():
    rectangle(ocv_cv2)


def test_rectangle_vf():
    rectangle(vf_cv2)


def putText(cv2):
    cap = cv2.VideoCapture(VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        TMP_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
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

    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)


def test_text_ocv():
    putText(ocv_cv2)


def test_text_vf():
    putText(vf_cv2)


def arrowedLine(cv2):
    cap = cv2.VideoCapture(VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        TMP_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
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

    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)


def test_arrowedLine_ocv():
    arrowedLine(ocv_cv2)


def test_arrowedLine_vf():
    arrowedLine(vf_cv2)


def line(cv2):
    cap = cv2.VideoCapture(VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        TMP_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
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

    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)


def test_line_ocv():
    line(ocv_cv2)


def test_line_vf():
    line(vf_cv2)


def circle(cv2):
    cap = cv2.VideoCapture(VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        TMP_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
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

    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)


def test_circle_ocv():
    circle(ocv_cv2)


def test_circle_vf():
    circle(vf_cv2)


def seek(cv2):
    # seek to 4 different places, two of which with msec and two with frames; read 3 seconds at each place

    cap = cv2.VideoCapture(VID_PATH)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        TMP_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
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

    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)


def test_seek_ocv():
    seek(ocv_cv2)


def test_seek_vf():
    seek(vf_cv2)


def test_getFontScaleFromHeight():
    import vidformer.cv2 as vf_cv2
    import cv2 as ocv_cv2

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
    import vidformer.cv2 as vf_cv2
    import cv2 as ocv_cv2

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

    cap1 = cv2.VideoCapture(VID_PATH)
    assert cap1.isOpened()

    cap2 = cv2.VideoCapture(VID_PATH)
    assert cap2.isOpened()

    fps = cap1.get(cv2.CAP_PROP_FPS)
    width = int(cap1.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap1.get(cv2.CAP_PROP_FRAME_HEIGHT))
    cap2.set(cv2.CAP_PROP_POS_MSEC, 1000)

    out = cv2.VideoWriter(
        TMP_PATH, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
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

    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)


def test_addWeighted_ocv():
    addWeighted(ocv_cv2)


def test_addWeighted_vf():
    addWeighted(vf_cv2)


def test_imread():
    import vidformer.cv2 as vf_cv2

    img = vf_cv2.imread("apollo.jpg")

    assert img._fmt["width"] == 3912
    assert img._fmt["height"] == 3936
    assert img._fmt["pix_fmt"] == "yuvj444p"


def imread(cv2):
    img = cv2.imread("apollo.jpg")

    assert img.shape[0] == 3936
    assert img.shape[1] == 3912
    assert img.shape[2] == 3


def test_imread_ocv():
    imread(ocv_cv2)


def test_imread_vf():
    imread(vf_cv2)


def imwrite(cv2):
    # from apollo.jpg
    img = cv2.imread("apollo.jpg")

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
    cap = cv2.VideoCapture(VID_PATH)
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


def test_imwrite_ocv():
    imwrite(ocv_cv2)


def test_imwrite_vf():
    imwrite(vf_cv2)
