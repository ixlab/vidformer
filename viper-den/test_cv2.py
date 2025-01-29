import pytest
import numpy as np

import vidformer as vf
import vidformer.cv2 as cv2

import cv2 as ocv_cv2

ENDPOINT = "http://localhost:8080/v2"
API_KEY = "test"


def test_connect():
    server = vf.IgniServer(ENDPOINT, API_KEY)
    cv2.set_server(server)

    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    cap = cv2.VideoCapture(tos)
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
    server = vf.IgniServer(ENDPOINT, API_KEY)
    cv2.set_server(server)

    cap = cv2.VideoCapture("https://f.dominik.win/data/dve2/tos_720p.mp4")
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
    server = vf.IgniServer(ENDPOINT, API_KEY)
    cv2.set_server(server)

    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    cap = cv2.VideoCapture(tos)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        None,
        cv2.VideoWriter_fourcc(*"mp4v"),
        fps,
        (height, width),
        batch_size=50,
        ttl=ttl,
    )
    video_url = cv2.vidplay(out, method="link")
    assert type(video_url) is str

    video_url = cv2.vidplay(out.spec(), method="link")
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
    server = vf.IgniServer(ENDPOINT, API_KEY)
    cv2.set_server(server)

    img = cv2.imread("https://f.dominik.win/data/dve2/apollo.jpg")
    assert type(img) is cv2.Frame
    assert img._fmt["width"] == 3912
    assert img._fmt["height"] == 3936
    assert img._fmt["pix_fmt"] == "yuvj444p"


def test_write_video_with_text():
    server = vf.IgniServer(ENDPOINT, API_KEY)
    cv2.set_server(server)

    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    cap = cv2.VideoCapture(tos)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        None, cv2.VideoWriter_fourcc(*"mp4v"), fps, (height, width), batch_size=101
    )

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break
        cv2.putText(
            frame,
            "Hello, world!",
            (50, 50),
            cv2.FONT_HERSHEY_SIMPLEX,
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
    server = vf.IgniServer(ENDPOINT, API_KEY)
    cv2.set_server(server)

    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    cap = cv2.VideoCapture(tos)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        None, cv2.VideoWriter_fourcc(*"mp4v"), fps, (height, width), batch_size=101
    )

    count = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break

        # Draw 25 rectangles
        for i in range(50):
            cv2.rectangle(
                frame,
                (i * 10, i * 10),
                (i * 10 + 100, i * 10 + 100),
                (255, 0, 0),
                2,
            )

        # Draw 25 circles
        for i in range(50):
            cv2.circle(
                frame,
                (i * 10, i * 10),
                50,
                (0, 255, 0),
                2,
            )

        # Draw 25 lines
        for i in range(50):
            cv2.line(
                frame,
                (i * 10, i * 10),
                (i * 10 + 100, i * 10 + 100),
                (0, 0, 255),
                2,
            )

        # Draw 25 texts
        for i in range(50):
            cv2.putText(
                frame,
                "Hello, world!",
                (i * 10, i * 10),
                cv2.FONT_HERSHEY_SIMPLEX,
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
    server = vf.IgniServer(ENDPOINT, API_KEY)
    cv2.set_server(server)

    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    cap = cv2.VideoCapture(tos)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        None,
        cv2.VideoWriter_fourcc(*"mp4v"),
        fps,
        (height, width),
        batch_size=50,
        compression=compression,
    )
    video_url = cv2.vidplay(out, method="link")
    assert type(video_url) is str

    video_url = cv2.vidplay(out.spec(), method="link")
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
    server = vf.IgniServer(ENDPOINT, API_KEY)
    cv2.set_server(server)

    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    cap = cv2.VideoCapture(tos)
    assert cap.isOpened()

    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    cap.set(cv2.CAP_PROP_POS_FRAMES, 100)

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


def test_numpy_gray8():
    server = vf.IgniServer(ENDPOINT, API_KEY)
    cv2.set_server(server)

    img = cv2.zeros((100, 150, 1), dtype=np.uint8)
    assert type(img) is cv2.Frame
    assert img._fmt["width"] == 150
    assert img._fmt["height"] == 100
    assert img._fmt["pix_fmt"] == "gray8"
    assert img.shape == (100, 150, 1)

    img_numpy = img.numpy()
    assert type(img_numpy) is np.ndarray
    assert img_numpy.dtype == np.uint8
    assert img_numpy.shape == (100, 150, 1)

    assert np.all(img_numpy == 0)
