import vidformer as vf
import vidformer.igni as igni
from fractions import Fraction

ENDPOINT = "http://localhost:8080/v2"
API_KEY = "test"


def test_connect():
    server = igni.IgniServer(ENDPOINT, API_KEY)


def test_create_source():
    server = igni.IgniServer(ENDPOINT, API_KEY)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    assert isinstance(tos, igni.IgniSource)

    assert len(tos) == 17616
    assert len(tos.ts()) == 17616
    for t in tos.ts():
        assert isinstance(t, Fraction)


def test_source():
    server = igni.IgniServer(ENDPOINT, API_KEY)

    # delete all specs first (since they depend on sources)
    specs = server.list_specs()
    for spec in specs:
        server.delete_spec(spec)

    # delete all sources first
    sources = server.list_sources()
    for source in sources:
        server.delete_source(source)

    # Get a source which doesn't exist
    tos = server.source("../tos_720p.mp4", 0, "fs", {"root": "."})
    assert isinstance(tos, igni.IgniSource)

    # Get a source which already exists
    tos2 = server.source("../tos_720p.mp4", 0, "fs", {"root": "."})
    assert isinstance(tos2, igni.IgniSource)

    assert tos.id() == tos2.id()

    # check only one source exists
    sources = server.list_sources()
    assert len(sources) == 1


def test_list_sources():
    server = igni.IgniServer(ENDPOINT, API_KEY)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    sources = server.list_sources()
    for source in sources:
        assert isinstance(source, str)
    assert tos.id() in sources


def test_search_source():
    server = igni.IgniServer(ENDPOINT, API_KEY)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    matching_sources = server.search_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    assert type(matching_sources) == list
    for source in matching_sources:
        assert isinstance(source, str)
    assert tos.id() in matching_sources


def test_delete_source():
    server = igni.IgniServer(ENDPOINT, API_KEY)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    server.delete_source(tos.id())
    sources = server.list_sources()
    assert tos.id() not in sources


def test_create_spec():
    server = igni.IgniServer(ENDPOINT, API_KEY)
    segment_legnth = Fraction(2, 1)
    spec_id = server.create_spec(1920, 1080, "yuv420p", segment_legnth, Fraction(30, 1))
    assert isinstance(spec_id, igni.IgniSpec)


def test_list_specs():
    server = igni.IgniServer(ENDPOINT, API_KEY)
    spec = server.create_spec(1920, 1080, "yuv420p", Fraction(2, 1), Fraction(30, 1))
    specs = server.list_specs()
    for s in specs:
        assert isinstance(s, str)
    assert spec.id() in specs


def test_delete_spec():
    server = igni.IgniServer(ENDPOINT, API_KEY)
    spec = server.create_spec(1920, 1080, "yuv420p", Fraction(2, 1), Fraction(30, 1))
    server.delete_spec(spec.id())
    specs = server.list_specs()
    assert spec.id() not in specs


def test_push_spec_part():
    server = igni.IgniServer(ENDPOINT, API_KEY)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    spec_id = server.create_spec(1920, 1080, "yuv420p", Fraction(2, 1), Fraction(30, 1))

    frames = []
    for i in range(100):
        t = Fraction(i, 30)
        f = tos.iloc[i]
        frames.append((t, f))

    server.push_spec_part(spec_id, 0, frames, True)


def test_push_spec_part_multiple():
    server = igni.IgniServer(ENDPOINT, API_KEY)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    spec_id = server.create_spec(1920, 1080, "yuv420p", Fraction(2, 1), Fraction(30, 1))

    for part in range(10):
        frames = []
        for i in range(10):
            t = Fraction(i, 30) + Fraction(part, 3)
            f = tos.iloc[i]
            frames.append((t, f))
        server.push_spec_part(spec_id, part * 10, frames, part == 9)
