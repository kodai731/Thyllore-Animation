"""Compare an effect's render_from_dump.py output against engine screenshots taken with the same dump.

    python compare.py --blend {alpha,additive} <out_dir> <engine_color.png> <engine_coverage.png> <viewport_x> <viewport_y> [result_dir]

`alpha` effects (wind) composite over the viewport background through their alpha channel and are
masked by it; `additive` effects (lightning) add onto the background and are masked by coverage red.
The engine display transfer is sRGB(ACES(exposure * x)) with the default ev100 = 0 exposure."""
import argparse

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


def coverage_channel(coverage, blend):
    return coverage[..., 3] if blend == "alpha" else coverage[..., 0]


def composite_over_background(color, blend):
    if blend == "alpha":
        alpha = color[..., 3:4]
        return color[..., :3] + (1.0 - alpha) * VIEWPORT_BACKGROUND_LINEAR, color[..., 3] > INSIDE_ALPHA
    return color[..., :3] + VIEWPORT_BACKGROUND_LINEAR, None


def parse_args():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--blend", choices=("alpha", "additive"), required=True)
    parser.add_argument("out_dir")
    parser.add_argument("color_png")
    parser.add_argument("coverage_png")
    parser.add_argument("viewport_x", type=int)
    parser.add_argument("viewport_y", type=int)
    parser.add_argument("result_dir", nargs="?")
    return parser.parse_args()


def main():
    args = parse_args()
    result_dir = args.result_dir or args.out_dir

    coverage = coverage_channel(np.load(f"{args.out_dir}/blender_coverage.npy"), args.blend)
    color = np.load(f"{args.out_dir}/blender_color.npy")
    h, w = coverage.shape[:2]
    engine_coverage = load_engine(args.coverage_png, args.viewport_x, args.viewport_y, h, w)[..., 0]
    engine_color = load_engine(args.color_png, args.viewport_x, args.viewport_y, h, w)

    inside = coverage > 0.5
    report("coverage", np.abs(engine_display(coverage) - engine_coverage), inside)

    composited, color_mask = composite_over_background(color, args.blend)
    report("color", np.abs(engine_display(composited) - engine_color).mean(axis=2), inside if color_mask is None else color_mask)

    ys, xs = np.where(coverage > 0)
    crop = (slice(ys.min(), ys.max() + 1), slice(xs.min(), xs.max() + 1))
    sheet = np.concatenate([engine_color[crop], engine_display(composited)[crop]], axis=1)
    Image.fromarray((np.clip(sheet, 0, 1) * 255).astype(np.uint8)).save(f"{result_dir}/engine_vs_blender.png")


main()
