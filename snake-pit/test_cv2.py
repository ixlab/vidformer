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
