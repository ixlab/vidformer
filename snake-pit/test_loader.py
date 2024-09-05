from fractions import Fraction
import vidformer as vf
import numpy as np


def test_loader_rgb24():
    server = vf.YrdenServer(bin="../target/release/vidformer-cli")
    tos = vf.Source(server, "tos_720p", "tos_720p.mp4", 0)

    domain = tos.ts()[:10]

    scale = vf.Filter("Scale")

    def render(t, i):
        return scale(tos.iloc[300 + i], pix_fmt="rgb24")

    fmt = tos.fmt()
    fmt["pix_fmt"] = "rgb24"
    spec = vf.Spec(domain, render, fmt)

    loader = spec.load(server)
    width = fmt["width"]
    height = fmt["height"]

    assert len(loader[:]) == len(domain)

    # Test single frame
    frame_raster_rgb24 = loader[0]
    assert type(frame_raster_rgb24) == bytes
    assert len(frame_raster_rgb24) == width * height * 3
    raw_data_array = np.frombuffer(frame_raster_rgb24, dtype=np.uint8)
    frame = raw_data_array.reshape((height, width, 3))
    assert frame.shape == (height, width, 3)

    # Test multiple frames
    multiple_frames = loader[1:9]
    assert type(multiple_frames) == list
    assert len(multiple_frames) == 8
    for i in range(8):
        assert type(multiple_frames[i]) == bytes
        assert len(multiple_frames[i]) == width * height * 3
        raw_data_array = np.frombuffer(multiple_frames[i], dtype=np.uint8)
        frame = raw_data_array.reshape((height, width, 3))
        assert frame.shape == (height, width, 3)

    # Test all frames
    all_frames = loader[:]
    assert type(all_frames) == list
    assert len(all_frames) == len(domain)
    for i in range(len(domain)):
        assert type(all_frames[i]) == bytes
        assert len(all_frames[i]) == width * height * 3
        raw_data_array = np.frombuffer(all_frames[i], dtype=np.uint8)
        frame = raw_data_array.reshape((height, width, 3))
        assert frame.shape == (height, width, 3)


def test_loader_yuv420p():
    server = vf.YrdenServer(bin="../target/release/vidformer-cli")
    tos = vf.Source(server, "tos_720p", "tos_720p.mp4", 0)

    domain = tos.ts()[:10]

    def render(t, i):
        return tos.iloc[300 + i]

    fmt = tos.fmt()
    assert fmt["pix_fmt"] == "yuv420p"
    spec = vf.Spec(domain, render, fmt)

    loader = spec.load(server)
    width = fmt["width"]
    height = fmt["height"]

    # Test single frame
    frame_raster_yuv420p = loader[0]
    assert type(frame_raster_yuv420p) == bytes
    assert len(frame_raster_yuv420p) == width * height * 3 // 2
    raw_data_array = np.frombuffer(frame_raster_yuv420p, dtype=np.uint8)
    frame = raw_data_array.reshape((height + height // 2, width))
    assert frame.shape == (height + height // 2, width)

    # Test multiple frames
    multiple_frames = loader[1:9]
    assert type(multiple_frames) == list
    assert len(multiple_frames) == 8
    for i in range(8):
        assert type(multiple_frames[i]) == bytes
        assert len(multiple_frames[i]) == width * height * 3 // 2
        raw_data_array = np.frombuffer(multiple_frames[i], dtype=np.uint8)
        frame = raw_data_array.reshape((height + height // 2, width))
        assert frame.shape == (height + height // 2, width)

    # Test all frames
    all_frames = loader[:]
    assert type(all_frames) == list
    assert len(all_frames) == len(domain)
    for i in range(len(domain)):
        assert type(all_frames[i]) == bytes
        assert len(all_frames[i]) == width * height * 3 // 2
        raw_data_array = np.frombuffer(all_frames[i], dtype=np.uint8)
        frame = raw_data_array.reshape((height + height // 2, width))
        assert frame.shape == (height + height // 2, width)
