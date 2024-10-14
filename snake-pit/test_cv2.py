import os

import cv2 as ocv_cv2
import vidformer.cv2 as vf_cv2

VID_PATH = "../tos_720p.mp4"
TMP_PATH = "tmp.mp4"


def rw(cv2):
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


def text(cv2):
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
    text(ocv_cv2)


def test_text_vf():
    text(vf_cv2)


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
