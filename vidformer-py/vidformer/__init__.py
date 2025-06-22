"""
vidformer-py is a Python 🐍 interface for [vidformer](https://github.com/ixlab/vidformer).

**Quick links:**
* [📦 PyPI](https://pypi.org/project/vidformer/)
* [📘 Documentation - vidformer-py](https://ixlab.github.io/vidformer/vidformer-py/pdoc/)
* [📘 Documentation - vidformer.cv2](https://ixlab.github.io/vidformer/vidformer-py/pdoc/vidformer/cv2.html)
* [📘 Documentation - vidformer.supervision](https://ixlab.github.io/vidformer/vidformer-py/pdoc/vidformer/supervision.html)
* [🧑‍💻 Source Code](https://github.com/ixlab/vidformer/tree/main/vidformer-py/)
"""

__version__ = "1.1.0"


import base64
import gzip
import json
import struct
import time
from fractions import Fraction
from urllib.parse import urlparse

import requests

_in_notebook = False
try:
    from IPython import get_ipython

    if "IPKernelApp" in get_ipython().config:
        _in_notebook = True
except Exception:
    pass


def _wait_for_url(url, max_attempts=150, delay=0.1):
    for attempt in range(max_attempts):
        try:
            response = requests.get(url)
            if response.status_code == 200:
                return response.text.strip()
            else:
                time.sleep(delay)
        except requests.exceptions.RequestException:
            time.sleep(delay)
    return None


def _play(namespace, hls_video_url, hls_js_url, method="html", status_url=None):
    # The namespace is so multiple videos in one tab don't conflict

    if method == "html":
        from IPython.display import HTML

        if not status_url:
            html_code = f"""
<!DOCTYPE html>
<html>
<head>
    <title>HLS Video Player</title>
    <!-- Include hls.js library -->
    <script src="{hls_js_url}"></script>
</head>
<body>
    <video id="video-{namespace}" controls width="640" height="360" autoplay></video>
    <script>
        var video = document.getElementById('video-{namespace}');
        var videoSrc = '{hls_video_url}';

        if (Hls.isSupported()) {{
            var hls = new Hls();
            hls.loadSource(videoSrc);
            hls.attachMedia(video);
            hls.on(Hls.Events.MANIFEST_PARSED, function() {{
                video.play();
            }});
        }} else if (video.canPlayType('application/vnd.apple.mpegurl')) {{
            video.src = videoSrc;
            video.addEventListener('loadedmetadata', function() {{
                video.play();
            }});
        }} else {{
            console.error('This browser does not appear to support HLS.');
        }}
    </script>
</body>
</html>
"""
            return HTML(data=html_code)
        else:
            html_code = f"""
<!DOCTYPE html>
<html>
<head>
    <title>HLS Video Player</title>
    <script src="{hls_js_url}"></script>
</head>
<body>
    <div id="container-{namespace}"></div>
    <script>
        var statusUrl = '{status_url}';
        var videoSrc = '{hls_video_url}';
        var videoNamespace = '{namespace}';

        function showWaiting() {{
            document.getElementById('container-{namespace}').textContent = 'Waiting...';
            pollStatus();
        }}

        function pollStatus() {{
            setTimeout(function() {{
                fetch(statusUrl)
                    .then(r => r.json())
                    .then(res => {{
                        if (res.ready) {{
                            document.getElementById('container-{namespace}').textContent = '';
                            attachHls();
                        }} else {{
                            pollStatus();
                        }}
                    }})
                    .catch(e => {{
                        console.error(e);
                        pollStatus();
                    }});
            }}, 250);
        }}

        function attachHls() {{
            var container = document.getElementById('container-{namespace}');
            container.textContent = '';
            var video = document.createElement('video');
            video.id = 'video-' + videoNamespace;
            video.controls = true;
            video.width = 640;
            video.height = 360;
            container.appendChild(video);
            if (Hls.isSupported()) {{
                var hls = new Hls();
                hls.loadSource(videoSrc);
                hls.attachMedia(video);
                hls.on(Hls.Events.MANIFEST_PARSED, function() {{
                    video.play();
                }});
            }} else if (video.canPlayType('application/vnd.apple.mpegurl')) {{
                video.src = videoSrc;
                video.addEventListener('loadedmetadata', function() {{
                    video.play();
                }});
            }}
        }}

        fetch(statusUrl)
            .then(r => r.json())
            .then(res => {{
                if (res.ready) {{
                    attachHls();
                }} else {{
                    showWaiting();
                }}
            }})
            .catch(e => {{
                console.error(e);
                showWaiting();
            }});
    </script>
</body>
</html>
"""
        return HTML(data=html_code)
    elif method == "link":
        return hls_video_url
    else:
        raise ValueError("Invalid method")


def _feb_expr_coded_as_scalar(expr) -> bool:
    if type(expr) is tuple:
        expr = list(expr)
    if type(expr) is FilterExpr:
        return False
    if type(expr) is list:
        if len(expr) > 3:
            return False
        else:
            return all([type(x) is int and x >= -(2**15) and x < 2**15 for x in expr])
    else:
        assert type(expr) in [int, float, str, bytes, SourceExpr, bool, list]
        return True


class _FrameExpressionBlock:
    def __init__(self):
        self._functions = []
        self._literals = []
        self._sources = []
        self._kwarg_keys = []
        self._source_fracs = []
        self._exprs = []
        self._frame_exprs = []

    def __len__(self):
        return len(self._frame_exprs)

    def insert_expr(self, expr):
        if type(expr) is SourceExpr or type(expr) is FilterExpr:
            return self.insert_frame_expr(expr)
        else:
            return self.insert_data_expr(expr)

    def insert_data_expr(self, data):
        if type(data) is tuple:
            data = list(data)
        if type(data) is bool:
            self._exprs.append(0x01000000_00000000 | int(data))
            return len(self._exprs) - 1
        elif type(data) is int:
            if data >= -(2**31) and data < 2**31:
                self._exprs.append(data & 0xFFFFFFFF)
            else:
                self._literals.append(_json_arg(data, skip_data_anot=True))
                self._exprs.append(0x40000000_00000000 | len(self._literals) - 1)
            return len(self._exprs) - 1
        elif type(data) is float:
            self._exprs.append(
                0x02000000_00000000 | int.from_bytes(struct.pack("f", data)[::-1])
            )
        elif type(data) is str:
            self._literals.append(_json_arg(data, skip_data_anot=True))
            self._exprs.append(0x40000000_00000000 | len(self._literals) - 1)
        elif type(data) is bytes:
            self._literals.append(_json_arg(data, skip_data_anot=True))
            self._exprs.append(0x40000000_00000000 | len(self._literals) - 1)
        elif type(data) is list:
            if len(data) == 0:
                self._exprs.append(0x03000000_00000000)
                return len(self._exprs) - 1
            if (
                len(data) == 1
                and type(data[0]) is int
                and data[0] >= -(2**15)
                and data[0] < 2**15
            ):
                self._exprs.append(0x04000000_00000000 | (data[0] & 0xFFFF))
                return len(self._exprs) - 1
            if (
                len(data) == 2
                and type(data[0]) is int
                and data[0] >= -(2**15)
                and data[0] < 2**15
                and type(data[1]) is int
                and data[1] >= -(2**15)
                and data[1] < 2**15
            ):
                self._exprs.append(
                    0x05000000_00000000
                    | ((data[0] & 0xFFFF) << 16)
                    | (data[1] & 0xFFFF)
                )
                return len(self._exprs) - 1
            if (
                len(data) == 3
                and type(data[0]) is int
                and data[0] >= -(2**15)
                and data[0] < 2**15
                and type(data[1]) is int
                and data[1] >= -(2**15)
                and data[1] < 2**15
                and type(data[2]) is int
                and data[2] >= -(2**15)
                and data[2] < 2**15
            ):
                self._exprs.append(
                    0x06000000_00000000
                    | ((data[0] & 0xFFFF) << 32)
                    | ((data[1] & 0xFFFF) << 16)
                    | (data[2] & 0xFFFF)
                )
                return len(self._exprs) - 1
            out = len(self._exprs)
            member_idxs = []
            for member in data:
                if _feb_expr_coded_as_scalar(member):
                    member_idxs.append(None)
                else:
                    member_idxs.append(self.insert_data_expr(member))

            self._exprs.append(0x42000000_00000000 | len(data))

            for i in range(len(data)):
                if member_idxs[i] is None:
                    self.insert_data_expr(data[i])
                else:
                    self._exprs.append(0x45000000_00000000 | member_idxs[i])

            return out
        else:
            raise Exception("Invalid data type")

    def insert_frame_expr(self, frame):
        if type(frame) is SourceExpr:
            source = frame._source._name
            if source in self._sources:
                source_idx = self._sources.index(source)
            else:
                source_idx = len(self._sources)
                self._sources.append(source)
            if frame._is_iloc:
                self._exprs.append(
                    0x43000000_00000000 | (source_idx << 32) | frame._idx
                )
            else:
                idx = len(self._source_fracs) // 2
                self._source_fracs.append(frame._idx.numerator)
                self._source_fracs.append(frame._idx.denominator)
                self._exprs.append(0x44000000_00000000 | (source_idx << 32) | idx)
            return len(self._exprs) - 1
        elif type(frame) is FilterExpr:
            func = frame._filter._func
            if func in self._functions:
                func_idx = self._functions.index(func)
            else:
                func_idx = len(self._functions)
                self._functions.append(func)
            len_args = len(frame._args)
            len_kwargs = len(frame._kwargs)

            arg_idxs = []
            for arg in frame._args:
                if _feb_expr_coded_as_scalar(arg):
                    arg_idxs.append(None)
                else:
                    arg_idxs.append(self.insert_expr(arg))
            kwarg_idxs = {}
            for k, v in frame._kwargs.items():
                if _feb_expr_coded_as_scalar(v):
                    kwarg_idxs[k] = None
                else:
                    kwarg_idxs[k] = self.insert_expr(v)

            out_idx = len(self._exprs)
            self._exprs.append(
                0x41000000_00000000 | (len_args << 24) | (len_kwargs << 16) | func_idx
            )
            for i in range(len_args):
                if arg_idxs[i] is None:
                    # It's a scalar
                    self.insert_expr(frame._args[i])
                else:
                    # It's an expression pointer
                    self._exprs.append(0x45000000_00000000 | arg_idxs[i])
            for k, v in frame._kwargs.items():
                if k in self._kwarg_keys:
                    k_idx = self._kwarg_keys.index(k)
                else:
                    k_idx = len(self._kwarg_keys)
                    self._kwarg_keys.append(k)
                self._exprs.append(0x46000000_00000000 | k_idx)
                if kwarg_idxs[k] is None:
                    # It's a scalar
                    self.insert_expr(v)
                else:
                    # It's an expression pointer
                    self._exprs.append(0x45000000_00000000 | kwarg_idxs[k])
            return out_idx
        else:
            raise Exception("Invalid frame type")

    def insert_frame(self, frame):
        idx = self.insert_frame_expr(frame)
        self._frame_exprs.append(idx)

    def as_dict(self):
        return {
            "functions": self._functions,
            "literals": self._literals,
            "sources": self._sources,
            "kwarg_keys": self._kwarg_keys,
            "source_fracs": self._source_fracs,
            "exprs": self._exprs,
            "frame_exprs": self._frame_exprs,
        }


class Source:
    def __init__(self, id: str, src):
        self._name = id
        self._fmt = {
            "width": src["width"],
            "height": src["height"],
            "pix_fmt": src["pix_fmt"],
        }
        self._ts = [Fraction(x[0], x[1]) for x in src["ts"]]
        self.iloc = _SourceILoc(self)

    def id(self) -> str:
        return self._name

    def fmt(self):
        return {**self._fmt}

    def ts(self) -> list[Fraction]:
        return self._ts.copy()

    def __len__(self):
        return len(self._ts)

    def __getitem__(self, idx):
        if type(idx) is not Fraction:
            raise Exception("Source index must be a Fraction")
        return SourceExpr(self, idx, False)

    def __repr__(self):
        return f"Source({self._name})"


class Spec:
    def __init__(self, id: str, src):
        self._id = id
        self._fmt = {
            "width": src["width"],
            "height": src["height"],
            "pix_fmt": src["pix_fmt"],
        }
        self._vod_endpoint = src["vod_endpoint"]
        parsed_url = urlparse(self._vod_endpoint)
        self._hls_js_url = f"{parsed_url.scheme}://{parsed_url.netloc}/hls.js"

    def id(self) -> str:
        return self._id

    def play(self, method):
        url = f"{self._vod_endpoint}playlist.m3u8"
        status_url = f"{self._vod_endpoint}status"
        hls_js_url = self._hls_js_url
        return _play(self._id, url, hls_js_url, method=method, status_url=status_url)


class Server:
    def __init__(self, endpoint: str, api_key: str):
        if not endpoint.startswith("http://") and not endpoint.startswith("https://"):
            raise Exception("Endpoint must start with http:// or https://")
        if endpoint.endswith("/"):
            raise Exception("Endpoint must not end with /")
        self._endpoint = endpoint

        self._api_key = api_key
        self._session = requests.Session()
        self._session.headers.update({"Authorization": f"Bearer {self._api_key}"})
        response = self._session.get(
            f"{self._endpoint}/v2/auth",
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"

    def get_source(self, id: str) -> Source:
        assert type(id) is str
        response = self._session.get(
            f"{self._endpoint}/v2/source/{id}",
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        return Source(response["id"], response)

    def list_sources(self) -> list[str]:
        response = self._session.get(
            f"{self._endpoint}/v2/source",
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        return response

    def delete_source(self, id: str):
        assert type(id) is str
        response = self._session.delete(
            f"{self._endpoint}/v2/source/{id}",
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"

    def search_source(
        self, name, stream_idx, storage_service, storage_config
    ) -> list[str]:
        assert type(name) is str
        assert type(stream_idx) is int
        assert type(storage_service) is str
        assert type(storage_config) is dict
        for k, v in storage_config.items():
            assert type(k) is str
            assert type(v) is str
        req = {
            "name": name,
            "stream_idx": stream_idx,
            "storage_service": storage_service,
            "storage_config": storage_config,
        }
        response = self._session.post(
            f"{self._endpoint}/v2/source/search",
            json=req,
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        return response

    def create_source(
        self, name, stream_idx, storage_service, storage_config
    ) -> Source:
        assert type(name) is str
        assert type(stream_idx) is int
        assert type(storage_service) is str
        assert type(storage_config) is dict
        for k, v in storage_config.items():
            assert type(k) is str
            assert type(v) is str
        req = {
            "name": name,
            "stream_idx": stream_idx,
            "storage_service": storage_service,
            "storage_config": storage_config,
        }
        response = self._session.post(
            f"{self._endpoint}/v2/source",
            json=req,
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"
        id = response["id"]
        return self.get_source(id)

    def source(self, name, stream_idx, storage_service, storage_config) -> Source:
        """Convenience function for accessing sources.

        Tries to find a source with the given name, stream_idx, storage_service, and storage_config.
        If no source is found, creates a new source with the given parameters.
        """

        sources = self.search_source(name, stream_idx, storage_service, storage_config)
        if len(sources) == 0:
            return self.create_source(name, stream_idx, storage_service, storage_config)
        return self.get_source(sources[0])

    def get_spec(self, id: str) -> Spec:
        assert type(id) is str
        response = self._session.get(
            f"{self._endpoint}/v2/spec/{id}",
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        return Spec(response["id"], response)

    def list_specs(self) -> list[str]:
        response = self._session.get(
            f"{self._endpoint}/v2/spec",
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
        ttl=None,
    ) -> Spec:
        assert type(width) is int
        assert type(height) is int
        assert type(pix_fmt) is str
        assert type(vod_segment_length) is Fraction
        assert type(frame_rate) is Fraction
        assert type(ready_hook) is str or ready_hook is None
        assert type(steer_hook) is str or steer_hook is None
        assert ttl is None or type(ttl) is int

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
            "ttl": ttl,
        }
        response = self._session.post(
            f"{self._endpoint}/v2/spec",
            json=req,
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"
        return self.get_spec(response["id"])

    def delete_spec(self, id: str):
        assert type(id) is str
        response = self._session.delete(
            f"{self._endpoint}/v2/spec/{id}",
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"

    def export_spec(
        self, id: str, path: str, encoder=None, encoder_opts=None, format=None
    ):
        assert type(id) is str
        assert type(path) is str
        req = {
            "path": path,
            "encoder": encoder,
            "encoder_opts": encoder_opts,
            "format": format,
        }
        response = self._session.post(
            f"{self._endpoint}/v2/spec/{id}/export",
            json=req,
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"

    def push_spec_part(self, spec_id, pos, frames, terminal):
        if type(spec_id) is Spec:
            spec_id = spec_id._id
        assert type(spec_id) is str
        assert type(pos) is int
        assert type(frames) is list
        assert type(terminal) is bool

        req_frames = []
        for frame in frames:
            assert type(frame) is tuple
            assert len(frame) == 2
            t = frame[0]
            f = frame[1]
            assert type(t) is Fraction
            assert f is None or type(f) is SourceExpr or type(f) is FilterExpr
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
        response = self._session.post(
            f"{self._endpoint}/v2/spec/{spec_id}/part",
            json=req,
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"

    def push_spec_part_block(
        self, spec_id: str, pos, blocks, terminal, compression="gzip"
    ):
        if type(spec_id) is Spec:
            spec_id = spec_id._id
        assert type(spec_id) is str
        assert type(pos) is int
        assert type(blocks) is list
        assert type(terminal) is bool
        assert compression is None or compression == "gzip"

        req_blocks = []
        for block in blocks:
            assert type(block) is _FrameExpressionBlock
            block_body = block.as_dict()
            block_frames = len(block_body["frame_exprs"])
            block_body = json.dumps(block_body).encode("utf-8")
            if compression == "gzip":
                block_body = gzip.compress(block_body, 1)
            block_body = base64.b64encode(block_body).decode("utf-8")
            req_blocks.append(
                {
                    "frames": block_frames,
                    "compression": compression,
                    "body": block_body,
                }
            )

        req = {
            "pos": pos,
            "terminal": terminal,
            "blocks": req_blocks,
        }
        response = self._session.post(
            f"{self._endpoint}/v2/spec/{spec_id}/part_block",
            json=req,
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response = response.json()
        assert response["status"] == "ok"

    def frame(self, width, height, pix_fmt, frame_expr, compression="gzip"):
        assert type(frame_expr) is FilterExpr or type(frame_expr) is SourceExpr
        assert compression is None or compression in ["gzip"]
        feb = _FrameExpressionBlock()
        feb.insert_frame(frame_expr)
        feb_body = feb.as_dict()

        feb_body = json.dumps(feb_body).encode("utf-8")
        if compression == "gzip":
            feb_body = gzip.compress(feb_body, 1)
        feb_body = base64.b64encode(feb_body).decode("utf-8")
        req = {
            "width": width,
            "height": height,
            "pix_fmt": pix_fmt,
            "compression": compression,
            "block": {
                "frames": 1,
                "compression": compression,
                "body": feb_body,
            },
        }
        response = self._session.post(
            f"{self._endpoint}/v2/frame",
            json=req,
            headers={"Authorization": f"Bearer {self._api_key}"},
        )
        if not response.ok:
            raise Exception(response.text)
        response_body = response.content
        assert type(response_body) is bytes
        if compression == "gzip":
            response_body = gzip.decompress(response_body)
        return response_body


class SourceExpr:
    def __init__(self, source, idx, is_iloc):
        self._source = source
        self._idx = idx
        self._is_iloc = is_iloc

    def __repr__(self):
        if self._is_iloc:
            return f"{self._source._name}.iloc[{self._idx}]"
        else:
            return f"{self._source._name}[{self._idx}]"

    def _to_json_spec(self):
        if self._is_iloc:
            return {
                "Source": {
                    "video": self._source._name,
                    "index": {"ILoc": int(self._idx)},
                }
            }
        else:
            return {
                "Source": {
                    "video": self._source._name,
                    "index": {"T": [self._idx.numerator, self._idx.denominator]},
                }
            }

    def _sources(self):
        return set([self._source])

    def _filters(self):
        return {}


class _SourceILoc:
    def __init__(self, source):
        self._source = source

    def __getitem__(self, idx):
        if type(idx) is not int:
            raise Exception(f"Source iloc index must be an integer, got a {type(idx)}")
        return SourceExpr(self._source, idx, True)


def _json_arg(arg, skip_data_anot=False):
    if type(arg) is FilterExpr or type(arg) is SourceExpr:
        return {"Frame": arg._to_json_spec()}
    elif type(arg) is int:
        if skip_data_anot:
            return {"Int": arg}
        return {"Data": {"Int": arg}}
    elif type(arg) is str:
        if skip_data_anot:
            return {"String": arg}
        return {"Data": {"String": arg}}
    elif type(arg) is bytes:
        arg = list(arg)
        if skip_data_anot:
            return {"Bytes": arg}
        return {"Data": {"Bytes": arg}}
    elif type(arg) is float:
        if skip_data_anot:
            return {"Float": arg}
        return {"Data": {"Float": arg}}
    elif type(arg) is bool:
        if skip_data_anot:
            return {"Bool": arg}
        return {"Data": {"Bool": arg}}
    elif type(arg) is tuple or type(arg) is list:
        if skip_data_anot:
            return {"List": [_json_arg(x, True) for x in list(arg)]}
        return {"Data": {"List": [_json_arg(x, True) for x in list(arg)]}}
    else:
        raise Exception(f"Unknown arg type: {type(arg)}")


class Filter:
    """A video filter."""

    def __init__(self, func: str):
        self._func = func

    def __call__(self, *args, **kwargs):
        return FilterExpr(self, args, kwargs)


class FilterExpr:
    def __init__(self, filter: Filter, args, kwargs):
        self._filter = filter
        self._args = args
        self._kwargs = kwargs

    def __repr__(self):
        args = []
        for arg in self._args:
            val = f'"{arg}"' if type(arg) is str else str(arg)
            args.append(str(val))
        for k, v in self._kwargs.items():
            val = f'"{v}"' if type(v) is str else str(v)
            args.append(f"{k}={val}")
        return f"{self._filter._name}({', '.join(args)})"

    def _to_json_spec(self):
        args = []
        for arg in self._args:
            args.append(_json_arg(arg))
        kwargs = {}
        for k, v in self._kwargs.items():
            kwargs[k] = _json_arg(v)
        return {"Filter": {"name": self._filter._name, "args": args, "kwargs": kwargs}}

    def _sources(self):
        s = set()
        for arg in self._args:
            if type(arg) is FilterExpr or type(arg) is SourceExpr:
                s = s.union(arg._sources())
        for arg in self._kwargs.values():
            if type(arg) is FilterExpr or type(arg) is SourceExpr:
                s = s.union(arg._sources())
        return s

    def _filters(self):
        f = {self._filter._name: self._filter}
        for arg in self._args:
            if type(arg) is FilterExpr:
                f = {**f, **arg._filters()}
        for arg in self._kwargs.values():
            if type(arg) is FilterExpr:
                f = {**f, **arg._filters()}
        return f
