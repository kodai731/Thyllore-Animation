"""Capture the lightning reference-matching arms and prepare them for analysis.

Each arm pairs a reference footage directory with a lightning preset. For every arm the tool
captures a background (core_intensity=0, glow_intensity=0) used to detect the viewport, then
captures the frame sequence three times: shaded colour into `<capture>/color/`, the coverage
debug view into `<capture>/coverage/`, and the core debug view into `<capture>/core/`.

The engine advances time by 1/60 s per batch frame, so the sequence starts at `time_start`
by rendering `round(time_start * 60)` frames before the first capture, and a stride of 2
yields 30 fps.

    uv run --with numpy --with pillow --with scipy --with opencv-python-headless \
        python3 tools/lightning_refmatch.py [--dood] [--arm bolt]

Exit code 0 = every requested arm was captured and analyzed.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
from engine_harness import dood_wrap, engine_env, engine_path, repo_root

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
from lightning_ref_extract import compute_masks
from lightning_ref_match import (
    compute_ceiling,
    describe_sequence,
    family_distances,
    FAMILIES,
    load_reference,
    read_color,
    VERDICT_FAMILIES,
)

SCENE = "assets/scenes/lightning_probe.scene.ron"
BACKGROUND_FRAMES = 60
BATCH_FRAMES_PER_SECOND = 60

REFERENCE_CONFIGS = {
    "bolt": {
        "reference_dir": "assets/textures/lightning/reydau_bolt_ref_seq",
        "preset": "bolt",
        "time_start": 0.05,
        "frames": 13,
        "stride": 2,
    },
    "charge": {
        "reference_dir": "assets/textures/lightning/reydau_charge_ref_seq",
        "preset": "charge",
        "time_start": 0.0,
        "frames": 37,
        "stride": 2,
    },
    "beam": {
        "reference_dir": "assets/textures/lightning/reydau_beam_ref_seq",
        "preset": "beam",
        "time_start": 0.0,
        "frames": 12,
        "stride": 2,
    },
    "arc": {
        "reference_dir": "assets/textures/lightning/thor_arc_ref_seq",
        "preset": "arc",
        "time_start": 0.0,
        "frames": 11,
        "stride": 2,
    },
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Lightning reference matching capture tool.")
    parser.add_argument("--out-dir", default="target/tmp_screens/lightning_refmatch",
                        help="output directory")
    parser.add_argument("--dood", action="store_true",
                        help="run the engine through the docker harness")
    parser.add_argument("--skip-capture", action="store_true",
                        help="analyze the sequences already present in the output directory")
    parser.add_argument("--arm", nargs="+", choices=list(REFERENCE_CONFIGS),
                        default=list(REFERENCE_CONFIGS),
                        help="arms to capture (default: all)")
    parser.add_argument("--lightning-set", action="append", default=[],
                        help="lightning set values for candidate arm capture (can be repeated)")
    parser.add_argument("--candidate", default="candidate",
                        help="name of the candidate arm (default: candidate)")
    return parser.parse_args()


def run_engine(command: list[str], label: str, dood: bool) -> None:
    """Run one engine batch command and fail on anything but an ok report."""
    if dood:
        command = dood_wrap(command)
    print(f"[lightning_refmatch] {label}: {' '.join(command)}", file=sys.stderr)

    proc = subprocess.run(
        command, capture_output=True, text=True, timeout=1800,
        cwd=str(repo_root()), env=engine_env(),
    )
    last_line = proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else ""
    report = json.loads(last_line) if last_line.startswith("{") else {"ok": False, "error": "no JSON"}
    if not report.get("ok"):
        raise SystemExit(f"{label} failed: {report.get('error')}\n{proc.stderr[-2000:]}")


def background_path(out_dir: Path, arm: str) -> Path:
    return out_dir / f"{arm}_background.png"


def capture_background(out_dir: Path, arm: str, dood: bool) -> None:
    """Capture the scene with lightning core_intensity=0 and glow_intensity=0."""
    command = [
        str(engine_path()),
        "--batch-screenshot", str(background_path(out_dir, arm)),
        "--batch-scene", SCENE,
        "--batch-lightning-set", "core_intensity=0",
        "--batch-lightning-set", "glow_intensity=0",
        "--batch-frames", str(BACKGROUND_FRAMES),
    ]
    run_engine(command, f"background {arm}", dood)


def capture_sequence(out_dir: Path, config: dict, dood: bool,
                     lightning_set: list[str], capture_name: str) -> None:
    """Capture the arm's color, coverage, and core sequences starting at its time."""
    start_frame = round(config["time_start"] * BATCH_FRAMES_PER_SECOND)

    for subdir, debug_view in (("color", None), ("coverage", "coverage"), ("core", "core")):
        output_dir = out_dir / capture_name / subdir
        sequence_spec = f"{output_dir},{config['frames']},{config['stride']}"

        command = [
            str(engine_path()),
            "--batch-screenshot-sequence", sequence_spec,
            "--batch-scene", SCENE,
            "--batch-lightning-preset", config["preset"],
            "--batch-lightning-set", "burst_jitter=0",
            "--batch-frames", str(start_frame),
        ]
        if debug_view:
            command.extend(["--batch-lightning-debug-view", debug_view])
        for value in lightning_set:
            command.extend(["--batch-lightning-set", value])
        run_engine(command, f"sequence {capture_name} {subdir}", dood)


def capture_all(out_dir: Path, arms: list[str], dood: bool, candidate: str,
                lightning_set: list[str]) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    for arm in arms:
        config = REFERENCE_CONFIGS[arm]
        capture_background(out_dir, arm, dood)
        capture_sequence(out_dir, config, dood, [], arm)
        if lightning_set:
            capture_sequence(out_dir, config, dood, lightning_set, f"{arm}_{candidate}")


CHROME_COVERAGE_RATIO = 0.95


def detect_viewport_from_background(background_png: Path) -> tuple[int, int, int, int]:
    """Bounding box of the 3D viewport from the chrome gaps containing the image center."""
    gray = np.asarray(Image.open(background_png).convert("RGB"), dtype=np.float32).mean(axis=2)
    clear_value = np.bincount(gray.astype(np.uint8).ravel()).argmax()
    mask = np.abs(gray - float(clear_value)) > 2.0
    height, width = mask.shape

    row_runs = _collect_chrome_runs(mask.sum(axis=1), CHROME_COVERAGE_RATIO * width)
    y0, y1 = _find_widest_interior(row_runs, height, height // 2)

    band_height = y1 - y0
    col_runs = _collect_chrome_runs(mask[y0:y1].sum(axis=0), CHROME_COVERAGE_RATIO * band_height)
    x0, x1 = _find_widest_interior(col_runs, width, width // 2)

    if band_height <= 0 or x1 - x0 <= 0:
        raise SystemExit(f"viewport detection failed: {background_png}")
    return x0, y0, x1, y1


def _collect_chrome_runs(counts: np.ndarray, threshold: float) -> list[tuple[int, int]]:
    """Runs of indices where counts >= threshold, as (start, end) pairs."""
    indices = np.where(counts >= threshold)[0]
    if len(indices) == 0:
        return []
    runs: list[tuple[int, int]] = []
    start = int(indices[0])
    prev = start
    for i in indices[1:]:
        i = int(i)
        if i == prev + 1:
            prev = i
        else:
            runs.append((start, prev + 1))
            start = i
            prev = i
    runs.append((start, prev + 1))
    return runs


def _find_widest_interior(runs: list[tuple[int, int]], length: int, center: int) -> tuple[int, int]:
    """Widest chrome-free span, preferring the one that contains center."""
    gaps: list[tuple[int, int]] = []
    cursor = 0
    for start, end in runs:
        if start > cursor:
            gaps.append((cursor, start))
        cursor = max(cursor, end)
    if cursor < length:
        gaps.append((cursor, length))
    if not gaps:
        return 0, length

    centered = [gap for gap in gaps if gap[0] <= center < gap[1]]
    return max(centered or gaps, key=lambda gap: gap[1] - gap[0])


OVERLAY_LUMINANCE_THRESHOLD = 200.0


def load_background_gray(background_png: Path,
                         viewport: tuple[int, int, int, int]) -> np.ndarray:
    """Viewport-cropped grayscale of the arm's background capture."""
    x0, y0, x1, y1 = viewport
    gray = np.asarray(Image.open(background_png).convert("L"), dtype=np.float32)

    return gray[y0:y1, x0:x1]


def load_render_fields(capture_dir: Path, config: dict, viewport: tuple[int, int, int, int],
                       background_gray: np.ndarray) -> dict:
    """Viewport-cropped render frames masked like the reference, for lightning_ref_match."""
    if not capture_dir.is_dir():
        raise SystemExit(f"no frames captured: {capture_dir}")

    color_paths = sorted((capture_dir / "color").glob("frame_*.png"))
    if not color_paths:
        raise SystemExit(f"no colour frames captured: {capture_dir / 'color'}")

    x0, y0, x1, y1 = viewport
    static_mask = background_gray > OVERLAY_LUMINANCE_THRESHOLD

    glow_frames, core_frames, color_frames = [], [], []
    for path in color_paths:
        bgr = read_color(path)[y0:y1, x0:x1]
        glow, core = compute_masks(bgr, background_gray, strict=False)

        glow_frames.append(glow & ~static_mask)
        core_frames.append(core & ~static_mask)
        color_frames.append(bgr)

    return {
        "glow": glow_frames,
        "core": core_frames,
        "color": color_frames,
        "fps": BATCH_FRAMES_PER_SECOND / config["stride"],
    }


def analyze_arm(out_dir: Path, arm: str, candidate: str, lightning_set: list[str]) -> dict:
    """Distance of the arm's rendered sequence to the reference against the reference's own spread."""
    config = REFERENCE_CONFIGS[arm]

    background_png = background_path(out_dir, arm)
    viewport = detect_viewport_from_background(background_png)
    background_gray = load_background_gray(background_png, viewport)
    floor_fields = load_render_fields(out_dir / arm, config, viewport, background_gray)
    ref = load_reference(repo_root() / config["reference_dir"])

    ref_desc = describe_sequence(ref, ref["fps"])
    ceiling = compute_ceiling(ref, ref["fps"])

    distances = family_distances(ref_desc, describe_sequence(floor_fields, floor_fields["fps"]))
    scores = {family: float(distances[family] / ceiling[family]) for family in FAMILIES}

    result = {"ceiling": ceiling, "floor": distances, "score": scores,
              "pass": {family: bool(scores[family] <= 1.0) for family in VERDICT_FAMILIES}}

    candidate_dir = out_dir / f"{arm}_{candidate}"
    if candidate_dir.is_dir():
        candidate_fields = load_render_fields(candidate_dir, config, viewport, background_gray)
        candidate_distances = family_distances(
            ref_desc, describe_sequence(candidate_fields, candidate_fields["fps"]))
        candidate_scores = {family: float(candidate_distances[family] / ceiling[family])
                            for family in FAMILIES}
        result["candidate"] = {
            "name": candidate,
            "lightning_set": lightning_set,
            "distance": candidate_distances,
            "score": candidate_scores,
            "pass": {family: bool(candidate_scores[family] <= 1.0) for family in VERDICT_FAMILIES},
        }

    return result


def main() -> None:
    args = parse_args()
    out_dir = Path(args.out_dir)
    if not out_dir.is_absolute():
        out_dir = repo_root() / out_dir

    if not args.skip_capture:
        capture_all(out_dir, args.arm, args.dood, args.candidate, args.lightning_set)

    arms = {}
    for arm in args.arm:
        arms[arm] = analyze_arm(out_dir, arm, args.candidate, args.lightning_set)
        arm_json = out_dir / f"{arm}.json"
        arm_json.write_text(json.dumps({"ok": True, "arms": {arm: arms[arm]}}, indent=2))
        print(f"[lightning_refmatch] {arm}: wrote {arm_json}", file=sys.stderr)

    verdict = all(all(result.get("candidate", result)["pass"].values()) for result in arms.values())
    print(json.dumps({"ok": verdict, "arms": arms, "verdict": "pass" if verdict else "fail"}))


if __name__ == "__main__":
    main()
