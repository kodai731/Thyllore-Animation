"""Gate score computation and summary printing for flame/dust ref-match."""

import numpy as np

ROOT_BAND = 7


def nan_corr(a, b):
    a, b = np.array(a, dtype=float), np.array(b, dtype=float)
    ok = ~(np.isnan(a) | np.isnan(b))
    if ok.sum() < 3:
        return float("nan")
    return float(np.corrcoef(a[ok], b[ok])[0, 1])


def nan_rms(a, b):
    a, b = np.array(a, dtype=float), np.array(b, dtype=float)
    ok = ~(np.isnan(a) | np.isnan(b))
    if not ok.any():
        return float("nan")
    return float(np.sqrt(np.mean((a[ok] - b[ok]) ** 2)))


def compare(ref, render):
    rows = []
    for name, rv, cv in zip(["lum_p10", "lum_p50", "lum_p90"], ref["lum_p10_50_90"], render["lum_p10_50_90"]):
        ratio = cv / rv
        rows.append((f"i   {name}", f"{rv:.0f}", f"{cv:.0f}", f"ratio {ratio:.2f}", 0.8 <= ratio <= 1.2))
    diff = (render["dark_fraction"] - ref["dark_fraction"]) * 100
    rows.append(("ii  dark(<80) %", f"{ref['dark_fraction'] * 100:.0f}", f"{render['dark_fraction'] * 100:.0f}", f"diff {diff:+.0f}pt", abs(diff) <= 8))
    diff = (render["bright_fraction"] - ref["bright_fraction"]) * 100
    rows.append(("    bright(>200) %", f"{ref['bright_fraction'] * 100:.0f}", f"{render['bright_fraction'] * 100:.0f}", f"diff {diff:+.0f}pt", None))
    for name in ["band_mean", "band_p90"]:
        corr = nan_corr(ref[name], render[name])
        rows.append((f"iii {name} corr", "", "", f"{corr:.2f}", corr >= 0.8))
    for name in ["gr_by_lum", "br_by_lum"]:
        rms = nan_rms(ref[name], render[name])
        rows.append((f"iv  {name} rms", "", "", f"{rms:.3f}", rms <= 0.08))
    log_ratio = np.log2(np.array(render["contrast_spectrum"]) / np.array(ref["contrast_spectrum"]))
    rms = float(np.sqrt(np.nanmean(log_ratio ** 2)))
    rows.append(("v   contrast log2 rms", "", "", f"{rms:.2f}", rms <= 0.5))
    diff = render["bright_largest_share"] - ref["bright_largest_share"]
    rows.append(("vi  bright largest share", f"{ref['bright_largest_share']:.2f}", f"{render['bright_largest_share']:.2f}",
                 f"diff {diff:+.2f}", abs(diff) <= 0.15))
    ratio = render["bright_fragments_per_k"] / ref["bright_fragments_per_k"]
    rows.append(("vi  bright fragments/k", f"{ref['bright_fragments_per_k']:.1f}", f"{render['bright_fragments_per_k']:.1f}",
                 f"ratio {ratio:.2f}", 0.5 <= ratio <= 2.0))
    rows.extend(compare_structure(ref, render))
    return rows


def gate_score(name, value, rv, cv):
    """Similarity in [0, 1] of one gate row, 1 = identical to the reference.

    ratio rows: min(r, 1/r); diff rows: 1 - |render - ref| / max(|ref|, |render|); correlation rows: the
    correlation; rms rows: 1 - rms (log2 rms rows: 1 - rms / 2, so a factor 4 scores 0)."""
    kind, _, number = value.partition(" ")
    if kind == "ratio":
        r = float(number)
        return float(min(r, 1.0 / r)) if r > 0 else 0.0
    if kind == "diff":
        a, b = float(rv), float(cv)
        scale = max(abs(a), abs(b))
        return 1.0 if scale == 0 else float(max(0.0, 1.0 - abs(b - a) / scale))
    x = float(kind)
    if "log2" in name:
        return float(max(0.0, 1.0 - x / 2.0))
    if "rms" in name:
        return float(max(0.0, 1.0 - x))
    return float(max(0.0, min(1.0, x)))


def score_rows(rows):
    return [(name, rv, cv, value, ok, gate_score(name, value, rv, cv)) for name, rv, cv, value, ok in rows]


def ratio_row(label, rv, cv, low, high, fmt=".2f"):
    ratio = cv / rv if rv else np.nan
    return (label, f"{rv:{fmt}}", f"{cv:{fmt}}", f"ratio {ratio:.2f}", bool(low <= ratio <= high))


def compare_structure(ref, render):
    rows = []
    diff = render["interior_hole_ratio"] - ref["interior_hole_ratio"]
    rows.append(("vii interior hole", f"{ref['interior_hole_ratio']:.3f}", f"{render['interior_hole_ratio']:.3f}",
                 f"diff {diff:+.3f}", diff <= 0.05))
    rows.append(ratio_row("viii puff isotropy", ref["puff_isotropy_scale"][0], render["puff_isotropy_scale"][0], 0.7, 1.4))
    rows.append(ratio_row("viii puff scale", ref["puff_isotropy_scale"][1], render["puff_isotropy_scale"][1], 0.6, 1.6, ".3f"))
    rows.append(ratio_row("ix  centerline bend", ref["centerline_straightness"], render["centerline_straightness"], 0.5, 2.0, ".3f"))
    middle_ref = float(np.nanmean(ref["centerline_amplitude"][2:ROOT_BAND]))
    middle_render = float(np.nanmean(render["centerline_amplitude"][2:ROOT_BAND]))
    rows.append(ratio_row("ix  middle amplitude", middle_ref, middle_render, 0.5, 2.0, ".3f"))
    rows.append(ratio_row("x   width p90/median", float(np.nanmean(ref["width_spread"])), float(np.nanmean(render["width_spread"])), 0.8, 1.25))
    rows.append(ratio_row("x   base width ratio", ref["base_width_ratio"], render["base_width_ratio"], 0.7, 1.4))
    count_ref, count_render = ref["top_fragments"][0], render["top_fragments"][0]
    diff = count_render - count_ref
    rows.append(("xi  top fragments", f"{count_ref:.1f}", f"{count_render:.1f}", f"diff {diff:+.1f}", abs(diff) <= 1.0))
    width_row = ratio_row("xi  top fragment width", ref["top_fragments"][1], render["top_fragments"][1], 0.5, 2.0, ".3f")
    if count_ref < 0.5:
        width_row = width_row[:4] + (None,)
    rows.append(width_row)
    rows.append(ratio_row("xii vertical mod w/8", ref["vertical_modulation"][0], render["vertical_modulation"][0], 0.6, 1.6, ".3f"))
    rows.append(ratio_row("xii vertical mod w/4", ref["vertical_modulation"][1], render["vertical_modulation"][1], 0.6, 1.6, ".3f"))
    rows.append(ratio_row("xvii halo extra width", ref["halo_spread"][0], render["halo_spread"][0], 0.7, 1.4))
    rows.append(ratio_row("xvii halo tongues", ref["halo_spread"][1], render["halo_spread"][1], 0.5, 2.0, ".1f"))
    return rows


def print_summary(rows):
    passed = sum(1 for r in rows if r[4])
    gated = sum(1 for r in rows if r[4] is not None)
    scores = [r[5] for r in rows if r[4] is not None]
    above = sum(1 for v in scores if v >= 0.95)
    print(f"\n{passed}/{gated} gates pass, score mean {np.mean(scores):.3f} min {min(scores):.2f}, {above}/{gated} >= 0.95")
    video_rows = [r for r in rows if r[0].startswith("xviii")]
    if video_rows:
        video_scores = [r[5] for r in video_rows]
        video_passed = sum(1 for r in video_rows if r[4])
        print(f"video score {np.mean(video_scores):.3f} ({video_passed}/{len(video_rows)} xviii gates pass)")
