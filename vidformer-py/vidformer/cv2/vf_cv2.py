from .. import vf

import uuid
from fractions import Fraction

server = vf.YrdenServer(
    bin="../target/release/vidformer-cli"
)  # TODO: don't hardcode this

CAP_PROP_FPS = "CAP_PROP_FPS"
CAP_PROP_FRAME_WIDTH = "CAP_PROP_FRAME_WIDTH"
CAP_PROP_FRAME_HEIGHT = "CAP_PROP_FRAME_HEIGHT"


def _ts_to_fps(timestamps):
    return int(1 / (timestamps[1] - timestamps[0]))  # TODO: Fix for non-integer fps


def _fps_to_ts(fps, n_frames):
    assert type(fps) == int
    return [Fraction(i, fps) for i in range(n_frames)]


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

    def write(self, frame):
        self._frames.append(frame)

    def release(self):
        spec = self.vf_spec()
        spec.save(server, self._path)

    def vf_spec(self):
        fmt = {
            "width": self._size[0],
            "height": self._size[1],
            "pix_fmt": "yuv420p",
        }
        domain = _fps_to_ts(self._fps, len(self._frames))
        spec = vf.Spec(domain, lambda t, i: self._frames[i], fmt)
        return spec


class VideoWriter_fourcc:
    def __init__(self, *args):
        self._args = args
