"""
vidformer.numpy mimics numpy's API while supporting vidformer Frames.

When Frames are detected, dispatches to cv2 functions. Otherwise falls back to numpy.
"""

import numpy as _np
import vidformer.cv2 as _vf_cv2

# Re-export numpy's uint8 and other common dtypes
uint8 = _np.uint8
float32 = _np.float32
float64 = _np.float64
int32 = _np.int32
int64 = _np.int64


def _is_frame(obj):
    return isinstance(obj, _vf_cv2.Frame)


def _contains_frame(iterable):
    return any(_is_frame(x) for x in iterable)


def _is_frame_shape(shape, dtype):
    """Check if shape/dtype combination should produce a Frame."""
    return (
        isinstance(shape, (tuple, list))
        and len(shape) == 3
        and shape[2] in (1, 3)
        and (dtype is None or dtype == _np.uint8)
    )


def zeros(shape, dtype=None, *args, **kwargs):
    """Mimics numpy.zeros. Returns a Frame for video-like shapes (h, w, 1|3) with uint8."""
    if not args and not kwargs and _is_frame_shape(shape, dtype):
        return _vf_cv2.zeros(shape, dtype=_np.uint8)

    if dtype is None:
        return _np.zeros(shape, *args, **kwargs)
    return _np.zeros(shape, dtype=dtype, *args, **kwargs)


def ones(shape, dtype=None, *args, **kwargs):
    """Mimics numpy.ones. Returns a Frame for video-like shapes (h, w, 1|3) with uint8."""
    if not args and not kwargs and _is_frame_shape(shape, dtype):
        return _vf_cv2.solid(shape, (1, 1, 1), dtype=_np.uint8)

    if dtype is None:
        return _np.ones(shape, *args, **kwargs)
    return _np.ones(shape, dtype=dtype, *args, **kwargs)


def _parse_fill_value(fill_value):
    """Convert fill_value to a color tuple, or return None if unsupported."""
    if isinstance(fill_value, (int, float)):
        v = int(fill_value)
        return (v, v, v)
    elif isinstance(fill_value, (tuple, list)) and len(fill_value) == 3:
        return tuple(int(c) for c in fill_value)
    return None


def full(shape, fill_value, dtype=None, *args, **kwargs):
    """Mimics numpy.full. Returns a Frame for video-like shapes (h, w, 1|3) with uint8."""
    if not args and not kwargs and _is_frame_shape(shape, dtype):
        color = _parse_fill_value(fill_value)
        if color is not None:
            return _vf_cv2.solid(shape, color, dtype=_np.uint8)

    if dtype is None:
        return _np.full(shape, fill_value, *args, **kwargs)
    return _np.full(shape, fill_value, dtype=dtype, *args, **kwargs)


def zeros_like(a, dtype=None, *args, **kwargs):
    """Mimics numpy.zeros_like. Returns a Frame if input is a Frame."""
    if _is_frame(a):
        return _vf_cv2.zeros(a.shape, dtype=_np.uint8)
    return _np.zeros_like(a, dtype=dtype, *args, **kwargs)


def ones_like(a, dtype=None, *args, **kwargs):
    """Mimics numpy.ones_like. Returns a Frame if input is a Frame."""
    if _is_frame(a):
        return _vf_cv2.solid(a.shape, (1, 1, 1), dtype=_np.uint8)
    return _np.ones_like(a, dtype=dtype, *args, **kwargs)


def full_like(a, fill_value, dtype=None, *args, **kwargs):
    """Mimics numpy.full_like. Returns a Frame if input is a Frame."""
    if _is_frame(a):
        color = _parse_fill_value(fill_value)
        if color is not None:
            return _vf_cv2.solid(a.shape, color, dtype=_np.uint8)
        # Fall back to numpy for unsupported fill_value types
        return _np.full_like(a.numpy(), fill_value, dtype=dtype, *args, **kwargs)
    return _np.full_like(a, fill_value, dtype=dtype, *args, **kwargs)


def hstack(tup, *args, **kwargs):
    """Mimics numpy.hstack. Uses cv2.hconcat for Frames."""
    if _contains_frame(tup):
        return _vf_cv2.hconcat(tup)
    return _np.hstack(tup, *args, **kwargs)


def vstack(tup, *args, **kwargs):
    """Mimics numpy.vstack. Uses cv2.vconcat for Frames."""
    if _contains_frame(tup):
        return _vf_cv2.vconcat(tup)
    return _np.vstack(tup, *args, **kwargs)


def concatenate(arrays, axis=0, *args, **kwargs):
    """Mimics numpy.concatenate. Uses cv2.hconcat/vconcat for Frames."""
    if _contains_frame(arrays):
        if axis == 0:
            return _vf_cv2.vconcat(arrays)
        elif axis == 1:
            return _vf_cv2.hconcat(arrays)
        else:
            raise ValueError(
                f"Unsupported axis {axis} for Frame concatenation (only 0 or 1)"
            )
    return _np.concatenate(arrays, axis=axis, *args, **kwargs)


def flip(m, axis=None, *args, **kwargs):
    """Mimics numpy.flip. Uses cv2.flip for Frames."""
    if _is_frame(m):
        if axis is None:
            return _vf_cv2.flip(m, -1)
        elif axis == 0:
            return _vf_cv2.flip(m, 0)
        elif axis == 1:
            return _vf_cv2.flip(m, 1)
        else:
            raise ValueError(
                f"Unsupported axis {axis} for Frame flip (only 0, 1, or None)"
            )
    return _np.flip(m, axis=axis, *args, **kwargs)


def rot90(m, k=1, axes=(0, 1), *args, **kwargs):
    """Mimics numpy.rot90. Uses cv2.rotate for Frames."""
    if _is_frame(m):
        if axes != (0, 1):
            raise ValueError("Only axes=(0, 1) supported for Frame rotation")
        k = k % 4
        if k == 0:
            return m
        elif k == 1:
            return _vf_cv2.rotate(m, _vf_cv2.ROTATE_90_COUNTERCLOCKWISE)
        elif k == 2:
            return _vf_cv2.rotate(m, _vf_cv2.ROTATE_180)
        elif k == 3:
            return _vf_cv2.rotate(m, _vf_cv2.ROTATE_90_CLOCKWISE)
    return _np.rot90(m, k=k, axes=axes, *args, **kwargs)


def __getattr__(name):
    """Passthrough for all other numpy attributes."""
    return getattr(_np, name)
