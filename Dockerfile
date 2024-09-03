FROM debian:bookworm AS build

RUN sed -i -e's/ main/ main contrib non-free/g' /etc/apt/sources.list.d/debian.sources && \
    apt update && \
    apt upgrade && \
    apt install -y curl build-essential pkg-config yasm libfdk-aac-dev libvpx-dev libx264-dev libopencv-dev clang && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /src

# Build ffmpeg
COPY ./scripts/ff_patch.patch ./
RUN curl https://ffmpeg.org/releases/ffmpeg-7.0.tar.xz | tar xJ && \
    mv ffmpeg-7.0 ffmpeg && \
    patch -p0 < ff_patch.patch
RUN cd ffmpeg && mkdir build && ./configure --prefix=/src/ffmpeg/build --pkg-config-flags="--static" --enable-nonfree --enable-gpl --enable-libx264 --enable-libvpx --enable-libfdk-aac --disable-stripping --disable-decoder=exr,phm && make -j$(nproc) && make install

# Install rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Build vidformer
COPY ./Cargo.toml ./
COPY ./vidformer ./vidformer
COPY ./vidformer-cli ./vidformer-cli
ENV FFMPEG_PKG_CONFIG_PATH="/src/ffmpeg/build/lib/pkgconfig" FFMPEG_INCLUDE_DIR="/src/ffmpeg/build/include"
RUN cargo build --release

FROM debian:bookworm

RUN sed -i -e's/ main/ main contrib non-free/g' /etc/apt/sources.list.d/debian.sources && \
    apt update && \
    apt upgrade && \
    apt install -y libopencv-dev libfdk-aac-dev && \
    rm -rf /var/lib/apt/lists/*

COPY --from=build /src/target/release/vidformer-cli /usr/local/bin/vidformer-cli

EXPOSE 8000
ENTRYPOINT [ "/usr/local/bin/vidformer-cli" ]
