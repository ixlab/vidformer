#!/usr/bin/env bash

set -e

sudo apt update
sudo apt-get -y install \
  autoconf \
  automake \
  build-essential \
  cmake \
  git-core \
  libass-dev \
  libfreetype6-dev \
  libgnutls28-dev \
  libmp3lame-dev \
  libsdl2-dev \
  libtool \
  libva-dev \
  libvdpau-dev \
  libvorbis-dev \
  libxcb1-dev \
  libxcb-shm0-dev \
  libxcb-xfixes0-dev \
  meson \
  ninja-build \
  pkg-config \
  texinfo \
  wget \
  yasm \
  zlib1g-dev \
  libvpx-dev \
  libopus-dev \
  libx264-dev \
  libfdk-aac-dev \
  libclang-dev \
  valgrind \
  clang \
  libopencv-dev

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y

rm -rf ffmpeg
curl https://ffmpeg.org/releases/ffmpeg-7.0.tar.xz | tar xJ
mv ffmpeg-7.0 ffmpeg
patch -p0 < ./scripts/ff_patch.patch
pushd ffmpeg
mkdir build
./configure --prefix=${PWD}/build --pkg-config-flags="--static" --enable-debug --extra-cflags="-g" --enable-nonfree --enable-gpl --enable-libx264 --enable-libvpx --enable-libfdk-aac --disable-stripping --disable-decoder=exr,phm
make -j$(nproc)
make install
popd

pip3 install --upgrade pip # There's some bug that causes installing vidformer-py to fail in CI without this
pip3 install pytest
pip3 install -r snake-pit/requirements.txt
