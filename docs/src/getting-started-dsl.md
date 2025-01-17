# Getting Started - DSL

This is a walkthrough of getting started with `vidformer-py` core DSL.

## Installation

See [Installation guide](./install.md)

## Hello, world!

> ⚠️ We assume this is in a Jupyter notebook. If not then [`.play()`](https://ixlab.github.io/vidformer/vidformer-py/vidformer.html#YrdenSpec.play) won't work and you have to use [`.save()`](https://ixlab.github.io/vidformer/vidformer-py/vidformer.html#YrdenSpec.save) instead.

We start by connecting to a server and registering a source:
```python
import vidformer as vf
from fractions import Fraction

server = vf.YrdenServer(domain='localhost', port=8000)

tos = vf.Source(
    server,
    "tos_720p",     # name (for pretty printing)
    "https://f.dominik.win/data/dve2/tos_720p.mp4",
    stream=0,       # index of the video stream we want to use
)

print(tos.ts())
print(tos.fmt())
```

This will print the timestamps of all the frames in the video, and then format information:
This may take a few seconds the first time, but frame times are cached afterwords.

```
> [Fraction(0, 1), Fraction(1, 24), Fraction(1, 12), Fraction(1, 8), ...]
> {'width': 1280, 'height': 720, 'pix_fmt': 'yuv420p'}
```

Now lets create a 30 second clip starting at the 5 minute mark.
The source video is at at a constant 24 FPS, so lets create a 24 FPS output as well:

```python
domain = [Fraction(i, 24) for i in range(24 * 30)]
```

Now we need to render each of these frames, so we define a render function.
```python
def render(t: Fraction, i: int):
    clip_start_point = Fraction(5 * 60, 1) # start at 5 * 60 seconds
    return tos[t + clip_start_point]
```
We used timestamp-based indexing here, but you can also use integer indexing (`tos.iloc[i + 5 * 60 * 24]`).

Now we can create a spec and play it in the browser.
We create a spec from the resulting video's frame timestamps (`domain`), a function to construct each output frame (`render`), and the output videos format (matching `tos.fmt()`).
```python
spec = vf.YrdenSpec(domain, render, tos.fmt())
spec.play(server)
```

This plays this result:
<video controls width="100%">
  <source src="https://f.dominik.win/data/dve2/quickstart-hello-world.mp4" type="video/mp4" />
</video>

> Some Jupyter environments are weird (i.e., VS Code), so  `.play()` might not work. Using `.play(..., method="iframe")` may help.

It's worth noting that we are playing frames in order here and outputing video at the same framerate we recieved, but that doesn't need to be the case.
Here are some things other things you can now try:

* Reversing the video
* Double the speed of the video
    * Either double the framerate or sample every other frame
* Shuffle the frames into a random order
* Combining frames from multiple videos
* Create a variable frame rate video
    * Note: `.play()` will not work with VFR, but `.save()` will.

## Bounding Boxes

Now let's overlay some bouding boxes over the entire clip:

```python
# Load some data
import urllib.request, json 
with urllib.request.urlopen("https://f.dominik.win/data/dve2/tos_720p-objects.json") as r:
    detections_per_frame = json.load(r)

bbox = vf.Filter("BoundingBox") # load the built-in BoundingBox filter

domain = tos.ts() # output should have same frame timestamps as our example clip

def render(t, i):
    return bbox(
        tos[t],
        bounds=detections_per_frame[i])

spec = vf.YrdenSpec(domain, render, tos.fmt())
spec.play(server)
```

This plays this result (video is just a sample clip):
<video controls width="100%">
  <source src="https://f.dominik.win/data/dve2/quickstart-bounding-box.mp4" type="video/mp4" />
</video>

## Composition

We can place frames next to each other with the `HStack` and `VStack` filters.
For example, `HStack(left_frame, middle_frame, right_frame, width=1280, height=720, format="yuv420p")` will place three frames side-by-side.

As a larger example, we can view a window function over frames as a 5x5 grid:

```python
hstack = vf.Filter("HStack")
vstack = vf.Filter("VStack")

w, h = 1920, 1080

def create_grid(tos, i, N, width, height, fmt="yuv420p"):
    grid = []
    for row in range(N):
        columns = []
        for col in range(N):
            index = row * N + col
            columns.append(tos.iloc[i + index])
        grid.append(hstack(*columns, width=width, height=height//N, format=fmt))
    final_grid = vstack(*grid, width=width, height=height, format=fmt)
    return final_grid

domain = [Fraction(i, 24) for i in range(0, 5000)]

def render(t, i):
    return create_grid(tos, i, 5, w, h)

fmt = {'width': w, 'height': h, 'pix_fmt': 'yuv420p'}

spec = vf.YrdenSpec(domain, render, fmt)
spec.play(server)
```

This plays this result (video is just a sample clip):
<video controls width="100%">
  <source src="https://f.dominik.win/data/dve2/quickstart-composition.mp4" type="video/mp4" />
</video>

## Viewing Telemetry (and User-Defined Filters)

This [notebook](https://github.com/ixlab/vidformer/blob/main/misc/UDFs-demo.ipynb) shows how to build custom filters to overlay data.

This plays this result (video is just a sample clip):
<video controls width="100%">
  <source src="https://f.dominik.win/data/dve2/quickstart-telemetry.mp4" type="video/mp4" />
</video>
