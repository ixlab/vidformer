"""
vidformer.supervision is the [supervision](https://supervision.roboflow.com/) frontend for [vidformer](https://github.com/ixlab/vidformer).
"""

from math import sqrt

import numpy as np
import supervision as _sv
from supervision import Color, ColorLookup, ColorPalette, Detections
from supervision.annotators.utils import resolve_color, resolve_text_background_xyxy
from supervision.config import CLASS_NAME_DATA_FIELD
from supervision.detection.utils import spread_out_boxes
from supervision.geometry.core import Position

import vidformer.cv2 as vf_cv2

try:
    import cv2 as ocv_cv2
except ImportError:
    ocv_cv2 = None

CV2_FONT = vf_cv2.FONT_HERSHEY_SIMPLEX


class BoxAnnotator:
    def __init__(
        self,
        color=ColorPalette.DEFAULT,
        thickness=2,
        color_lookup=ColorLookup.CLASS,
    ):
        self.color = color
        self.thickness = thickness
        self.color_lookup = color_lookup

    def annotate(
        self,
        scene: vf_cv2.Frame,
        detections: Detections,
        custom_color_lookup=None,
    ):
        for detection_idx in range(len(detections)):
            x1, y1, x2, y2 = detections.xyxy[detection_idx].astype(int)
            color = resolve_color(
                color=self.color,
                detections=detections,
                detection_idx=detection_idx,
                color_lookup=(
                    self.color_lookup
                    if custom_color_lookup is None
                    else custom_color_lookup
                ),
            )
            vf_cv2.rectangle(
                img=scene,
                pt1=(x1, y1),
                pt2=(x2, y2),
                color=color.as_rgb(),
                thickness=self.thickness,
            )
        return scene


class RoundBoxAnnotator:
    def __init__(
        self,
        color=ColorPalette.DEFAULT,
        thickness: int = 2,
        color_lookup: ColorLookup = ColorLookup.CLASS,
        roundness: float = 0.6,
    ):
        self.color = color
        self.thickness = thickness
        self.color_lookup = color_lookup
        if not 0 < roundness <= 1.0:
            raise ValueError("roundness attribute must be float between (0, 1.0]")
        self.roundness = roundness

    def annotate(
        self,
        scene: vf_cv2.Frame,
        detections: _sv.Detections,
        custom_color_lookup=None,
    ):
        for detection_idx in range(len(detections)):
            x1, y1, x2, y2 = detections.xyxy[detection_idx].astype(int)
            color = resolve_color(
                color=self.color,
                detections=detections,
                detection_idx=detection_idx,
                color_lookup=(
                    self.color_lookup
                    if custom_color_lookup is None
                    else custom_color_lookup
                ),
            )
            radius = (
                int((x2 - x1) // 2 * self.roundness)
                if abs(x1 - x2) < abs(y1 - y2)
                else int((y2 - y1) // 2 * self.roundness)
            )
            circle_coordinates = [
                ((x1 + radius), (y1 + radius)),
                ((x2 - radius), (y1 + radius)),
                ((x2 - radius), (y2 - radius)),
                ((x1 + radius), (y2 - radius)),
            ]
            line_coordinates = [
                ((x1 + radius, y1), (x2 - radius, y1)),
                ((x2, y1 + radius), (x2, y2 - radius)),
                ((x1 + radius, y2), (x2 - radius, y2)),
                ((x1, y1 + radius), (x1, y2 - radius)),
            ]
            start_angles = (180, 270, 0, 90)
            end_angles = (270, 360, 90, 180)
            for center_coordinates, line, start_angle, end_angle in zip(
                circle_coordinates, line_coordinates, start_angles, end_angles
            ):
                vf_cv2.ellipse(
                    img=scene,
                    center=center_coordinates,
                    axes=(radius, radius),
                    angle=0,
                    startAngle=start_angle,
                    endAngle=end_angle,
                    color=color.as_rgb(),
                    thickness=self.thickness,
                )
                vf_cv2.line(
                    img=scene,
                    pt1=line[0],
                    pt2=line[1],
                    color=color.as_rgb(),
                    thickness=self.thickness,
                )
        return scene


class BoxCornerAnnotator:
    def __init__(
        self,
        color=ColorPalette.DEFAULT,
        thickness=4,
        corner_length=15,
        color_lookup=ColorLookup.CLASS,
    ):
        self.color = color
        self.thickness: int = thickness
        self.corner_length: int = corner_length
        self.color_lookup: ColorLookup = color_lookup

    def annotate(
        self,
        scene: vf_cv2.Frame,
        detections: Detections,
        custom_color_lookup=None,
    ):
        for detection_idx in range(len(detections)):
            x1, y1, x2, y2 = detections.xyxy[detection_idx].astype(int)
            color = resolve_color(
                color=self.color,
                detections=detections,
                detection_idx=detection_idx,
                color_lookup=(
                    self.color_lookup
                    if custom_color_lookup is None
                    else custom_color_lookup
                ),
            )
            corners = [(x1, y1), (x2, y1), (x1, y2), (x2, y2)]
            for x, y in corners:
                x_end = x + self.corner_length if x == x1 else x - self.corner_length
                vf_cv2.line(
                    scene, (x, y), (x_end, y), color.as_rgb(), thickness=self.thickness
                )

                y_end = y + self.corner_length if y == y1 else y - self.corner_length
                vf_cv2.line(
                    scene, (x, y), (x, y_end), color.as_rgb(), thickness=self.thickness
                )
        return scene


class ColorAnnotator:
    def __init__(
        self,
        color=ColorPalette.DEFAULT,
        opacity: float = 0.5,
        color_lookup: ColorLookup = ColorLookup.CLASS,
    ):
        self.color = color
        self.color_lookup: ColorLookup = color_lookup
        self.opacity = opacity

    def annotate(
        self,
        scene: vf_cv2.Frame,
        detections: Detections,
        custom_color_lookup=None,
    ):
        scene_with_boxes = scene.copy()
        for detection_idx in range(len(detections)):
            x1, y1, x2, y2 = detections.xyxy[detection_idx].astype(int)
            color = resolve_color(
                color=self.color,
                detections=detections,
                detection_idx=detection_idx,
                color_lookup=(
                    self.color_lookup
                    if custom_color_lookup is None
                    else custom_color_lookup
                ),
            )
            vf_cv2.rectangle(
                img=scene_with_boxes,
                pt1=(x1, y1),
                pt2=(x2, y2),
                color=color.as_rgb(),
                thickness=-1,
            )

        vf_cv2.addWeighted(
            scene_with_boxes, self.opacity, scene, 1 - self.opacity, gamma=0, dst=scene
        )
        return scene


class CircleAnnotator:
    def __init__(
        self,
        color=ColorPalette.DEFAULT,
        thickness: int = 2,
        color_lookup: ColorLookup = ColorLookup.CLASS,
    ):
        self.color = color
        self.thickness: int = thickness
        self.color_lookup: ColorLookup = color_lookup

    def annotate(
        self,
        scene: vf_cv2.Frame,
        detections: Detections,
        custom_color_lookup=None,
    ):
        for detection_idx in range(len(detections)):
            x1, y1, x2, y2 = detections.xyxy[detection_idx].astype(int)
            center = ((x1 + x2) // 2, (y1 + y2) // 2)
            distance = sqrt((x1 - center[0]) ** 2 + (y1 - center[1]) ** 2)
            color = resolve_color(
                color=self.color,
                detections=detections,
                detection_idx=detection_idx,
                color_lookup=(
                    self.color_lookup
                    if custom_color_lookup is None
                    else custom_color_lookup
                ),
            )
            vf_cv2.circle(
                img=scene,
                center=center,
                radius=int(distance),
                color=color.as_rgb(),
                thickness=self.thickness,
            )

        return scene


class DotAnnotator:
    def __init__(
        self,
        color=ColorPalette.DEFAULT,
        radius: int = 4,
        position: Position = Position.CENTER,
        color_lookup: ColorLookup = ColorLookup.CLASS,
        outline_thickness: int = 0,
        outline_color=Color.BLACK,
    ):
        self.color = color
        self.radius: int = radius
        self.position: Position = position
        self.color_lookup: ColorLookup = color_lookup
        self.outline_thickness = outline_thickness
        self.outline_color = outline_color

    def annotate(
        self,
        scene: vf_cv2.Frame,
        detections: Detections,
        custom_color_lookup=None,
    ):
        xy = detections.get_anchors_coordinates(anchor=self.position)
        for detection_idx in range(len(detections)):
            color = resolve_color(
                color=self.color,
                detections=detections,
                detection_idx=detection_idx,
                color_lookup=(
                    self.color_lookup
                    if custom_color_lookup is None
                    else custom_color_lookup
                ),
            )
            center = (int(xy[detection_idx, 0]), int(xy[detection_idx, 1]))

            vf_cv2.circle(scene, center, self.radius, color.as_rgb(), -1)
            if self.outline_thickness:
                outline_color = resolve_color(
                    color=self.outline_color,
                    detections=detections,
                    detection_idx=detection_idx,
                    color_lookup=(
                        self.color_lookup
                        if custom_color_lookup is None
                        else custom_color_lookup
                    ),
                )
                vf_cv2.circle(
                    scene,
                    center,
                    self.radius,
                    outline_color.as_rgb(),
                    self.outline_thickness,
                )
        return scene


class LabelAnnotator:
    def __init__(
        self,
        color=ColorPalette.DEFAULT,
        text_color=Color.WHITE,
        text_scale: float = 0.5,
        text_thickness: int = 1,
        text_padding: int = 10,
        text_position: Position = Position.TOP_LEFT,
        color_lookup: ColorLookup = ColorLookup.CLASS,
        border_radius: int = 0,
        smart_position: bool = False,
    ):
        self.border_radius: int = border_radius
        self.color = color
        self.text_color = text_color
        self.text_scale: float = text_scale
        self.text_thickness: int = text_thickness
        self.text_padding: int = text_padding
        self.text_anchor: Position = text_position
        self.color_lookup: ColorLookup = color_lookup
        self.smart_position = smart_position

    def annotate(
        self,
        scene,
        detections: Detections,
        labels,
        custom_color_lookup=None,
    ):
        self._validate_labels(labels, detections)

        labels = self._get_labels_text(detections, labels)
        label_properties = self._get_label_properties(detections, labels)

        if self.smart_position:
            xyxy = label_properties[:, :4]
            xyxy = spread_out_boxes(xyxy)
            label_properties[:, :4] = xyxy

        self._draw_labels(
            scene=scene,
            labels=labels,
            label_properties=label_properties,
            detections=detections,
            custom_color_lookup=custom_color_lookup,
        )

        return scene

    def _validate_labels(self, labels, detections: Detections):
        if labels is not None and len(labels) != len(detections):
            raise ValueError(
                f"The number of labels ({len(labels)}) does not match the "
                f"number of detections ({len(detections)}). Each detection "
                f"should have exactly 1 label."
            )

    def _get_label_properties(
        self,
        detections: Detections,
        labels,
    ):
        label_properties = []
        anchors_coordinates = detections.get_anchors_coordinates(
            anchor=self.text_anchor
        ).astype(int)

        for label, center_coords in zip(labels, anchors_coordinates):
            (text_w, text_h) = vf_cv2.getTextSize(
                text=label,
                fontFace=CV2_FONT,
                fontScale=self.text_scale,
                thickness=self.text_thickness,
            )[0]

            width_padded = text_w + 2 * self.text_padding
            height_padded = text_h + 2 * self.text_padding

            text_background_xyxy = resolve_text_background_xyxy(
                center_coordinates=tuple(center_coords),
                text_wh=(width_padded, height_padded),
                position=self.text_anchor,
            )

            label_properties.append(
                [
                    *text_background_xyxy,
                    text_h,
                ]
            )

        return np.array(label_properties).reshape(-1, 5)

    @staticmethod
    def _get_labels_text(detections: Detections, custom_labels):
        if custom_labels is not None:
            return custom_labels

        labels = []
        for idx in range(len(detections)):
            if CLASS_NAME_DATA_FIELD in detections.data:
                labels.append(detections.data[CLASS_NAME_DATA_FIELD][idx])
            elif detections.class_id is not None:
                labels.append(str(detections.class_id[idx]))
            else:
                labels.append(str(idx))
        return labels

    def _draw_labels(
        self,
        scene,
        labels,
        label_properties,
        detections,
        custom_color_lookup,
    ) -> None:
        assert len(labels) == len(label_properties) == len(detections), (
            f"Number of label properties ({len(label_properties)}), "
            f"labels ({len(labels)}) and detections ({len(detections)}) "
            "do not match."
        )

        color_lookup = (
            custom_color_lookup
            if custom_color_lookup is not None
            else self.color_lookup
        )

        for idx, label_property in enumerate(label_properties):
            background_color = resolve_color(
                color=self.color,
                detections=detections,
                detection_idx=idx,
                color_lookup=color_lookup,
            )
            text_color = resolve_color(
                color=self.text_color,
                detections=detections,
                detection_idx=idx,
                color_lookup=color_lookup,
            )

            box_xyxy = label_property[:4]
            text_height_padded = label_property[4]
            self.draw_rounded_rectangle(
                scene=scene,
                xyxy=box_xyxy,
                color=background_color.as_rgb(),
                border_radius=self.border_radius,
            )

            text_x = box_xyxy[0] + self.text_padding
            text_y = box_xyxy[1] + self.text_padding + text_height_padded
            vf_cv2.putText(
                img=scene,
                text=labels[idx],
                org=(text_x, text_y),
                fontFace=CV2_FONT,
                fontScale=self.text_scale,
                color=text_color.as_rgb(),
                thickness=self.text_thickness,
                lineType=vf_cv2.LINE_AA,
            )

    @staticmethod
    def draw_rounded_rectangle(
        scene: np.ndarray,
        xyxy,
        color,
        border_radius: int,
    ) -> np.ndarray:
        x1, y1, x2, y2 = xyxy
        width = x2 - x1
        height = y2 - y1

        border_radius = min(border_radius, min(width, height) // 2)

        if border_radius <= 0:
            vf_cv2.rectangle(
                img=scene,
                pt1=(x1, y1),
                pt2=(x2, y2),
                color=color,
                thickness=-1,
            )
        else:
            rectangle_coordinates = [
                ((x1 + border_radius, y1), (x2 - border_radius, y2)),
                ((x1, y1 + border_radius), (x2, y2 - border_radius)),
            ]
            circle_centers = [
                (x1 + border_radius, y1 + border_radius),
                (x2 - border_radius, y1 + border_radius),
                (x1 + border_radius, y2 - border_radius),
                (x2 - border_radius, y2 - border_radius),
            ]

            for coordinates in rectangle_coordinates:
                vf_cv2.rectangle(
                    img=scene,
                    pt1=coordinates[0],
                    pt2=coordinates[1],
                    color=color,
                    thickness=-1,
                )
            for center in circle_centers:
                vf_cv2.circle(
                    img=scene,
                    center=center,
                    radius=border_radius,
                    color=color,
                    thickness=-1,
                )
        return scene


class MaskAnnotator:
    def __init__(
        self,
        color=ColorPalette.DEFAULT,
        opacity: float = 0.5,
        color_lookup: ColorLookup = ColorLookup.CLASS,
    ):
        self.color = color
        self.opacity = opacity
        self.color_lookup: ColorLookup = color_lookup

    def annotate(
        self,
        scene,
        detections: Detections,
        custom_color_lookup=None,
    ):
        if detections.mask is None:
            return scene

        colored_mask = scene.copy()

        for detection_idx in np.flip(np.argsort(detections.box_area)):
            color = resolve_color(
                color=self.color,
                detections=detections,
                detection_idx=detection_idx,
                color_lookup=(
                    self.color_lookup
                    if custom_color_lookup is None
                    else custom_color_lookup
                ),
            )
            mask = detections.mask[detection_idx]
            colored_mask[mask] = color.as_bgr()

        vf_cv2.addWeighted(
            colored_mask, self.opacity, scene, 1 - self.opacity, 0, dst=scene
        )
        return scene


class MaskStreamWriter:
    def __init__(self, path: str, shape: tuple):
        # Shape should be (width, height)
        assert ocv_cv2 is not None, "OpenCV cv2 is required for ExternDetectionsBuilder"
        assert type(shape) is tuple, "shape must be a tuple"
        assert len(shape) == 2, "shape must be a tuple of length 2"
        self._shape = (shape[1], shape[0])
        self._writer = ocv_cv2.VideoWriter(
            path, ocv_cv2.VideoWriter_fourcc(*"FFV1"), 1, shape, isColor=False
        )
        assert self._writer.isOpened(), f"Failed to open video writer at {path}"
        self._i = 0

    def write_detections(self, detections: Detections):
        if len(detections) == 0:
            return self._i

        mask = detections.mask
        assert (
            mask.shape[1:] == self._shape
        ), f"mask shape ({mask.shape[:1]}) must match the shape of the video ({self._shape})"
        for i in range(mask.shape[0]):
            frame_uint8 = detections.mask[i].astype(np.uint8)
            self._writer.write(frame_uint8)
            self._i += 1
        return self._i

    def release(self):
        self._writer.release()


def populate_mask(
    detections: Detections, mask_stream: vf_cv2.VideoCapture, frame_idx: int
):
    assert type(detections) is Detections
    assert detections.mask is None
    detections.mask = []
    assert len(detections) + frame_idx <= len(mask_stream)
    for i in range(len(detections)):
        mask = mask_stream[frame_idx + i]
        assert mask.shape[2] == 1, "mask must be a single channel image"
        detections.mask.append(mask)
