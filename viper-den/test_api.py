import requests
import re
import pytest
import random
import os
import subprocess as sp

ENDPOINT = "http://localhost:8080/"
AUTH_HEADERS = {"Authorization": "Bearer test"}

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
    "frame_rate": [30, 1],
    "ready_hook": None,
    "steer_hook": None,
}


def test_version():
    response = requests.get(ENDPOINT)
    response.raise_for_status()
    assert re.match(r"vidformer-igni v\d+\.\d+\.\d+", response.text)


def test_auth():
    response = requests.get(ENDPOINT + "v2/auth", headers=AUTH_HEADERS)
    response.raise_for_status()


def test_auth_err_no_header():
    response = requests.get(ENDPOINT + "v2/auth")
    assert response.status_code == 401
    assert response.text == "Unauthorized"


def test_auth_err_invalid_header():
    response = requests.get(
        ENDPOINT + "v2/auth", headers={"Authorization": "Bearer invalid"}
    )
    assert response.status_code == 401
    assert response.text == "Unauthorized"


def _create_tos_source():
    response = requests.post(
        ENDPOINT + "v2/source", json=TOS_SOURCE, headers=AUTH_HEADERS
    )
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


def test_error_get_source_not_exists():
    resp = requests.get(
        ENDPOINT + "v2/source/ca584794-54cd-4d65-9073-ebb88529708b",
        headers=AUTH_HEADERS,
    )
    assert resp.status_code == 404
    assert resp.text == "Not found"


def _get_source(source_id):
    response = requests.get(ENDPOINT + "v2/source/" + source_id, headers=AUTH_HEADERS)
    if response.status_code != 200:
        response.raise_for_status()
    resp = response.json()
    return resp


def test_list_sources():
    source_id = _create_tos_source()
    response = requests.get(ENDPOINT + "v2/source", headers=AUTH_HEADERS)
    response.raise_for_status()
    resp = response.json()
    assert type(resp) == list
    for sid in resp:
        assert type(sid) == str
    assert source_id in resp


def test_delete_source():
    source_id = _create_tos_source()
    response = requests.delete(
        ENDPOINT + "v2/source/" + source_id, headers=AUTH_HEADERS
    )
    response.raise_for_status()
    resp = response.json()
    assert resp["status"] == "ok"

    response = requests.get(ENDPOINT + "v2/source/" + source_id, headers=AUTH_HEADERS)
    assert response.status_code == 404
    assert response.text == "Not found"

    response = requests.get(ENDPOINT + "v2/source", headers=AUTH_HEADERS)
    response.raise_for_status()
    resp = response.json()
    assert source_id not in resp


def _create_example_spec(fps=30):
    req = EXAMPLE_SPEC.copy()
    req["frame_rate"] = [fps, 1]
    response = requests.post(ENDPOINT + "v2/spec", json=req, headers=AUTH_HEADERS)
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
    assert spec["terminated"] == False
    assert spec["closed"] == False
    assert spec["vod_endpoint"] == ENDPOINT + "vod/" + spec_id + "/"


def test_error_get_spec_not_exists():
    resp = requests.get(
        ENDPOINT + "v2/spec/ca584794-54cd-4d65-9073-ebb88529708b", headers=AUTH_HEADERS
    )
    assert resp.status_code == 404
    assert resp.text == "Not found"


def _get_spec(spec_id):
    response = requests.get(ENDPOINT + "v2/spec/" + spec_id, headers=AUTH_HEADERS)
    if response.status_code != 200:
        response.raise_for_status()
    resp = response.json()
    return resp


def test_list_specs():
    spec_id = _create_example_spec()
    response = requests.get(ENDPOINT + "v2/spec", headers=AUTH_HEADERS)
    response.raise_for_status()
    resp = response.json()
    assert type(resp) == list
    for sid in resp:
        assert type(sid) == str
    assert spec_id in resp


def test_delete_spec():
    spec_id = _create_example_spec()
    response = requests.delete(ENDPOINT + "v2/spec/" + spec_id, headers=AUTH_HEADERS)
    response.raise_for_status()
    resp = response.json()
    assert resp["status"] == "ok"

    response = requests.get(ENDPOINT + "v2/spec/" + spec_id, headers=AUTH_HEADERS)
    assert response.status_code == 404
    assert response.text == "Not found"

    response = requests.get(ENDPOINT + "v2/spec", headers=AUTH_HEADERS)
    response.raise_for_status()
    resp = response.json()
    assert spec_id not in resp


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
    response = requests.post(
        ENDPOINT + "v2/spec/" + spec_id + "/part", json=req, headers=AUTH_HEADERS
    )
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
    assert spec["frames_applied"] == 1
    assert spec["terminated"] == False


def test_error_push_part_empty():
    spec_id = _create_example_spec()
    ts = []
    req = {"pos": 0, "terminal": False, "frames": ts}
    response = requests.post(
        ENDPOINT + "v2/spec/" + spec_id + "/part", json=req, headers=AUTH_HEADERS
    )
    assert response.status_code == 400
    assert response.text == "Cannot push a non-terminal part with no frames"


def test_error_push_part_after_termination():
    source_id = _create_tos_source()
    spec_id = _create_example_spec()
    ts = [[0, 30]]
    _push_frames(spec_id, source_id, ts, 0, True)

    req = {
        "pos": 1,
        "terminal": False,
        "frames": [[[1, 30], _sample_frame_expr(source_id)]],
    }
    response = requests.post(
        ENDPOINT + "v2/spec/" + spec_id + "/part", json=req, headers=AUTH_HEADERS
    )
    assert response.status_code == 400
    assert response.text == "Can not push past the terminal frame"


def test_error_push_part_after_staged_termination():
    source_id = _create_tos_source()
    spec_id = _create_example_spec()
    ts = [[1, 30]]
    # Push a terminal part at pos 1; pos 0 is never pushed
    _push_frames(spec_id, source_id, ts, 1, True)

    req = {
        "pos": 2,
        "terminal": False,
        "frames": [[[1, 30], _sample_frame_expr(source_id)]],
    }
    response = requests.post(
        ENDPOINT + "v2/spec/" + spec_id + "/part", json=req, headers=AUTH_HEADERS
    )
    assert response.status_code == 400
    assert response.text == "Can not push past the terminal frame"


def test_error_push_two_parts_same_pos():
    source_id = _create_tos_source()
    spec_id = _create_example_spec()
    ts = [[0, 30]]
    _push_frames(spec_id, source_id, ts, 0, False)

    req = {
        "pos": 0,
        "terminal": False,
        "frames": [[[1, 30], _sample_frame_expr(source_id)]],
    }
    response = requests.post(
        ENDPOINT + "v2/spec/" + spec_id + "/part", json=req, headers=AUTH_HEADERS
    )
    assert response.status_code == 400
    assert response.text == "Can not push to an existing position (position 0)"


def test_push_two_parts_backwards():
    source_id = _create_tos_source()
    spec_id = _create_example_spec()

    ts = [[3, 30], [4, 30], [5, 30]]
    _push_frames(spec_id, source_id, ts, 3, False)
    spec = _get_spec(spec_id)
    assert spec["frames_applied"] == 0
    assert spec["terminated"] == False

    ts = [[0, 30], [1, 30], [2, 30]]
    _push_frames(spec_id, source_id, ts, 0, False)
    spec = _get_spec(spec_id)
    assert spec["frames_applied"] == 6
    assert spec["terminated"] == False


def test_terminate():
    source_id = _create_tos_source()
    spec_id = _create_example_spec()
    ts = [[0, 30], [1, 30], [2, 30]]
    _push_frames(spec_id, source_id, ts, 0, True)
    spec = _get_spec(spec_id)
    assert spec["frames_applied"] == 3
    assert spec["terminated"] == True


def test_terminate_delayed():
    source_id = _create_tos_source()
    spec_id = _create_example_spec()

    ts = [[3, 30], [4, 30], [5, 30]]
    _push_frames(spec_id, source_id, ts, 3, True)
    spec = _get_spec(spec_id)
    assert spec["frames_applied"] == 0
    assert not spec["terminated"]

    ts = [[0, 30], [1, 30], [2, 30]]
    _push_frames(spec_id, source_id, ts, 0, False)
    spec = _get_spec(spec_id)
    assert spec["frames_applied"] == 6
    assert spec["terminated"]


def test_status_endpoint():
    spec_id = _create_example_spec()

    status_url = f"{ENDPOINT}vod/{spec_id}/status"
    response = requests.get(status_url)
    response.raise_for_status()
    response = response.json()
    assert response["closed"] == False
    assert response["terminated"] == False
    assert response["ready"] == False

    # Push 60 frames (enough for 1 segment)
    source_id = _create_tos_source()
    ts = [[i, 30] for i in range(60)]
    _push_frames(spec_id, source_id, ts, 0, False)

    response = requests.get(status_url)
    response.raise_for_status()
    response = response.json()
    assert response["closed"] == False
    assert response["terminated"] == False
    assert response["ready"] == True

    # Terminate
    ts = []
    _push_frames(spec_id, source_id, ts, 60, True)

    response = requests.get(status_url)
    response.raise_for_status()
    response = response.json()
    assert response["closed"] == False
    assert response["terminated"] == True
    assert response["ready"] == True


def test_empty_playlist_endpoint():
    spec_id = _create_example_spec()
    spec = _get_spec(spec_id)

    vod_endpoint = spec["vod_endpoint"]
    assert vod_endpoint == f"{ENDPOINT}vod/{spec_id}/"
    playlist_url = f"{vod_endpoint}playlist.m3u8"
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
#EXT-X-PLAYLIST-TYPE:EVENT
#EXT-X-TARGETDURATION:2
#EXT-X-VERSION:4
#EXT-X-MEDIA-SEQUENCE:0
#EXT-X-START:TIME-OFFSET=0
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
#EXT-X-PLAYLIST-TYPE:EVENT
#EXT-X-TARGETDURATION:2
#EXT-X-VERSION:4
#EXT-X-MEDIA-SEQUENCE:0
#EXT-X-START:TIME-OFFSET=0
#EXTINF:2,
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

    prefix = "#EXTM3U\n#EXT-X-PLAYLIST-TYPE:EVENT\n#EXT-X-TARGETDURATION:2\n#EXT-X-VERSION:4\n#EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-START:TIME-OFFSET=0\n"
    assert txt.startswith(prefix)
    terminal = False
    if txt.endswith("\n#EXT-X-ENDLIST\n"):
        terminal = True
        txt = txt.replace("#EXT-X-ENDLIST\n", "")
    body = txt[len(prefix) :]
    body = body.strip().strip("\n")
    if body == "":
        return 0, terminal

    lines = body.split("\n")
    assert len(lines) % 2 == 0
    for i, line in enumerate(lines):
        if i % 2 == 0:
            assert line.startswith("#EXTINF:")
            assert line.endswith(",")
        else:
            assert re.match(r".*/segment-\d+\.ts", line)
    return len(lines) // 2, terminal


@pytest.mark.parametrize("fps", [24, 60])
def test_multiple_segments_in_order(fps):
    source_id = _create_tos_source()
    spec_id = _create_example_spec(fps)

    for i in range(250):
        ts = [[i, fps]]
        _push_frames(spec_id, source_id, ts, i, False)
        print(_count_segments(spec_id)[0])
        assert _count_segments(spec_id)[0] == (i + 1) // (fps * 2)
        assert _count_segments(spec_id)[1] == False

    # Terminate
    ts = []
    _push_frames(spec_id, source_id, ts, 250, True)
    expected = 250 // (fps * 2)
    if 250 % (fps * 2) > 0:
        expected += 1
    assert _count_segments(spec_id)[0] == expected
    assert _count_segments(spec_id)[1] == True


@pytest.mark.parametrize("fps", [25, 30])
def test_multiple_segments_random_order(fps):
    source_id = _create_tos_source()
    spec_id = _create_example_spec(fps)

    pushes = []
    for i in range(250):
        ts = [[i, fps]]
        pushes.append((i, ts, False))
    pushes.append((250, [], True))
    not_pushed_i = set(range(251))

    random.shuffle(pushes)

    for i, ts, term in pushes:
        _push_frames(spec_id, source_id, ts, i, term)
        not_pushed_i.remove(i)

        if len(not_pushed_i) == 0:
            # Terminated
            expected = 250 // (fps * 2)
            if 250 % (fps * 2) > 0:
                expected += 1
            assert _count_segments(spec_id)[0] == expected
            assert _count_segments(spec_id)[1] == True
        else:
            lowest_contiguous = max(1, min(not_pushed_i)) - 1
            expected = (lowest_contiguous + 1) // (fps * 2)
            assert _count_segments(spec_id)[0] == expected
            assert _count_segments(spec_id)[1] == False


def test_ffmpeg_two_segment_terminal():
    assert os.path.exists("../ffmpeg/build/bin/ffmpeg")

    source_id = _create_tos_source()
    spec_id = _create_example_spec()

    ts = [[i, 30] for i in range(95)]
    _push_frames(spec_id, source_id, ts, 0, True)

    spec = _get_spec(spec_id)
    assert spec["frames_applied"] == 95
    assert spec["terminated"] == True

    endpoint_url = spec["vod_endpoint"]
    playlist_url = f"{endpoint_url}playlist.m3u8"

    p = sp.run(["../ffmpeg/build/bin/ffmpeg", "-i", playlist_url, "out.mp4", "-y"])
    assert p.returncode == 0
    assert os.path.exists("out.mp4")
    os.remove("out.mp4")
