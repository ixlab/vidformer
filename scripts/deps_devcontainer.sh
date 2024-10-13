#!/usr/bin/env bash

# This script runs after the devcontainer is built

set -e

bash ./scripts/deps_ffmpeg.sh
bash ./scripts/deps_python.sh
pip3 install -e ./vidformer-py

# Download test video if it doesn't exist
[ -f tos_720p.mp4 ] || curl -O https://f.dominik.win/data/dve2/tos_720p.mp4
