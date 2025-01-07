import vidformer as vf
from fractions import Fraction

ENDPOINT = "http://localhost:8080/"


def test_connect():
    server = vf.IgniServer(ENDPOINT)


def test_source():
    server = vf.IgniServer(ENDPOINT)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    assert isinstance(tos, vf.IgniSource)

    assert len(tos) == 17616
    assert len(tos.ts()) == 17616
    for t in tos.ts():
        assert isinstance(t, Fraction)


def test_create_spec():
    server = vf.IgniServer(ENDPOINT)
    segment_legnth = Fraction(2, 1)
    spec_id = server.create_spec(1920, 1080, "yuv420p", segment_legnth)
    assert isinstance(spec_id, str)


def test_push_spec_part():
    server = vf.IgniServer(ENDPOINT)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    spec_id = server.create_spec(1920, 1080, "yuv420p", Fraction(2, 1))

    frames = []
    for i in range(100):
        t = Fraction(i, 30)
        f = tos.iloc[i]
        frames.append((t, f))

    server.push_spec_part(spec_id, 0, frames, True)


def test_push_spec_part_multiple():
    server = vf.IgniServer(ENDPOINT)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    spec_id = server.create_spec(1920, 1080, "yuv420p", Fraction(2, 1))

    for pos in range(10):
        frames = []
        for i in range(10):
            t = Fraction(i, 30) + Fraction(pos, 3)
            f = tos.iloc[i]
            frames.append((t, f))
        server.push_spec_part(spec_id, pos, frames, pos == 9)