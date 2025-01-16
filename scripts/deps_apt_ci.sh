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
  libopencv-dev \
  wait-for-it \
  docker-compose
