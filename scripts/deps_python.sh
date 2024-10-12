#!/usr/bin/env bash

set -e

pip3 install --upgrade pip # There's some bug that causes installing vidformer-py to fail in CI without this
pip3 install pytest
pip3 install -r snake-pit/requirements.txt
