import vidformer as vf
import vidformer.igni as igni
from fractions import Fraction

ENDPOINT = "http://localhost:8080"


def test_connect():
    server = igni.IgniServer(ENDPOINT)


def test_source():
    server = igni.IgniServer(ENDPOINT)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    assert isinstance(tos, igni.IgniSource)

    assert len(tos) == 17616
    assert len(tos.ts()) == 17616
    for t in tos.ts():
        assert isinstance(t, Fraction)


def test_create_spec():
    server = igni.IgniServer(ENDPOINT)
    segment_legnth = Fraction(2, 1)
    spec_id = server.create_spec(1920, 1080, "yuv420p", segment_legnth, Fraction(30, 1))
    assert isinstance(spec_id, igni.IgniSpec)


def test_push_spec_part():
    server = igni.IgniServer(ENDPOINT)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    spec_id = server.create_spec(1920, 1080, "yuv420p", Fraction(2, 1), Fraction(30, 1))

    frames = []
    for i in range(100):
        t = Fraction(i, 30)
        f = tos.iloc[i]
        frames.append((t, f))

    server.push_spec_part(spec_id, 0, frames, True)


def test_push_spec_part_multiple():
    server = igni.IgniServer(ENDPOINT)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    spec_id = server.create_spec(1920, 1080, "yuv420p", Fraction(2, 1), Fraction(30, 1))

    for part in range(10):
        frames = []
        for i in range(10):
            t = Fraction(i, 30) + Fraction(part, 3)
            f = tos.iloc[i]
            frames.append((t, f))
        server.push_spec_part(spec_id, part * 10, frames, part == 9)
