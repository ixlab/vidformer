# OpenCV/cv2 Functions

See [vidformer.cv2 API docs](https://ixlab.github.io/vidformer/vidformer-py/vidformer/cv2.html).

![cv2 showcase](./showcase_vidformer.png)

Legend:
* ✅ - Support
* 🔸 - Support via OpenCV cv2
* ❌ - Not yet implemented

## Vidformer-specific Functions

* `cv2.vidplay(video)` - Play a VideoWriter, Spec, or Source
* `VideoWriter.spec()` - Return the Spec of an output video
* `Frame.numpy()` - Return the frame as a numpy array
* `cv2.setTo` - The OpenCV `Mat.setTo` function (not in cv2)
* `cv2.zeros` - Create a black frame (equivalent to `numpy.zeros`)

## opencv

|**Class**|**Status**|
|---|---|
|VideoCapture|✅|
|VideoWriter|✅|
|VideoWriter_fourcc|✅|

|**Function**|**Status**|
|---|---|
|imread|✅|
|imwrite|✅|


## opencv.imgproc

Drawing Functions:

|**Function**|**Status**|
|---|---|
|arrowedLine|✅|
|circle|✅|
|clipLine|🔸|
|drawContours|✅|
|drawMarker|✅|
|ellipse|✅|
|ellipse2Poly|🔸|
|fillConvexPoly|✅|
|fillPoly|✅|
|getFontScaleFromHeight|🔸|
|getTextSize|🔸|
|line|✅|
|polylines|✅|
|putText|✅|
|rectangle|✅|

## opencv.core

|**Function**|**Status**|
|---|---|
|addWeighted|✅|
|copyMakeBorder|✅|
|flip|✅|
|hconcat|✅|
|resize|✅|
|rotate|✅|
|vconcat|✅|

## Output Comparison

Vidformer's cv2 output compared to native OpenCV ([source code](https://github.com/ixlab/vidformer/blob/main/snake-pit/test_cv2_showcase.py)):

**Vidformer:**

![Vidformer cv2 output](./showcase_vidformer.png)

**OpenCV:**

![OpenCV output](./showcase_opencv.png)

**Per-pixel Diff:**

![Difference between outputs](./showcase_diff.png)

The differences mainly come from Vidformer using FFmpeg's swscale for resize instead of OpenCV's resize.
