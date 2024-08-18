from fractions import Fraction
import os
import json

import vidformer as vf
import cv2


def test_udf():
    server = vf.YrdenServer(bin="../target/release/vidformer-cli")
    tos = vf.Source(server, "tos_720p", "tos_720p.mp4", 0)

    class MyFilter(vf.UDF):

        def filter(self, frame: vf.UDFFrame, name: str):
            """Return the result frame."""

            text = f"Hello, {name}!"

            image = frame.data().copy()
            cv2.putText(
                image,
                text,
                (100, 100),
                cv2.FONT_HERSHEY_SIMPLEX,
                1,
                (255, 0, 0),
                1,
            )
            return vf.UDFFrame(image, frame.frame_type())

        def filter_type(self, frame: vf.UDFFrameType, _name: str):
            """Returns the type of the output frame."""
            return frame

    mf_udf = MyFilter("MyFilter")
    my_filter = mf_udf.into_filter()

    scale = vf.Filter("Scale")

    domain = tos.ts()[:500]

    def render(t, i):
        f = scale(tos[t + Fraction(5 * 60)], format="rgb24", width=1280, height=720)
        f = my_filter(f, "world")
        f = scale(f, format="yuv420p", width=1280, height=720)
        return f

    spec = vf.Spec(domain, render, tos.fmt())
    spec.save(server, "udf.mp4")

    assert os.path.exists("udf.mp4")
    os.remove("udf.mp4")
