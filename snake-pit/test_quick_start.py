import json
import os
from fractions import Fraction

import pandas as pd

import vidformer


def test_quick_start():
    server = vidformer.YrdenServer()
    tos = vidformer.YrdenSource(server, "tos_720p", "tos_720p.mp4", 0)

    df = pd.read_csv("https://f.dominik.win/data/dve2/detections-tos.csv")

    grouped = df.groupby("frame")
    detections_per_frame = []
    max_frame = df["frame"].max()
    for frame_number in range(1, max_frame + 1):
        if frame_number in grouped.groups:
            frame_data = grouped.get_group(frame_number)
            detections = frame_data[
                ["class", "confidence", "x1", "y1", "x2", "y2"]
            ].to_dict("records")
        else:
            detections = []

        detections_per_frame.append(json.dumps(detections))

    while len(detections_per_frame) < len(tos.ts()):
        detections_per_frame.append("[]")

    # visualize it
    bbox = vidformer.Filter("BoundingBox")

    domain = tos.ts()[:500]

    def render(t, i):
        return bbox(tos[t], bounds=detections_per_frame[i])

    spec = vidformer.YrdenSpec(domain, render, tos.fmt())

    spec.save(server, "tos-bb.mp4")

    # check if the file exists
    assert os.path.exists("tos-bb.mp4")

    # delete the file
    os.remove("tos-bb.mp4")


def test_hello_world():
    from fractions import Fraction

    import vidformer as vf

    server = vf.YrdenServer()
    tos = vidformer.YrdenSource(
        server, "tos_720p", "https://f.dominik.win/data/dve2/tos_720p.mp4", 0
    )

    tos.ts()
    tos.fmt()

    domain = [Fraction(i, 24) for i in range(24 * 30)]

    def render(t: Fraction, i: int):
        clip_start_point = Fraction(5 * 60, 1)
        return tos[t + clip_start_point]

    spec = vf.YrdenSpec(domain, render, tos.fmt())
    spec.save(server, "quickstart-hello-world.mp4")

    assert os.path.exists("quickstart-hello-world.mp4")
    os.remove("quickstart-hello-world.mp4")


def test_bounding_boxes():
    import vidformer as vf

    server = vf.YrdenServer()
    tos = vidformer.YrdenSource(server, "tos_720p", "tos_720p.mp4", 0)

    # Load some data
    import json
    import urllib.request

    with urllib.request.urlopen(
        "https://f.dominik.win/data/dve2/tos_720p-objects.json"
    ) as r:
        detections_per_frame = json.load(r)

    bbox = vidformer.Filter("BoundingBox")  # load the built-in BoundingBox filter

    domain = tos.ts()[:500]

    def render(t, i):
        return bbox(
            tos[t + Fraction(5 * 60)], bounds=detections_per_frame[i + 5 * 60 * 24]
        )

    spec = vf.YrdenSpec(domain, render, tos.fmt())
    spec.save(server, "quickstart-bounding-box.mp4")

    assert os.path.exists("quickstart-bounding-box.mp4")
    os.remove("quickstart-bounding-box.mp4")


def test_composition():
    import vidformer as vf

    server = vf.YrdenServer()
    tos = vidformer.YrdenSource(server, "tos_720p", "tos_720p.mp4", 0)

    hstack = vf.Filter("HStack")
    vstack = vf.Filter("VStack")

    w, h = 1920, 1080

    def create_grid(tos, i, N, width, height, fmt="yuv420p"):
        grid = []
        for row in range(N):
            columns = []
            for col in range(N):
                index = row * N + col
                columns.append(tos.iloc[i + index])
            grid.append(hstack(*columns, width=width, height=height // N, format=fmt))
        final_grid = vstack(*grid, width=width, height=height, format=fmt)
        return final_grid

    domain = [Fraction(i, 24) for i in range(0, 500)]  # keep it short for a test

    def render(t, i):
        return create_grid(tos, i, 5, w, h)

    fmt = {"width": w, "height": h, "pix_fmt": "yuv420p"}

    spec = vf.YrdenSpec(domain, render, fmt)
    spec.save(server, "quickstart-composition.mp4")

    assert os.path.exists("quickstart-composition.mp4")
    os.remove("quickstart-composition.mp4")
