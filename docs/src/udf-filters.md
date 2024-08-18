# User-Defined Filters

To implement a new user-defined filter (UDF) you need to host a filter server over a UNIX Domain Socket.
The `vidformer-py` library makes this easy.

Filters take some combination of frames and data (string, int, bool) and return a *single* frame result.
The vidformer project uses Python-style arguments, allowing ordered and named arguments (`*args` and `**kwargs` style).

To do this we define a new filter class and host it:

```python
import vidformer as vf
import cv2

class MyFilter(vf.UDF):

    def filter(self, frame: vf.UDFFrame, name: str):
        """Return the result frame."""

        text = f"Hello, {name}!"

        image = frame.data().copy()
        cv2.putText(
		    image,
            text, 
            (100,100),
            cv2.FONT_HERSHEY_SIMPLEX,
            1,
            (255, 0, 0),
            1,
        )
        return vf.UDFFrame(image, frame.frame_type())

    def filter_type(self, frame: vf.UDFFrameType, _name: str):
        """Returns the type of the output frame."""
        return frame

mf_udf = MyFilter("MyFilter") # name used for pretty printing

my_filter = mf_udf.into_filter() # host the UDF in a subprocess, returns a vf.Filter
```

Now we can use our newly-created filter in specs: `my_filter(some_frame, "vidformer")`.

There is a catch, UDFs currently only support `rgb24` pixel formats.
So invoking `my_filter` will need to convert around this:

```python
scale = vf.Filter('Scale')

def render(t, i):
    f = scale(tos[t], format="rgb24", width=1280, height=720)
    f = my_filter(f, "world")
    f = scale(f, format="yuv420p", width=1280, height=720)
    return f
```
