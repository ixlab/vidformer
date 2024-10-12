#!/usr/bin/env bash

set -e

bash ./scripts/deps_apt_ci.sh
bash ./scripts/deps_ffmpeg.sh
./scripts/deps_rust.sh
./scripts/deps_python.sh
