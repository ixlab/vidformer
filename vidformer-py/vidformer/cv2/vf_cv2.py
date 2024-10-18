from .. import vf

try:
    import cv2 as _opencv2
except:
    _opencv2 = None

import uuid
from fractions import Fraction
from bisect import bisect_right

CAP_PROP_POS_MSEC = 0
CAP_PROP_POS_FRAMES = 1
CAP_PROP_FRAME_WIDTH = 3
CAP_PROP_FRAME_HEIGHT = 4
CAP_PROP_FPS = 5

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

_filter_scale = vf.Filter("Scale")
_filter_rectangle = vf.Filter("cv2.rectangle")
_filter_putText = vf.Filter("cv2.putText")
_filter_arrowedLine = vf.Filter("cv2.arrowedLine")
_filter_line = vf.Filter("cv2.line")
_filter_circle = vf.Filter("cv2.circle")
_filter_addWeighted = vf.Filter("cv2.addWeighted")


def _ts_to_fps(timestamps):
    return int(1 / (timestamps[1] - timestamps[0]))  # TODO: Fix for non-integer fps


def _fps_to_ts(fps, n_frames):
    assert type(fps) == int
    return [Fraction(i, fps) for i in range(n_frames)]


_global_cv2_server = None


def _server():
    global _global_cv2_server
    if _global_cv2_server is None:
        _global_cv2_server = vf.YrdenServer()
    return _global_cv2_server


def set_cv2_server(server):
    """Set the server to use for the cv2 frontend."""
    global _global_cv2_server
    assert isinstance(server, vf.YrdenServer)
    _global_cv2_server = server


class _Frame:
    def __init__(self, f):
        self._f = f

        # denotes that the frame has not yet been modified
        # when a frame is modified, it is converted to rgb24 first
        self._modified = False

    def _mut(self):
        self._modified = True
        self._f = _filter_scale(self._f, pix_fmt="rgb24")


class VideoCapture:
    def __init__(self, path):
        self._path = path
        server = _server()
        self._source = vf.Source(server, str(uuid.uuid4()), path, 0)
        self._next_frame_idx = 0

    def isOpened(self):
        return True

    def get(self, prop):
        if prop == CAP_PROP_FPS:
            return _ts_to_fps(self._source.ts())
        elif prop == CAP_PROP_FRAME_WIDTH:
            return self._source.fmt()["width"]
        elif prop == CAP_PROP_FRAME_HEIGHT:
            return self._source.fmt()["height"]

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
        if self._next_frame_idx >= len(self._source.ts()):
            return False, None
        frame = self._source.iloc[self._next_frame_idx]
        self._next_frame_idx += 1
        frame = _Frame(frame)
        return True, frame

    def release(self):
        pass


class VideoWriter:
    def __init__(self, path, fourcc, fps, size):
        assert isinstance(fourcc, VideoWriter_fourcc)
        self._path = path
        self._fourcc = fourcc
        self._fps = fps
        self._size = size

        self._frames = []
        self._pix_fmt = "yuv420p"

    def write(self, frame):
        if not isinstance(frame, _Frame):
            raise Exception("frame must be a _Frame object")
        if frame._modified:
            f_obj = _filter_scale(frame._f, pix_fmt=self._pix_fmt)
            self._frames.append(f_obj)
        else:
            self._frames.append(frame._f)

    def release(self):
        spec = self.vf_spec()
        server = _server()
        spec.save(server, self._path)

    def vf_spec(self):
        fmt = {
            "width": self._size[0],
            "height": self._size[1],
            "pix_fmt": self._pix_fmt,
        }
        domain = _fps_to_ts(self._fps, len(self._frames))
        spec = vf.Spec(domain, lambda t, i: self._frames[i], fmt)
        return spec


class VideoWriter_fourcc:
    def __init__(self, *args):
        self._args = args


def rectangle(img, pt1, pt2, color, thickness=None, lineType=None, shift=None):
    """
    cv.rectangle(	img, pt1, pt2, color[, thickness[, lineType[, shift]]]	)
    """

    assert isinstance(img, _Frame)
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

    assert isinstance(img, _Frame)
    img._mut()

    assert isinstance(text, str)

    assert len(org) == 2
    assert all(isinstance(x, int) for x in org)

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
    assert isinstance(img, _Frame)
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
    assert isinstance(img, _Frame)
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
    assert isinstance(img, _Frame)
    img._mut()

    assert len(center) == 2
    assert all(isinstance(x, int) for x in center)

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
    assert isinstance(src1, _Frame)
    assert isinstance(src2, _Frame)
    src1._mut()
    src2._mut()

    if dst is None:
        dst = _Frame(src1._f)
    else:
        assert isinstance(dst, _Frame)
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


# Stubs for unimplemented functions


def clipLine(*args, **kwargs):
    raise NotImplementedError("clipLine is not yet implemented in the cv2 frontend")


def drawContours(*args, **kwargs):
    raise NotImplementedError("drawContours is not yet implemented in the cv2 frontend")


def drawMarker(*args, **kwargs):
    raise NotImplementedError("drawMarker is not yet implemented in the cv2 frontend")


def ellipse(*args, **kwargs):
    raise NotImplementedError("ellipse is not yet implemented in the cv2 frontend")


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
