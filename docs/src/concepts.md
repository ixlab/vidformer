# Concepts & Data Model

vidformer uses the data model introduced in the [V2V paper](https://ixlab.github.io/v2v/).

**Videos** are sequences of frames represented as an array.
We index these arrays by rational numbers corresponding to their timestamp.

vidformer defines edits over video as **specs**, which are analogous to a SQL queries as a transformation over relations.
A spec *declarativly* defines how to construct a result output video.

