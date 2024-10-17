from fractions import Fraction
import os
import json

import vidformer
import pandas as pd


def test_http_path():
    import vidformer as vf
    from fractions import Fraction

    server = vf.YrdenServer()
    tos = vidformer.Source(
        server, "tos_720p-XYZ", "https://f.dominik.win/data/dve2/tos_720p.mp4", 0
    )

    assert tos._server == server
    assert tos._name == "tos_720p-XYZ"
    assert tos._path == "data/dve2/tos_720p.mp4"
    assert tos._stream == 0
    assert type(tos._service) == vf.StorageService
    assert tos._service._service == "http"
    assert type(tos._service._config) == dict
    assert len(tos._service._config) == 1
    assert tos._service._config["endpoint"] == "https://f.dominik.win"

    # run these to make sure they don't crash
    tos.ts()
    tos.fmt()
