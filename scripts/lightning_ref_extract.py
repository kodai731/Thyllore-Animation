"""Extract the lightning reference sequences for scripts/flame_ref_match.py style gates.

usage:
  python scripts/lightning_ref_extract.py reydau_bolt|reydau_charge|reydau_beam|thor_arc|thor_sweep
      [--video <mp4>] [--out assets/textures/lightning/<name>_ref_seq] [--strict]
  python scripts/lightning_ref_extract.py all

The windows, crops and mask thresholds are the ones measured in
assets/textures/lightning/measure_lightning_masks.py: the luminance median of a quiet background
window is subtracted, glow keeps the bright white or cyan-blue pixels and core the near-white ones.
--strict tightens glow to the blue arc of the thor_* clips, whose sky is bright enough to leak.

Writes frame_NNN.png (source colour, glow mask off set to black), mask_NNN.png (glow 0/255),
core_NNN.png (core 0/255) and meta.json. Re-running overwrites the same files.

The clips are expected at assets/textures/lightning/src_videos; yt-dlp is never run, a missing
video is reported with the URL and the command that fetches it.
"""

import argparse
import json
from pathlib import Path

import cv2
import numpy as np
from scipy import ndimage

VIDEO_DIR = Path("assets/textures/lightning/src_videos")
VIDEO_URLS = {
    "BZcYJCYkaqc.mp4": "https://www.youtube.com/shorts/BZcYJCYkaqc",
    "T-smw3SQ698.mp4": "https://www.youtube.com/shorts/T-smw3SQ698",
}

SEQUENCES = {
    "reydau_bolt": ("BZcYJCYkaqc.mp4", 2.50, 2.92, (0, 150, 480, 700), 1.30, 1.45),
    "reydau_charge": ("BZcYJCYkaqc.mp4", 1.30, 2.50, (0, 150, 480, 700), 1.30, 1.45),
    "reydau_beam": ("BZcYJCYkaqc.mp4", 4.43, 4.80, (0, 150, 480, 700), 4.30, 4.45),
    "thor_arc": ("T-smw3SQ698.mp4", 14.00, 14.45, (0, 100, 480, 710), 14.26, 14.45),
    "thor_sweep": ("T-smw3SQ698.mp4", 12.93, 13.65, (0, 100, 480, 710), 13.35, 13.42),
}

GLOW_LUM, GLOW_DELTA = 110.0, 50.0
GLOW_SAT_MAX, GLOW_HUE = 70.0, (80.0, 125.0)
CORE_LUM, CORE_SAT_MAX = 235.0, 40.0

STRICT_DELTA = 80.0
STRICT_HUE = (95.0, 120.0)
STRICT_SAT_MIN = 30.0


def read_frames(video_path, t0, t1, crop):
    capture = cv2.VideoCapture(str(video_path))
    fps = capture.get(cv2.CAP_PROP_FPS)
    first, last = int(round(t0 * fps)), int(round(t1 * fps))
    capture.set(cv2.CAP_PROP_POS_FRAMES, first)
    indices, frames = [], []
    for index in range(first, last + 1):
        ok, bgr = capture.read()
        if not ok:
            break
        x0, y0, x1, y1 = crop
        indices.append(index)
        frames.append(bgr[y0:y1, x0:x1])
    capture.release()
    return fps, indices, frames


def compute_background(frames):
    grays = np.stack([cv2.cvtColor(bgr, cv2.COLOR_BGR2GRAY).astype(np.float32) for bgr in frames])
    return np.median(grays, axis=0)


def compute_masks(bgr, background, strict):
    lum = cv2.cvtColor(bgr, cv2.COLOR_BGR2GRAY).astype(np.float32)
    hsv = cv2.cvtColor(bgr, cv2.COLOR_BGR2HSV)
    hue, sat = hsv[..., 0].astype(np.float32), hsv[..., 1].astype(np.float32)
    delta = lum - background

    bluish = (hue >= GLOW_HUE[0]) & (hue <= GLOW_HUE[1])
    glow = (lum > GLOW_LUM) & (delta > GLOW_DELTA) & ((sat < GLOW_SAT_MAX) | bluish)
    if strict:
        arc = (hue >= STRICT_HUE[0]) & (hue <= STRICT_HUE[1])
        glow &= (delta > STRICT_DELTA) & arc & (sat > STRICT_SAT_MIN)
    glow = ndimage.binary_opening(glow, iterations=1)

    core = glow & (lum > CORE_LUM) & (sat < CORE_SAT_MAX)
    return glow, core


def apply_glow(bgr, glow):
    masked = bgr.copy()
    masked[~glow] = 0
    return masked


def build_thresholds(strict):
    thresholds = {
        "glow_lum": GLOW_LUM,
        "glow_delta": GLOW_DELTA,
        "glow_sat_max": GLOW_SAT_MAX,
        "glow_hue": list(GLOW_HUE),
        "core_lum": CORE_LUM,
        "core_sat_max": CORE_SAT_MAX,
        "opening": 3,
    }
    if strict:
        thresholds.update(
            {"strict_delta": STRICT_DELTA, "strict_hue": list(STRICT_HUE), "strict_sat_min": STRICT_SAT_MIN}
        )
    return thresholds


def write_sequence(out_dir, frames, masks, meta):
    out_dir.mkdir(parents=True, exist_ok=True)
    for old in out_dir.glob("frame_*.png"):
        old.unlink()
    for old in out_dir.glob("mask_*.png"):
        old.unlink()
    for old in out_dir.glob("core_*.png"):
        old.unlink()

    for index, (bgr, (glow, core)) in enumerate(zip(frames, masks)):
        cv2.imwrite(str(out_dir / f"frame_{index:03d}.png"), apply_glow(bgr, glow))
        cv2.imwrite(str(out_dir / f"mask_{index:03d}.png"), glow.astype(np.uint8) * 255)
        cv2.imwrite(str(out_dir / f"core_{index:03d}.png"), core.astype(np.uint8) * 255)
    (out_dir / "meta.json").write_text(json.dumps(meta, indent=1) + "\n")


def resolve_video(name, video_override):
    video_name = SEQUENCES[name][0]
    video_path = Path(video_override) if video_override else VIDEO_DIR / video_name
    if not video_path.exists():
        url = VIDEO_URLS[video_name]
        raise SystemExit(
            f"{video_path} does not exist. Download it first ({url}):\n"
            f'  yt-dlp -f "bv*[vcodec^=avc1][height<=1080]" -o {VIDEO_DIR / video_name} {url}'
        )
    return video_path


def extract_sequence(name, video_override, out_override, strict):
    video_name, t0, t1, crop, bg_t0, bg_t1 = SEQUENCES[name]
    video_path = resolve_video(name, video_override)
    out_dir = Path(out_override) if out_override else Path(f"assets/textures/lightning/{name}_ref_seq")

    fps, indices, frames = read_frames(video_path, t0, t1, crop)
    if not frames:
        raise SystemExit(f"{name}: no frames read from {video_path} at {t0}-{t1}s")
    _, _, quiet = read_frames(video_path, bg_t0, bg_t1, crop)
    if not quiet:
        raise SystemExit(f"{name}: no background frames read from {video_path} at {bg_t0}-{bg_t1}s")
    background = compute_background(quiet)

    masks = [compute_masks(bgr, background, strict) for bgr in frames]
    meta = {
        "source": f"youtube {video_name} {t0}-{t1}s",
        "source_video": video_path.name,
        "fps": fps,
        "source_frame_indices": indices,
        "frames": len(frames),
        "crop_in_source": list(crop),
        "background_window": [bg_t0, bg_t1],
        "strict": strict,
        "mask_threshold": build_thresholds(strict),
    }
    write_sequence(out_dir, frames, masks, meta)

    glow_ratios = [float(glow.mean()) for glow, _ in masks]
    core_ratios = [float(core.mean()) for _, core in masks]
    print(
        f"{name}: {len(frames)} frames at {fps:.3f} fps, glow ratio median {np.median(glow_ratios):.4f}, "
        f"core ratio median {np.median(core_ratios):.4f} -> {out_dir}"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("clip", choices=list(SEQUENCES) + ["all"])
    parser.add_argument("--video", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--strict", action="store_true", help="tighter blue-arc glow for the thor_* clips")
    args = parser.parse_args()

    if args.clip == "all":
        if args.video or args.out:
            raise SystemExit("--video and --out apply to a single sequence, not to all")
        for name in SEQUENCES:
            extract_sequence(name, None, None, args.strict and name.startswith("thor_"))
        return

    if args.strict and not args.clip.startswith("thor_"):
        raise SystemExit("--strict is defined for the thor_* sequences only")
    extract_sequence(args.clip, args.video, args.out, args.strict)


if __name__ == "__main__":
    main()
