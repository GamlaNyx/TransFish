"""Generate the TransFish app icon (a cream koi on a vermilion seal).

Usage: python scripts/make_icon.py
Output: assets/icon.png (1024×1024)
"""

from __future__ import annotations

import math
from pathlib import Path

from PIL import Image, ImageDraw

S = 4096  # supersampled canvas
VERMILION = (176, 56, 42, 255)
VERMILION_DEEP = (150, 43, 31, 255)
CREAM = (247, 240, 225, 255)


def bezier(p0, p1, p2, p3, steps=80):
    pts = []
    for i in range(steps + 1):
        t = i / steps
        mt = 1 - t
        x = mt**3 * p0[0] + 3 * mt**2 * t * p1[0] + 3 * mt * t**2 * p2[0] + t**3 * p3[0]
        y = mt**3 * p0[1] + 3 * mt**2 * t * p1[1] + 3 * mt * t**2 * p2[1] + t**3 * p3[1]
        pts.append((x, y))
    return pts


def koi_shapes() -> tuple[list[tuple[float, float]], list[tuple[float, float]]]:
    """Side view koi facing right, in 0..1 coordinates: (body, tail)."""
    # Body: blunt tail joint, arched back, rounded head.
    upper = bezier((0.355, 0.395), (0.44, 0.315), (0.60, 0.285), (0.755, 0.400))
    nose = bezier((0.755, 0.400), (0.815, 0.445), (0.825, 0.510), (0.815, 0.570))
    lower = bezier((0.815, 0.570), (0.700, 0.680), (0.500, 0.705), (0.355, 0.605))
    body = upper + nose + lower + [(0.355, 0.395)]

    # Tail: two flicked lobes rooted in the body.
    tail = [
        (0.375, 0.400),
        (0.245, 0.330),
        (0.130, 0.285),
        (0.195, 0.415),
        (0.220, 0.500),
        (0.195, 0.585),
        (0.130, 0.715),
        (0.245, 0.670),
        (0.375, 0.600),
    ]
    return body, tail


def rotated(points, deg: float, cx: float, cy: float):
    rad = math.radians(deg)
    cos, sin = math.cos(rad), math.sin(rad)
    out = []
    for x, y in points:
        dx, dy = x - cx, y - cy
        out.append((cx + dx * cos - dy * sin, cy + dx * sin + dy * cos))
    return out


def main() -> None:
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    # Rounded square seal.
    pad = int(S * 0.035)
    d.rounded_rectangle((pad, pad, S - pad, S - pad), radius=int(S * 0.235), fill=VERMILION)
    # Inner hairline for a printed-seal feel.
    inset = int(S * 0.095)
    d.rounded_rectangle(
        (inset, inset, S - inset, S - inset),
        radius=int(S * 0.175),
        outline=(255, 242, 226, 40),
        width=int(S * 0.007),
    )

    angle = -7
    body, tail = koi_shapes()
    for shape in (tail, body):
        d.polygon([(x * S, y * S) for x, y in rotated(shape, angle, 0.5, 0.5)], fill=CREAM)

    # Gill arc: a fine vermilion cut near the head.
    gill = bezier((0.635, 0.355), (0.605, 0.43), (0.605, 0.55), (0.635, 0.640), steps=60)
    d.line([(x * S, y * S) for x, y in rotated(gill, angle, 0.5, 0.5)],
           fill=VERMILION_DEEP, width=int(S * 0.009), joint="curve")

    # Eye.
    eye = rotated([(0.725, 0.455)], angle, 0.5, 0.5)[0]
    r = S * 0.020
    d.ellipse((eye[0] * S - r, eye[1] * S - r, eye[0] * S + r, eye[1] * S + r), fill=VERMILION_DEEP)

    img = img.resize((1024, 1024), Image.LANCZOS)
    out = Path(__file__).resolve().parent.parent / "assets" / "icon.png"
    out.parent.mkdir(parents=True, exist_ok=True)
    img.save(out)
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
