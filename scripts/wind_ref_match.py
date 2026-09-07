"""Measure how closely rendered wind/dust frames match a storm reference sequence.

Usage:
  python scripts/wind_ref_match.py --ref-dir <dir> --render <png|dir> [--fps 29.97]
      [--ref-window first,end] [--json out.json]

The W1-W3 descriptors (luminance statistics, height-band profile, contrast/structure gates) are the
flame_ref_match ones, measured with the dust silhouette (luminance on a black background). Two
descriptor families are added on top:

  W4 flow direction: structure tensor of the Sobel gradients (gaussian sigma 3 px), principal
      direction histogram over 0..pi in 16 bins inside the silhouette plus the median anisotropy
      (l1 - l2) / (l1 + l2), averaged over the sequence. Distance = histogram L1 + |anisotropy diff|.
  W5 transport: phase correlation of adjacent frames (silhouette-masked luminance), the median
      horizontal/vertical shift as a width fraction per second. Distance = L2 of the shifts.
"""

import argparse
import json
from pathlib import Path

import cv2
import numpy as np
from scipy import ndimage

import flame_ref_match as flame

DIRECTION_BINS = 16
STRUCTURE_SIGMA = 3.0
MIN_MASK_PIXELS = 100
EPS = 1e-9


def frame_fields(path):
    rgb = flame.to_rgb(flame.load_image(path))
    return flame.luminance(rgb).astype(np.float32), flame.silhouette_mask(rgb)


def structure_tensor_direction(lum, mask):
    gx = cv2.Sobel(lum, cv2.CV_32F, 1, 0, ksize=3)
    gy = cv2.Sobel(lum, cv2.CV_32F, 0, 1, ksize=3)
    jxx = ndimage.gaussian_filter(gx * gx, STRUCTURE_SIGMA)
    jyy = ndimage.gaussian_filter(gy * gy, STRUCTURE_SIGMA)
    jxy = ndimage.gaussian_filter(gx * gy, STRUCTURE_SIGMA)

    direction = np.mod(0.5 * np.arctan2(2.0 * jxy, jxx - jyy), np.pi)
    eigen_gap = np.sqrt((jxx - jyy) ** 2 + 4.0 * jxy ** 2)
    anisotropy = eigen_gap / (jxx + jyy + EPS)

    counts, _ = np.histogram(direction[mask], bins=DIRECTION_BINS, range=(0.0, np.pi))
    if counts.sum() == 0:
        return None
    return counts / counts.sum(), float(np.median(anisotropy[mask]))


def measure_direction(fields):
    histograms, anisotropies = [], []
    for lum, mask in fields:
        if mask.sum() < MIN_MASK_PIXELS:
            continue
        measured = structure_tensor_direction(lum, mask)
        if measured is None:
            continue
        histogram, anisotropy = measured
        histograms.append(histogram)
        anisotropies.append(anisotropy)
    if not histograms:
        return {"direction_histogram": [float("nan")] * DIRECTION_BINS, "anisotropy": float("nan"), "frames": 0}
    return {
        "direction_histogram": np.mean(histograms, axis=0).tolist(),
        "anisotropy": float(np.mean(anisotropies)),
        "frames": len(histograms),
    }


def measure_transport(fields, fps):
    shifts = []
    for (lum_a, mask_a), (lum_b, mask_b) in zip(fields[:-1], fields[1:]):
        if lum_a.shape != lum_b.shape or mask_a.sum() < MIN_MASK_PIXELS or mask_b.sum() < MIN_MASK_PIXELS:
            continue
        windowed_a = np.ascontiguousarray(lum_a * mask_a, dtype=np.float32)
        windowed_b = np.ascontiguousarray(lum_b * mask_b, dtype=np.float32)
        (dx, dy), response = cv2.phaseCorrelate(windowed_a, windowed_b)
        width = float(lum_a.shape[1])
        shifts.append((dx / width * fps, dy / width * fps, response))
    if not shifts:
        return {"displacement": [float("nan"), float("nan")], "response": float("nan"), "pairs": 0}
    medians = np.median(np.array(shifts), axis=0)
    return {"displacement": [float(medians[0]), float(medians[1])], "response": float(medians[2]),
            "pairs": len(shifts)}


def direction_distance(ref, render):
    difference = np.abs(np.array(ref["direction_histogram"]) - np.array(render["direction_histogram"]))
    return float(difference.sum() + abs(ref["anisotropy"] - render["anisotropy"]))


def transport_distance(ref, render):
    return float(np.linalg.norm(np.array(ref["displacement"]) - np.array(render["displacement"])))


def measure_wind(paths, column_width, fps, resample):
    measured = flame.measure(paths, column_width, resample=resample)
    fields = [frame_fields(path) for path in paths]
    measured["w4"] = measure_direction(fields)
    measured["w5"] = measure_transport(fields, fps)
    return measured


def print_wind(label, measured):
    w4, w5 = measured["w4"], measured["w5"]
    peak = int(np.argmax(w4["direction_histogram"])) if w4["frames"] else -1
    print(f"{label}: w4 anisotropy {w4['anisotropy']:.3f}, peak direction bin {peak}/{DIRECTION_BINS}; "
          f"w5 displacement {w5['displacement'][0]:+.5f},{w5['displacement'][1]:+.5f} width fraction per second "
          f"over {w5['pairs']} pairs")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--render", required=True, help="rendered png or directory of pngs")
    parser.add_argument("--ref-dir", required=True, help="reference frame directory")
    parser.add_argument("--fps", type=float, default=30.0, help="frame rate of the render sequence")
    parser.add_argument("--ref-window", default=None,
                        help="first,end reference frame for the sequence descriptors (default: every frame)")
    parser.add_argument("--json", help="write reference/render/distance results to this path")
    args = parser.parse_args()
    flame.silhouette_mode = "dust"

    ref_fps, caption_frames = flame.load_ref_meta(args.ref_dir)
    ref_paths = flame.collect_frames(args.ref_dir)
    first, end = (int(v) for v in args.ref_window.split(",")) if args.ref_window else (0, len(ref_paths))
    ref_paths = [p for i, p in enumerate(ref_paths[first:end], first) if i not in caption_frames]
    column_width = flame.reference_column_width(ref_paths)

    ref = measure_wind(ref_paths, column_width, ref_fps, resample=False)
    render = measure_wind(flame.collect_frames(args.render), column_width, args.fps, resample=True)
    print(f"reference: {ref['frames']} frames at {ref_fps:.2f} fps, column width {column_width:.0f} px")
    print(f"render:    {render['frames']} frames at {args.fps:.2f} fps")
    print_wind("reference", ref)
    print_wind("render   ", render)

    rows = flame.score_rows(flame.compare(ref, render))
    print(f"\n{'gate':<26}{'ref':>6}{'render':>8}  {'value':<14}{'status':<6}score")
    for name, rv, cv, value, ok, score in rows:
        status = "-" if ok is None else ("PASS" if ok else "FAIL")
        print(f"{name:<26}{rv:>6}{cv:>8}  {value:<14}{status:<6}{score:.2f}")
    flame.print_summary(rows)

    distances = {"w4": direction_distance(ref["w4"], render["w4"]),
                 "w5": transport_distance(ref["w5"], render["w5"])}
    print(f"\nw4 direction distance {distances['w4']:.3f}, w5 transport distance {distances['w5']:.5f}")

    if args.json:
        Path(args.json).write_text(json.dumps({"reference": ref, "render": render, "distances": distances,
                                               "gates": [{"name": r[0], "value": r[3], "pass": r[4], "score": r[5]}
                                                         for r in rows]}, indent=2))


if __name__ == "__main__":
    main()
