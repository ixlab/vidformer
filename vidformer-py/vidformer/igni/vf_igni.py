from .. import vf

import requests
from fractions import Fraction
from urllib.parse import urlparse


class IgniServer:
    def __init__(self, endpoint: str, api_key: str):
        if not endpoint.startswith("http://") and not endpoint.startswith("https://"):
            raise Exception("Endpoint must start with http:// or https://")
        if endpoint.endswith("/"):
            raise Exception("Endpoint must not end with /")
        self._endpoint = endpoint

        self._api_key = api_key
        response = requests.get(
            f"{self._endpoint}/auth",
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"

    def get_source(self, id: str):
        assert type(id) == str
        response = requests.get(
            f"{self._endpoint}/source/{id}",
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        return IgniSource(response["id"], response)

    def list_sources(self):
        response = requests.get(
            f"{self._endpoint}/source",
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        return response

    def delete_source(self, id: str):
        assert type(id) == str
        response = requests.delete(
            f"{self._endpoint}/source/{id}",
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"

    def search_source(self, name, stream_idx, storage_service, storage_config):
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
        response = requests.post(
            f"{self._endpoint}/source/search",
            json=req,
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        return response

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
        response = requests.post(
            f"{self._endpoint}/source",
            json=req,
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"
        id = response["id"]
        return self.get_source(id)

    def source(self, name, stream_idx, storage_service, storage_config):
        """Convenience function for accessing sources.

        Tries to find a source with the given name, stream_idx, storage_service, and storage_config.
        If no source is found, creates a new source with the given parameters.
        """

        sources = self.search_source(name, stream_idx, storage_service, storage_config)
        if len(sources) == 0:
            return self.create_source(name, stream_idx, storage_service, storage_config)
        return self.get_source(sources[0])

    def get_spec(self, id: str):
        assert type(id) == str
        response = requests.get(
            f"{self._endpoint}/spec/{id}",
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        return IgniSpec(response["id"], response)

    def list_specs(self):
        response = requests.get(
            f"{self._endpoint}/spec",
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        return response

    def create_spec(
        self,
        width,
        height,
        pix_fmt,
        vod_segment_length,
        frame_rate,
        ready_hook=None,
        steer_hook=None,
    ):
        assert type(width) == int
        assert type(height) == int
        assert type(pix_fmt) == str
        assert type(vod_segment_length) == Fraction
        assert type(frame_rate) == Fraction
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
            "frame_rate": [frame_rate.numerator, frame_rate.denominator],
            "ready_hook": ready_hook,
            "steer_hook": steer_hook,
        }
        response = requests.post(
            f"{self._endpoint}/spec",
            json=req,
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"
        return self.get_spec(response["id"])

    def delete_spec(self, id: str):
        assert type(id) == str
        response = requests.delete(
            f"{self._endpoint}/spec/{id}",
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"

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
            assert f is None or type(f) == vf.SourceExpr or type(f) == vf.FilterExpr
            req_frames.append(
                [
                    [t.numerator, t.denominator],
                    f._to_json_spec() if f is not None else None,
                ]
            )

        req = {
            "pos": pos,
            "frames": req_frames,
            "terminal": terminal,
        }
        response = requests.post(
            f"{self._endpoint}/spec/{spec_id}/part",
            json=req,
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
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

    def id(self):
        return self._name

    def fmt(self):
        return {**self._fmt}

    def ts(self):
        return self._ts.copy()

    def __len__(self):
        return len(self._ts)

    def __getitem__(self, idx):
        if type(idx) != Fraction:
            raise Exception("Source index must be a Fraction")
        return vf.SourceExpr(self, idx, False)

    def __repr__(self):
        return f"IgniSource({self._name})"


class IgniSpec:
    def __init__(self, id, src):
        self._id = id
        self._fmt = {
            "width": src["width"],
            "height": src["height"],
            "pix_fmt": src["pix_fmt"],
        }
        self._vod_endpoint = src["vod_endpoint"]
        parsed_url = urlparse(self._vod_endpoint)
        self._hls_js_url = f"{parsed_url.scheme}://{parsed_url.netloc}/hls.js"

    def id(self):
        return self._id

    def play(self, *args, **kwargs):
        url = f"{self._vod_endpoint}playlist.m3u8"
        status_url = f"{self._vod_endpoint}status"
        hls_js_url = self._hls_js_url
        return vf._play(
            self._id, url, hls_js_url, *args, **kwargs, status_url=status_url
        )
