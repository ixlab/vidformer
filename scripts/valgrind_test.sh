#!/usr/bin/env bash

set -e

./ffmpeg/build/bin/ffmpeg -y -i tos_720p.mp4 -c:v copy -c:a copy -t 30 tos_720p_short.mp4

valgrind --error-exitcode=1 --leak-check=full target/debug/vidformer-cli validate --name tos_720p_short --vid-path tos_720p_short.mp4 --stream 0

# `--errors-for-leak-kinds=definite`: the Rust test harness leaves its own
# thread-local `Thread` in "possibly lost" on every run.
BASIC_TESTS=$(cargo test -p vidformer --test basic_tests --no-run --message-format=json \
    | jq -r 'select(.target.name == "basic_tests" and .executable != null) | .executable' \
    | tail -n 1)

LIB_TESTS=$(cargo test -p vidformer --lib --no-run --message-format=json \
    | jq -r 'select(.target.name == "vidformer" and .executable != null) | .executable' \
    | tail -n 1)

cd vidformer
valgrind --error-exitcode=1 --leak-check=full --errors-for-leak-kinds=definite "$BASIC_TESTS" \
    test_tos_transcode_1dec_1pool --exact --test-threads 1

valgrind --error-exitcode=1 --leak-check=full --errors-for-leak-kinds=definite "$LIB_TESTS" \
    stride_tests --test-threads 1

valgrind --error-exitcode=1 --leak-check=full --errors-for-leak-kinds=definite "$BASIC_TESTS" \
    test_tos_cv2_filter_chain --exact --test-threads 1
