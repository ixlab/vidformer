import os
import numpy as np
import cv2 as ocv
import vidformer.cv2 as vf_cv2

WIDTH = 3840
HEIGHT = 2160

WHITE = (255, 255, 255)
BLACK = (0, 0, 0)
RED = (0, 0, 255)
GREEN = (0, 255, 0)
BLUE = (255, 0, 0)
YELLOW = (0, 255, 255)
CYAN = (255, 255, 0)
MAGENTA = (255, 0, 255)
ORANGE = (0, 165, 255)
PURPLE = (128, 0, 128)
PINK = (203, 192, 255)
GRAY = (128, 128, 128)

TITLE_BAR_HEIGHT = 70
COLS = 4
ROWS = 4
CELL_W = WIDTH // COLS
CELL_H = (HEIGHT - TITLE_BAR_HEIGHT) // ROWS
PADDING = 30
TITLE_HEIGHT = 50

TEST_IMG_PATH = "../apollo.jpg"


def draw_cell_border(img, col, row, title, cv2_module):
    x = col * CELL_W
    y = TITLE_BAR_HEIGHT + row * CELL_H

    cv2_module.rectangle(img, (x + 5, y + 5), (x + CELL_W - 5, y + CELL_H - 5), GRAY, 2)
    cv2_module.rectangle(
        img, (x + 10, y + 10), (x + CELL_W - 10, y + TITLE_HEIGHT), (40, 40, 40), -1
    )
    cv2_module.putText(
        img, title, (x + 15, y + 38), ocv.FONT_HERSHEY_SIMPLEX, 0.8, WHITE, 2
    )

    return x + PADDING, y + TITLE_HEIGHT + PADDING // 2


def demo_rectangle(img, ox, oy, cv2_module):
    cv2_module.rectangle(img, (ox, oy), (ox + 150, oy + 80), RED, -1)
    cv2_module.putText(
        img, "filled", (ox + 30, oy + 50), ocv.FONT_HERSHEY_SIMPLEX, 0.6, WHITE, 1
    )

    cv2_module.rectangle(img, (ox + 180, oy), (ox + 330, oy + 80), GREEN, 1)
    cv2_module.rectangle(img, (ox + 360, oy), (ox + 510, oy + 80), BLUE, 3)
    cv2_module.rectangle(img, (ox + 540, oy), (ox + 690, oy + 80), YELLOW, 5)

    cv2_module.putText(
        img, "t=1", (ox + 230, oy + 100), ocv.FONT_HERSHEY_SIMPLEX, 0.5, WHITE, 1
    )
    cv2_module.putText(
        img, "t=3", (ox + 410, oy + 100), ocv.FONT_HERSHEY_SIMPLEX, 0.5, WHITE, 1
    )
    cv2_module.putText(
        img, "t=5", (ox + 590, oy + 100), ocv.FONT_HERSHEY_SIMPLEX, 0.5, WHITE, 1
    )


def demo_circle(img, ox, oy, cv2_module):
    cv2_module.circle(img, (ox + 60, oy + 50), 40, RED, -1)
    cv2_module.circle(img, (ox + 160, oy + 50), 35, GREEN, -1)
    cv2_module.circle(img, (ox + 250, oy + 50), 30, BLUE, -1)

    cv2_module.circle(img, (ox + 370, oy + 50), 40, CYAN, 2)
    cv2_module.circle(img, (ox + 480, oy + 50), 40, MAGENTA, 4)
    cv2_module.circle(img, (ox + 590, oy + 50), 40, YELLOW, 6)

    for i, r in enumerate([15, 30, 45, 60]):
        cv2_module.circle(img, (ox + 700, oy + 60), r, (50 + i * 50, 100, 200), 2)


def demo_line(img, ox, oy, cv2_module):
    cv2_module.line(img, (ox, oy + 15), (ox + 180, oy + 15), RED, 1)
    cv2_module.line(img, (ox, oy + 40), (ox + 180, oy + 40), GREEN, 2)
    cv2_module.line(img, (ox, oy + 70), (ox + 180, oy + 70), BLUE, 4)
    cv2_module.line(img, (ox, oy + 105), (ox + 180, oy + 105), YELLOW, 8)

    cv2_module.line(img, (ox + 220, oy), (ox + 380, oy + 120), CYAN, 2)
    cv2_module.line(img, (ox + 380, oy), (ox + 220, oy + 120), MAGENTA, 2)

    for i in range(5):
        cv2_module.line(
            img, (ox + 420 + i * 40, oy), (ox + 420 + i * 40, oy + 120), GRAY, 1
        )
        cv2_module.line(img, (ox + 420, oy + i * 30), (ox + 580, oy + i * 30), GRAY, 1)


def demo_arrowed_line(img, ox, oy, cv2_module):
    cv2_module.arrowedLine(img, (ox + 50, oy + 70), (ox + 180, oy + 20), RED, 2)
    cv2_module.arrowedLine(img, (ox + 50, oy + 70), (ox + 180, oy + 120), GREEN, 2)
    cv2_module.arrowedLine(img, (ox + 50, oy + 70), (ox + 180, oy + 70), BLUE, 2)

    cv2_module.arrowedLine(img, (ox + 240, oy + 25), (ox + 380, oy + 25), YELLOW, 1)
    cv2_module.arrowedLine(img, (ox + 240, oy + 65), (ox + 380, oy + 65), CYAN, 3)
    cv2_module.arrowedLine(img, (ox + 240, oy + 110), (ox + 380, oy + 110), MAGENTA, 5)

    cv2_module.arrowedLine(
        img, (ox + 440, oy + 25), (ox + 600, oy + 25), WHITE, 2, 8, 0, 0.1
    )
    cv2_module.arrowedLine(
        img, (ox + 440, oy + 65), (ox + 600, oy + 65), WHITE, 2, 8, 0, 0.2
    )
    cv2_module.arrowedLine(
        img, (ox + 440, oy + 110), (ox + 600, oy + 110), WHITE, 2, 8, 0, 0.4
    )


def demo_ellipse(img, ox, oy, cv2_module):
    cv2_module.ellipse(img, (ox + 70, oy + 60), (60, 35), 0, 0, 360, RED, -1)
    cv2_module.ellipse(img, (ox + 220, oy + 60), (45, 55), 45, 0, 360, GREEN, 2)

    cv2_module.ellipse(img, (ox + 370, oy + 60), (50, 50), 0, 0, 180, BLUE, 3)
    cv2_module.ellipse(img, (ox + 370, oy + 60), (50, 50), 0, 180, 270, YELLOW, 3)

    cv2_module.ellipse(img, (ox + 530, oy + 40), (70, 25), 30, 0, 360, CYAN, 2)
    cv2_module.ellipse(img, (ox + 530, oy + 90), (70, 25), -30, 0, 360, MAGENTA, 2)


def demo_puttext(img, ox, oy, cv2_module):
    fonts = [
        (ocv.FONT_HERSHEY_SIMPLEX, "SIMPLEX"),
        (ocv.FONT_HERSHEY_PLAIN, "PLAIN"),
        (ocv.FONT_HERSHEY_DUPLEX, "DUPLEX"),
    ]

    for i, (font, name) in enumerate(fonts):
        y_pos = oy + 25 + i * 40
        cv2_module.putText(img, name, (ox, y_pos), font, 0.9, WHITE, 1)

    cv2_module.putText(
        img, "Small", (ox + 220, oy + 35), ocv.FONT_HERSHEY_SIMPLEX, 0.6, RED, 1
    )
    cv2_module.putText(
        img, "Medium", (ox + 220, oy + 75), ocv.FONT_HERSHEY_SIMPLEX, 1.0, GREEN, 1
    )
    cv2_module.putText(
        img, "Large", (ox + 220, oy + 120), ocv.FONT_HERSHEY_SIMPLEX, 1.4, BLUE, 2
    )

    cv2_module.putText(
        img, "Thin", (ox + 450, oy + 45), ocv.FONT_HERSHEY_SIMPLEX, 1.0, YELLOW, 1
    )
    cv2_module.putText(
        img, "Bold", (ox + 450, oy + 95), ocv.FONT_HERSHEY_SIMPLEX, 1.0, CYAN, 3
    )


def demo_polylines(img, ox, oy, cv2_module):
    # Open polyline
    pts1 = np.array(
        [
            [ox + 20, oy + 40],
            [ox + 70, oy + 10],
            [ox + 120, oy + 50],
            [ox + 170, oy + 20],
            [ox + 220, oy + 60],
        ],
        np.int32,
    )
    cv2_module.polylines(img, [pts1], False, RED, 2)
    cv2_module.putText(
        img, "open", (ox + 90, oy + 80), ocv.FONT_HERSHEY_SIMPLEX, 0.4, WHITE, 1
    )

    # Closed polyline (pentagon)
    pts2 = np.array(
        [
            [ox + 330, oy + 10],
            [ox + 390, oy + 35],
            [ox + 370, oy + 95],
            [ox + 290, oy + 95],
            [ox + 270, oy + 35],
        ],
        np.int32,
    )
    cv2_module.polylines(img, [pts2], True, GREEN, 2)
    cv2_module.putText(
        img, "closed", (ox + 295, oy + 115), ocv.FONT_HERSHEY_SIMPLEX, 0.4, WHITE, 1
    )

    # Star shape
    pts3 = np.array(
        [
            [ox + 520, oy + 5],
            [ox + 535, oy + 45],
            [ox + 580, oy + 45],
            [ox + 545, oy + 70],
            [ox + 560, oy + 115],
            [ox + 520, oy + 85],
            [ox + 480, oy + 115],
            [ox + 495, oy + 70],
            [ox + 460, oy + 45],
            [ox + 505, oy + 45],
        ],
        np.int32,
    )
    cv2_module.polylines(img, [pts3], True, YELLOW, 2)


def demo_fillpoly(img, ox, oy, cv2_module):
    import math

    # Triangle
    pts1 = np.array(
        [[ox + 50, oy + 100], [ox + 120, oy + 15], [ox + 190, oy + 100]], np.int32
    )
    cv2_module.fillPoly(img, [pts1], RED)

    # Hexagon
    hex_pts = []
    cx, cy, r = ox + 310, oy + 55, 50
    for i in range(6):
        angle = i * 60 * math.pi / 180
        hex_pts.append([int(cx + r * math.cos(angle)), int(cy + r * math.sin(angle))])
    cv2_module.fillPoly(img, [np.array(hex_pts, np.int32)], GREEN)

    # Arrow shape
    pts3 = np.array(
        [
            [ox + 420, oy + 55],
            [ox + 490, oy + 15],
            [ox + 490, oy + 35],
            [ox + 560, oy + 35],
            [ox + 560, oy + 75],
            [ox + 490, oy + 75],
            [ox + 490, oy + 95],
        ],
        np.int32,
    )
    cv2_module.fillPoly(img, [pts3], BLUE)


def demo_fillconvexpoly(img, ox, oy, cv2_module):
    import math

    pts1 = np.array(
        [
            [ox + 30, oy + 90],
            [ox + 70, oy + 15],
            [ox + 160, oy + 25],
            [ox + 130, oy + 100],
        ],
        np.int32,
    )
    cv2_module.fillConvexPoly(img, pts1, MAGENTA)

    pts2 = np.array(
        [
            [ox + 280, oy + 15],
            [ox + 350, oy + 55],
            [ox + 280, oy + 95],
            [ox + 210, oy + 55],
        ],
        np.int32,
    )
    cv2_module.fillConvexPoly(img, pts2, CYAN)

    pent_pts = []
    cx, cy, r = ox + 470, oy + 55, 45
    for i in range(5):
        angle = (i * 72 - 90) * math.pi / 180
        pent_pts.append([int(cx + r * math.cos(angle)), int(cy + r * math.sin(angle))])
    cv2_module.fillConvexPoly(img, np.array(pent_pts, np.int32), ORANGE)


def demo_drawmarker(img, ox, oy, cv2_module):
    markers = [
        (ocv.MARKER_CROSS, "CROSS"),
        (ocv.MARKER_TILTED_CROSS, "X"),
        (ocv.MARKER_STAR, "STAR"),
        (ocv.MARKER_DIAMOND, "DIAM"),
        (ocv.MARKER_SQUARE, "SQ"),
        (ocv.MARKER_TRIANGLE_UP, "TRI"),
    ]
    colors = [RED, GREEN, BLUE, YELLOW, CYAN, MAGENTA]

    for i, ((marker, name), color) in enumerate(zip(markers, colors)):
        x = ox + 50 + (i % 3) * 150
        y = oy + 35 + (i // 3) * 70
        cv2_module.drawMarker(img, (x, y), color, marker, 25, 2)
        cv2_module.putText(
            img, name, (x - 20, y + 40), ocv.FONT_HERSHEY_SIMPLEX, 0.35, WHITE, 1
        )


def demo_drawcontours(img, ox, oy, cv2_module):
    contour1 = np.array(
        [
            [ox + 20, oy + 20],
            [ox + 110, oy + 20],
            [ox + 110, oy + 80],
            [ox + 20, oy + 80],
        ],
        np.int32,
    )
    contour2 = np.array(
        [
            [ox + 140, oy + 30],
            [ox + 230, oy + 15],
            [ox + 245, oy + 75],
            [ox + 155, oy + 90],
        ],
        np.int32,
    )
    contour3 = np.array(
        [
            [ox + 280, oy + 50],
            [ox + 330, oy + 15],
            [ox + 380, oy + 50],
            [ox + 350, oy + 100],
            [ox + 300, oy + 100],
        ],
        np.int32,
    )

    contours = [contour1, contour2, contour3]
    cv2_module.drawContours(img, contours, -1, GREEN, 2)
    cv2_module.drawContours(img, contours, 1, RED, 3)

    contour4 = np.array(
        [
            [ox + 430, oy + 20],
            [ox + 520, oy + 20],
            [ox + 540, oy + 85],
            [ox + 410, oy + 85],
        ],
        np.int32,
    )
    cv2_module.drawContours(img, [contour4], 0, BLUE, -1)


def demo_addweighted(img, ox, oy, cv2_module):
    h, w = 90, 140

    for i, alpha in enumerate([0.25, 0.5, 0.75]):
        x = ox + 20 + i * 160
        r1 = img[oy : oy + h, x : x + w].copy()
        r2 = img[oy : oy + h, x : x + w].copy()
        cv2_module.rectangle(r1, (0, 0), (w, h), RED, -1)
        cv2_module.rectangle(r2, (0, 0), (w, h), BLUE, -1)
        blended = cv2_module.addWeighted(r1, alpha, r2, 1 - alpha, 0)
        img[oy : oy + h, x : x + w] = blended
        cv2_module.putText(
            img,
            f"a={alpha}",
            (x + 35, oy + h + 20),
            ocv.FONT_HERSHEY_SIMPLEX,
            0.5,
            WHITE,
            1,
        )


def demo_imread_resize(img, ox, oy, cv2_module, is_vidformer):
    apollo = cv2_module.imread(TEST_IMG_PATH)
    r1 = cv2_module.resize(apollo, (200, 130))
    r2 = cv2_module.resize(apollo, (120, 80))
    r3 = cv2_module.resize(apollo, (70, 46))
    img[oy : oy + 130, ox : ox + 200] = r1
    img[oy : oy + 80, ox + 220 : ox + 340] = r2
    img[oy : oy + 46, ox + 360 : ox + 430] = r3

    cv2_module.putText(
        img, "200x130", (ox + 50, oy + 145), ocv.FONT_HERSHEY_SIMPLEX, 0.4, WHITE, 1
    )
    cv2_module.putText(
        img, "120x80", (ox + 245, oy + 95), ocv.FONT_HERSHEY_SIMPLEX, 0.4, WHITE, 1
    )
    cv2_module.putText(
        img, "70x46", (ox + 365, oy + 62), ocv.FONT_HERSHEY_SIMPLEX, 0.4, WHITE, 1
    )


def demo_slice_write(img, ox, oy, cv2_module, is_vidformer):
    patch_h, patch_w = 60, 80

    if is_vidformer:
        p1 = vf_cv2.zeros((patch_h, patch_w, 3), dtype=np.uint8)
        p2 = vf_cv2.zeros((patch_h, patch_w, 3), dtype=np.uint8)
        p3 = vf_cv2.zeros((patch_h, patch_w, 3), dtype=np.uint8)
    else:
        p1 = np.zeros((patch_h, patch_w, 3), dtype=np.uint8)
        p2 = np.zeros((patch_h, patch_w, 3), dtype=np.uint8)
        p3 = np.zeros((patch_h, patch_w, 3), dtype=np.uint8)

    cv2_module.rectangle(p1, (0, 0), (patch_w, patch_h), RED, -1)
    cv2_module.circle(p1, (patch_w // 2, patch_h // 2), 20, WHITE, -1)

    cv2_module.rectangle(p2, (0, 0), (patch_w, patch_h), GREEN, -1)
    cv2_module.line(p2, (0, 0), (patch_w, patch_h), WHITE, 2)
    cv2_module.line(p2, (patch_w, 0), (0, patch_h), WHITE, 2)

    cv2_module.rectangle(p3, (0, 0), (patch_w, patch_h), BLUE, -1)
    cv2_module.rectangle(p3, (10, 10), (patch_w - 10, patch_h - 10), YELLOW, 2)

    img[oy : oy + patch_h, ox : ox + patch_w] = p1
    img[oy : oy + patch_h, ox + 100 : ox + 100 + patch_w] = p2
    img[oy : oy + patch_h, ox + 200 : ox + 200 + patch_w] = p3

    cv2_module.putText(
        img,
        "zeros() + draw + slice assign",
        (ox, oy + patch_h + 20),
        ocv.FONT_HERSHEY_SIMPLEX,
        0.4,
        WHITE,
        1,
    )


def demo_slice_ops(img, ox, oy, cv2_module, is_vidformer):
    region_h, region_w = 100, 400

    if is_vidformer:
        region = vf_cv2.zeros((region_h, region_w, 3), dtype=np.uint8)
    else:
        region = np.zeros((region_h, region_w, 3), dtype=np.uint8)

    cv2_module.rectangle(region, (0, 0), (region_w, region_h), (40, 40, 40), -1)

    sub = region[10:90, 10:130]
    cv2_module.rectangle(sub, (0, 0), (120, 80), RED, -1)
    cv2_module.circle(sub, (60, 40), 25, WHITE, -1)

    sub2 = region[10:90, 140:270]
    cv2_module.rectangle(sub2, (0, 0), (130, 80), GREEN, -1)
    cv2_module.line(sub2, (0, 0), (130, 80), WHITE, 2)
    cv2_module.line(sub2, (130, 0), (0, 80), WHITE, 2)

    sub3 = region[10:90, 280:390]
    cv2_module.rectangle(sub3, (0, 0), (110, 80), BLUE, -1)
    cv2_module.ellipse(sub3, (55, 40), (40, 25), 0, 0, 360, YELLOW, 2)

    img[oy : oy + region_h, ox : ox + region_w] = region

    cv2_module.putText(
        img,
        "slice[y:y2,x:x2] -> draw -> propagates",
        (ox + 10, oy + region_h + 18),
        ocv.FONT_HERSHEY_SIMPLEX,
        0.4,
        WHITE,
        1,
    )


def generate_mandelbrot(h, w, max_iter=80):
    # Create coordinate grids centered on interesting region
    y, x = np.ogrid[-1.2 : 1.2 : h * 1j, -2.0 : 0.6 : w * 1j]
    c = x + y * 1j
    z = np.zeros_like(c)
    output = np.zeros(c.shape, dtype=np.float64)

    for i in range(max_iter):
        mask = np.abs(z) <= 2
        z[mask] = z[mask] ** 2 + c[mask]
        # Record iteration for points that just escaped
        escaped = (np.abs(z) > 2) & (output == 0)
        output[escaped] = i

    # Points that never escaped get max value
    output[output == 0] = max_iter

    # Normalize to 0-255
    output = ((output / max_iter) * 255).astype(np.uint8)
    return output


def generate_plasma(h, w):
    y = np.linspace(0, 4 * np.pi, h)
    x = np.linspace(0, 4 * np.pi, w)
    X, Y = np.meshgrid(x, y)

    # Combine sine waves for plasma effect
    v1 = np.sin(X)
    v2 = np.sin(Y)
    v3 = np.sin(X + Y)
    v4 = np.sin(np.sqrt(X**2 + Y**2))

    v = v1 + v2 + v3 + v4

    # Normalize to 0-1
    v = (v - v.min()) / (v.max() - v.min())

    # Create RGB channels with different phase shifts
    r = (np.sin(v * 2 * np.pi) * 127 + 128).astype(np.uint8)
    g = (np.sin(v * 2 * np.pi + 2 * np.pi / 3) * 127 + 128).astype(np.uint8)
    b = (np.sin(v * 2 * np.pi + 4 * np.pi / 3) * 127 + 128).astype(np.uint8)

    return np.stack([b, g, r], axis=-1)  # BGR format


def demo_numpy_embed(img, ox, oy, cv2_module, is_vidformer):
    # Generate a plasma pattern
    plasma = generate_plasma(120, 200)

    # Generate Mandelbrot and colorize it
    mandel_gray = generate_mandelbrot(120, 200)
    # Colorize with a vibrant palette
    t = mandel_gray / 255.0
    mandel = np.zeros((120, 200, 3), dtype=np.uint8)
    # Create smooth color gradient (blue -> cyan -> green -> yellow -> red)
    mandel[:, :, 2] = (np.sin(t * np.pi) * 255).astype(np.uint8)  # R
    mandel[:, :, 1] = (np.sin(t * np.pi + np.pi / 3) * 200).astype(np.uint8)  # G
    mandel[:, :, 0] = (np.cos(t * np.pi * 0.5) * 255).astype(np.uint8)  # B
    # Make the set (escaped late) dark
    mandel[mandel_gray > 250] = [0, 0, 0]

    # Embed into the image
    img[oy : oy + 120, ox : ox + 200] = plasma
    img[oy : oy + 120, ox + 220 : ox + 420] = mandel

    cv2_module.putText(
        img, "plasma", (ox + 60, oy + 140), ocv.FONT_HERSHEY_SIMPLEX, 0.5, WHITE, 1
    )
    cv2_module.putText(
        img, "mandelbrot", (ox + 270, oy + 140), ocv.FONT_HERSHEY_SIMPLEX, 0.5, WHITE, 1
    )


def create_showcase_image(cv2_module, is_vidformer=False):
    if is_vidformer:
        img = vf_cv2.zeros((HEIGHT, WIDTH, 3), dtype=np.uint8)
    else:
        img = np.zeros((HEIGHT, WIDTH, 3), dtype=np.uint8)

    cv2_module.rectangle(img, (0, 0), (WIDTH, TITLE_BAR_HEIGHT), (30, 30, 30), -1)
    title = "VIDFORMER / CV2 SHOWCASE"
    (text_w, text_h), _ = ocv.getTextSize(title, ocv.FONT_HERSHEY_DUPLEX, 1.4, 2)
    cv2_module.putText(
        img,
        title,
        (WIDTH // 2 - text_w // 2, TITLE_BAR_HEIGHT // 2 + text_h // 2),
        ocv.FONT_HERSHEY_DUPLEX,
        1.4,
        WHITE,
        2,
    )

    ox, oy = draw_cell_border(img, 0, 0, "rectangle()", cv2_module)
    demo_rectangle(img, ox, oy, cv2_module)

    ox, oy = draw_cell_border(img, 1, 0, "circle()", cv2_module)
    demo_circle(img, ox, oy, cv2_module)

    ox, oy = draw_cell_border(img, 2, 0, "line()", cv2_module)
    demo_line(img, ox, oy, cv2_module)

    ox, oy = draw_cell_border(img, 3, 0, "arrowedLine()", cv2_module)
    demo_arrowed_line(img, ox, oy, cv2_module)

    ox, oy = draw_cell_border(img, 0, 1, "ellipse()", cv2_module)
    demo_ellipse(img, ox, oy, cv2_module)

    ox, oy = draw_cell_border(img, 1, 1, "putText()", cv2_module)
    demo_puttext(img, ox, oy, cv2_module)

    ox, oy = draw_cell_border(img, 2, 1, "polylines()", cv2_module)
    demo_polylines(img, ox, oy, cv2_module)

    ox, oy = draw_cell_border(img, 3, 1, "fillPoly()", cv2_module)
    demo_fillpoly(img, ox, oy, cv2_module)

    ox, oy = draw_cell_border(img, 0, 2, "fillConvexPoly()", cv2_module)
    demo_fillconvexpoly(img, ox, oy, cv2_module)

    ox, oy = draw_cell_border(img, 1, 2, "drawMarker()", cv2_module)
    demo_drawmarker(img, ox, oy, cv2_module)

    ox, oy = draw_cell_border(img, 2, 2, "drawContours()", cv2_module)
    demo_drawcontours(img, ox, oy, cv2_module)

    ox, oy = draw_cell_border(img, 3, 2, "addWeighted()", cv2_module)
    demo_addweighted(img, ox, oy, cv2_module)

    ox, oy = draw_cell_border(img, 0, 3, "imread() + resize()", cv2_module)
    demo_imread_resize(img, ox, oy, cv2_module, is_vidformer)

    ox, oy = draw_cell_border(img, 1, 3, "zeros() + slice write", cv2_module)
    demo_slice_write(img, ox, oy, cv2_module, is_vidformer)

    ox, oy = draw_cell_border(img, 2, 3, "slice draw propagation", cv2_module)
    demo_slice_ops(img, ox, oy, cv2_module, is_vidformer)

    ox, oy = draw_cell_border(img, 3, 3, "numpy array embed", cv2_module)
    demo_numpy_embed(img, ox, oy, cv2_module, is_vidformer)

    return img


def test_cv2_showcase(tmp_path):
    ocv_img = create_showcase_image(ocv, is_vidformer=False)

    vf_img_frame = create_showcase_image(vf_cv2, is_vidformer=True)
    vf_img = vf_img_frame.numpy()

    ocv_path = "showcase_opencv.png"
    vf_path = "showcase_vidformer.png"
    diff_path = "showcase_diff.png"

    ocv.imwrite(ocv_path, ocv_img)
    ocv.imwrite(vf_path, vf_img)

    # Exclude imread+resize cell (row 3, col 0) - different resize algorithms
    comparison_mask = np.ones((HEIGHT, WIDTH), dtype=bool)
    resize_cell_y_start = TITLE_BAR_HEIGHT + 3 * CELL_H
    comparison_mask[resize_cell_y_start:, :CELL_W] = False

    pixel_matches = np.isclose(ocv_img, vf_img, rtol=0, atol=2)
    masked_matches = pixel_matches[comparison_mask]
    match_ratio = masked_matches.mean()

    diff = np.abs(ocv_img.astype(np.int16) - vf_img.astype(np.int16)).astype(np.uint8)
    diff_vis = np.clip(diff * 10, 0, 255).astype(np.uint8)
    ocv.imwrite(diff_path, diff_vis)

    print(f"\nPixel match ratio: {match_ratio:.6f} ({match_ratio*100:.4f}%)")
    print(f"Max pixel difference: {diff.max()}")
    print(f"OpenCV image saved to: {ocv_path}")
    print(f"Vidformer image saved to: {vf_path}")
    print(f"Diff image saved to: {diff_path}")

    assert (
        match_ratio > 0.9999
    ), f"Only {match_ratio*100:.4f}% of pixels match (need >99.99%)"


if __name__ == "__main__":
    import tempfile
    from pathlib import Path

    with tempfile.TemporaryDirectory() as tmpdir:
        test_cv2_showcase(Path(tmpdir))
