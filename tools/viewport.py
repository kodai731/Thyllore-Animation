from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image

CHROME_COVERAGE_RATIO = 0.95


def detect_viewport_from_background(background_png: Path) -> tuple[int, int, int, int]:
    """Bounding box of the 3D viewport from the chrome gaps containing the image center."""
    gray = np.asarray(Image.open(background_png).convert("RGB"), dtype=np.float32).mean(axis=2)
    clear_value = np.bincount(gray.astype(np.uint8).ravel()).argmax()
    mask = np.abs(gray - float(clear_value)) > 2.0
    height, width = mask.shape

    row_runs = _collect_chrome_runs(mask.sum(axis=1), CHROME_COVERAGE_RATIO * width)
    y0, y1 = _find_widest_interior(row_runs, height, height // 2)

    band_height = y1 - y0
    col_runs = _collect_chrome_runs(mask[y0:y1].sum(axis=0), CHROME_COVERAGE_RATIO * band_height)
    x0, x1 = _find_widest_interior(col_runs, width, width // 2)

    if band_height <= 0 or x1 - x0 <= 0:
        raise SystemExit(f"viewport detection failed: {background_png}")
    return x0, y0, x1, y1


def trim_uniform_frame(image_rgb: np.ndarray,
                       viewport: tuple[int, int, int, int]) -> tuple[int, int, int, int]:
    """Shrink the viewport past the solid-colored window padding that surrounds the 3D image."""
    x0, y0, x1, y1 = viewport
    frame_color = image_rgb[y0, x0]

    def is_frame(pixels: np.ndarray) -> bool:
        return bool((pixels == frame_color).all())

    while y0 < y1 and is_frame(image_rgb[y0, x0:x1]):
        y0 += 1
    while y1 > y0 and is_frame(image_rgb[y1 - 1, x0:x1]):
        y1 -= 1
    while x0 < x1 and is_frame(image_rgb[y0:y1, x0]):
        x0 += 1
    while x1 > x0 and is_frame(image_rgb[y0:y1, x1 - 1]):
        x1 -= 1
    return x0, y0, x1, y1


def _collect_chrome_runs(counts: np.ndarray, threshold: float) -> list[tuple[int, int]]:
    """Runs of indices where counts >= threshold, as (start, end) pairs."""
    indices = np.where(counts >= threshold)[0]
    if len(indices) == 0:
        return []
    runs: list[tuple[int, int]] = []
    start = int(indices[0])
    prev = start
    for i in indices[1:]:
        i = int(i)
        if i == prev + 1:
            prev = i
        else:
            runs.append((start, prev + 1))
            start = i
            prev = i
    runs.append((start, prev + 1))
    return runs


def _find_widest_interior(runs: list[tuple[int, int]], length: int, center: int) -> tuple[int, int]:
    """Widest chrome-free span, preferring the one that contains center."""
    gaps: list[tuple[int, int]] = []
    cursor = 0
    for start, end in runs:
        if start > cursor:
            gaps.append((cursor, start))
        cursor = max(cursor, end)
    if cursor < length:
        gaps.append((cursor, length))
    if not gaps:
        return 0, length

    centered = [gap for gap in gaps if gap[0] <= center < gap[1]]
    return max(centered or gaps, key=lambda gap: gap[1] - gap[0])
