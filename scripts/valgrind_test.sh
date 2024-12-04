#!/usr/bin/env bash

set -e

./ffmpeg/build/bin/ffmpeg -y -i tos_720p.mp4 -c:v copy -c:a copy -t 30 tos_720p_short.mp4

if [[ $(rustc --version) == *"1.83"* ]]; then
    valgrind --error-exitcode=1 target/debug/vidformer-cli validate --name tos_720p_short --vid-path tos_720p_short.mp4 --stream 0
else
    # https://github.com/rust-lang/rust/issues/133574
    echo "Hey there! Looks like stable rust finally got past 1.83 so hopefully valgrind full check works again."
    echo "You need to re-add --leak-check=full to make CI pass again."
    exit 1
fi
