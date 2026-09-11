"""Compare render_from_dump.py output against engine screenshots taken with the same dump.

    python compare.py <out_dir> <engine_color.png> <engine_coverage.png> <viewport_x> <viewport_y> [result_dir]
The engine display transfer is sRGB(ACES(exposure * x)) with the default ev100 = 0 exposure."""
import sys

import numpy as np
from PIL import Image

DEFAULT_EXPOSURE = 1.0 / 1.2
VIEWPORT_BACKGROUND_LINEAR = 0.053
INSIDE_ALPHA = 0.6


def srgb_encode(v):
    v = np.clip(v, 0.0, 1.0)
    return np.where(v <= 0.0031308, v * 12.92, 1.055 * np.power(v, 1 / 2.4) - 0.055)


def aces_filmic(v):
    return np.clip((v * (2.51 * v + 0.03)) / (v * (2.43 * v + 0.59) + 0.14), 0.0, 1.0)


def engine_display(v, exposure=DEFAULT_EXPOSURE):
    return srgb_encode(aces_filmic(exposure * v))


def load_engine(path, x, y, h, w):
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.float32)[y:y + h, x:x + w] / 255.0


def report(label, difference, mask):
    d = difference[mask]
    print(f"{label}: n={int(mask.sum())} mean={d.mean():.4f} p95={np.percentile(d, 95):.4f} max={d.max():.4f}")


def main():
    out_dir, color_png, coverage_png = sys.argv[1:4]
    x, y = int(sys.argv[4]), int(sys.argv[5])
    result_dir = sys.argv[6] if len(sys.argv) > 6 else out_dir

    coverage = np.load(f"{out_dir}/blender_coverage.npy")
    color = np.load(f"{out_dir}/blender_color.npy")
    h, w = coverage.shape[:2]
    engine_coverage = load_engine(coverage_png, x, y, h, w)[..., 0]
    engine_color = load_engine(color_png, x, y, h, w)

    inside = coverage[..., 3] > 0.5
    report("coverage", np.abs(engine_display(coverage[..., 0]) - engine_coverage), inside)

    alpha = color[..., 3:4]
    composited = color[..., :3] + (1.0 - alpha) * VIEWPORT_BACKGROUND_LINEAR
    report("color", np.abs(engine_display(composited) - engine_color).mean(axis=2), alpha[..., 0] > INSIDE_ALPHA)

    ys, xs = np.where(coverage[..., 3] > 0)
    crop = (slice(ys.min(), ys.max() + 1), slice(xs.min(), xs.max() + 1))
    sheet = np.concatenate([engine_color[crop], engine_display(composited)[crop]], axis=1)
    Image.fromarray((np.clip(sheet, 0, 1) * 255).astype(np.uint8)).save(f"{result_dir}/engine_vs_blender.png")


main()
