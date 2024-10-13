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
  && rm -rf /var/lib/apt/lists/*

USER vscode
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y && . "$HOME/.cargo/env" && cargo install mdbook

# We manage our own packages without debian's package manager
RUN mkdir -p ~/.config/pip && cat >> ~/.config/pip/pip.conf <<EOF
[global]
break-system-packages = true
EOF

USER root
