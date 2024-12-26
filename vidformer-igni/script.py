#!/usr/bin/env python3

import requests
import json
import os
import subprocess


ENDPOINT = "http://localhost:8080/"

# curl http://localhost:8080/
response = requests.get(ENDPOINT)
if response.status_code != 200:
    print(response.text)
    response.raise_for_status()
assert response.text.startswith("vidformer-igni v")

assert os.path.exists("../tos_720p.mp4")

# curl -X POST -H "Content-Type: application/json" -d '{"name":"../tos_720p.mp4","stream_idx":0,"storage_service":"fs","storage_config":{"root":"."}}' http://localhost:8080/v2/source
req = {
    "name": "../tos_720p.mp4",
    "stream_idx": 0,
    "storage_service": "fs",
    "storage_config": {"root": "."},
}
response = requests.post(ENDPOINT + "v2/source", json=req)
if response.status_code != 200:
    print(response.text)
    response.raise_for_status()

resp = response.json()
assert resp["status"] == "ok"
source_id = resp["id"]
print("Added source with id:", source_id)

# curl http://localhost:8080/v2/source/<uuid>
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

# Create a spec
#  Request:
#     {
#     "width": 1920,
#     "height": 1080,
#     "pix_fmt": "yuv420p",
#     "vod_segment_length": [2, 1], // Rational; 2 seconds
#     "ready_hook": "http://myapp.com/hook/...",
#     "steer_hook": "http://myapp.com/hook/..."
# } */

req = {
    "width": 1280,
    "height": 720,
    "pix_fmt": "yuv420p",
    "vod_segment_length": [2, 1],
    "ready_hook": None,
    "steer_hook": None,
}
response = requests.post(ENDPOINT + "v2/spec", json=req)
if response.status_code != 200:
    print(response.text)
    response.raise_for_status()
resp = response.json()
assert resp["status"] == "ok"
spec_id = resp["id"]
print("Added spec with id:", spec_id)

# curl http://localhost:8080/v2/spec/<uuid>
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

# push some frames
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
frames = []
for i in range(50):  # Needs to be at least 48 frames total to create a segment
    t = [i + 10, 24]
    frames.append([t, frame_expr])

req = {"pos": 1, "terminal": False, "frames": frames}
response = requests.post(ENDPOINT + "v2/spec/" + spec_id + "/part", json=req)
if response.status_code != 200:
    print(response.text)
    response.raise_for_status()

# check that zero parts are applied
response = requests.get(ENDPOINT + "v2/spec/" + spec_id)
if response.status_code != 200:
    print(response.text)
    response.raise_for_status()
resp = response.json()
assert resp["applied_parts"] == 0

# push the first pos
frames = []
for i in range(10):
    t = [i, 24]
    frames.append([t, frame_expr])

req = {"pos": 0, "terminal": False, "frames": frames}
response = requests.post(ENDPOINT + "v2/spec/" + spec_id + "/part", json=req)
if response.status_code != 200:
    print(response.text)
    response.raise_for_status()

# check that two parts are applied
response = requests.get(ENDPOINT + "v2/spec/" + spec_id)
if response.status_code != 200:
    print(response.text)
    response.raise_for_status()
resp = response.json()
assert resp["applied_parts"] == 2

# check ffmpeg can read the video
playlist_url = resp["playlist"]
p = subprocess.run(["../ffmpeg/build/bin/ffmpeg", "-i", playlist_url, "out.mp4", "-y"])
assert p.returncode == 0
os.remove("out.mp4")

print("Done!")
