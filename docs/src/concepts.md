# Concepts & Data Model

vidformer builds on the data model introduced in the [V2V paper](https://ixlab.github.io/v2v/).

* **Frames** are a single image.
Frames are represented as their resolution and pixel format (the type and layout of pixels in memory, such as `rgb24`, `gray8`, or `yuv420p`).

* **Videos** are sequences of frames represented as an array.
We index these arrays by rational numbers corresponding to their timestamp.

* **Filters** are functions which construct a frame.
Filters can take inputs, such as frames or data.
For example, `DrawText` may draw some text on a frame.

* **Specs** declarativly represent a video synthesis task.
They represent the construction of a result videos, which is itself modeled as an array.
    * Specs primairly contan `domain` and `render` functions.
        * A spec's ***domain*** function returns the timestamps of the output frames.
        * A spec's ***render*** function returns a composition of filters used to construct a frame at a spesific timestamp.

* **Data Arrays** allow using data in specs symbolically, as opposed to inserting constants directly into the spec.
These allow for deduplication and loading large data blobs efficiently.
    * Data Arrays can be backed by external data sources, such as SQL databases.
