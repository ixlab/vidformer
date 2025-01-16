import vidformer.cv2 as cv2
import vidformer.igni as vf_igni
from fractions import Fraction

ENDPOINT = "http://localhost:8080/v2"
API_KEY = "test"


def test_connect():
    server = vf_igni.IgniServer(ENDPOINT, API_KEY)
    cv2.set_cv2_server(server)

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
    server = vf_igni.IgniServer(ENDPOINT, API_KEY)
    cv2.set_cv2_server(server)

    cap = cv2.VideoCapture("https://f.dominik.win/data/dve2/tos_720p.mp4")
    assert cap.isOpened()
    count = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break
        count += 1
    assert count == 17616


def test_write_video():
    server = vf_igni.IgniServer(ENDPOINT, API_KEY)
    cv2.set_cv2_server(server)

    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    cap = cv2.VideoCapture(tos)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        None, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height), batch_size=50
    )
    video_url = cv2.vidplay(out, method="link")
    assert type(video_url) == str

    video_url = cv2.vidplay(out.spec(), method="link")
    assert type(video_url) == str

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


def test_write_video_with_text():
    server = vf_igni.IgniServer(ENDPOINT, API_KEY)
    cv2.set_cv2_server(server)

    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    cap = cv2.VideoCapture(tos)
    assert cap.isOpened()

    fps = cap.get(cv2.CAP_PROP_FPS)
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    out = cv2.VideoWriter(
        None, cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height), batch_size=101
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
