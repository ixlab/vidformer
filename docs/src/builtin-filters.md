# Built-in Filters

While most applications will use user-defined filters, vidformer ships with a handful of built-in filters to get you started:

## DrawText

`DrawText` does exactly what it sounds like: draw text on a frame.

For example:
```python
DrawText(frame, text="Hello, world!", x=100, y=100, size=48, color="white")
```

## BoundingBox

`BoundingBox` draws bounding boxes on a frame.

For example:
```python
BoundingBox(frame, bounds=obj)
```

Where `obj` is JSON with this schema:

```json
[
  {
    "class": "person",
    "confidence": 0.916827917098999,
    "x1": 683.0721842447916,
    "y1": 100.92174338626751,
    "x2": 1006.863525390625,
    "y2": 720
  },
  {
    "class": "dog",
    "confidence": 0.902531921863556,
    "x1": 360.8750813802083,
    "y1": 47.983140622720974,
    "x2": 606.76171875,
    "y2": 717.9591837897462
  }
]
```

## Scale

The `Scale` filter transforms one frame type to another.
It changes both resolution and pixel format.
This is *the most important filter* and is *essential* for building with vidformer.

Arguments:
```python
Scale(
    frame: Frame,
    width: int = None,
    height: int = None,
    pix_fmt: str = None)
```

By default missing `width`, `height` and `format` values are set to match `frame`.
`pix_fmt` must match ffmpeg's name for a pixel format.

For example:

```python
frame = Scale(frame, width=1280, height=720, pix_fmt="rgb24")
```

## IPC

IPC allows for calling User-Defined Filters (UDFs) running on the same system.
It is an **infrastructure-level filter** and is used to implement other filters.
It is configured with a `socket` and `func`, the filter's name, both strings.

The `IPC` filter can not be directly invoked, rather IPC filters are constructed by a server upon request.
This can be difficult, but [vidformer-py](./vidformer-py.md) handles this for you.
As of right now `IPC` only supports `rgb24` frames.

## HStack & VStack

HStack & VStack allow for composing multiple frames together, stacking them either horizontally or vertically.
It tries to automatically find a reasonable layout.

Arguments:
```python
HStack(
    *frames: list[Frame],
    width: int,
    height: int,
    format: str)
```

At least one frame is required, along with a `width`, `height` and `format`.

For example:
```python
compilation = HStack(left_frame, right_frame, width=1280, height=720, format="rgb24")
```
