from .. import vf

import uuid
from fractions import Fraction

server = vf.YrdenServer(
    bin="../target/release/vidformer-cli"
)  # TODO: don't hardcode this

CAP_PROP_FPS = "CAP_PROP_FPS"
CAP_PROP_FRAME_WIDTH = "CAP_PROP_FRAME_WIDTH"
CAP_PROP_FRAME_HEIGHT = "CAP_PROP_FRAME_HEIGHT"

FONT_HERSHEY_SIMPLEX = 0
FONT_HERSHEY_PLAIN = 1
FONT_HERSHEY_DUPLEX = 2
FONT_HERSHEY_COMPLEX = 3
FONT_HERSHEY_TRIPLEX = 4
FONT_HERSHEY_COMPLEX_SMALL = 5
FONT_HERSHEY_SCRIPT_SIMPLEX = 6
FONT_HERSHEY_SCRIPT_COMPLEX = 7
FONT_ITALIC = 16

_filter_scale = vf.Filter("Scale")
_filter_rectangle = vf.Filter("cv2.rectangle")
_filter_putText = vf.Filter("cv2.putText")


def _ts_to_fps(timestamps):
    return int(1 / (timestamps[1] - timestamps[0]))  # TODO: Fix for non-integer fps


def _fps_to_ts(fps, n_frames):
    assert type(fps) == int
    return [Fraction(i, fps) for i in range(n_frames)]


class _Frame:
    def __init__(self, f):
        self._f = f
        self._modified = False

    def _mut(self):
        self._modified = True
        self._f = _filter_scale(self._f, pix_fmt="rgb24")


class VideoCapture:
    def __init__(self, path):
        self._path = path
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
