"""Common flame/dust reference match statistics.

Constants and per-frame metrics shared by flame_ref_match.py and wind_ref_match.py:
luminance percentiles, height-band profiles, spectral contrast, bright coherence, interior
hole ratio, puff isotropy, vertical modulation, centerline amplitude/straightness, width
profile, top fragments, halo spread, and the frame_stats / aggregate / measure entry points.
"""

import sys

import numpy as np
from scipy import ndimage

from ref_match.frames import halo_mask, load_image, luminance, resample_to_column_width, silhouette_mask, to_rgb
from ref_match.gates import ROOT_BAND

LUM_BINS = [(40, 80), (80, 120), (120, 160), (160, 200), (200, 256)]
BAND_COUNT = 10
DARK_LUM = 80.0
CONTRAST_SIGMA_WIDTH_FRACTIONS = [1 / 256, 1 / 128, 1 / 64, 1 / 32, 1 / 16, 1 / 8, 1 / 4]
BRIGHT_PERCENTILE = 70.0
HALO_TONGUE_PROTRUSION = 0.25
HALO_BASE_POOL_FRACTION = 0.2


def row_edges(mask):
    first = np.where(mask.any(axis=1), mask.argmax(axis=1), np.nan)
    last = np.where(mask.any(axis=1), mask.shape[1] - 1 - mask[:, ::-1].argmax(axis=1), np.nan)
    return first.astype(float), last.astype(float)


def halo_spread(rgb, mask, column_width, mode):
    """(extra width / column width, tongue count) of the dim halo above the base pool."""
    core = ndimage.binary_fill_holes(mask)
    halo = ndimage.binary_fill_holes(halo_mask(rgb, mode=mode) | mask)
    ys = np.where(core.any(axis=1))[0]
    y0, y1 = ys.min(), ys.max() + 1
    cut = y0 + int((y1 - y0) * (1.0 - HALO_BASE_POOL_FRACTION))
    core, halo = core[y0:cut], halo[y0:cut]
    rows = core.any(axis=1)
    if rows.sum() < 10:
        return np.nan, np.nan

    extra_width = float(((halo.sum(axis=1) - core.sum(axis=1))[rows]).mean() / column_width)
    core_left, core_right = row_edges(core)
    halo_left, halo_right = row_edges(halo)
    protrusion = np.nan_to_num(np.maximum(halo_right - core_right, core_left - halo_left)) / column_width
    runs, count = ndimage.label(protrusion > HALO_TONGUE_PROTRUSION)
    run_heights = ndimage.sum(np.ones_like(runs), runs, range(1, count + 1)) if count else []
    tongues = float(sum(1 for height in run_heights if height >= column_width / 4.0))
    return extra_width, tongues


def contrast_spectrum(lum, mask, column_width):
    erosion = max(1, round(column_width / 32))
    inner = ndimage.binary_erosion(mask, iterations=erosion)
    if inner.sum() < 50:
        return [np.nan] * len(CONTRAST_SIGMA_WIDTH_FRACTIONS)
    filled = np.where(mask, lum, lum[mask].mean())
    mean_lum = lum[inner].mean()
    spectrum = []
    for fraction in CONTRAST_SIGMA_WIDTH_FRACTIONS:
        sigma = fraction * column_width
        band = ndimage.gaussian_filter(filled, sigma * 0.5) - ndimage.gaussian_filter(filled, sigma)
        spectrum.append(float(band[inner].std() / mean_lum))
    return spectrum


def bright_coherence(lum, mask):
    lum_n, mask_n = lum, mask
    if mask_n.sum() < 50:
        return np.nan, np.nan
    bright = mask_n & (lum_n >= np.percentile(lum_n[mask_n], BRIGHT_PERCENTILE))
    labels, count = ndimage.label(bright)
    if count == 0:
        return np.nan, np.nan
    sizes = ndimage.sum(bright, labels, range(1, count + 1))
    largest_share = float(sizes.max() / bright.sum())
    fragments_per_k = float(count / bright.sum() * 1000.0)
    return largest_share, fragments_per_k


def interior_hole_ratio(lum, mask, column_width):
    filled = ndimage.binary_fill_holes(mask)
    distance = ndimage.distance_transform_edt(filled)
    interior = distance > column_width / 8.0
    if interior.sum() < 50:
        return np.nan
    return float((lum[interior] < DARK_LUM).mean())


def correlation_length(signal, valid, axis, max_lag):
    lags = np.arange(1, max_lag + 1)
    correlations = []
    variance = signal[valid].var()
    for lag in lags:
        a = np.take(signal, range(signal.shape[axis] - lag), axis=axis)
        b = np.take(signal, range(lag, signal.shape[axis]), axis=axis)
        va = np.take(valid, range(valid.shape[axis] - lag), axis=axis)
        vb = np.take(valid, range(lag, valid.shape[axis]), axis=axis)
        both = va & vb
        if both.sum() < 50 or variance <= 0:
            return np.nan
        correlations.append((a[both] * b[both]).mean() / variance)
    correlations = np.array(correlations)
    below = np.where(correlations < np.exp(-1.0))[0]
    if below.size == 0:
        return float(max_lag)
    k = below[0]
    if k == 0:
        return float(lags[0] * correlations[0])
    c0, c1 = correlations[k - 1], correlations[k]
    return float(lags[k - 1] + (c0 - np.exp(-1.0)) / max(c0 - c1, 1e-6))

def puff_isotropy(lum, mask, column_width):
    interior = ndimage.binary_erosion(mask, iterations=max(1, round(column_width / 32)))
    if interior.sum() < 200:
        return np.nan, np.nan
    filled = np.where(mask, lum, lum[mask].mean())
    detail = filled - ndimage.gaussian_filter(filled, column_width / 4.0)
    detail = detail - detail[interior].mean()
    max_lag = max(2, round(column_width / 2))
    length_x = correlation_length(detail, interior, 1, max_lag)
    length_y = correlation_length(detail, interior, 0, max_lag)
    if np.isnan(length_x) or np.isnan(length_y) or length_x <= 0:
        return np.nan, np.nan
    return float(length_y / length_x), float((length_x + length_y) / 2.0 / column_width)


def vertical_modulation(lum, mask, column_width):
    rows = np.where(mask.any(axis=1))[0]
    y0, y1 = rows.min(), rows.max() + 1
    row_counts = mask[y0:y1].sum(axis=1)
    row_means = (lum[y0:y1] * mask[y0:y1]).sum(axis=1) / np.maximum(row_counts, 1)
    profile = row_means[row_counts > 0.3 * column_width]
    if profile.size < column_width:
        return [np.nan, np.nan]
    out = []
    for sigma in (column_width / 8.0, column_width / 4.0):
        band = ndimage.gaussian_filter1d(profile, sigma) - ndimage.gaussian_filter1d(profile, 2.0 * sigma)
        out.append(float(band.std() / profile.mean()))
    return out


def row_bands(mask):
    ys = np.where(mask.any(axis=1))[0]
    y0, y1 = ys.min(), ys.max() + 1
    edges = [y0 + (y1 - y0) * band // BAND_COUNT for band in range(BAND_COUNT + 1)]
    return list(zip(edges[:-1], edges[1:]))


def row_centers(mask):
    xs = np.arange(mask.shape[1])
    counts = mask.sum(axis=1)
    return np.where(counts > 0, (mask * xs).sum(axis=1) / np.maximum(counts, 1), np.nan)


def centerline_amplitude(mask, column_width):
    centers = row_centers(mask)
    amplitudes = []
    for ya, yb in row_bands(mask):
        band = centers[ya:yb]
        band = band[~np.isnan(band)]
        amplitudes.append(float(band.std() / column_width) if band.size >= 3 else np.nan)
    return amplitudes


def centerline_straightness(mask, column_width):
    centers = row_centers(mask)
    bands = row_bands(mask)
    ya, yb = bands[1][0], bands[ROOT_BAND][0]
    rows = np.arange(ya, yb)
    values = centers[ya:yb]
    valid = ~np.isnan(values)
    if valid.sum() < 10:
        return np.nan
    slope, intercept = np.polyfit(rows[valid], values[valid], 1)
    deviation = values[valid] - (slope * rows[valid] + intercept)
    return float(np.sqrt((deviation ** 2).mean()) / column_width)


def width_profile(mask, column_width):
    widths = mask.sum(axis=1).astype(float)
    spread = []
    for ya, yb in row_bands(mask):
        band = widths[ya:yb]
        band = band[band > 0]
        spread.append(float(np.percentile(band, 90) / np.median(band)) if band.size >= 3 else np.nan)
    ys = np.where(widths > 0)[0]
    y0, y1 = ys.min(), ys.max() + 1
    rows = y1 - y0
    base = widths[y1 - max(1, int(rows * 0.15)):y1]
    middle = widths[y0 + int(rows * 0.3):y0 + int(rows * 0.7)]
    base_ratio = float(base.max() / np.median(middle[middle > 0]))
    return spread, base_ratio


def top_fragments(lum, mask, column_width):
    ys = np.where(mask.any(axis=1))[0]
    y0, y1 = ys.min(), ys.max() + 1
    cut = y0 + (y1 - y0) // 4
    labels, count = ndimage.label(mask)
    attached = set(np.unique(labels[cut - 1:cut + 1])) - {0}
    min_area = (column_width / 16.0) ** 2
    widths = []
    for label in range(1, count + 1):
        if label in attached:
            continue
        rows, cols = np.where(labels[:cut] == label)
        if rows.size < min_area:
            continue
        widths.append((cols.max() - cols.min() + 1) / column_width)
    median_width = float(np.median(widths)) if widths else 0.0
    return float(len(widths)), median_width


def frame_stats(rgb, column_width, mode="flame"):
    mask = silhouette_mask(rgb, mode=mode)
    if mask.sum() < 100:
        return None
    lum = luminance(rgb)
    largest_share, fragments_per_k = bright_coherence(lum, mask)
    r, g, b = rgb[:, :, 0], rgb[:, :, 1], rgb[:, :, 2]
    ys = np.where(mask.any(axis=1))[0]
    y0, y1 = ys.min(), ys.max() + 1

    band_mean, band_p90 = [], []
    for band in range(BAND_COUNT):
        ya = y0 + (y1 - y0) * band // BAND_COUNT
        yb = y0 + (y1 - y0) * (band + 1) // BAND_COUNT
        values = lum[ya:yb][mask[ya:yb]]
        band_mean.append(float(values.mean()) if values.size else np.nan)
        band_p90.append(float(np.percentile(values, 90)) if values.size else np.nan)

    gr_curve, br_curve = [], []
    for lo, hi in LUM_BINS:
        selected = mask & (lum >= lo) & (lum < hi)
        if selected.sum() >= 30:
            gr_curve.append(float((g[selected] / r[selected]).mean()))
            br_curve.append(float((b[selected] / r[selected]).mean()))
        else:
            gr_curve.append(np.nan)
            br_curve.append(np.nan)

    return {
        "pixels": int(mask.sum()),
        "lum_p10_50_90": [float(v) for v in np.percentile(lum[mask], [10, 50, 90])],
        "dark_fraction": float((lum[mask] < DARK_LUM).mean()),
        "bright_fraction": float((lum[mask] > 200).mean()),
        "band_mean": band_mean,
        "band_p90": band_p90,
        "gr_by_lum": gr_curve,
        "br_by_lum": br_curve,
        "contrast_spectrum": contrast_spectrum(lum, mask, column_width),
        "bright_largest_share": largest_share,
        "bright_fragments_per_k": fragments_per_k,
        "interior_hole_ratio": interior_hole_ratio(lum, mask, column_width),
        "puff_isotropy_scale": list(puff_isotropy(lum, mask, column_width)),
        "halo_spread": list(halo_spread(rgb, mask, column_width, mode)),
        "centerline_amplitude": centerline_amplitude(mask, column_width),
        "centerline_straightness": centerline_straightness(mask, column_width),
        "width_spread_base": width_profile(mask, column_width),
        "top_fragments": list(top_fragments(lum, mask, column_width)),
        "vertical_modulation": vertical_modulation(lum, mask, column_width),
    }


def aggregate(stat_list):
    keys = ["lum_p10_50_90", "dark_fraction", "bright_fraction", "band_mean", "band_p90", "gr_by_lum", "br_by_lum",
            "contrast_spectrum", "bright_largest_share", "bright_fragments_per_k", "interior_hole_ratio",
            "puff_isotropy_scale", "centerline_amplitude", "centerline_straightness", "top_fragments",
            "vertical_modulation", "halo_spread"]
    out = {"frames": len(stat_list)}
    for key in keys:
        out[key] = np.nanmean(np.array([s[key] for s in stat_list], dtype=float), axis=0).tolist()
    out["width_spread"] = np.nanmean(np.array([s["width_spread_base"][0] for s in stat_list]), axis=0).tolist()
    out["base_width_ratio"] = float(np.nanmean([s["width_spread_base"][1] for s in stat_list]))
    return out


def measure(paths, column_width, crop=None, resample=True, mode="flame"):
    stats = []
    for path in paths:
        image = load_image(path, crop)
        if resample:
            image = resample_to_column_width(image, column_width, mode=mode)
        stat = frame_stats(to_rgb(image), column_width, mode=mode)
        if stat is not None:
            stats.append(stat)
    if not stats:
        sys.exit(f"no flame silhouette found in {paths[0]} ...")
    aggregated = aggregate(stats)
    aggregated["column_width_px"] = column_width
    return aggregated
