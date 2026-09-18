"""Measure how closely rendered lightning frames match a reference sequence.

Usage:
  python scripts/lightning_ref_match.py --ref-dir <dir> --render <dir> [--fps 30]
      [--json out.json]
  python scripts/lightning_ref_match.py --ref-dir <dir> --self-check [--fps 30] [--json out.json]

The reference side is the extract output (frame_NNN.png / mask_NNN.png / core_NNN.png / meta.json).
The render side is color/ coverage/ core/ triplets of sequential frames.
"""

import argparse
import json
import sys
from pathlib import Path

import cv2
import numpy as np
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).parent.parent / "assets" / "textures" / "lightning"))
from measure_lightning_masks import zhang_suen  # noqa: E402

from wind_ref_match import DIRECTION_BINS, measure_direction  # noqa: E402


def parse_args():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--ref-dir", required=True, help="reference sequence directory")
    parser.add_argument("--render", help="render output directory (required without --self-check)")
    parser.add_argument("--fps", type=float, default=30.0, help="frames per second (default: 30)")
    parser.add_argument("--json", help="optional JSON output path")
    parser.add_argument("--self-check", action="store_true", help="compare the reference halves instead of a render")
    args = parser.parse_args()

    if not args.self_check and args.render is None:
        parser.error("--render is required unless --self-check is given")

    return args


REFERENCE_MASK_THRESHOLD = 127
RENDER_MASK_THRESHOLD = 200
ON_AREA_RATIO = 0.002
PROFILE_BINS = 16
PROFILE_REFERENCE_WIDTH = 480
EPS = 1e-9


def read_color(path: Path):
    image = cv2.imread(str(path))
    if image is None:
        raise SystemExit(f"Failed to read image: {path}")

    return image


def read_mask(path: Path, threshold: int):
    image = cv2.imread(str(path), cv2.IMREAD_GRAYSCALE)
    if image is None:
        raise SystemExit(f"Failed to read image: {path}")

    return image > threshold


def check_frame_counts(label: str, counts: dict):
    if len(set(counts.values())) > 1:
        detail = " ".join(f"{name}={count}" for name, count in counts.items())
        raise SystemExit(f"{label} frame count mismatch: {detail}")

    if not any(counts.values()):
        raise SystemExit(f"{label} sequence is empty")


def load_reference(ref_dir: Path) -> dict:
    """Reference extract output as {"glow", "core", "color", "fps"}."""
    fps = float(json.loads((ref_dir / "meta.json").read_text())["fps"])

    glow_paths = sorted(ref_dir.glob("mask_*.png"))
    core_paths = sorted(ref_dir.glob("core_*.png"))
    color_paths = sorted(ref_dir.glob("frame_*.png"))
    check_frame_counts(
        "Reference",
        {"glow": len(glow_paths), "core": len(core_paths), "color": len(color_paths)},
    )

    return {
        "glow": [read_mask(p, REFERENCE_MASK_THRESHOLD) for p in glow_paths],
        "core": [read_mask(p, REFERENCE_MASK_THRESHOLD) for p in core_paths],
        "color": [read_color(p) for p in color_paths],
        "fps": fps,
    }


def load_render(render_dir: Path, fps: float) -> dict:
    """Render color/ coverage/ core/ triplets as {"glow", "core", "color", "fps"}."""
    color_paths = sorted((render_dir / "color").glob("frame_*.png"))
    coverage_paths = sorted((render_dir / "coverage").glob("frame_*.png"))
    core_paths = sorted((render_dir / "core").glob("frame_*.png"))
    check_frame_counts(
        "Render",
        {"color": len(color_paths), "coverage": len(coverage_paths), "core": len(core_paths)},
    )

    return {
        "glow": [read_mask(p, RENDER_MASK_THRESHOLD) for p in coverage_paths],
        "core": [read_mask(p, RENDER_MASK_THRESHOLD) for p in core_paths],
        "color": [read_color(p) for p in color_paths],
        "fps": float(fps),
    }


def measure_skeleton_width(mask, skeleton, screen_width) -> float:
    distance = cv2.distanceTransform(mask.astype(np.uint8), cv2.DIST_L2, 3)

    return float(np.median(distance[skeleton])) * 2.0 / screen_width


def count_skeleton_neighbours(skeleton):
    padded = np.pad(skeleton.astype(int), 1)

    return (
        padded[:-2, :-2] + padded[:-2, 1:-1] + padded[:-2, 2:]
        + padded[1:-1, :-2] + padded[1:-1, 2:]
        + padded[2:, :-2] + padded[2:, 1:-1] + padded[2:, 2:]
    )


def measure_shape_frame(glow, core, screen_width) -> dict:
    """L2 shape descriptors of one frame, or None when the glow mask is empty."""
    if not glow.any():
        return None

    skeleton = zhang_suen(glow)
    skeleton_pixels = int(skeleton.sum())
    if skeleton_pixels == 0:
        return None

    rows, cols = np.nonzero(skeleton)
    diagonal = float(np.hypot(rows.max() - rows.min(), cols.max() - cols.min()))
    neighbours = count_skeleton_neighbours(skeleton)[skeleton]

    glow_width = measure_skeleton_width(glow, skeleton, screen_width)
    core_width = measure_skeleton_width(core, zhang_suen(core), screen_width) if core.any() else 0.0
    _, blobs = ndimage.label(glow)

    direction = measure_direction([(glow.astype(np.float32) * 255.0, glow)])

    return {
        "tort": skeleton_pixels / max(diagonal, EPS),
        "ends": float((neighbours == 1).sum()) / skeleton_pixels,
        "junctions": float((neighbours >= 3).sum()) / skeleton_pixels,
        "blobs": int(blobs),
        "glow_width": glow_width,
        "core_width": core_width,
        "width_ratio": glow_width / max(core_width, EPS),
        "direction_histogram": direction["direction_histogram"],
        "anisotropy": direction["anisotropy"],
    }


def measure_glow_areas(seq: dict) -> list:
    return [float(glow.sum()) / glow.size for glow in seq["glow"]]


def measure_mask_iou(left, right) -> float:
    union = float((left | right).sum())

    return float((left & right).sum()) / max(union, EPS)


def measure_attack_frames(areas: list, half_peak: float) -> int:
    return next((index for index, area in enumerate(areas) if area >= half_peak), 0)


def measure_release_frames(areas: list, peak_index: int, half_peak: float) -> int:
    tail = range(peak_index + 1, len(areas))

    return next((index - peak_index for index in tail if areas[index] < half_peak), len(areas) - peak_index)


def measure_timing(seq: dict, fps: float) -> dict:
    """L1 timing descriptors of one sequence."""
    areas = measure_glow_areas(seq)
    if not areas:
        return {
            "on_rate": 0.0,
            "peak": 0.0,
            "attack_seconds": 0.0,
            "release_seconds": 0.0,
            "iou_median": 0.0,
            "area_cv": 0.0,
        }

    peak = max(areas)
    half_peak = 0.5 * peak
    attack_frames = measure_attack_frames(areas, half_peak)
    release_frames = measure_release_frames(areas, areas.index(peak), half_peak)

    masks = seq["glow"]
    ious = [
        measure_mask_iou(masks[index], masks[index + 1])
        for index in range(len(masks) - 1)
        if masks[index].any() or masks[index + 1].any()
    ]

    on_areas = [area for area in areas if area > ON_AREA_RATIO]
    mean_on_area = float(np.mean(on_areas)) if on_areas else 0.0

    return {
        "on_rate": len(on_areas) / len(areas),
        "peak": peak,
        "attack_seconds": attack_frames / max(fps, EPS),
        "release_seconds": release_frames / max(fps, EPS),
        "iou_median": float(np.median(ious)) if ious else 0.0,
        "area_cv": float(np.std(on_areas)) / mean_on_area if mean_on_area > EPS else 0.0,
    }


def measure_distance_profile(glow, core, color):
    gray = cv2.cvtColor(color, cv2.COLOR_BGR2GRAY)
    distance = ndimage.distance_transform_edt(~core)
    width_scale = PROFILE_REFERENCE_WIDTH / glow.shape[1]
    bins = np.clip((distance[glow] * width_scale).astype(int), 0, PROFILE_BINS - 1)
    luminance = gray[glow].astype(np.float32)

    profile = np.zeros(PROFILE_BINS)
    filled = np.zeros(PROFILE_BINS)
    for index in range(PROFILE_BINS):
        selected = luminance[bins == index]
        if selected.size:
            profile[index] = float(np.median(selected))
            filled[index] = 1.0

    return profile, filled


def measure_profile(seq: dict) -> dict:
    """L3 luminance profile descriptors of one sequence."""
    profile_sums = np.zeros(PROFILE_BINS)
    profile_counts = np.zeros(PROFILE_BINS)
    halo_hues = []
    halo_sats = []
    core_sats = []

    for glow, core, color in zip(seq["glow"], seq["core"], seq["color"]):
        if not glow.any() or not core.any():
            continue

        frame_profile, filled = measure_distance_profile(glow, core, color)
        profile_sums += frame_profile
        profile_counts += filled

        hsv = cv2.cvtColor(color, cv2.COLOR_BGR2HSV)
        halo = glow & ~core
        if halo.any():
            halo_hues.append(float(np.median(hsv[halo, 0])))
            halo_sats.append(float(np.median(hsv[halo, 1])))
        core_sats.append(float(np.median(hsv[core, 1])))

    profile = profile_sums / np.maximum(profile_counts, 1.0)
    peak = float(profile.max())
    if peak > EPS:
        profile = profile / peak

    return {
        "profile": profile.tolist(),
        "halo_hue": float(np.median(halo_hues)) if halo_hues else 0.0,
        "halo_sat": float(np.median(halo_sats)) if halo_sats else 0.0,
        "core_sat": float(np.median(core_sats)) if core_sats else 0.0,
    }


def measure_outside_luminance(color, glow) -> float:
    outside = ~glow
    if not outside.any():
        return 0.0

    gray = cv2.cvtColor(color, cv2.COLOR_BGR2GRAY).astype(np.float32)

    return float(gray[outside].mean())


def measure_flash(seq: dict) -> dict:
    """L4 ambient flash descriptor of one sequence."""
    areas = measure_glow_areas(seq)
    if not areas:
        return {"flash_ratio": 0.0}

    on_index = int(np.argmax(areas))
    off_index = int(np.argmin(areas))

    luminance_on = measure_outside_luminance(seq["color"][on_index], seq["glow"][on_index])
    luminance_off = measure_outside_luminance(seq["color"][off_index], seq["glow"][off_index])

    return {"flash_ratio": luminance_on / max(luminance_off, EPS)}


def log_ratio(a: float, b: float, eps: float = 1e-6) -> float:
    """Log-ratio distance between two positive values."""
    return abs(np.log((a + eps) / (b + eps)))


SHAPE_SCALARS = (
    "tort",
    "ends",
    "junctions",
    "blobs",
    "glow_width",
    "core_width",
    "width_ratio",
    "anisotropy",
)


def is_finite_value(value) -> bool:
    return value is not None and np.isfinite(value)


def aggregate_shape(seq: dict, screen_width: int) -> dict:
    """L2 shape descriptor of one sequence: per key median over the frames that measured."""
    frames = [
        frame
        for glow, core in zip(seq["glow"], seq["core"])
        for frame in [measure_shape_frame(glow, core, screen_width)]
        if frame is not None
    ]

    shape = {}
    for key in SHAPE_SCALARS:
        values = [frame[key] for frame in frames if is_finite_value(frame[key])]
        shape[key] = float(np.median(values)) if values else 0.0

    histograms = [
        frame["direction_histogram"]
        for frame in frames
        if np.all(np.isfinite(frame["direction_histogram"]))
    ]
    shape["direction_histogram"] = (
        np.median(histograms, axis=0).tolist() if histograms else [0.0] * DIRECTION_BINS
    )

    return shape


L2_SHAPE_SCALARS = ("tort", "ends", "junctions", "blobs", "glow_width", "width_ratio")
L2_SHAPE_WEIGHTS = {"blobs": 0.25, "ends": 0.5}
HUE_PERIOD = 180.0
FAMILIES = ("L1", "L2", "L3", "L4")
CEILING_FLOOR = {"L1": 1e-3, "L2": 1e-3, "L3": 0.08, "L4": 1e-3}


def hue_distance(left: float, right: float) -> float:
    delta = abs(left - right) % HUE_PERIOD

    return min(delta, HUE_PERIOD - delta)


def timing_distance(ref: dict, ren: dict) -> float:
    return float(np.mean([
        log_ratio(ref["on_rate"], ren["on_rate"]),
        log_ratio(ref["attack_seconds"], ren["attack_seconds"]),
        log_ratio(ref["release_seconds"], ren["release_seconds"]),
        abs(ref["iou_median"] - ren["iou_median"]),
        log_ratio(ref["area_cv"], ren["area_cv"]),
    ]))


def shape_distance(ref: dict, ren: dict) -> float:
    parts = [log_ratio(ref[key], ren[key]) for key in L2_SHAPE_SCALARS]
    weights = [L2_SHAPE_WEIGHTS.get(key, 1.0) for key in L2_SHAPE_SCALARS]
    parts.append(float(np.sum(np.abs(
        np.asarray(ref["direction_histogram"]) - np.asarray(ren["direction_histogram"])
    ))))
    weights.append(1.0)
    parts.append(abs(ref["anisotropy"] - ren["anisotropy"]))
    weights.append(1.0)

    return float(sum(p * w for p, w in zip(parts, weights)) / sum(weights))


def profile_distance(ref: dict, ren: dict) -> float:
    return float(np.mean([
        float(np.linalg.norm(np.asarray(ref["profile"]) - np.asarray(ren["profile"]))),
        hue_distance(ref["halo_hue"], ren["halo_hue"]) / 90.0,
        abs(ref["halo_sat"] - ren["halo_sat"]) / 64.0,
        abs(ref["core_sat"] - ren["core_sat"]) / 64.0,
    ]))


def family_distances(ref_desc: dict, ren_desc: dict) -> dict:
    """Per family descriptor distances between two sequences."""
    return {
        "L1": timing_distance(ref_desc["timing"], ren_desc["timing"]),
        "L2": shape_distance(ref_desc["shape"], ren_desc["shape"]),
        "L3": profile_distance(ref_desc["profile"], ren_desc["profile"]),
        "L4": log_ratio(ref_desc["flash"]["flash_ratio"], ren_desc["flash"]["flash_ratio"]),
    }


def describe_sequence(seq: dict, fps: float) -> dict:
    """L1 timing / L2 shape / L3 profile / L4 flash descriptors of one sequence."""
    screen_width = seq["glow"][0].shape[1]

    return {
        "timing": measure_timing(seq, fps),
        "shape": aggregate_shape(seq, screen_width),
        "profile": measure_profile(seq),
        "flash": measure_flash(seq),
    }


def slice_sequence(seq: dict, start: int, stop: int) -> dict:
    return {
        "glow": seq["glow"][start:stop],
        "core": seq["core"][start:stop],
        "color": seq["color"][start:stop],
    }


def split_sequence(seq: dict) -> list:
    """Reference split in halves, or in thirds once the sequence is long enough."""
    count = len(seq["glow"])
    parts = 3 if count >= 30 else 2
    bounds = [count * index // parts for index in range(parts + 1)]

    return [slice_sequence(seq, bounds[index], bounds[index + 1]) for index in range(parts)]


def split_distances(ref: dict, fps: float) -> dict:
    """Distance of the reference against itself, averaged over every pair of splits."""
    if len(ref["glow"]) < 2:
        return dict.fromkeys(FAMILIES, 0.0)

    descriptors = [describe_sequence(split, fps) for split in split_sequence(ref)]

    pairs = [
        family_distances(descriptors[left], descriptors[right])
        for left in range(len(descriptors))
        for right in range(left + 1, len(descriptors))
    ]

    return {family: float(np.mean([pair[family] for pair in pairs])) for family in FAMILIES}


def compute_random_split_p90_ceiling(ref: dict, fps: float) -> float:
    """L2 distance between random halves of the reference frames, p90 over 40 fixed-seed draws."""
    count = len(ref["glow"])
    if count < 2:
        return 0.0

    screen_width = ref["glow"][0].shape[1]
    rng = np.random.default_rng(0)
    distances = []
    for _ in range(40):
        order = rng.permutation(count)
        halves = [order[: count // 2], order[count // 2 :]]
        shapes = [
            aggregate_shape(
                {"glow": [ref["glow"][i] for i in half], "core": [ref["core"][i] for i in half]},
                screen_width,
            )
            for half in halves
        ]
        distances.append(shape_distance(shapes[0], shapes[1]))

    return float(np.percentile(distances, 90))


def compute_ceiling(ref: dict, fps: float) -> dict:
    distances = split_distances(ref, fps)

    return {
        "L1": max(distances["L1"], CEILING_FLOOR["L1"]),
        "L2": max(distances["L2"], compute_random_split_p90_ceiling(ref, fps), CEILING_FLOOR["L2"]),
        "L3": max(distances["L3"], CEILING_FLOOR["L3"]),
        "L4": max(distances["L4"], CEILING_FLOOR["L4"]),
    }


VERDICT_FAMILIES = ("L1", "L2", "L3")


def print_table(header: str, columns: list, rows: dict):
    print(header)
    print(f"{'family':<8}" + "".join(f"{name:>12}" for name in columns))
    for family in FAMILIES:
        print(f"{family:<8}" + "".join(f"{value:>12.6f}" for value in rows[family]))


def write_json(path: str, report: dict):
    Path(path).write_text(json.dumps(report, indent=2))


def run_self_check(ref: dict, json_path):
    distances = split_distances(ref, ref["fps"])
    ceiling = compute_ceiling(ref, ref["fps"])

    print_table(
        "Reference self-check (split against split)",
        ["distance", "ceiling"],
        {family: (distances[family], ceiling[family]) for family in FAMILIES},
    )

    if json_path:
        write_json(json_path, {"distances": distances, "ceiling": ceiling})


def run_match(ref: dict, render: dict, json_path):
    distances = family_distances(
        describe_sequence(ref, ref["fps"]), describe_sequence(render, render["fps"])
    )
    ceiling = compute_ceiling(ref, ref["fps"])
    scores = {family: distances[family] / ceiling[family] for family in FAMILIES}

    print_table(
        "Reference against render",
        ["distance", "ceiling", "score"],
        {family: (distances[family], ceiling[family], scores[family]) for family in FAMILIES},
    )

    verdict = all(scores[family] <= 1.0 for family in VERDICT_FAMILIES)
    print(f"verdict: {'pass' if verdict else 'fail'}")

    if json_path:
        write_json(
            json_path,
            {"distances": distances, "ceiling": ceiling, "scores": scores, "verdict": verdict},
        )


def main():
    args = parse_args()

    ref = load_reference(Path(args.ref_dir))

    if args.self_check:
        run_self_check(ref, args.json)
        return

    run_match(ref, load_render(Path(args.render), args.fps), args.json)


if __name__ == "__main__":
    main()
