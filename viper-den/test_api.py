import requests
import re
import pytest
import random


ENDPOINT = "http://localhost:8080/"
TOS_SOURCE = {
    "name": "../tos_720p.mp4",
    "stream_idx": 0,
    "storage_service": "fs",
    "storage_config": {"root": "."},
}

EXAMPLE_SPEC = {
    "width": 1280,
    "height": 720,
    "pix_fmt": "yuv420p",
    "vod_segment_length": [2, 1],
    "ready_hook": None,
    "steer_hook": None,
}


def test_version():
    response = requests.get(ENDPOINT)
    response.raise_for_status()
    assert re.match(r"vidformer-igni v\d+\.\d+\.\d+", response.text)


def _create_tos_source():
    response = requests.post(ENDPOINT + "v2/source", json=TOS_SOURCE)
    response.raise_for_status()
    resp = response.json()
    assert resp["status"] == "ok"
    return resp["id"]


def test_create_source():
    _create_tos_source()


def test_get_source():
    source_id = _create_tos_source()

    source = _get_source(source_id)
    assert source["id"] == source_id
    assert source["name"] == "../tos_720p.mp4"
    assert source["stream_idx"] == 0
    assert source["storage_service"] == "fs"
    assert source["storage_config"]["root"] == "."
    assert source["width"] == 1280
    assert source["height"] == 720
    assert source["pix_fmt"] == "yuv420p"
    assert len(source["ts"]) == 17616
    for ts in source["ts"]:
        assert len(ts) == 3
        assert isinstance(ts[0], int)
        assert isinstance(ts[1], int)
        assert isinstance(ts[2], bool)


def _get_source(source_id):
    response = requests.get(ENDPOINT + "v2/source/" + source_id)
    if response.status_code != 200:
        response.raise_for_status()
    resp = response.json()
    return resp


def _create_example_spec():
    response = requests.post(ENDPOINT + "v2/spec", json=EXAMPLE_SPEC)
    response.raise_for_status()
    resp = response.json()
    assert resp["status"] == "ok"
    return resp["id"]


def test_create_spec():
    _create_example_spec()


def test_get_spec():
    spec_id = _create_example_spec()

    spec = _get_spec(spec_id)
    assert spec["id"] == spec_id
    assert spec["width"] == 1280
    assert spec["height"] == 720
    assert spec["pix_fmt"] == "yuv420p"
    assert spec["vod_segment_length"] == [2, 1]
    assert spec["ready_hook"] == None
    assert spec["steer_hook"] == None
    assert spec["playlist"] == ENDPOINT + "vod/" + spec_id + "/playlist.m3u8"
    assert spec["applied_parts"] == 0
    assert spec["terminated"] == False
    assert spec["closed"] == False


def _get_spec(spec_id):
    response = requests.get(ENDPOINT + "v2/spec/" + spec_id)
    if response.status_code != 200:
        response.raise_for_status()
    resp = response.json()
    return resp


def _sample_frame_expr(source_id):
    frame_expr = {
        "Filter": {
            "name": "Scale",
            "args": [
                {
                    "Frame": {
                        "Filter": {
                            "name": "cv2.rectangle",
                            "args": [
                                {
                                    "Frame": {
                                        "Filter": {
                                            "name": "Scale",
                                            "args": [
                                                {
                                                    "Frame": {
                                                        "Source": {
                                                            "video": source_id,
                                                            "index": {"ILoc": 0},
                                                        }
                                                    }
                                                }
                                            ],
                                            "kwargs": {
                                                "pix_fmt": {"Data": {"String": "rgb24"}}
                                            },
                                        }
                                    }
                                },
                                {"Data": {"List": [{"Int": 100}, {"Int": 100}]}},
                                {"Data": {"List": [{"Int": 200}, {"Int": 200}]}},
                                {
                                    "Data": {
                                        "List": [
                                            {"Float": 0.0},
                                            {"Float": 255.0},
                                            {"Float": 0.0},
                                            {"Float": 255.0},
                                        ]
                                    }
                                },
                                {"Data": {"Int": 3}},
                            ],
                            "kwargs": {},
                        }
                    }
                }
            ],
            "kwargs": {"pix_fmt": {"Data": {"String": "yuv420p"}}},
        }
    }
    return frame_expr


def _push_frames(spec_id, source_id, ts, pos, terminal):
    frame_expr = _sample_frame_expr(source_id)
    frames = []
    for t in ts:
        frames.append([t, frame_expr])
    req = {"pos": pos, "terminal": terminal, "frames": frames}
    response = requests.post(ENDPOINT + "v2/spec/" + spec_id + "/part", json=req)
    response.raise_for_status()
    resp = response.json()
    assert len(resp) == 1
    assert resp["status"] == "ok"


def test_push_part():
    source_id = _create_tos_source()
    spec_id = _create_example_spec()
    ts = [[0, 30]]
    _push_frames(spec_id, source_id, ts, 0, False)

    spec = _get_spec(spec_id)
    assert spec["applied_parts"] == 1
    assert spec["terminated"] == False


def test_push_two_parts_backwards():
    source_id = _create_tos_source()
    spec_id = _create_example_spec()

    ts = [[3, 30], [4, 30], [5, 30]]
    _push_frames(spec_id, source_id, ts, 1, False)
    spec = _get_spec(spec_id)
    assert spec["applied_parts"] == 0
    assert spec["terminated"] == False

    ts = [[0, 30], [1, 30], [2, 30]]
    _push_frames(spec_id, source_id, ts, 0, False)
    spec = _get_spec(spec_id)
    assert spec["applied_parts"] == 2
    assert spec["terminated"] == False


def test_terminate():
    source_id = _create_tos_source()
    spec_id = _create_example_spec()
    ts = [[0, 30], [1, 30], [2, 30]]
    _push_frames(spec_id, source_id, ts, 0, True)
    spec = _get_spec(spec_id)
    assert spec["applied_parts"] == 1
    assert spec["terminated"] == True


def test_terminate_delayed():
    source_id = _create_tos_source()
    spec_id = _create_example_spec()

    ts = [[3, 30], [4, 30], [5, 30]]
    _push_frames(spec_id, source_id, ts, 1, True)
    spec = _get_spec(spec_id)
    assert spec["applied_parts"] == 0
    assert spec["terminated"] == False

    ts = [[0, 30], [1, 30], [2, 30]]
    _push_frames(spec_id, source_id, ts, 0, False)
    spec = _get_spec(spec_id)
    assert spec["applied_parts"] == 2
    assert spec["terminated"] == True


def test_empty_playlist_endpoint():
    spec_id = _create_example_spec()
    spec = _get_spec(spec_id)

    playlist_url = spec["playlist"]
    assert playlist_url == f"{ENDPOINT}vod/{spec_id}/playlist.m3u8"
    response = requests.get(playlist_url)
    response.raise_for_status()
    assert (
        response.text
        == f"#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=640000\n{ENDPOINT}vod/{spec_id}/stream.m3u8\n"
    )
    assert response.headers["Content-Type"].lower() == "application/vnd.apple.mpegurl"


def test_empty_stream_endpoint():
    spec_id = _create_example_spec()

    stream_url = f"{ENDPOINT}vod/{spec_id}/stream.m3u8"
    response = requests.get(stream_url)
    response.raise_for_status()
    assert (
        response.text
        == f"""#EXTM3U
#EXT-X-PLAYLIST-TYPE:VOD
#EXT-X-TARGETDURATION:2
#EXT-X-VERSION:4
#EXT-X-MEDIA-SEQUENCE:0
#EXT-X-ENDLIST
"""
    )
    assert response.headers["Content-Type"].lower() == "application/vnd.apple.mpegurl"


def test_single_segment_stream_endpoint():
    source_id = _create_tos_source()
    spec_id = _create_example_spec()
    ts = [[i, 30] for i in range(60)]
    _push_frames(spec_id, source_id, ts, 0, True)

    stream_url = f"{ENDPOINT}vod/{spec_id}/stream.m3u8"
    response = requests.get(stream_url)
    response.raise_for_status()
    assert (
        response.text
        == f"""#EXTM3U
#EXT-X-PLAYLIST-TYPE:VOD
#EXT-X-TARGETDURATION:2
#EXT-X-VERSION:4
#EXT-X-MEDIA-SEQUENCE:0
#EXTINF:2.0,
{ENDPOINT}vod/{spec_id}/segment-0.ts
#EXT-X-ENDLIST
"""
    )
    assert response.headers["Content-Type"].lower() == "application/vnd.apple.mpegurl"


def test_single_segment():
    source_id = _create_tos_source()
    spec_id = _create_example_spec()
    ts = [[i, 30] for i in range(60)]
    _push_frames(spec_id, source_id, ts, 0, True)

    segment_0_url = f"{ENDPOINT}vod/{spec_id}/segment-0.ts"
    response = requests.get(segment_0_url)
    response.raise_for_status()
    assert response.headers["Content-Type"].lower() == "video/mp2t"


def _count_segments(spec_id):
    stream_url = f"{ENDPOINT}vod/{spec_id}/stream.m3u8"
    response = requests.get(stream_url)
    response.raise_for_status()
    txt = response.text

    prefix = "#EXTM3U\n#EXT-X-PLAYLIST-TYPE:VOD\n#EXT-X-TARGETDURATION:2\n#EXT-X-VERSION:4\n#EXT-X-MEDIA-SEQUENCE:0\n"
    assert txt.startswith(prefix)
    suffix = "\n#EXT-X-ENDLIST\n"
    assert txt.endswith(suffix)

    body = txt[len(prefix) : -len(suffix)]
    if body == "":
        return 0

    lines = body.split("\n")
    assert len(lines) % 2 == 0
    for i, line in enumerate(lines):
        if i % 2 == 0:
            assert line == f"#EXTINF:2.0,"
        else:
            assert re.match(r".*/segment-\d+\.ts", line)
    return len(lines) // 2


@pytest.mark.parametrize("fps", [24, 25, 30, 60])
def test_multiple_segments_in_order(fps):
    source_id = _create_tos_source()
    spec_id = _create_example_spec()

    for i in range(500):
        ts = [[i, fps]]
        _push_frames(spec_id, source_id, ts, i, False)
        assert _count_segments(spec_id) == i // (fps * 2)

    # Terminate
    ts = []
    _push_frames(spec_id, source_id, ts, 500, True)
    expected = 500 // (fps * 2)
    if 500 % (fps * 2) > 0:
        expected += 1
    assert _count_segments(spec_id) == expected


@pytest.mark.parametrize("fps", [24, 25, 30, 60])
def test_multiple_segments_random_order(fps):
    source_id = _create_tos_source()
    spec_id = _create_example_spec()

    pushes = []
    for i in range(500):
        ts = [[i, fps]]
        pushes.append((i, ts, False))
    pushes.append((500, [], True))
    not_pushed_i = set(range(501))

    random.shuffle(pushes)

    for i, ts, term in pushes:
        _push_frames(spec_id, source_id, ts, i, term)
        not_pushed_i.remove(i)

        if len(not_pushed_i) == 0:
            # Terminated
            expected = 500 // (fps * 2)
            if 500 % (fps * 2) > 0:
                expected += 1
            assert _count_segments(spec_id) == expected
        else:
            lowest_contiguous = max(1, min(not_pushed_i)) - 1
            expected = lowest_contiguous // (fps * 2)
            assert _count_segments(spec_id) == expected
