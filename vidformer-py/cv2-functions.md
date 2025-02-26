# OpenCV/cv2 Functions

See [vidformer.cv2 API docs](https://ixlab.github.io/vidformer/vidformer-py-cv2/).

> ⚠️ The `cv2` module is a work in progress. If you find a bug or need a missing feature implemented feel free to [file an issue](https://github.com/ixlab/vidformer/issues) or contribute yourself!

Legend:
* ✅ - Support
* 🔸 - Support via OpenCV cv2
* ❌ - Not yet implemented

## Vidformer-specific Functions

* `cv2.vidplay(video2)` - Play a VideoWriter, Spec, or Source
* `VideoWriter.spec()` - Return the Spec of an output video
* `Frame.numpy()` - Return the frame as a numpy array
* `cv2.setTo` - The OpenCV `Mat.setTo` function (not in cv2)
* `cv2.zeros` - Create a black frame (equiv to `numpy.zeros`)

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
|clipLine|❌|
|drawContours|❌|
|drawMarker|❌|
|ellipse|✅|
|ellipse2Poly|❌|
|fillConvexPoly|❌|
|fillPoly|❌|
|getFontScaleFromHeight|🔸|
|getTextSize|🔸|
|line|✅|
|polylines|❌|
|putText|✅|
|rectangle|✅|

## opencv.core

|**Function**|**Status**|
|---|---|
|addWeighted|✅|
|resize|✅|
