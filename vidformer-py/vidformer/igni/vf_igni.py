from .. import vf

import requests
from fractions import Fraction


class IgniServer:
    def __init__(self, endpoint: str):
        self._endpoint = endpoint

        version = vf._check_hls_link_exists(f"{self._endpoint}/")
        if version is None:
            raise Exception("Failed to connect to server")

        if not version.startswith("vidformer-igni"):
            raise Exception(f"Endpoint not a vidformer-igni server!")

    def get_source(self, id: str):
        response = requests.get(f"{self._endpoint}/v2/source/{id}")
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        return IgniSource(response["id"], response)

    def create_source(self, name, stream_idx, storage_service, storage_config):
        assert type(name) == str
        assert type(stream_idx) == int
        assert type(storage_service) == str
        assert type(storage_config) == dict
        for k, v in storage_config.items():
            assert type(k) == str
            assert type(v) == str
        req = {
            "name": name,
            "stream_idx": stream_idx,
            "storage_service": storage_service,
            "storage_config": storage_config,
        }
        response = requests.post(f"{self._endpoint}/v2/source", json=req)
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"
        id = response["id"]
        return self.get_source(id)

    def get_spec(self, id: str):
        response = requests.get(f"{self._endpoint}/v2/spec/{id}")
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        return IgniSpec(response["id"], response)

    def create_spec(
        self,
        width,
        height,
        pix_fmt,
        vod_segment_length,
        ready_hook=None,
        steer_hook=None,
    ):
        assert type(width) == int
        assert type(height) == int
        assert type(pix_fmt) == str
        assert type(vod_segment_length) == Fraction
        assert type(ready_hook) == str or ready_hook is None
        assert type(steer_hook) == str or steer_hook is None

        req = {
            "width": width,
            "height": height,
            "pix_fmt": pix_fmt,
            "vod_segment_length": [
                vod_segment_length.numerator,
                vod_segment_length.denominator,
            ],
            "ready_hook": ready_hook,
            "steer_hook": steer_hook,
        }
        response = requests.post(f"{self._endpoint}/v2/spec", json=req)
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"
        return self.get_spec(response["id"])

    def push_spec_part(self, spec_id, pos, frames, terminal):
        if type(spec_id) == IgniSpec:
            spec_id = spec_id._id
        assert type(spec_id) == str
        assert type(pos) == int
        assert type(frames) == list
        assert type(terminal) == bool

        req_frames = []
        for frame in frames:
            assert type(frame) == tuple
            assert len(frame) == 2
            t = frame[0]
            f = frame[1]
            assert type(t) == Fraction
            assert type(f) == vf.SourceExpr or type(f) == vf.FilterExpr
            req_frames.append([[t.numerator, t.denominator], f._to_json_spec()])

        req = {
            "pos": pos,
            "frames": req_frames,
            "terminal": terminal,
        }
        response = requests.post(f"{self._endpoint}/v2/spec/{spec_id}/part", json=req)
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"


class IgniSource:
    def __init__(self, id, src):
        self._name = id
        self._fmt = {
            "width": src["width"],
            "height": src["height"],
            "pix_fmt": src["pix_fmt"],
        }
        self._ts = [Fraction(x[0], x[1]) for x in src["ts"]]
        self.iloc = vf.SourceILoc(self)

    def fmt(self):
        return {**self._fmt}

    def ts(self):
        return [x for x in self._ts]

    def __len__(self):
        return len(self._ts)

    def __getitem__(self, idx):
        if type(idx) != Fraction:
            raise Exception("Source index must be a Fraction")
        return vf.SourceExpr(self, idx, False)


class IgniSpec:
    def __init__(self, id, src):
        self._id = id
        self._fmt = {
            "width": src["width"],
            "height": src["height"],
            "pix_fmt": src["pix_fmt"],
        }
        self._playlist = src["playlist"]

    def play(self, *args, **kwargs):
        url = self._playlist
        hls_js_url = "http://localhost:8080/hls.js"  # TODO: Fix this
        return vf._play(self._id, url, hls_js_url, *args, **kwargs)
