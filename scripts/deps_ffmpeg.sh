#!/usr/bin/env bash

set -e

rm -rf ffmpeg
curl https://ffmpeg.org/releases/ffmpeg-7.0.tar.xz | tar xJ
mv ffmpeg-7.0 ffmpeg
pushd ffmpeg
mkdir build
./configure --prefix=${PWD}/build --pkg-config-flags="--static" --enable-debug --extra-cflags="-g" --enable-nonfree --enable-gpl --enable-libx264 --enable-libvpx --enable-libfdk-aac --disable-stripping --disable-decoder=exr,phm
make -j$(nproc)
make install
popd
