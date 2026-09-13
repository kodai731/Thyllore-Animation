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
  W6 shape: mask driven descriptors averaged over the sequence. S1 is the contour isoperimetric ratio
      plus the RMS of three curvature spectrum bands, S2 is the scale normalized LoG blob radius
      histogram plus the blob count density, S3 reuses the W4 structure tensor direction.
      Distance = S1 log ratios + S2 histogram L1 and log density ratio + W4 direction distance.
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
CONTOUR_SAMPLES = 256
CURVATURE_BANDS = ((12, 23), (24, 47), (48, 96))
BLOB_SCALES = 6
BLOB_SCALE_RANGE = (1.0 / 128.0, 1.0 / 8.0)
BLOB_AMPLITUDE_FRACTION = 0.1
MIN_COUNT_DENSITY = 1e-3
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


def resample_contour(points, sample_count):
    closed = np.vstack([points, points[:1]])
    arc_lengths = np.concatenate([[0.0], np.cumsum(np.linalg.norm(np.diff(closed, axis=0), axis=1))])
    if arc_lengths[-1] <= 0.0:
        return None, 0.0
    sample_positions = np.linspace(0.0, arc_lengths[-1], sample_count, endpoint=False)
    resampled = np.stack([np.interp(sample_positions, arc_lengths, closed[:, axis]) for axis in (0, 1)], axis=1)
    return resampled, arc_lengths[-1] / sample_count


def measure_contour(mask):
    contours, _ = cv2.findContours(mask.astype(np.uint8), cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    if not contours:
        return None
    largest = max(contours, key=cv2.contourArea)
    area = float(cv2.contourArea(largest))
    perimeter = float(cv2.arcLength(largest, True))
    if area <= 0.0 or perimeter <= 0.0:
        return None
    isoperimetric = (perimeter ** 2) / (4.0 * np.pi * area)

    resampled, arc_step = resample_contour(largest.reshape(-1, 2).astype(np.float64), CONTOUR_SAMPLES)
    if resampled is None:
        return None
    tangents = np.roll(resampled, -1, axis=0) - np.roll(resampled, 1, axis=0)
    angles = np.arctan2(tangents[:, 1], tangents[:, 0])
    angle_steps = np.mod(np.roll(angles, -1) - angles + np.pi, 2.0 * np.pi) - np.pi
    curvature = np.abs(angle_steps) / arc_step

    amplitudes = np.abs(np.fft.rfft(curvature))
    curvature_bands = [float(np.sqrt(np.mean(amplitudes[low:high + 1] ** 2))) for low, high in CURVATURE_BANDS]
    return {"isoperimetric": isoperimetric, "curvature_bands": curvature_bands}


def measure_blobs(lum, mask):
    mask_pixels = int(mask.sum())
    equiv_diameter = np.sqrt(mask_pixels / np.pi) * 2.0
    sigmas = equiv_diameter * np.geomspace(BLOB_SCALE_RANGE[0], BLOB_SCALE_RANGE[1], BLOB_SCALES)

    bright_amplitudes = []
    for sigma in sigmas:
        amplitude = -(ndimage.gaussian_laplace(lum, sigma) * (sigma ** 2))
        local_peaks = mask & (ndimage.maximum_filter(amplitude, size=3) == amplitude) & (amplitude > 0.0)
        bright_amplitudes.append(np.where(local_peaks, amplitude, 0.0))

    peak_threshold = BLOB_AMPLITUDE_FRACTION * max(float(amplitude.max()) for amplitude in bright_amplitudes)
    counts = np.array([float((amplitude > peak_threshold).sum()) for amplitude in bright_amplitudes])

    total = counts.sum()
    radius_histogram = (counts / total).tolist() if total > 0.0 else [0.0] * BLOB_SCALES
    count_density = float(total / (mask_pixels / 1000.0))
    return {"radius_histogram": radius_histogram, "count_density": count_density}


def measure_shape(fields):
    contour_results, blob_results, direction_results = [], [], []
    for lum, mask in fields:
        if mask.sum() < MIN_MASK_PIXELS:
            continue
        contour = measure_contour(mask)
        if contour is None:
            continue
        contour_results.append(contour)
        blob_results.append(measure_blobs(lum, mask))
        measured = structure_tensor_direction(lum, mask)
        if measured is not None:
            direction_results.append(measured)

    if direction_results:
        histograms, anisotropies = zip(*direction_results)
        s3 = {"direction_histogram": np.mean(histograms, axis=0).tolist(),
              "anisotropy": float(np.mean(anisotropies))}
    else:
        s3 = {"direction_histogram": [float("nan")] * DIRECTION_BINS, "anisotropy": float("nan")}
    if not contour_results:
        return {"s1": {"isoperimetric": float("nan"), "curvature_bands": [float("nan")] * len(CURVATURE_BANDS)},
                "s2": {"radius_histogram": [float("nan")] * BLOB_SCALES, "count_density": float("nan")},
                "s3": s3,
                "frames": 0}

    return {
        "s1": {"isoperimetric": float(np.mean([c["isoperimetric"] for c in contour_results])),
               "curvature_bands": np.mean([c["curvature_bands"] for c in contour_results], axis=0).tolist()},
        "s2": {"radius_histogram": np.mean([b["radius_histogram"] for b in blob_results], axis=0).tolist(),
               "count_density": float(np.mean([b["count_density"] for b in blob_results]))},
        "s3": s3,
        "frames": len(contour_results),
    }


def s1_distance(a, b):
    isoperimetric_diff = abs(float(np.log((a["isoperimetric"] + EPS) / (b["isoperimetric"] + EPS))))
    band_ratio = (np.array(a["curvature_bands"]) + EPS) / (np.array(b["curvature_bands"]) + EPS)
    band_diff = float(np.sqrt(np.mean(np.log(band_ratio) ** 2)))
    return isoperimetric_diff + band_diff


def s2_distance(a, b):
    histogram_diff = float(np.sum(np.abs(np.array(a["radius_histogram"]) - np.array(b["radius_histogram"]))))
    density_a = max(a["count_density"], MIN_COUNT_DENSITY)
    density_b = max(b["count_density"], MIN_COUNT_DENSITY)
    return histogram_diff + abs(float(np.log(density_a / density_b)))


def s3_distance(a, b):
    return direction_distance(a, b)


def measure_wind(paths, column_width, fps, resample):
    measured = flame.measure(paths, column_width, resample=resample)
    fields = [frame_fields(path) for path in paths]
    measured["w4"] = measure_direction(fields)
    measured["w5"] = measure_transport(fields, fps)
    measured["w6"] = measure_shape(fields)
    return measured


def print_wind(label, measured):
    w4, w5, w6 = measured["w4"], measured["w5"], measured["w6"]
    peak = int(np.argmax(w4["direction_histogram"])) if w4["frames"] else -1
    print(f"{label}: w4 anisotropy {w4['anisotropy']:.3f}, peak direction bin {peak}/{DIRECTION_BINS}; "
          f"w5 displacement {w5['displacement'][0]:+.5f},{w5['displacement'][1]:+.5f} width fraction per second "
          f"over {w5['pairs']} pairs; w6 isoperimetric {w6['s1']['isoperimetric']:.3f}, "
          f"count density {w6['s2']['count_density']:.3f}, anisotropy {w6['s3']['anisotropy']:.3f}")


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
                 "w5": transport_distance(ref["w5"], render["w5"]),
                 "w6": s1_distance(ref["w6"]["s1"], render["w6"]["s1"])
                 + s2_distance(ref["w6"]["s2"], render["w6"]["s2"])
                 + s3_distance(ref["w6"]["s3"], render["w6"]["s3"])}
    print(f"\nw4 direction distance {distances['w4']:.3f}, w5 transport distance {distances['w5']:.5f}, "
          f"w6 shape distance {distances['w6']:.3f}")

    if args.json:
        Path(args.json).write_text(json.dumps({"reference": ref, "render": render, "distances": distances,
                                               "gates": [{"name": r[0], "value": r[3], "pass": r[4], "score": r[5]}
                                                         for r in rows]}, indent=2))


if __name__ == "__main__":
    main()
