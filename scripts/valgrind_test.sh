#!/usr/bin/env bash

set -e

./ffmpeg/build/bin/ffmpeg -y -i tos_720p.mp4 -c:v copy -c:a copy -t 30 tos_720p_short.mp4

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
valgrind --error-exitcode=1 --leak-check=full --suppressions="$SCRIPT_DIR/vidformer.supp" target/debug/vidformer-cli validate --name tos_720p_short --vid-path tos_720p_short.mp4 --stream 0
rm tos_720p_short.mp4
