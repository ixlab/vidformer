from fractions import Fraction
import os

import vidformer
import pytest


def test_source_ts():
    server = vidformer.YrdenServer(bin="../target/release/vidformer-cli")
    tos = vidformer.Source(server, "tos_720p", "tos_720p.mp4", 0)
    assert len(tos.ts()) == 17616


def test_short_clip():
    server = vidformer.YrdenServer(bin="../target/release/vidformer-cli")
    tos = vidformer.Source(server, "tos_720p", "tos_720p.mp4", 0)
    domain = [Fraction(i, 24) for i in range(0, 24 * 30)]

    def render(t, i):
        return tos[t + Fraction(24 * 5 * 60, 24)]

    spec = vidformer.Spec(domain, render, tos.fmt())
    spec.save(server, "short_clip.mp4")

    # check if the file exists
    assert os.path.exists("short_clip.mp4")

    # delete the file
    os.remove("short_clip.mp4")


def test_source_not_exists():
    server = vidformer.YrdenServer(bin="../target/release/vidformer-cli")
    with pytest.raises(Exception) as exception:
        tos = vidformer.Source(server, "fake", "fake.mp4", 0)

    # We want to make sure the most common user error has a clear error message
    assert (
        str(exception.value)
        == "Error loading source: IO error: File `fake.mp4` not found"
    )
