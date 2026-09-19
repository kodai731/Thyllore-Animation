"""Reference image loading, silhouette mask, and frame statistics for flame/dust ref-match."""

import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

SILHOUETTE_MODES = ("flame", "dust")
silhouette_mode = "flame"
DUST_SILHOUETTE_LUM = 60.0
DUST_HALO_LUM = 20.0


def silhouette_mask(rgb):
    if silhouette_mode == "dust":
        return luminance(rgb) > DUST_SILHOUETTE_LUM
    r, b = rgb[:, :, 0], rgb[:, :, 2]
    return (r > 90) & (r - b > 40)


def halo_mask(rgb):
    if silhouette_mode == "dust":
        return luminance(rgb) > DUST_HALO_LUM
    r, b = rgb[:, :, 0], rgb[:, :, 2]
    return (r > 35) & (r - b > 15)


def load_ref_meta(ref_dir):
    meta_path = Path(ref_dir) / "meta.json"
    if not meta_path.exists():
        return 10.0, set()
    meta = json.loads(meta_path.read_text())
    return float(meta.get("fps", 10.0)), set(meta.get("caption_frames", []))


def load_image(path, crop=None):
    image = Image.open(path).convert("RGB")
    if crop is not None:
        image = image.crop(crop)
    return image


def to_rgb(image):
    return np.asarray(image).astype(np.float64)


def median_column_width(rgb):
    widths = silhouette_mask(rgb).sum(axis=1)
    return float(np.median(widths[widths > 0])) if (widths > 0).any() else 0.0


def resample_to_column_width(image, target_width):
    width = median_column_width(to_rgb(image))
    if width <= 0.0:
        return image
    scale = target_width / width
    if scale > 1.01:
        print(f"warning: render column width {width:.0f} px is below the reference {target_width:.0f} px; "
              "upscaling loses the fine detail the gates measure", file=sys.stderr)
    size = (max(1, round(image.width * scale)), max(1, round(image.height * scale)))
    resample = Image.BOX if scale < 1.0 else Image.BICUBIC
    return image.resize(size, resample)


def luminance(rgb):
    return 0.299 * rgb[:, :, 0] + 0.587 * rgb[:, :, 1] + 0.114 * rgb[:, :, 2]


def collect_frames(target):
    target = Path(target)
    if target.is_dir():
        return sorted(p for p in target.iterdir() if p.suffix.lower() == ".png")
    return [target]


def reference_column_width(paths):
    widths = [median_column_width(to_rgb(load_image(p))) for p in paths]
    return float(np.median([w for w in widths if w > 0]))
