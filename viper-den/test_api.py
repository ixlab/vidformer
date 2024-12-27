import requests
import re

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

    response = requests.get(ENDPOINT + "v2/source/" + source_id)
    if response.status_code != 200:
        print(response.text)
        response.raise_for_status()
    resp = response.json()
    assert resp["id"] == source_id
    assert resp["name"] == "../tos_720p.mp4"
    assert resp["stream_idx"] == 0
    assert resp["storage_service"] == "fs"
    assert resp["storage_config"]["root"] == "."
    assert resp["width"] == 1280
    assert resp["height"] == 720
    assert resp["pix_fmt"] == "yuv420p"
    assert len(resp["ts"]) == 17616
    for ts in resp["ts"]:
        assert len(ts) == 3
        assert isinstance(ts[0], int)
        assert isinstance(ts[1], int)
        assert isinstance(ts[2], bool)


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

    response = requests.get(ENDPOINT + "v2/spec/" + spec_id)
    if response.status_code != 200:
        print(response.text)
        response.raise_for_status()
    resp = response.json()
    assert resp["id"] == spec_id
    assert resp["width"] == 1280
    assert resp["height"] == 720
    assert resp["pix_fmt"] == "yuv420p"
    assert resp["vod_segment_length"] == [2, 1]
    assert resp["ready_hook"] == None
    assert resp["steer_hook"] == None
    assert resp["playlist"] == ENDPOINT + "vod/" + spec_id + "/playlist.m3u8"
    assert resp["applied_parts"] == 0
    assert resp["terminated"] == False
    assert resp["closed"] == False
