#!/usr/bin/env bash

set -e

# Make sure we're in the right directory
cd "$(dirname "$0")/../snake-pit"


# Check if media files exist. Try to copy them from project root if they don't. Else, download them.

if [ ! -f tos_720p.mp4 ]; then
    if [ -f ../tos_720p.mp4 ]; then
        ln -s ../tos_720p.mp4 .
    else
        curl -O https://f.dominik.win/data/dve2/tos_720p.mp4
    fi
fi

if [ ! -f apollo.jpg ]; then
    if [ -f ../apollo.jpg ]; then
        cp ../apollo.jpg .
    else
        curl -O https://f.dominik.win/data/dve2/apollo.jpg
    fi
fi

export VIDFORMER_BIN='../target/debug/vidformer-cli'
if [ ! -f $VIDFORMER_BIN ]; then
    echo "Binary '$VIDFORMER_BIN' not found. Run 'cargo build' in the project root."
    exit 1
fi

pytest . --verbose
