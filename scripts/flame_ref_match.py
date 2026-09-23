"""Measure how closely rendered flame frames match the pillar_ref_seq_stable reference.

Usage:
  python scripts/flame_ref_match.py --render <png|dir> [--crop x0,y0,x1,y1] [--json out.json]
  python scripts/flame_ref_match.py --ref-only
  python scripts/flame_ref_match.py --render <dir> --temporal [--fps 10]   (sequence gates xiii-xvi)

The reference is measured at its native resolution. Every render frame is first resampled
with an area filter so that its median flame column width equals the reference's median
column width, so the render may be captured at any higher resolution and the comparison
happens at the reference pixel density.

Metrics (silhouette = R>90 && R-B>40, lum = 0.299R+0.587G+0.114B):
  i   lum p10/50/90 within +-20% of reference
  ii  fraction of silhouette pixels with lum<80 within +-8 pt
  iii height-band (10 bands) mean/p90 luminance profile correlation >= 0.8
  iv  G/R and B/R vs luminance-bin curves, RMS difference <= 0.08
  v   spatial contrast spectrum (band-pass std / mean, sigma = column width / 256 .. / 4), RMS log2 ratio <= 0.5
  vi  bright coherence: largest bright (>=p70) component share of bright pixels within +-0.15,
      bright fragment count per 1000 bright px ratio within [0.5, 2]
  vii interior (deeper than width/8 from the silhouette edge) dark (<80) fraction, diff <= +0.05
  viii puff isotropy (vertical / horizontal 1/e correlation length of the detail) ratio within [0.7, 1.4],
      puff scale (mean correlation length / width) ratio within [0.6, 1.6]
  ix  centerline bend: RMS deviation of the row centre from a straight-line fit over bands 1..6 (of 0..9,
      above the base pool) / width, ratio within [0.5, 2]; per-band centre std of bands 2..6 ratio within [0.5, 2]
  x   width p90/median per band ratio within [0.8, 1.25]; widest row of the bottom 15% / middle median width
      ratio within [0.7, 1.4]
  xi  detached silhouette fragments (area >= (width/16)^2) in the top quarter: count diff within +-1,
      median width ratio within [0.5, 2] (only gated when the reference has fragments)
  xvii dim halo (the flame outside the silhouette but above R>35 && R-B>15, rows above the base pool):
      extra width (halo width - silhouette width) / column width ratio within [0.7, 1.4]; tongues = halo
      runs at least width/4 tall protruding > 0.25 width beyond the silhouette on one side, count ratio
      within [0.5, 2]
  xviii video gate: see ref_match/video.py (--video, implies --temporal); whole-clip streaming: moving
      pair fraction, upward transport of the width and centre fields, centre phase lag between bands,
      centre return. The summary line reports the video score (mean of the xviii scores) separately.
  xiii-xvi sequence gates: see ref_match/sequence.py (--temporal; the render must be a sequence at
      --fps). The reference frame rate is read from the reference meta.json; --ref-window narrows the
      reference span (default: every frame). Frames listed as caption_frames in meta.json carry burned-in
      text and are excluded from the static reference (the silhouette fields the sequence gates use are
      hole-filled, so captions inside the column do not disturb them).
"""

import argparse
import json
from functools import partial
from pathlib import Path

from ref_match import sequence, video
from ref_match.frames import (SILHOUETTE_MODES, collect_frames, load_image, load_ref_meta, median_column_width,
                              reference_column_width, silhouette_mask, to_rgb)
from ref_match.gates import compare, print_summary, ratio_row, score_rows
from ref_match.stats import BAND_COUNT, CONTRAST_SIGMA_WIDTH_FRACTIONS, LUM_BINS, measure, row_centers

REF_DIR = Path(__file__).resolve().parents[1] / "assets/textures/flames/pillar_ref_seq_stable"


def print_profiles(ref, render):
    print("\nheight bands (top -> bottom), lum mean / p90:")
    for band in range(BAND_COUNT):
        rm, rp = ref["band_mean"][band], ref["band_p90"][band]
        cm, cp = render["band_mean"][band], render["band_p90"][band]
        print(f"  band {band}: ref {rm:5.0f}/{rp:5.0f}   render {cm:5.0f}/{cp:5.0f}")
    width = ref["column_width_px"]
    print(f"\ncontrast spectrum (band-pass std / mean, sigma in px at column width {width:.0f}):")
    for fraction, rv, cv in zip(CONTRAST_SIGMA_WIDTH_FRACTIONS, ref["contrast_spectrum"], render["contrast_spectrum"]):
        print(f"  sigma {fraction * width:5.1f} (1/{1 / fraction:.0f} width): ref {rv:.3f} render {cv:.3f}  x{cv / rv:.2f}")
    print(f"\ncenterline bend (rms from line / width): ref {ref['centerline_straightness']:.3f} render {render['centerline_straightness']:.3f}")
    print("centerline amplitude / width p90/median by band (top -> bottom):")
    for band in range(BAND_COUNT):
        print(f"  band {band}: amp ref {ref['centerline_amplitude'][band]:.3f} render {render['centerline_amplitude'][band]:.3f}"
              f"   spread ref {ref['width_spread'][band]:.2f} render {render['width_spread'][band]:.2f}")
    print("\nG/R and B/R by lum bin:")
    for (lo, hi), rg, cg, rb, cb in zip(LUM_BINS, ref["gr_by_lum"], render["gr_by_lum"], ref["br_by_lum"], render["br_by_lum"]):
        print(f"  lum {lo:3d}-{hi:3d}: G/R ref {rg:.2f} render {cg:.2f} | B/R ref {rb:.2f} render {cb:.2f}")


def measure_sequence(paths, column_width, dt, crop=None, resample=True, with_video=False, mode="flame"):
    rgbs = sequence.load_sequence(paths, column_width, crop, resample, load_image, to_rgb,
                                   partial(median_column_width, mode=mode))
    mode_silhouette_mask = partial(silhouette_mask, mode=mode)
    temporal = sequence.measure_temporal(rgbs, column_width, dt, mode_silhouette_mask, row_centers)
    if with_video:
        temporal["video"] = video.measure_video(rgbs, column_width, dt, mode_silhouette_mask, row_centers)
    return temporal


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--render", help="rendered png or directory of pngs")
    parser.add_argument("--crop", help="x0,y0,x1,y1 crop applied to every rendered png")
    parser.add_argument("--ref-dir", default=str(REF_DIR))
    parser.add_argument("--ref-only", action="store_true")
    parser.add_argument("--json", help="write reference/render/gate results to this path")
    parser.add_argument("--temporal", action="store_true", help="also measure the sequence gates xiii-xvi")
    parser.add_argument("--video", action="store_true", help="also measure the whole-clip video gate xviii (implies --temporal)")
    parser.add_argument("--fps", type=float, default=10.0, help="frame rate of the render sequence")
    parser.add_argument("--silhouette", choices=SILHOUETTE_MODES, default="flame",
                        help="silhouette definition: flame colours, or dust (luminance on a black background)")
    parser.add_argument("--ref-window", default=None,
                        help="first,end reference frame for the sequence gates (default: every frame)")
    args = parser.parse_args()
    args.temporal = args.temporal or args.video
    mode = args.silhouette

    ref_fps, caption_frames = load_ref_meta(args.ref_dir)
    ref_paths = collect_frames(args.ref_dir)
    static_paths = [p for i, p in enumerate(ref_paths) if i not in caption_frames]
    column_width = reference_column_width(static_paths, mode=mode)
    ref = measure(static_paths, column_width, resample=False, mode=mode)
    print(f"reference: {ref['frames']} frames, column width {column_width:.0f} px, lum p10/50/90 = "
          f"{ref['lum_p10_50_90'][0]:.0f}/{ref['lum_p10_50_90'][1]:.0f}/{ref['lum_p10_50_90'][2]:.0f}, "
          f"dark {ref['dark_fraction'] * 100:.0f}%, bright {ref['bright_fraction'] * 100:.0f}%")
    ref_temporal = None
    if args.temporal:
        first, end = (int(v) for v in args.ref_window.split(",")) if args.ref_window else (0, len(ref_paths))
        ref_temporal = measure_sequence(ref_paths[first:end], column_width, 1.0 / ref_fps, resample=False,
                                        with_video=args.video, mode=mode)
    if args.ref_only or not args.render:
        if ref_temporal is not None:
            sequence.print_temporal(ref_temporal, ref_temporal)
            if args.video:
                video.print_video(ref_temporal["video"], ref_temporal["video"])
            if args.json:
                Path(args.json).write_text(json.dumps({"reference_temporal": ref_temporal}, indent=2))
        return

    crop = tuple(int(v) for v in args.crop.split(",")) if args.crop else None
    render = measure(collect_frames(args.render), column_width, crop, mode=mode)
    print(f"render:    {render['frames']} frames, lum p10/50/90 = "
          f"{render['lum_p10_50_90'][0]:.0f}/{render['lum_p10_50_90'][1]:.0f}/{render['lum_p10_50_90'][2]:.0f}, "
          f"dark {render['dark_fraction'] * 100:.0f}%, bright {render['bright_fraction'] * 100:.0f}%")

    rows = compare(ref, render)
    render_temporal = None
    if args.temporal:
        render_temporal = measure_sequence(collect_frames(args.render), column_width, 1.0 / args.fps, crop,
                                           with_video=args.video, mode=mode)
        rows.extend(sequence.compare_temporal(ref_temporal, render_temporal, ratio_row))
        if args.video:
            rows.extend(video.compare_video(ref_temporal["video"], render_temporal["video"]))
    rows = score_rows(rows)
    print(f"\n{'gate':<26}{'ref':>6}{'render':>8}  {'value':<14}{'status':<6}score")
    for name, rv, cv, value, ok, score in rows:
        status = "-" if ok is None else ("PASS" if ok else "FAIL")
        print(f"{name:<26}{rv:>6}{cv:>8}  {value:<14}{status:<6}{score:.2f}")
    print_summary(rows)
    print_profiles(ref, render)
    if render_temporal is not None:
        sequence.print_temporal(ref_temporal, render_temporal)
        if args.video:
            video.print_video(ref_temporal["video"], render_temporal["video"])

    if args.json:
        Path(args.json).write_text(json.dumps({"reference": ref, "render": render,
                                               "reference_temporal": ref_temporal, "render_temporal": render_temporal,
                                               "gates": [{"name": r[0], "value": r[3], "pass": r[4], "score": r[5]} for r in rows]}, indent=2))


if __name__ == "__main__":
    main()
