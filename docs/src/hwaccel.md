# Hardware Acceleration

This page details how to compile vidformer with NVIDIA NVENC and similar hardware accelerated codecs.
We assume Docker is running on a system with CUDA.
Other codecs also work, see [FFmpeg docs](https://trac.ffmpeg.org/wiki/HWAccelIntro).
Testing this with GitHub Actions is impossible, so it may be a tad outdated.

The container must be run with these arguments: `--gpus all --runtime=nvidia -e NVIDIA_DRIVER_CAPABILITIES=all`.
If using Dev Containers, these can be added to the `devcontainer.json` file under `runArgs`.

The `scripts/deps_ffmpeg.sh` needs to be patched to include `--enable-ffnvcodec`.

Then you can run this in the container:
```bash
# From project root delete old FFmpeg build
rm -rf ffmpeg

sudo apt update -y
sudo apt install build-essential yasm cmake libtool libc6 libc6-dev unzip wget libnuma1 libnuma-dev -y

# Install (see https://trac.ffmpeg.org/wiki/HWAccelIntro)
rm -rf nv-codec-headers
git clone https://git.videolan.org/git/ffmpeg/nv-codec-headers.git
cd nv-codec-headers
# NOTE: Depending on your driver, you may want to checkout an older version tag here
make
sudo make install
cd -

# Install cuda
curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg \
  && curl -s -L https://nvidia.github.io/libnvidia-container/stable/deb/nvidia-container-toolkit.list | \
    sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | \
    sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list
sudo sed -i -e '/experimental/ s/^#//g' /etc/apt/sources.list.d/nvidia-container-toolkit.list
sudo apt update -y
sudo apt-get install -y nvidia-container-toolkit nvidia-cuda-toolkit

# Build ffmpeg
./scripts/deps_ffmpeg.sh

# Test it out
./ffmpeg/build/bin/ffmpeg -i myinputvid.mp4 -c:v h264_nvenc out.mp4 -y
```

Now you can recompile vidformer and use hardware accelerated codecs.
