"""
vidformer.supervision is the supervision frontend for [vidformer](https://github.com/ixlab/vidformer).
"""

import vidformer.cv2 as vf_cv2

import supervision as _sv
from supervision import Color, ColorPalette, ColorLookup, Detections
from supervision.annotators.utils import resolve_color
from math import sqrt
from supervision.geometry.core import Position


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
                color=color.as_bgr(),
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
                    color=color.as_bgr(),
                    thickness=self.thickness,
                )
                vf_cv2.line(
                    img=scene,
                    pt1=line[0],
                    pt2=line[1],
                    color=color.as_bgr(),
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
                    scene, (x, y), (x_end, y), color.as_bgr(), thickness=self.thickness
                )

                y_end = y + self.corner_length if y == y1 else y - self.corner_length
                vf_cv2.line(
                    scene, (x, y), (x, y_end), color.as_bgr(), thickness=self.thickness
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
                color=color.as_bgr(),
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
                color=color.as_bgr(),
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

            vf_cv2.circle(scene, center, self.radius, color.as_bgr(), -1)
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
                    outline_color.as_bgr(),
                    self.outline_thickness,
                )
        return scene
