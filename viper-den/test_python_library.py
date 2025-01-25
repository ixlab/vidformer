from fractions import Fraction

import vidformer as vf

ENDPOINT = "http://localhost:8080/v2"
API_KEY = "test"


def test_connect():
    _server = vf.IgniServer(ENDPOINT, API_KEY)


def test_create_source():
    server = vf.IgniServer(ENDPOINT, API_KEY)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    assert isinstance(tos, vf.IgniSource)

    assert len(tos) == 17616
    assert len(tos.ts()) == 17616
    for t in tos.ts():
        assert isinstance(t, Fraction)


def test_source():
    server = vf.IgniServer(ENDPOINT, API_KEY)

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
    assert isinstance(tos, vf.IgniSource)

    # Get a source which already exists
    tos2 = server.source("../tos_720p.mp4", 0, "fs", {"root": "."})
    assert isinstance(tos2, vf.IgniSource)

    assert tos.id() == tos2.id()

    # check only one source exists
    sources = server.list_sources()
    assert len(sources) == 1


def test_list_sources():
    server = vf.IgniServer(ENDPOINT, API_KEY)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    sources = server.list_sources()
    for source in sources:
        assert isinstance(source, str)
    assert tos.id() in sources


def test_search_source():
    server = vf.IgniServer(ENDPOINT, API_KEY)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    matching_sources = server.search_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    assert type(matching_sources) is list
    for source in matching_sources:
        assert isinstance(source, str)
    assert tos.id() in matching_sources


def test_delete_source():
    server = vf.IgniServer(ENDPOINT, API_KEY)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    server.delete_source(tos.id())
    sources = server.list_sources()
    assert tos.id() not in sources


def test_create_spec():
    server = vf.IgniServer(ENDPOINT, API_KEY)
    segment_legnth = Fraction(2, 1)
    spec_id = server.create_spec(1920, 1080, "yuv420p", segment_legnth, Fraction(30, 1))
    assert isinstance(spec_id, vf.IgniSpec)


def test_list_specs():
    server = vf.IgniServer(ENDPOINT, API_KEY)
    spec = server.create_spec(1920, 1080, "yuv420p", Fraction(2, 1), Fraction(30, 1))
    specs = server.list_specs()
    for s in specs:
        assert isinstance(s, str)
    assert spec.id() in specs


def test_delete_spec():
    server = vf.IgniServer(ENDPOINT, API_KEY)
    spec = server.create_spec(1920, 1080, "yuv420p", Fraction(2, 1), Fraction(30, 1))
    server.delete_spec(spec.id())
    specs = server.list_specs()
    assert spec.id() not in specs


def test_push_spec_part():
    server = vf.IgniServer(ENDPOINT, API_KEY)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    spec_id = server.create_spec(1920, 1080, "yuv420p", Fraction(2, 1), Fraction(30, 1))

    frames = []
    for i in range(100):
        t = Fraction(i, 30)
        f = tos.iloc[i]
        frames.append((t, f))

    server.push_spec_part(spec_id, 0, frames, True)


def test_push_spec_part_block():
    server = vf.IgniServer(ENDPOINT, API_KEY)
    tos = server.create_source("../tos_720p.mp4", 0, "fs", {"root": "."})
    spec_id = server.create_spec(1920, 1080, "yuv420p", Fraction(2, 1), Fraction(30, 1))

    fb = vf._FrameExpressionBlock()
    for i in range(100):
        fb.insert_frame(tos.iloc[i])

    server.push_spec_part_block(spec_id, 0, [fb], True)
