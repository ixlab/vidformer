"""
vidformer.cv2 is the cv2 frontend for [vidformer](https://github.com/ixlab/vidformer).

> ⚠️ This module is a work in progress. See the [implemented functions list](https://ixlab.github.io/vidformer/opencv-filters.html).

**Quick links:**
* [📦 PyPI](https://pypi.org/project/vidformer/)
* [📘 Documentation - vidformer-py](https://ixlab.github.io/vidformer/vidformer-py/pdoc/)
* [📘 Documentation - vidformer.cv2](https://ixlab.github.io/vidformer/vidformer-py/pdoc/vidformer/cv2.html)
* [📘 Documentation - vidformer.supervision](https://ixlab.github.io/vidformer/vidformer-py/pdoc/vidformer/supervision.html)
* [🧑‍💻 Source Code](https://github.com/ixlab/vidformer/tree/main/vidformer-py/)
"""

import vidformer as vf

try:
    import cv2 as _opencv2
except Exception:
    _opencv2 = None

import re
import zlib
from bisect import bisect_right
from fractions import Fraction
import os

import numpy as np

CAP_PROP_POS_MSEC = 0
CAP_PROP_POS_FRAMES = 1
CAP_PROP_FRAME_WIDTH = 3
CAP_PROP_FRAME_HEIGHT = 4
CAP_PROP_FPS = 5
CAP_PROP_FRAME_COUNT = 7

FONT_HERSHEY_SIMPLEX = 0
FONT_HERSHEY_PLAIN = 1
FONT_HERSHEY_DUPLEX = 2
FONT_HERSHEY_COMPLEX = 3
FONT_HERSHEY_TRIPLEX = 4
FONT_HERSHEY_COMPLEX_SMALL = 5
FONT_HERSHEY_SCRIPT_SIMPLEX = 6
FONT_HERSHEY_SCRIPT_COMPLEX = 7
FONT_ITALIC = 16

FILLED = -1
LINE_4 = 4
LINE_8 = 8
LINE_AA = 16

_inline_mat = vf.Filter("_inline_mat")
_slice_mat = vf.Filter("_slice_mat")
_slice_write_mat = vf.Filter("_slice_write_mat")
_black = vf.Filter("_black")


_filter_scale = vf.Filter("Scale")
_filter_rectangle = vf.Filter("cv2.rectangle")
_filter_putText = vf.Filter("cv2.putText")
_filter_arrowedLine = vf.Filter("cv2.arrowedLine")
_filter_line = vf.Filter("cv2.line")
_filter_circle = vf.Filter("cv2.circle")
_filter_addWeighted = vf.Filter("cv2.addWeighted")
_filter_ellipse = vf.Filter("cv2.ellipse")
_set_to = vf.Filter("cv2.setTo")


def _ts_to_fps(timestamps):
    return int(1 / (timestamps[1] - timestamps[0]))  # TODO: Fix for non-integer fps


def _fps_to_ts(fps, n_frames):
    assert type(fps) is int
    return [Fraction(i, fps) for i in range(n_frames)]


_global_cv2_server = None


def _server():
    global _global_cv2_server
    if _global_cv2_server is None:
        if "VF_IGNI_ENDPOINT" in os.environ:
            server_endpoint = os.environ["VF_IGNI_ENDPOINT"]
            if "VF_IGNI_API_KEY" not in os.environ:
                raise Exception("VF_IGNI_API_KEY must be set")
            api_key = os.environ["VF_IGNI_API_KEY"]
            _global_cv2_server = vf.Server(server_endpoint, api_key)
        else:
            raise Exception(
                "No server set for the cv2 frontend (https://ixlab.github.io/vidformer/install.html). Set VF_IGNI_ENDPOINT and VF_IGNI_API_KEY environment variables or use cv2.set_server() before use."
            )
    return _global_cv2_server


def set_server(server):
    """Set the server to use for the cv2 frontend."""
    global _global_cv2_server
    assert isinstance(server, vf.Server)
    _global_cv2_server = server


def get_server():
    """Get the server used by the cv2 frontend."""
    return _server()


_PIX_FMT_MAP = {
    "rgb24": "rgb24",
    "yuv420p": "rgb24",
    "yuv422p": "rgb24",
    "yuv422p10le": "rgb24",
    "yuv444p": "rgb24",
    "yuvj420p": "rgb24",
    "yuvj422p": "rgb24",
    "yuvj444p": "rgb24",
    "gray": "gray",
}


def _top_level_pix_fmt(pix_fmt):
    if pix_fmt in _PIX_FMT_MAP:
        return _PIX_FMT_MAP[pix_fmt]
    raise Exception(f"Unsupported pix_fmt {pix_fmt}")


class Frame:
    def __init__(self, f, fmt):
        self._f = f
        self._fmt = fmt
        channels = 3 if _top_level_pix_fmt(fmt["pix_fmt"]) == "rgb24" else 1
        self.shape = (fmt["height"], fmt["width"], channels)

        # denotes that the frame has not yet been modified
        # when a frame is modified, it is converted to rgb24 first
        self._modified = False

    def _mut(self):
        if self._modified:
            assert self._fmt["pix_fmt"] in ["rgb24", "gray"]
            return

        self._modified = True
        if (
            self._fmt["pix_fmt"] != "rgb24"
            and _top_level_pix_fmt(self._fmt["pix_fmt"]) == "rgb24"
        ):
            self._f = _filter_scale(self._f, pix_fmt="rgb24")
            self._fmt["pix_fmt"] = "rgb24"
        elif (
            self._fmt["pix_fmt"] != "gray"
            and _top_level_pix_fmt(self._fmt["pix_fmt"]) == "gray"
        ):
            self._f = _filter_scale(self._f, pix_fmt="gray")
            self._fmt["pix_fmt"] = "gray"

    def copy(self):
        return Frame(self._f, self._fmt.copy())

    def numpy(self):
        """
        Return the frame as a numpy array.
        """

        self._mut()
        server = _server()
        frame = server.frame(
            self.shape[1], self.shape[0], self._fmt["pix_fmt"], self._f
        )
        assert type(frame) is bytes
        assert len(frame) == self.shape[0] * self.shape[1] * self.shape[2]
        raw_data_array = np.frombuffer(frame, dtype=np.uint8)
        frame = raw_data_array.reshape(self.shape)
        if self.shape[2] == 3:
            frame = frame[:, :, ::-1]  # convert RGB to BGR
        return frame

    def __getitem__(self, key):
        if not isinstance(key, tuple):
            raise NotImplementedError("Only 2D slicing is supported")

        if len(key) != 2:
            raise NotImplementedError("Only 2D slicing is supported")

        if not all(isinstance(x, slice) for x in key):
            raise NotImplementedError("Only 2D slicing is supported")

        miny = key[0].start if key[0].start is not None else 0
        maxy = key[0].stop if key[0].stop is not None else self.shape[0]
        minx = key[1].start if key[1].start is not None else 0
        maxx = key[1].stop if key[1].stop is not None else self.shape[1]

        # handle negative indices
        if miny < 0:
            miny = self.shape[0] + miny
        if maxy < 0:
            maxy = self.shape[0] + maxy
        if minx < 0:
            minx = self.shape[1] + minx
        if maxx < 0:
            maxx = self.shape[1] + maxx

        if (
            maxy <= miny
            or maxx <= minx
            or miny < 0
            or minx < 0
            or maxy > self.shape[0]
            or maxx > self.shape[1]
        ):
            raise NotImplementedError("Invalid slice")

        f = _slice_mat(self._f, miny, maxy, minx, maxx)
        fmt = self._fmt.copy()
        fmt["width"] = maxx - minx
        fmt["height"] = maxy - miny
        return Frame(f, fmt)

    def __setitem__(self, key, value):
        if type(key) is tuple:
            value = frameify(value, "value")

            if len(key) != 2:
                raise NotImplementedError("Only 2D slicing is supported")

            if not all(isinstance(x, slice) for x in key):
                raise NotImplementedError("Only 2D slicing is supported")

            miny = key[0].start if key[0].start is not None else 0
            maxy = key[0].stop if key[0].stop is not None else self.shape[0]
            minx = key[1].start if key[1].start is not None else 0
            maxx = key[1].stop if key[1].stop is not None else self.shape[1]

            # handle negative indices
            if miny < 0:
                miny = self.shape[0] + miny
            if maxy < 0:
                maxy = self.shape[0] + maxy
            if minx < 0:
                minx = self.shape[1] + minx
            if maxx < 0:
                maxx = self.shape[1] + maxx

            if (
                maxy <= miny
                or maxx <= minx
                or miny < 0
                or minx < 0
                or maxy > self.shape[0]
                or maxx > self.shape[1]
            ):
                raise NotImplementedError("Invalid slice")

            if value.shape[0] != maxy - miny or value.shape[1] != maxx - minx:
                raise NotImplementedError("Shape mismatch")

            self._mut()
            value._mut()

            self._f = _slice_write_mat(self._f, value._f, miny, maxy, minx, maxx)
        elif type(key) is Frame or type(key) is np.ndarray:
            key = frameify(key, "key")

            if key.shape[0] != self.shape[0] or key.shape[1] != self.shape[1]:
                raise NotImplementedError("Shape mismatch")

            if key.shape[2] != 1:
                raise NotImplementedError("Only 1-channel mask frames are supported")

            # Value should be a bgr or bgra color
            if (type(value) is not list and type(value) is not tuple) or len(
                value
            ) not in [3, 4]:
                raise NotImplementedError(
                    "Value should be a 3 or 4 element list or tuple"
                )
            value = [float(x) for x in value]
            if len(value) == 3:
                value.append(255.0)

            self._mut()
            key._mut()

            self._f = _set_to(self._f, value, key._f)
        else:
            raise NotImplementedError(
                "__setitem__ only supports slicing by a 2d tuple or a mask frame"
            )


def _inline_frame(arr):
    if arr.dtype != np.uint8:
        raise Exception("Only uint8 arrays are supported")
    if len(arr.shape) != 3:
        raise Exception("Only 3D arrays are supported")
    if arr.shape[2] != 3:
        raise Exception("To inline a frame, the array must have 3 channels")

    arr = arr[:, :, ::-1]
    if not arr.flags["C_CONTIGUOUS"]:
        arr = np.ascontiguousarray(arr)

    width = arr.shape[1]
    height = arr.shape[0]
    pix_fmt = "rgb24"

    data_gzip = zlib.compress(memoryview(arr), level=1)

    f = _inline_mat(
        data_gzip, width=width, height=height, pix_fmt=pix_fmt, compression="zlib"
    )
    fmt = {"width": width, "height": height, "pix_fmt": pix_fmt}

    # Return the resulting Frame object
    return Frame(f, fmt)


def _check_opencv2(method_name):
    if _opencv2 is None:
        raise NotImplementedError(
            f"{method_name} requires python OpenCV cv2. Either it's not installed or the import failed (such as a mission libGL.so.1)."
        )


class VideoCapture:
    def __init__(self, path: str):
        server = _server()
        if type(path) is str:
            match = re.match(r"(http|https)://([^/]+)(.*)", path)
            if match is not None:
                endpoint = f"{match.group(1)}://{match.group(2)}"
                path = match.group(3)
                if path.startswith("/"):
                    path = path[1:]
                self._path = path
                self._source = server.source(path, 0, "http", {"endpoint": endpoint})
            else:
                self._path = path
                self._source = server.source(path, 0, "fs", {"root": "."})
        elif isinstance(path, vf.Source):
            assert isinstance(server, vf.Server)
            self._path = path._name
            self._source = path
        self._next_frame_idx = 0

    def isOpened(self) -> bool:
        return True

    def get(self, prop):
        if prop == CAP_PROP_FPS:
            return _ts_to_fps(self._source.ts())
        elif prop == CAP_PROP_FRAME_WIDTH:
            return self._source.fmt()["width"]
        elif prop == CAP_PROP_FRAME_HEIGHT:
            return self._source.fmt()["height"]
        elif prop == CAP_PROP_FRAME_COUNT:
            return len(self._source)
        elif prop == CAP_PROP_POS_FRAMES:
            return self._next_frame_idx

        raise Exception(f"Unknown property {prop}")

    def set(self, prop, value):
        if prop == CAP_PROP_POS_FRAMES:
            assert value >= 0 and value < len(self._source.ts())
            self._next_frame_idx = value
        elif prop == CAP_PROP_POS_MSEC:
            t = Fraction(value, 1000)
            ts = self._source.ts()
            next_frame_idx = bisect_right(ts, t)
            self._next_frame_idx = next_frame_idx
        else:
            raise Exception(f"Unsupported property {prop}")

    def read(self):
        if self._next_frame_idx >= len(self._source):
            return False, None
        frame = self._source.iloc[self._next_frame_idx]
        self._next_frame_idx += 1
        frame = Frame(frame, self._source.fmt())
        return True, frame

    def __getitem__(self, key):
        if not isinstance(key, int):
            raise NotImplementedError("Only integer indexing is supported")
        if key < 0:
            key = len(self._source) + key
        if key < 0 or key >= len(self._source):
            raise IndexError("Index out of bounds")
        frame = self._source.iloc[key]
        frame = Frame(frame, self._source.fmt())
        return frame

    def __len__(self):
        return len(self._source)

    def release(self):
        pass


class VideoWriter:
    def __init__(
        self,
        path,
        fourcc,
        fps,
        size,
        batch_size=1024,
        compression="gzip",
        ttl=3600,
        pix_fmt="yuv420p",
        vod_segment_length=Fraction(2, 1),
    ):
        server = _server()
        assert isinstance(server, vf.Server)
        assert path is None or type(path) is str
        self._path = path
        if isinstance(fps, int):
            self._f_time = Fraction(1, fps)
        elif isinstance(fps, Fraction):
            self._f_time = 1 / fps
        else:
            raise Exception("fps must be an integer or a Fraction")

        assert isinstance(size, tuple) or isinstance(size, list)
        assert len(size) == 2
        width, height = size
        assert ttl is None or isinstance(ttl, int)
        self._spec = server.create_spec(
            width, height, pix_fmt, vod_segment_length, 1 / self._f_time, ttl=ttl
        )
        self._batch_size = batch_size
        assert compression is None or compression in ["gzip"]
        self._compression = compression
        self._idx = 0
        self._feb = vf._FrameExpressionBlock()

    def _flush(self, terminal=False):
        server = _server()
        if len(self._feb) > 0:
            server.push_spec_part_block(
                self._spec,
                self._idx - len(self._feb),
                [self._feb],
                terminal=terminal,
                compression=self._compression,
            )
            self._feb = vf._FrameExpressionBlock()
        else:
            server.push_spec_part_block(
                self._spec,
                self._idx - len(self._feb),
                [],
                terminal=terminal,
            )

    def spec(self):
        return self._spec

    def write(self, frame):
        if frame is not None:
            frame = frameify(frame, "frame")
            if frame._fmt["width"] != self._spec._fmt["width"]:
                raise Exception(
                    f"Frame type error; expected width {self._spec._fmt['width']}, got {frame._fmt['width']}"
                )
            if frame._fmt["height"] != self._spec._fmt["height"]:
                raise Exception(
                    f"Frame type error; expected height {self._spec._fmt['height']}, got {frame._fmt['height']}"
                )
            if frame._fmt["pix_fmt"] != self._spec._fmt["pix_fmt"]:
                f_obj = _filter_scale(frame._f, pix_fmt=self._spec._fmt["pix_fmt"])
                frame = Frame(f_obj, self._spec._fmt)
        self._feb.insert_frame(frame._f if frame is not None else None)
        self._idx += 1

        if len(self._feb) >= self._batch_size:
            self._flush()

    def release(self):
        self._flush(True)
        if self._path is not None:
            server = _server()
            server.export_spec(self._spec.id(), self._path)


class VideoWriter_fourcc:
    def __init__(self, *args):
        self._args = args


def frameify(obj, field_name=None):
    """
    Turn an object (e.g., ndarray) into a Frame.
    """

    if isinstance(obj, Frame):
        return obj
    elif isinstance(obj, np.ndarray):
        return _inline_frame(obj)
    else:
        if field_name is not None:
            raise Exception(
                f"Unsupported type for field {field_name}, expected Frame or np.ndarray"
            )
        else:
            raise Exception("Unsupported type, expected Frame or np.ndarray")


def imread(path, *args):
    if len(args) > 0:
        raise NotImplementedError("imread does not support additional arguments")
    assert path.lower().endswith((".jpg", ".jpeg", ".png"))
    server = _server()

    cap = VideoCapture(path)
    assert cap.isOpened()
    assert len(cap._source) == 1
    ret, frame = cap.read()
    assert ret
    cap.release()
    return frame


def imwrite(path, img, *args):
    if len(args) > 0:
        raise NotImplementedError("imwrite does not support additional arguments")

    img = frameify(img)
    fmt = img._fmt.copy()
    width = fmt["width"]
    height = fmt["height"]

    if path.lower().endswith(".png"):
        out_pix_fmt = "rgb24"
        encoder = "png"
    elif path.lower().endswith((".jpg", ".jpeg")):
        encoder = "mjpeg"
        if img._fmt["pix_fmt"] not in ["yuvj420p", "yuvj422p", "yuvj444p"]:
            out_pix_fmt = "yuvj420p"
        else:
            out_pix_fmt = img._fmt["pix_fmt"]
    else:
        raise Exception("Unsupported image format")

    if img._fmt["pix_fmt"] != out_pix_fmt:
        f = _filter_scale(img._f, pix_fmt=out_pix_fmt)
        img = Frame(f, {"width": width, "height": height, "pix_fmt": out_pix_fmt})

    writer = VideoWriter(None, None, 1, (width, height), pix_fmt=out_pix_fmt)
    writer.write(img)
    writer.release()

    spec = writer.spec()
    server = _server()
    server.export_spec(spec.id(), path, encoder=encoder)


def vidplay(video, method="html"):
    """
    Play a vidformer video specification.
    """
    if isinstance(video, VideoWriter):
        return video.spec().play(method=method)
    elif isinstance(video, vf.Spec):
        return video.play(method=method)
    else:
        raise Exception("Unsupported video type to vidplay")


def zeros(shape, dtype=np.uint8):
    """
    Create a black frame. Mimics numpy.zeros.
    """
    assert isinstance(shape, tuple) or isinstance(shape, list)
    assert len(shape) == 3
    assert shape[2] in [1, 3]
    assert dtype == np.uint8

    height, width, channels = shape
    if channels == 1:
        pix_fmt = "gray"
    else:
        pix_fmt = "rgb24"

    f = _black(width=width, height=height, pix_fmt=pix_fmt)
    fmt = {"width": width, "height": height, "pix_fmt": pix_fmt}
    return Frame(f, fmt)


def resize(src, dsize):
    src = frameify(src)
    src._mut()

    assert isinstance(dsize, tuple) or isinstance(dsize, list)
    assert len(dsize) == 2
    width, height = dsize

    f = _filter_scale(src._f, width=width, height=height)
    fmt = {"width": width, "height": height, "pix_fmt": src._fmt["pix_fmt"]}
    return Frame(f, fmt)


def rectangle(img, pt1, pt2, color, thickness=None, lineType=None, shift=None):
    """
    cv.rectangle(	img, pt1, pt2, color[, thickness[, lineType[, shift]]]	)
    """

    img = frameify(img)
    img._mut()

    assert len(pt1) == 2
    assert len(pt2) == 2
    pt1 = [int(x) for x in pt1]
    pt2 = [int(x) for x in pt2]

    assert len(color) == 3 or len(color) == 4
    color = [float(x) for x in color]
    if len(color) == 3:
        color.append(255.0)

    args = []
    if thickness is not None:
        assert isinstance(thickness, int)
        args.append(thickness)
    if lineType is not None:
        assert isinstance(lineType, int)
        assert thickness is not None
        args.append(lineType)
    if shift is not None:
        assert isinstance(shift, int)
        assert shift is not None
        args.append(shift)

    img._f = _filter_rectangle(img._f, pt1, pt2, color, *args)


def putText(
    img,
    text,
    org,
    fontFace,
    fontScale,
    color,
    thickness=None,
    lineType=None,
    bottomLeftOrigin=None,
):
    """
    cv.putText(	img, text, org, fontFace, fontScale, color[, thickness[, lineType[, bottomLeftOrigin]]]	)
    """

    img = frameify(img)
    img._mut()

    assert isinstance(text, str)

    assert len(org) == 2
    org = [int(x) for x in org]

    assert isinstance(fontFace, int)
    assert isinstance(fontScale, float) or isinstance(fontScale, int)
    fontScale = float(fontScale)

    assert len(color) == 3 or len(color) == 4
    color = [float(x) for x in color]
    if len(color) == 3:
        color.append(255.0)

    args = []
    if thickness is not None:
        assert isinstance(thickness, int)
        args.append(thickness)
    if lineType is not None:
        assert isinstance(lineType, int)
        assert thickness is not None
        args.append(lineType)
    if bottomLeftOrigin is not None:
        assert isinstance(bottomLeftOrigin, bool)
        assert lineType is not None
        args.append(bottomLeftOrigin)

    img._f = _filter_putText(img._f, text, org, fontFace, fontScale, color, *args)


def arrowedLine(
    img, pt1, pt2, color, thickness=None, line_type=None, shift=None, tipLength=None
):
    """
    cv.arrowedLine(	img, pt1, pt2, color[, thickness[, line_type[, shift[, tipLength]]]]	)
    """
    img = frameify(img)
    img._mut()

    assert len(pt1) == 2
    assert len(pt2) == 2
    assert all(isinstance(x, int) for x in pt1)
    assert all(isinstance(x, int) for x in pt2)

    assert len(color) == 3 or len(color) == 4
    color = [float(x) for x in color]
    if len(color) == 3:
        color.append(255.0)

    args = []
    if thickness is not None:
        assert isinstance(thickness, int)
        args.append(thickness)
    if line_type is not None:
        assert isinstance(line_type, int)
        assert thickness is not None
        args.append(line_type)
    if shift is not None:
        assert isinstance(shift, int)
        assert shift is not None
        args.append(shift)
    if tipLength is not None:
        assert isinstance(tipLength, float)
        assert shift is not None
        args.append(tipLength)

    img._f = _filter_arrowedLine(img._f, pt1, pt2, color, *args)


def line(img, pt1, pt2, color, thickness=None, lineType=None, shift=None):
    img = frameify(img)
    img._mut()

    assert len(pt1) == 2
    assert len(pt2) == 2
    pt1 = [int(x) for x in pt1]
    pt2 = [int(x) for x in pt2]

    assert len(color) == 3 or len(color) == 4
    color = [float(x) for x in color]
    if len(color) == 3:
        color.append(255.0)

    args = []
    if thickness is not None:
        assert isinstance(thickness, int)
        args.append(thickness)
    if lineType is not None:
        assert isinstance(lineType, int)
        assert thickness is not None
        args.append(lineType)
    if shift is not None:
        assert isinstance(shift, int)
        assert shift is not None
        args.append(shift)

    img._f = _filter_line(img._f, pt1, pt2, color, *args)


def circle(img, center, radius, color, thickness=None, lineType=None, shift=None):
    img = frameify(img)
    img._mut()

    assert len(center) == 2
    center = [int(x) for x in center]

    assert isinstance(radius, int)

    assert len(color) == 3 or len(color) == 4
    color = [float(x) for x in color]
    if len(color) == 3:
        color.append(255.0)

    args = []
    if thickness is not None:
        assert isinstance(thickness, int)
        args.append(thickness)
    if lineType is not None:
        assert isinstance(lineType, int)
        assert thickness is not None
        args.append(lineType)
    if shift is not None:
        assert isinstance(shift, int)
        assert shift is not None
        args.append(shift)

    img._f = _filter_circle(img._f, center, radius, color, *args)


def getFontScaleFromHeight(*args, **kwargs):
    """
    cv.getFontScaleFromHeight(	fontFace, pixelHeight[, thickness]	)
    """
    _check_opencv2("getFontScaleFromHeight")
    return _opencv2.getFontScaleFromHeight(*args, **kwargs)


def getTextSize(*args, **kwargs):
    """
    cv.getTextSize(	text, fontFace, fontScale, thickness	)
    """
    _check_opencv2("getTextSize")
    return _opencv2.getTextSize(*args, **kwargs)


def addWeighted(src1, alpha, src2, beta, gamma, dst=None, dtype=-1):
    """
    cv.addWeighted(	src1, alpha, src2, beta, gamma[, dst[, dtype]]	) -> 	dst
    """
    src1 = frameify(src1, "src1")
    src2 = frameify(src2, "src2")
    src1._mut()
    src2._mut()

    if dst is None:
        dst = Frame(src1._f, src1._fmt.copy())
    else:
        assert isinstance(dst, Frame), "dst must be a Frame"
    dst._mut()

    assert isinstance(alpha, float) or isinstance(alpha, int)
    assert isinstance(beta, float) or isinstance(beta, int)
    assert isinstance(gamma, float) or isinstance(gamma, int)
    alpha = float(alpha)
    beta = float(beta)
    gamma = float(gamma)

    if dtype != -1:
        raise Exception("addWeighted does not support the dtype argument")

    dst._f = _filter_addWeighted(src1._f, alpha, src2._f, beta, gamma)
    return dst


def ellipse(
    img,
    center,
    axes,
    angle,
    startAngle,
    endAngle,
    color,
    thickness=1,
    lineType=LINE_8,
    shift=0,
):
    img = frameify(img)
    img._mut()

    assert len(center) == 2
    center = [int(x) for x in center]

    assert len(axes) == 2
    axes = [int(x) for x in axes]

    assert isinstance(angle, float) or isinstance(angle, int)
    assert isinstance(startAngle, float) or isinstance(startAngle, int)
    assert isinstance(endAngle, float) or isinstance(endAngle, int)
    angle = float(angle)
    startAngle = float(startAngle)
    endAngle = float(endAngle)

    assert len(color) == 3 or len(color) == 4
    color = [float(x) for x in color]
    if len(color) == 3:
        color.append(255.0)

    assert isinstance(thickness, int)
    assert isinstance(lineType, int)
    assert isinstance(shift, int)

    img._f = _filter_ellipse(
        img._f,
        center,
        axes,
        angle,
        startAngle,
        endAngle,
        color,
        thickness,
        lineType,
        shift,
    )


# Stubs for unimplemented functions


def clipLine(*args, **kwargs):
    raise NotImplementedError("clipLine is not yet implemented in the cv2 frontend")


def drawContours(*args, **kwargs):
    raise NotImplementedError("drawContours is not yet implemented in the cv2 frontend")


def drawMarker(*args, **kwargs):
    raise NotImplementedError("drawMarker is not yet implemented in the cv2 frontend")


def ellipse2Poly(*args, **kwargs):
    raise NotImplementedError("ellipse2Poly is not yet implemented in the cv2 frontend")


def fillConvexPoly(*args, **kwargs):
    raise NotImplementedError(
        "fillConvexPoly is not yet implemented in the cv2 frontend"
    )


def fillPoly(*args, **kwargs):
    raise NotImplementedError("fillPoly is not yet implemented in the cv2 frontend")


def polylines(*args, **kwargs):
    raise NotImplementedError("polylines is not yet implemented in the cv2 frontend")
