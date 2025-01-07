"""
vidformer-py is a Python 🐍 interface for [vidformer](https://github.com/ixlab/vidformer).

**Quick links:**
* [📦 PyPI](https://pypi.org/project/vidformer/)
* [📘 Documentation - vidformer-py](https://ixlab.github.io/vidformer/vidformer-py/)
* [📘 Documentation - vidformer.cv2](https://ixlab.github.io/vidformer/vidformer-py-cv2/)
* [🧑‍💻 Source Code](https://github.com/ixlab/vidformer/tree/main/vidformer-py/)
"""

import subprocess
from fractions import Fraction
import random
import time
import json
import socket
import os
import sys
import multiprocessing
import uuid
import threading
import gzip
import base64
import re

import requests
import msgpack
import numpy as np

from . import __version__

_in_notebook = False
try:
    from IPython import get_ipython

    if "IPKernelApp" in get_ipython().config:
        _in_notebook = True
except:
    pass


def _check_hls_link_exists(url, max_attempts=150, delay=0.1):
    for attempt in range(max_attempts):
        try:
            response = requests.get(url)
            if response.status_code == 200:
                return response.text.strip()
            else:
                time.sleep(delay)
        except requests.exceptions.RequestException as e:
            time.sleep(delay)
    return None


class Spec:
    """
    A video transformation specification.

    See https://ixlab.github.io/vidformer/concepts.html for more information.
    """

    def __init__(self, domain: list[Fraction], render, fmt: dict):
        self._domain = domain
        self._render = render
        self._fmt = fmt

    def __repr__(self):
        if len(self._domain) <= 20:
            lines = []
            for i, t in enumerate(self._domain):
                frame_expr = self._render(t, i)
                lines.append(
                    f"{t.numerator}/{t.denominator} => {frame_expr}",
                )
            return "\n".join(lines)
        else:
            lines = []
            for i, t in enumerate(self._domain[:10]):
                frame_expr = self._render(t, i)
                lines.append(
                    f"{t.numerator}/{t.denominator} => {frame_expr}",
                )
            lines.append("...")
            for i, t in enumerate(self._domain[-10:]):
                frame_expr = self._render(t, i)
                lines.append(
                    f"{t.numerator}/{t.denominator} => {frame_expr}",
                )
            return "\n".join(lines)

    def _sources(self):
        s = set()
        for i, t in enumerate(self._domain):
            frame_expr = self._render(t, i)
            s = s.union(frame_expr._sources())
        return s

    def _to_json_spec(self):
        frames = []
        s = set()
        f = {}
        for i, t in enumerate(self._domain):
            frame_expr = self._render(t, i)
            s = s.union(frame_expr._sources())
            f = {**f, **frame_expr._filters()}
            frame = [[t.numerator, t.denominator], frame_expr._to_json_spec()]
            frames.append(frame)
        return {"frames": frames}, s, f

    def play(self, server, method="html", verbose=False):
        """Play the video live in the notebook."""

        spec, sources, filters = self._to_json_spec()
        spec_json_bytes = json.dumps(spec).encode("utf-8")
        spec_obj_json_gzip = gzip.compress(spec_json_bytes, compresslevel=1)
        spec_obj_json_gzip_b64 = base64.b64encode(spec_obj_json_gzip).decode("utf-8")

        sources = [
            {
                "name": s._name,
                "path": s._path,
                "stream": s._stream,
                "service": s._service.as_json() if s._service is not None else None,
            }
            for s in sources
        ]
        filters = {
            k: {
                "filter": v._func,
                "args": v._kwargs,
            }
            for k, v in filters.items()
        }
        arrays = []

        if verbose:
            print(f"Sending to server. Spec is {len(spec_obj_json_gzip_b64)} bytes")

        resp = server._new(spec_obj_json_gzip_b64, sources, filters, arrays, self._fmt)
        hls_video_url = resp["stream_url"]
        hls_player_url = resp["player_url"]
        namespace = resp["namespace"]
        hls_js_url = server.hls_js_url()

        if method == "link":
            return hls_video_url
        if method == "player":
            return hls_player_url
        if method == "iframe":
            from IPython.display import IFrame

            return IFrame(hls_player_url, width=1280, height=720)
        if method == "html":
            from IPython.display import HTML

            # We add a namespace to the video element to avoid conflicts with other videos
            html_code = f"""
<!DOCTYPE html>
<html>
<head>
    <title>HLS Video Player</title>
    <!-- Include hls.js library -->
    <script src="{hls_js_url}"></script>
</head>
<body>
    <!-- Video element -->
    <video id="video-{namespace}" controls width="640" height="360" autoplay></video>
    <script>
        var video = document.getElementById('video-{namespace}');
        var videoSrc = '{hls_video_url}';
        var hls = new Hls();
        hls.loadSource(videoSrc);
        hls.attachMedia(video);
        hls.on(Hls.Events.MANIFEST_PARSED, function() {{
            video.play();
        }});
    </script>
</body>
</html>
"""
            return HTML(data=html_code)
        else:
            return hls_player_url

    def load(self, server):
        spec, sources, filters = self._to_json_spec()
        spec_json_bytes = json.dumps(spec).encode("utf-8")
        spec_obj_json_gzip = gzip.compress(spec_json_bytes, compresslevel=1)
        spec_obj_json_gzip_b64 = base64.b64encode(spec_obj_json_gzip).decode("utf-8")

        sources = [
            {
                "name": s._name,
                "path": s._path,
                "stream": s._stream,
                "service": s._service.as_json() if s._service is not None else None,
            }
            for s in sources
        ]
        filters = {
            k: {
                "filter": v._func,
                "args": v._kwargs,
            }
            for k, v in filters.items()
        }
        arrays = []

        resp = server._new(spec_obj_json_gzip_b64, sources, filters, arrays, self._fmt)
        namespace = resp["namespace"]
        return Loader(server, namespace, self._domain)

    def save(self, server, pth, encoder=None, encoder_opts=None, format=None):
        """Save the video to a file."""

        assert encoder is None or type(encoder) == str
        assert encoder_opts is None or type(encoder_opts) == dict
        if encoder_opts is not None:
            for k, v in encoder_opts.items():
                assert type(k) == str and type(v) == str

        spec, sources, filters = self._to_json_spec()
        spec_json_bytes = json.dumps(spec).encode("utf-8")
        spec_obj_json_gzip = gzip.compress(spec_json_bytes, compresslevel=1)
        spec_obj_json_gzip_b64 = base64.b64encode(spec_obj_json_gzip).decode("utf-8")

        sources = [
            {
                "name": s._name,
                "path": s._path,
                "stream": s._stream,
                "service": s._service.as_json() if s._service is not None else None,
            }
            for s in sources
        ]
        filters = {
            k: {
                "filter": v._func,
                "args": v._kwargs,
            }
            for k, v in filters.items()
        }
        arrays = []

        resp = server._export(
            pth,
            spec_obj_json_gzip_b64,
            sources,
            filters,
            arrays,
            self._fmt,
            encoder,
            encoder_opts,
            format,
        )

        return resp

    def _vrod_bench(self, server):
        out = {}
        pth = "spec.json"
        start_t = time.time()
        with open(pth, "w") as outfile:
            spec, sources, filters = self._to_json_spec()
            outfile.write(json.dumps(spec))

        sources = [
            {
                "name": s._name,
                "path": s._path,
                "stream": s._stream,
                "service": s._service.as_json() if s._service is not None else None,
            }
            for s in sources
        ]
        filters = {
            k: {
                "filter": v._func,
                "args": v._kwargs,
            }
            for k, v in filters.items()
        }
        arrays = []
        end_t = time.time()
        out["vrod_create_spec"] = end_t - start_t

        start = time.time()
        resp = server._new(pth, sources, filters, arrays, self._fmt)
        end = time.time()
        out["vrod_register"] = end - start

        stream_url = resp["stream_url"]
        first_segment = stream_url.replace("stream.m3u8", "segment-0.ts")

        start = time.time()
        r = requests.get(first_segment)
        r.raise_for_status()
        end = time.time()
        out["vrod_first_segment"] = end - start
        return out

    def _dve2_bench(self, server):
        pth = "spec.json"
        out = {}
        start_t = time.time()
        with open(pth, "w") as outfile:
            spec, sources, filters = self._to_json_spec()
            outfile.write(json.dumps(spec))

        sources = [
            {
                "name": s._name,
                "path": s._path,
                "stream": s._stream,
                "service": s._service.as_json() if s._service is not None else None,
            }
            for s in sources
        ]
        filters = {
            k: {
                "filter": v._func,
                "args": v._kwargs,
            }
            for k, v in filters.items()
        }
        arrays = []
        end_t = time.time()
        out["dve2_create_spec"] = end_t - start_t

        start = time.time()
        resp = server._export(pth, sources, filters, arrays, self._fmt, None, None)
        end = time.time()
        out["dve2_exec"] = end - start
        return out


class Loader:
    def __init__(self, server, namespace: str, domain):
        self._server = server
        self._namespace = namespace
        self._domain = domain

    def _chunk(self, start_i, end_i):
        return self._server._raw(self._namespace, start_i, end_i)

    def __len__(self):
        return len(self._domain)

    def _find_index_by_rational(self, value):
        if value not in self._domain:
            raise ValueError(f"Rational timestamp {value} is not in the domain")
        return self._domain.index(value)

    def __getitem__(self, index):
        if isinstance(index, slice):
            start = index.start if index.start is not None else 0
            end = index.stop if index.stop is not None else len(self._domain)
            assert start >= 0 and start < len(self._domain)
            assert end >= 0 and end <= len(self._domain)
            assert start <= end
            num_frames = end - start
            all_bytes = self._chunk(start, end - 1)
            all_bytes_len = len(all_bytes)
            assert all_bytes_len % num_frames == 0
            return [
                all_bytes[
                    i
                    * all_bytes_len
                    // num_frames : (i + 1)
                    * all_bytes_len
                    // num_frames
                ]
                for i in range(num_frames)
            ]
        elif isinstance(index, int):
            assert index >= 0 and index < len(self._domain)
            return self._chunk(index, index)
        else:
            raise TypeError(
                "Invalid argument type for iloc. Use a slice or an integer."
            )


class YrdenServer:
    """
    A connection to a Yrden server.

    A yrden server is the main API for local use of vidformer.
    """

    def __init__(self, domain=None, port=None, bin=None, hls_prefix=None):
        """
        Connect to a Yrden server

        Can either connect to an existing server, if domain and port are provided, or start a new server using the provided binary.
        If no domain or binary is provided, the `VIDFORMER_BIN` environment variable is used.
        """

        self._domain = domain
        self._port = port
        self._proc = None
        if self._port is None:
            if bin is None:
                if os.getenv("VIDFORMER_BIN") is not None:
                    bin = os.getenv("VIDFORMER_BIN")
                else:
                    bin = "vidformer-cli"

            self._domain = "localhost"
            self._port = random.randint(49152, 65535)
            cmd = [bin, "yrden", "--port", str(self._port)]
            if _in_notebook:
                # We need to print the URL in the notebook
                # This is a trick to get VS Code to forward the port
                cmd += ["--print-url"]

            if hls_prefix is not None:
                if not type(hls_prefix) == str:
                    raise Exception("hls_prefix must be a string")
                cmd += ["--hls-prefix", hls_prefix]

            self._proc = subprocess.Popen(cmd)

        version = _check_hls_link_exists(f"http://{self._domain}:{self._port}/")
        if version is None:
            raise Exception("Failed to connect to server")

        expected_version = f"vidformer-yrden v{__version__}"
        if version != expected_version:
            print(
                f"Warning: Expected version `{expected_version}`, got `{version}`. API may not be compatible!"
            )

    def _source(self, name: str, path: str, stream: int, service):
        r = requests.post(
            f"http://{self._domain}:{self._port}/source",
            json={
                "name": name,
                "path": path,
                "stream": stream,
                "service": service.as_json() if service is not None else None,
            },
        )
        if not r.ok:
            raise Exception(r.text)

        resp = r.json()
        resp["ts"] = [Fraction(x[0], x[1]) for x in resp["ts"]]
        return resp

    def _new(self, spec, sources, filters, arrays, fmt):
        req = {
            "spec": spec,
            "sources": sources,
            "filters": filters,
            "arrays": arrays,
            "width": fmt["width"],
            "height": fmt["height"],
            "pix_fmt": fmt["pix_fmt"],
        }

        r = requests.post(f"http://{self._domain}:{self._port}/new", json=req)
        if not r.ok:
            raise Exception(r.text)

        return r.json()

    def _export(
        self, pth, spec, sources, filters, arrays, fmt, encoder, encoder_opts, format
    ):
        req = {
            "spec": spec,
            "sources": sources,
            "filters": filters,
            "arrays": arrays,
            "width": fmt["width"],
            "height": fmt["height"],
            "pix_fmt": fmt["pix_fmt"],
            "output_path": pth,
            "encoder": encoder,
            "encoder_opts": encoder_opts,
            "format": format,
        }

        r = requests.post(f"http://{self._domain}:{self._port}/export", json=req)
        if not r.ok:
            raise Exception(r.text)

        return r.json()

    def _raw(self, namespace, start_i, end_i):
        r = requests.get(
            f"http://{self._domain}:{self._port}/{namespace}/raw/{start_i}-{end_i}"
        )
        if not r.ok:
            raise Exception(r.text)
        return r.content

    def hls_js_url(self):
        """Return the link to the yrden-hosted hls.js file"""
        return f"http://{self._domain}:{self._port}/hls.js"

    def __del__(self):
        if self._proc is not None:
            self._proc.kill()


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


class SourceILoc:
    def __init__(self, source):
        self._source = source

    def __getitem__(self, idx):
        if type(idx) != int:
            raise Exception(f"Source iloc index must be an integer, got a {type(idx)}")
        return SourceExpr(self._source, idx, True)


class Source:
    """A video source."""

    def __init__(
        self, server: YrdenServer, name: str, path: str, stream: int, service=None
    ):
        if service is None:
            # check if path is a http URL and, if so, automatically set the service
            # for example, the following code should work with just vf.Source(server, "tos_720p", "https://f.dominik.win/data/dve2/tos_720p.mp4")
            # this creates a storage service with endpoint "https://f.dominik.win/" and path "data/dve2/tos_720p.mp4"
            # don't use the root parameter in this case

            match = re.match(r"(http|https)://([^/]+)(.*)", path)
            if match is not None:
                endpoint = f"{match.group(1)}://{match.group(2)}"
                path = match.group(3)
                # remove leading slash
                if path.startswith("/"):
                    path = path[1:]
                service = StorageService("http", endpoint=endpoint)

        self._server = server
        self._name = name
        self._path = path
        self._stream = stream
        self._service = service

        self.iloc = SourceILoc(self)

        self._src = self._server._source(
            self._name, self._path, self._stream, self._service
        )

    def fmt(self):
        return {
            "width": self._src["width"],
            "height": self._src["height"],
            "pix_fmt": self._src["pix_fmt"],
        }

    def ts(self):
        return self._src["ts"]

    def __len__(self):
        return len(self._src["ts"])

    def __getitem__(self, idx):
        if type(idx) != Fraction:
            raise Exception("Source index must be a Fraction")
        return SourceExpr(self, idx, False)

    def play(self, *args, **kwargs):
        """Play the video live in the notebook."""

        domain = self.ts()
        render = lambda t, i: self[t]
        spec = Spec(domain, render, self.fmt())
        return spec.play(*args, **kwargs)


class StorageService:
    def __init__(self, service: str, **kwargs):
        if type(service) != str:
            raise Exception("Service name must be a string")
        self._service = service
        for k, v in kwargs.items():
            if type(v) != str:
                raise Exception(f"Value of {k} must be a string")
        self._config = kwargs

    def as_json(self):
        return {"service": self._service, "config": self._config}

    def __repr__(self):
        return f"{self._service}(config={self._config})"


def _json_arg(arg, skip_data_anot=False):
    if type(arg) == FilterExpr or type(arg) == SourceExpr:
        return {"Frame": arg._to_json_spec()}
    elif type(arg) == int:
        if skip_data_anot:
            return {"Int": arg}
        return {"Data": {"Int": arg}}
    elif type(arg) == str:
        if skip_data_anot:
            return {"String": arg}
        return {"Data": {"String": arg}}
    elif type(arg) == bytes:
        arg = list(arg)
        if skip_data_anot:
            return {"Bytes": arg}
        return {"Data": {"Bytes": arg}}
    elif type(arg) == float:
        if skip_data_anot:
            return {"Float": arg}
        return {"Data": {"Float": arg}}
    elif type(arg) == bool:
        if skip_data_anot:
            return {"Bool": arg}
        return {"Data": {"Bool": arg}}
    elif type(arg) == tuple or type(arg) == list:
        if skip_data_anot:
            return {"List": [_json_arg(x, True) for x in list(arg)]}
        return {"Data": {"List": [_json_arg(x, True) for x in list(arg)]}}
    else:
        raise Exception(f"Unknown arg type: {type(arg)}")


class Filter:
    """A video filter."""

    def __init__(self, name: str, tl_func=None, **kwargs):
        self._name = name

        # tl_func is the top level func, which is the true implementation, not just a pretty name
        if tl_func is None:
            self._func = name
        else:
            self._func = tl_func

        # filter infra args, not invocation args
        for k, v in kwargs.items():
            if type(v) != str:
                raise Exception(f"Value of {k} must be a string")
        self._kwargs = kwargs

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
            val = f'"{arg}"' if type(arg) == str else str(arg)
            args.append(str(val))
        for k, v in self._kwargs.items():
            val = f'"{v}"' if type(v) == str else str(v)
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
            if type(arg) == FilterExpr or type(arg) == SourceExpr:
                s = s.union(arg._sources())
        for arg in self._kwargs.values():
            if type(arg) == FilterExpr or type(arg) == SourceExpr:
                s = s.union(arg._sources())
        return s

    def _filters(self):
        f = {self._filter._name: self._filter}
        for arg in self._args:
            if type(arg) == FilterExpr:
                f = {**f, **arg._filters()}
        for arg in self._kwargs.values():
            if type(arg) == FilterExpr:
                f = {**f, **arg._filters()}
        return f


class UDF:
    """User-defined filter superclass"""

    def __init__(self, name: str):
        self._name = name
        self._socket_path = None
        self._p = None

    def filter(self, *args, **kwargs):
        raise Exception("User must implement the filter method")

    def filter_type(self, *args, **kwargs):
        raise Exception("User must implement the filter_type method")

    def into_filter(self):
        assert self._socket_path is None
        self._socket_path = f"/tmp/vidformer-{self._name}-{str(uuid.uuid4())}.sock"
        self._p = multiprocessing.Process(
            target=_run_udf_host, args=(self, self._socket_path)
        )
        self._p.start()
        return Filter(
            name=self._name, tl_func="IPC", socket=self._socket_path, func=self._name
        )

    def _handle_connection(self, connection):
        try:
            while True:
                frame_len = connection.recv(4)
                if not frame_len or len(frame_len) != 4:
                    break
                frame_len = int.from_bytes(frame_len, byteorder="big")
                data = connection.recv(frame_len)
                if not data:
                    break

                while len(data) < frame_len:
                    new_data = connection.recv(frame_len - len(data))
                    if not new_data:
                        raise Exception("Partial data received")
                    data += new_data

                obj = msgpack.unpackb(data, raw=False)
                f_func, f_op, f_args, f_kwargs = (
                    obj["func"],
                    obj["op"],
                    obj["args"],
                    obj["kwargs"],
                )

                response = None
                if f_op == "filter":
                    f_args = [self._deser_filter(x) for x in f_args]
                    f_kwargs = {k: self._deser_filter(v) for k, v in f_kwargs}
                    response = self.filter(*f_args, **f_kwargs)
                    if type(response) != UDFFrame:
                        raise Exception(
                            f"filter must return a UDFFrame, got {type(response)}"
                        )
                    if response.frame_type().pix_fmt() != "rgb24":
                        raise Exception(
                            f"filter must return a frame with pix_fmt 'rgb24', got {response.frame_type().pix_fmt()}"
                        )

                    response = response._response_ser()
                elif f_op == "filter_type":
                    f_args = [self._deser_filter_type(x) for x in f_args]
                    f_kwargs = {k: self._deser_filter_type(v) for k, v in f_kwargs}
                    response = self.filter_type(*f_args, **f_kwargs)
                    if type(response) != UDFFrameType:
                        raise Exception(
                            f"filter_type must return a UDFFrameType, got {type(response)}"
                        )
                    if response.pix_fmt() != "rgb24":
                        raise Exception(
                            f"filter_type must return a frame with pix_fmt 'rgb24', got {response.pix_fmt()}"
                        )
                    response = response._response_ser()
                else:
                    raise Exception(f"Unknown operation: {f_op}")

                response = msgpack.packb(response, use_bin_type=True)
                response_len = len(response).to_bytes(4, byteorder="big")
                connection.sendall(response_len)
                connection.sendall(response)
        finally:
            connection.close()

    def _deser_filter_type(self, obj):
        assert type(obj) == dict
        keys = list(obj.keys())
        assert len(keys) == 1
        type_key = keys[0]
        assert type_key in ["FrameType", "String", "Int", "Bool"]

        if type_key == "FrameType":
            frame = obj[type_key]
            assert type(frame) == dict
            assert "width" in frame
            assert "height" in frame
            assert "format" in frame
            assert type(frame["width"]) == int
            assert type(frame["height"]) == int
            assert frame["format"] == 2  # AV_PIX_FMT_RGB24
            return UDFFrameType(frame["width"], frame["height"], "rgb24")
        elif type_key == "String":
            assert type(obj[type_key]) == str
            return obj[type_key]
        elif type_key == "Int":
            assert type(obj[type_key]) == int
            return obj[type_key]
        elif type_key == "Bool":
            assert type(obj[type_key]) == bool
            return obj[type_key]
        else:
            assert False, f"Unknown type: {type_key}"

    def _deser_filter(self, obj):
        assert type(obj) == dict
        keys = list(obj.keys())
        assert len(keys) == 1
        type_key = keys[0]
        assert type_key in ["Frame", "String", "Int", "Bool"]

        if type_key == "Frame":
            frame = obj[type_key]
            assert type(frame) == dict
            assert "data" in frame
            assert "width" in frame
            assert "height" in frame
            assert "format" in frame
            assert type(frame["width"]) == int
            assert type(frame["height"]) == int
            assert frame["format"] == "rgb24"
            assert type(frame["data"]) == bytes

            data = np.frombuffer(frame["data"], dtype=np.uint8)
            data = data.reshape(frame["height"], frame["width"], 3)
            return UDFFrame(
                data, UDFFrameType(frame["width"], frame["height"], "rgb24")
            )
        elif type_key == "String":
            assert type(obj[type_key]) == str
            return obj[type_key]
        elif type_key == "Int":
            assert type(obj[type_key]) == int
            return obj[type_key]
        elif type_key == "Bool":
            assert type(obj[type_key]) == bool
            return obj[type_key]
        else:
            assert False, f"Unknown type: {type_key}"

    def _host(self, socket_path: str):
        if os.path.exists(socket_path):
            os.remove(socket_path)

        # start listening on the socket
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.bind(socket_path)
        sock.listen(1)

        while True:
            # accept incoming connection
            connection, client_address = sock.accept()
            thread = threading.Thread(
                target=self._handle_connection, args=(connection,)
            )
            thread.start()

    def __del__(self):
        if self._socket_path is not None:
            self._p.terminate()
            if os.path.exists(self._socket_path):
                # it's possible the process hasn't even created the socket yet
                os.remove(self._socket_path)


class UDFFrameType:
    """
    Frame type for use in UDFs.
    """

    def __init__(self, width: int, height: int, pix_fmt: str):
        assert type(width) == int
        assert type(height) == int
        assert type(pix_fmt) == str

        self._width = width
        self._height = height
        self._pix_fmt = pix_fmt

    def width(self):
        return self._width

    def height(self):
        return self._height

    def pix_fmt(self):
        return self._pix_fmt

    def _response_ser(self):
        return {
            "frame_type": {
                "width": self._width,
                "height": self._height,
                "format": 2,  # AV_PIX_FMT_RGB24
            }
        }

    def __repr__(self):
        return f"FrameType<{self._width}x{self._height}, {self._pix_fmt}>"


class UDFFrame:
    """A symbolic reference to a frame for use in UDFs."""

    def __init__(self, data: np.ndarray, f_type: UDFFrameType):
        assert type(data) == np.ndarray
        assert type(f_type) == UDFFrameType

        # We only support RGB24 for now
        assert data.dtype == np.uint8
        assert data.shape[2] == 3

        # check type matches
        assert data.shape[0] == f_type.height()
        assert data.shape[1] == f_type.width()
        assert f_type.pix_fmt() == "rgb24"

        self._data = data
        self._f_type = f_type

    def data(self):
        return self._data

    def frame_type(self):
        return self._f_type

    def _response_ser(self):
        return {
            "frame": {
                "data": self._data.tobytes(),
                "width": self._f_type.width(),
                "height": self._f_type.height(),
                "format": "rgb24",
            }
        }

    def __repr__(self):
        return f"Frame<{self._f_type.width()}x{self._f_type.height()}, {self._f_type.pix_fmt()}>"


def _run_udf_host(udf: UDF, socket_path: str):
    udf._host(socket_path)
