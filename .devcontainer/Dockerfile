FROM mcr.microsoft.com/devcontainers/base:bookworm

RUN sed -i -e's/ main/ main contrib non-free/g' /etc/apt/sources.list.d/debian.sources && apt update && apt -y install \
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
  clang \
  libopencv-dev \
  python3 \
  python3-pip \
  r-base \
  r-base-dev \
  r-cran-ggplot2 \
  r-cran-irkernel \
  && rm -rf /var/lib/apt/lists/*

RUN sed -i -e's/fpic/fPIC/g' /etc/R/Makeconf && echo 'MAKEFLAGS = -j8' >> /etc/R/Makeconf && R -e "install.packages('duckdb')"

USER vscode
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y && . "$HOME/.cargo/env" && cargo install mdbook
USER root
