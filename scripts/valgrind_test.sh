#!/usr/bin/env bash

set -e

./ffmpeg/build/bin/ffmpeg -y -i tos_720p.mp4 -c:v copy -c:a copy -t 30 tos_720p_short.mp4

valgrind --error-exitcode=1 --leak-check=full target/debug/vidformer-cli validate --name tos_720p_short --vid-path tos_720p_short.mp4 --stream 0
