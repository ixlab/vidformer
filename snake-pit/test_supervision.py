import vidformer.cv2 as vf_cv2
import vidformer.supervision as vf_sv
import supervision as sv
import numpy as np
import os
import pytest


IMG_PATH = "apollo.jpg"
TMP_PATH = "tmp.png"


def apollo_detections():
    return sv.Detections(
        xyxy=np.array([[1515.3, 0, 2675.9, 2298.1]], dtype=np.float32),
        mask=None,
        confidence=np.array([0.86027], dtype=np.float32),
        class_id=np.array([0]),
        tracker_id=None,
        data={"class_name": np.array(["person"], dtype="<U6")},
        metadata={},
    )


@pytest.mark.parametrize(
    "box_annotator_kwargs",
    [
        {},
        {"thickness": 10},
        {"color": sv.Color.WHITE},
        {"color": sv.ColorPalette.ROBOFLOW},
        {"color_lookup": sv.ColorLookup.INDEX},
        {
            "color": sv.ColorPalette.ROBOFLOW,
            "thickness": 10,
            "color_lookup": sv.ColorLookup.INDEX,
        },
    ],
)
def test_box_annotator(box_annotator_kwargs):
    detections = apollo_detections()
    img = vf_cv2.imread("apollo.jpg")

    box_annotator = vf_sv.BoxAnnotator(**box_annotator_kwargs)
    annotated_img = box_annotator.annotate(img.copy(), detections)

    vf_cv2.imwrite(TMP_PATH, annotated_img)
    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)


@pytest.mark.parametrize(
    "round_box_annotator_kwargs",
    [
        {},
        {"thickness": 10},
        {"color": sv.Color.WHITE},
        {"color": sv.ColorPalette.ROBOFLOW},
        {"color_lookup": sv.ColorLookup.INDEX},
        {"roundness": 0.6},
        {
            "color": sv.ColorPalette.ROBOFLOW,
            "thickness": 10,
            "color_lookup": sv.ColorLookup.INDEX,
            "roundness": 0.6,
        },
    ],
)
def test_round_box_annotator(round_box_annotator_kwargs):
    detections = apollo_detections()
    img = vf_cv2.imread("apollo.jpg")

    round_box_annotator = vf_sv.RoundBoxAnnotator(**round_box_annotator_kwargs)
    annotated_img = round_box_annotator.annotate(img.copy(), detections)

    vf_cv2.imwrite(TMP_PATH, annotated_img)
    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)


@pytest.mark.parametrize(
    "box_corner_annotator_kwargs",
    [
        {},
        {"thickness": 10},
        {"corner_length": 250},
        {"color": sv.Color.WHITE},
        {"color": sv.ColorPalette.ROBOFLOW},
        {"color_lookup": sv.ColorLookup.INDEX},
        {
            "color": sv.ColorPalette.ROBOFLOW,
            "thickness": 10,
            "corner_length": 250,
            "color_lookup": sv.ColorLookup.INDEX,
        },
    ],
)
def test_box_corner_annotator(box_corner_annotator_kwargs):
    detections = apollo_detections()
    img = vf_cv2.imread("apollo.jpg")

    box_annotator = vf_sv.BoxCornerAnnotator(**box_corner_annotator_kwargs)
    annotated_img = box_annotator.annotate(img.copy(), detections)

    vf_cv2.imwrite(TMP_PATH, annotated_img)
    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)


@pytest.mark.parametrize(
    "color_annotator_kwargs",
    [
        {},
        {"color": sv.Color.WHITE},
        {"color": sv.ColorPalette.ROBOFLOW},
        {"opacity": 0.9},
        {"color_lookup": sv.ColorLookup.INDEX},
        {
            "color": sv.ColorPalette.ROBOFLOW,
            "opacity": 0.9,
            "color_lookup": sv.ColorLookup.INDEX,
        },
    ],
)
def test_color_annotator(color_annotator_kwargs):
    detections = apollo_detections()
    img = vf_cv2.imread("apollo.jpg")

    color_annotator = vf_sv.ColorAnnotator(**color_annotator_kwargs)
    annotated_img = color_annotator.annotate(img.copy(), detections)

    vf_cv2.imwrite(TMP_PATH, annotated_img)
    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)


@pytest.mark.parametrize(
    "circle_annotator_kwargs",
    [
        {},
        {"thickness": 10},
        {"color": sv.Color.WHITE},
        {"color": sv.ColorPalette.ROBOFLOW},
        {"color_lookup": sv.ColorLookup.INDEX},
        {
            "color": sv.ColorPalette.ROBOFLOW,
            "thickness": 10,
            "color_lookup": sv.ColorLookup.INDEX,
        },
    ],
)
def test_circle_annotator(circle_annotator_kwargs):
    detections = apollo_detections()
    img = vf_cv2.imread("apollo.jpg")

    circle_annotator = vf_sv.CircleAnnotator(**circle_annotator_kwargs)
    annotated_img = circle_annotator.annotate(img.copy(), detections)

    vf_cv2.imwrite(TMP_PATH, annotated_img)
    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)


@pytest.mark.parametrize(
    "dot_annotator_kwargs",
    [
        {},
        {"color": sv.Color.WHITE},
        {"radius": 10},
        {"position": sv.Position.TOP_LEFT},
        {"color_lookup": sv.ColorLookup.INDEX},
        {"outline_thickness": 2},
        {"outline_thickness": 2, "outline_color": sv.Color.WHITE},
        {
            "color": sv.ColorPalette.ROBOFLOW,
            "radius": 10,
            "position": sv.Position.TOP_LEFT,
            "color_lookup": sv.ColorLookup.INDEX,
            "outline_thickness": 2,
            "outline_color": sv.Color.WHITE,
        },
    ],
)
def test_dot_annotator(dot_annotator_kwargs):
    detections = apollo_detections()
    img = vf_cv2.imread("apollo.jpg")

    dot_annotator = vf_sv.DotAnnotator(**dot_annotator_kwargs)
    annotated_img = dot_annotator.annotate(img.copy(), detections)

    vf_cv2.imwrite(TMP_PATH, annotated_img)
    assert os.path.exists(TMP_PATH)
    os.remove(TMP_PATH)
