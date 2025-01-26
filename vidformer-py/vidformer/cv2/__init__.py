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
import uuid
import zlib
from bisect import bisect_right
from fractions import Fraction

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


def _ts_to_fps(timestamps):
    return int(1 / (timestamps[1] - timestamps[0]))  # TODO: Fix for non-integer fps


def _fps_to_ts(fps, n_frames):
    assert type(fps) is int
    return [Fraction(i, fps) for i in range(n_frames)]


_global_cv2_server = None


def _server():
    global _global_cv2_server
    if _global_cv2_server is None:
        _global_cv2_server = vf.YrdenServer()
    return _global_cv2_server


def set_server(server):
    """Set the server to use for the cv2 frontend."""
    global _global_cv2_server
    assert isinstance(server, vf.YrdenServer) or isinstance(server, vf.IgniServer)
    _global_cv2_server = server


class Frame:
    def __init__(self, f, fmt):
        self._f = f
        self._fmt = fmt
        self.shape = (fmt["height"], fmt["width"], 3)

        # denotes that the frame has not yet been modified
        # when a frame is modified, it is converted to rgb24 first
        self._modified = False

    def _mut(self):
        if self._modified:
            assert self._fmt["pix_fmt"] == "rgb24"
            return

        self._modified = True
        if self._fmt["pix_fmt"] != "rgb24":
            self._f = _filter_scale(self._f, pix_fmt="rgb24")
            self._fmt["pix_fmt"] = "rgb24"

    def copy(self):
        return Frame(self._f, self._fmt.copy())

    def numpy(self):
        """
        Return the frame as a numpy array.
        """

        self._mut()
        spec = vf.YrdenSpec([Fraction(0, 1)], lambda t, i: self._f, self._fmt)
        loader = spec.load(_server())

        frame_raster_rgb24 = loader[0]
        assert type(frame_raster_rgb24) is bytes
        assert len(frame_raster_rgb24) == self.shape[0] * self.shape[1] * 3
        raw_data_array = np.frombuffer(frame_raster_rgb24, dtype=np.uint8)
        frame = raw_data_array.reshape(self.shape)
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
        value = frameify(value, "value")

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

        if value.shape[0] != maxy - miny or value.shape[1] != maxx - minx:
            raise NotImplementedError("Shape mismatch")

        self._mut()
        value._mut()

        self._f = _slice_write_mat(self._f, value._f, miny, maxy, minx, maxx)


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


class VideoCapture:
    def __init__(self, path: str):
        server = _server()
        if type(path) is str:
            if isinstance(server, vf.YrdenServer):
                self._path = path
                self._source = vf.YrdenSource(server, str(uuid.uuid4()), path, 0)
            else:
                assert isinstance(server, vf.IgniServer)
                match = re.match(r"(http|https)://([^/]+)(.*)", path)
                if match is not None:
                    endpoint = f"{match.group(1)}://{match.group(2)}"
                    path = match.group(3)
                    if path.startswith("/"):
                        path = path[1:]
                    self._path = path
                    self._source = server.source(
                        path, 0, "http", {"endpoint": endpoint}
                    )
                else:
                    raise Exception(
                        "Using a VideoCapture source by name only works with http(s) URLs. You need to pass an IgniSource instead."
                    )
        elif isinstance(path, vf.IgniSource):
            assert isinstance(server, vf.IgniServer)
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
            return len(self._source.ts())
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

    def release(self):
        pass


class VideoWriter:
    def __init__(self, *args, **kwargs):
        server = _server()
        if isinstance(server, vf.YrdenServer):
            self._writer = _YrdenVideoWriter(*args, **kwargs)
        elif isinstance(server, vf.IgniServer):
            self._writer = _IgniVideoWriter(*args, **kwargs)
        else:
            raise Exception("Unsupported server type")

    def write(self, *args, **kwargs):
        return self._writer.write(*args, **kwargs)

    def release(self, *args, **kwargs):
        return self._writer.release(*args, **kwargs)

    def spec(self, *args, **kwargs):
        return self._writer.spec(*args, **kwargs)


class _IgniVideoWriter:
    def __init__(
        self,
        path,
        fourcc,
        fps,
        size,
        batch_size=1024,
        compression="gzip",
        vod_segment_length=Fraction(2, 1),
    ):
        server = _server()
        assert isinstance(server, vf.IgniServer)
        if path is not None:
            raise Exception(
                "Igni does not support writing to a file. VideoWriter path must be None"
            )
        if isinstance(fps, int):
            self._f_time = Fraction(1, fps)
        elif isinstance(fps, Fraction):
            self._f_time = 1 / fps
        else:
            raise Exception("fps must be an integer or a Fraction")

        assert isinstance(size, tuple) or isinstance(size, list)
        assert len(size) == 2
        width, height = size
        self._spec = server.create_spec(
            width, height, "yuv420p", vod_segment_length, 1 / self._f_time
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


class _YrdenVideoWriter:
    def __init__(self, path, fourcc, fps, size):
        assert isinstance(fourcc, VideoWriter_fourcc)
        if path is not None and not isinstance(path, str):
            raise Exception("path must be a string or None")
        self._path = path
        self._fourcc = fourcc
        self._fps = fps
        self._size = size

        self._frames = []
        self._pix_fmt = "yuv420p"

    def write(self, frame):
        frame = frameify(frame, "frame")

        if frame._fmt["pix_fmt"] != self._pix_fmt:
            f_obj = _filter_scale(frame._f, pix_fmt=self._pix_fmt)
            self._frames.append(f_obj)
        else:
            self._frames.append(frame._f)

    def release(self):
        if self._path is None:
            return

        spec = self.spec()
        server = _server()
        spec.save(server, self._path)

    def spec(self) -> vf.YrdenSpec:
        fmt = {
            "width": self._size[0],
            "height": self._size[1],
            "pix_fmt": self._pix_fmt,
        }
        domain = _fps_to_ts(self._fps, len(self._frames))
        spec = vf.YrdenSpec(domain, lambda t, i: self._frames[i], fmt)
        return spec


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
    source = vf.YrdenSource(server, str(uuid.uuid4()), path, 0)
    frame = Frame(source.iloc[0], source.fmt())
    return frame


def imwrite(path, img, *args):
    if len(args) > 0:
        raise NotImplementedError("imwrite does not support additional arguments")

    img = frameify(img)

    fmt = img._fmt.copy()
    width = fmt["width"]
    height = fmt["height"]
    f = img._f

    domain = [Fraction(0, 1)]

    if path.lower().endswith(".png"):
        img._mut()  # Make sure it's in rgb24
        spec = vf.YrdenSpec(
            domain,
            lambda t, i: img._f,
            {"width": width, "height": height, "pix_fmt": "rgb24"},
        )
        spec.save(_server(), path, encoder="png")
    elif path.lower().endswith((".jpg", ".jpeg")):
        if img._modified:
            # it's rgb24, we need to convert to something jpeg can handle
            f = _filter_scale(img._f, pix_fmt="yuv420p")
            fmt["pix_fmt"] = "yuv420p"
        else:
            if fmt["pix_fmt"] not in ["yuvj420p", "yuvj422p", "yuvj444p"]:
                f = _filter_scale(img._f, pix_fmt="yuvj420p")
                fmt["pix_fmt"] = "yuvj420p"

        spec = vf.YrdenSpec(domain, lambda t, i: f, fmt)
        spec.save(_server(), path, encoder="mjpeg")
    else:
        raise Exception("Unsupported image format")


def vidplay(video, *args, **kwargs):
    """
    Play a vidformer video specification.

    Args:
        video: one of [vidformer.Spec, vidformer.Source, vidformer.cv2.VideoWriter]
    """
    if isinstance(video, vf.YrdenSpec):
        return video.play(_server(), *args, **kwargs)
    elif isinstance(video, vf.YrdenSource):
        return video.play(_server(), *args, **kwargs)
    elif isinstance(video, VideoWriter):
        return vidplay(video._writer, *args, **kwargs)
    elif isinstance(video, _YrdenVideoWriter):
        return video.spec().play(_server(), *args, **kwargs)
    elif isinstance(video, _IgniVideoWriter):
        return video._spec.play(*args, **kwargs)
    elif isinstance(video, vf.IgniSpec):
        return video.play(*args, **kwargs)
    else:
        raise Exception("Unsupported video type to vidplay")


def zeros(shape, dtype=np.uint8):
    """
    Create a black frame. Mimics numpy.zeros.
    """
    assert isinstance(shape, tuple) or isinstance(shape, list)
    assert len(shape) == 3
    assert shape[2] == 3
    assert dtype == np.uint8

    height, width, _ = shape
    f = _black(width=width, height=height, pix_fmt="rgb24")
    fmt = {"width": width, "height": height, "pix_fmt": "rgb24"}
    return Frame(f, fmt)


def resize(src, dsize):
    src = frameify(src)
    src._mut()

    assert isinstance(dsize, tuple) or isinstance(dsize, list)
    assert len(dsize) == 2
    height, width = dsize

    f = _filter_scale(src._f, width=width, height=height)
    fmt = {"width": width, "height": height, "pix_fmt": "rgb24"}
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
    if _opencv2 is None:
        raise NotImplementedError("getFontScaleFromHeight requires the cv2 module")
    return _opencv2.getFontScaleFromHeight(*args, **kwargs)


def getTextSize(*args, **kwargs):
    """
    cv.getTextSize(	text, fontFace, fontScale, thickness	)
    """
    if _opencv2 is None:
        raise NotImplementedError("getTextSize requires the cv2 module")
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
