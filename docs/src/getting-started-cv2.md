# Getting Started - cv2

This is a walkthrough of getting started with vidformer OpenCV `cv2` compatability layer.

> ⚠️ Adding `cv2` functions is a work in progress. See the [cv2 filters page](./opencv-filters.md) for which functions have been implemented.

## Installation

See [Installation guide](./install.md)

Or you can [![Open In Colab](https://colab.research.google.com/assets/colab-badge.svg)](https://colab.research.google.com/github/ixlab/vidformer/blob/main/misc/Colab_Vidformer.ipynb).
> ⚠️ Due to how Colab networking works, vidformer can't stream/play results in Colab, only save them to disk. `cv2.vidplay()` will not work!

## Hello, world!

Copy in your video, or use ours:

```bash
curl -O https://f.dominik.win/data/dve2/tos_720p.mp4
```

Then just replace `import cv2` with `import vidformer.cv2 as cv2`.
Here's our example script:

```python
import vidformer.cv2 as cv2

cap = cv2.VideoCapture("tos_720p.mp4")
fps = cap.get(cv2.CAP_PROP_FPS)
width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

out = cv2.VideoWriter("output.mp4", cv2.VideoWriter_fourcc(*"mp4v"),
                        fps, (width, height))
while True:
    ret, frame = cap.read()
    if not ret:
      break

    cv2.putText(frame, "Hello, World!", (100, 100), cv2.FONT_HERSHEY_SIMPLEX,
                1, (255, 0, 0), 1)
    out.write(frame)

cap.release()
out.release()
```

### Stream the Results

Saving videos to disk works, but we can also display them in the notebook.
Since we stream the results and only render them on demand this can start practically instantly!

First, replace `"output.mp4"` with `None` to skip writing the video to disk.
Then you can use `cv2.vidplay()` to play the video!

```python
import vidformer.cv2 as cv2

cap = cv2.VideoCapture("tos_720p.mp4")
fps = cap.get(cv2.CAP_PROP_FPS)
width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

out = cv2.VideoWriter(None, cv2.VideoWriter_fourcc(*"mp4v"),
                        fps, (width, height))
while True:
    ret, frame = cap.read()
    if not ret:
      break

    cv2.putText(frame, "Hello, World!", (100, 100), cv2.FONT_HERSHEY_SIMPLEX,
                1, (255, 0, 0), 1)
    out.write(frame)

cap.release()
out.release()

cv2.vidplay(out)
```

> ⚠️ By default `cv2.vidplay()` will return a video which plays in a Jupyter Notebook. If running outside a jupyter notebook you can pass `method="link"` to return a link instead.
