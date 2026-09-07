"""Capture the wind tornado reference-matching arms and prepare them for analysis.

Each arm pairs a reference footage directory with a camera. For every arm the tool captures a
background (wind density set to 0), captures a frame sequence that starts at the arm's wind time,
then crops both by the detected viewport and subtracts the background (clamp 0) into `<arm>_prep/`
next to the engine written meta.json.

The engine advances wind time by 1/60 s per batch frame, so the sequence starts at
`wind_time_start` by rendering `wind_time_start * 60` frames before the first capture, and a stride
of 2 yields 30 fps.

    uv run --with numpy --with pillow --with scipy python3 tools/wind_refmatch.py [--dood] [--arm far]

Exit code 0 = every requested arm was captured and preprocessed.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).resolve().parent))
from engine_harness import dood_wrap, engine_env, engine_path, repo_root

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
import flame_ref_match as flame
from wind_ref_match import direction_distance, measure_wind, transport_distance

flame.silhouette_mode = "dust"

SCENE = "assets/scenes/wind_probe.scene.ron"
BACKGROUND_FRAMES = 60
BATCH_FRAMES_PER_SECOND = 60

REFERENCE_CONFIGS = {
    "wall_outside": {
        "reference_dir": "assets/textures/wind/storm_ref_seq/shot_22_46.18s",
        "ref_window": (114, None),
        "camera": "80,10,2,0,1.0,0",
        "wind_time_start": 1.25,
        "frames": 40,
        "stride": 2,
        "resample": True,
    },
    "inside_tunnel": {
        "reference_dir": "assets/textures/wind/storm_ref_seq/shot_18_41.78s",
        "ref_window": None,
        "camera": "0,-85,1.2,0,1.5,0",
        "wind_time_start": 1.25,
        "frames": 40,
        "stride": 2,
        "resample": False,
    },
    "far": {
        "reference_dir": "assets/textures/wind/castle_ref_seq",
        "ref_window": None,
        "camera": "80,25,4,0,0.3,0",
        "wind_time_start": 1.25,
        "frames": 40,
        "stride": 1,
        "resample": True,
    },
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Wind reference matching capture tool.")
    parser.add_argument("--out-dir", default="target/tmp_screens/wind_refmatch",
                        help="output directory")
    parser.add_argument("--dood", action="store_true",
                        help="run the engine through the docker harness")
    parser.add_argument("--skip-capture", action="store_true",
                        help="preprocess the sequences already present in the output directory")
    parser.add_argument("--arm", nargs="+", choices=list(REFERENCE_CONFIGS),
                        default=list(REFERENCE_CONFIGS),
                        help="arms to capture (default: all)")
    parser.add_argument("--wind-set", action="append", default=[],
                        help="wind set values for candidate arm capture (can be repeated)")
    parser.add_argument("--candidate", default="candidate",
                        help="name of the candidate arm (default: candidate)")
    return parser.parse_args()


def run_engine(command: list[str], label: str, dood: bool) -> None:
    """Run one engine batch command and fail on anything but an ok report."""
    if dood:
        command = dood_wrap(command)
    print(f"[wind_refmatch] {label}: {' '.join(command)}", file=sys.stderr)

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


def capture_background(out_dir: Path, arm: str, config: dict, dood: bool,
                       wind_set: list[str]) -> None:
    """Capture the arm's camera with wind density set to 0."""
    command = [
        str(engine_path()),
        "--batch-screenshot", str(background_path(out_dir, arm)),
        "--batch-scene", SCENE,
        "--batch-camera", config["camera"],
        "--batch-wind-set", "density=0",
        "--batch-frames", str(BACKGROUND_FRAMES),
    ]
    run_engine(command, f"background {arm}", dood)


def capture_sequence(out_dir: Path, arm: str, config: dict, dood: bool,
                     wind_set: list[str]) -> None:
    """Capture the arm's frame sequence starting at its wind time."""
    start_frame = round(config["wind_time_start"] * BATCH_FRAMES_PER_SECOND)
    sequence = f"{out_dir / arm},{config['frames']},{config['stride']}"
    command = [
        str(engine_path()),
        "--batch-screenshot-sequence", sequence,
        "--batch-scene", SCENE,
        "--batch-camera", config["camera"],
        "--batch-frames", str(start_frame),
    ]
    for value in wind_set:
        command.extend(["--batch-wind-set", value])
    run_engine(command, f"sequence {arm}", dood)


def capture_all(out_dir: Path, arms: list[str], dood: bool, candidate: str,
                wind_set: list[str]) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    for arm in arms:
        config = REFERENCE_CONFIGS[arm]
        capture_background(out_dir, arm, config, dood, [])
        capture_sequence(out_dir, arm, config, dood, [])
        if wind_set:
            candidate_arm = f"{arm}_{candidate}"
            capture_background(out_dir, candidate_arm, config, dood, wind_set)
            capture_sequence(out_dir, candidate_arm, config, dood, wind_set)


def detect_viewport_from_background(background_png: Path) -> tuple[int, int, int, int]:
    """Bounding box of the 3D viewport from the largest connected component in the background."""
    gray = np.asarray(Image.open(background_png).convert("RGB"), dtype=np.float32).mean(axis=2)
    clear_value = np.bincount(gray.astype(np.uint8).ravel()).argmax()
    mask = np.abs(gray - float(clear_value)) > 2.0
    mask = ndimage.binary_opening(mask, iterations=3)

    labeled, num_features = ndimage.label(mask)
    if num_features == 0:
        raise SystemExit(f"viewport detection failed: {background_png}")

    component_sizes = np.bincount(labeled.ravel())
    largest = int(np.argmax(component_sizes[1:]) + 1)
    rows, cols = np.where(labeled == largest)
    return int(cols.min()), int(rows.min()), int(cols.max()) + 1, int(rows.max()) + 1


def preprocess_sequences(out_dir: Path, arms: list[str], candidate: str) -> None:
    """Crop frames by the detected viewport and subtract the background (clamp 0)."""
    for arm in arms:
        preprocess_capture(out_dir, arm, arm)
        candidate_arm = f"{arm}_{candidate}"
        if (out_dir / candidate_arm).is_dir():
            preprocess_capture(out_dir, candidate_arm, arm)


def preprocess_capture(out_dir: Path, capture: str, arm: str) -> None:
    src_dir = out_dir / capture
    prep_dir = out_dir / f"{capture}_prep"
    prep_dir.mkdir(parents=True, exist_ok=True)

    background_png = background_path(out_dir, capture)
    if not background_png.is_file():
        raise SystemExit(f"background not found for {capture}: {background_png}")

    x0, y0, x1, y1 = detect_viewport_from_background(background_png)
    print(f"[wind_refmatch] {capture} viewport: {x0},{y0},{x1},{y1}", file=sys.stderr)

    with Image.open(background_png) as image:
        background = np.asarray(image.convert("RGB"), dtype=np.int16)[y0:y1, x0:x1]

    frames = sorted(src_dir.glob("frame_*.png"))
    if not frames:
        raise SystemExit(f"no frames captured for {capture}: {src_dir}")
    for frame_path in frames:
        with Image.open(frame_path) as image:
            frame = np.asarray(image.convert("RGB"), dtype=np.int16)[y0:y1, x0:x1]
        subtracted = np.clip(frame - background, 0, 255).astype(np.uint8)
        Image.fromarray(subtracted).save(prep_dir / frame_path.name)

    meta_src = src_dir / "meta.json"
    if meta_src.is_file():
        shutil.copy2(meta_src, prep_dir / "meta.json")
    else:
        fps = BATCH_FRAMES_PER_SECOND / REFERENCE_CONFIGS[arm]["stride"]
        (prep_dir / "meta.json").write_text(json.dumps({"fps": fps}))

    print(f"[wind_refmatch] {capture}: preprocessed {len(frames)} frames into {prep_dir}",
          file=sys.stderr)


GATED_SEPARATION = 0.5
GAP_CLOSED_PASS = 0.5
MATCH_PASS = 0.6


def contrast_distance(left: dict, right: dict) -> float:
    log_ratio = np.log2(np.array(left["contrast_spectrum"], dtype=np.float64)
                        / np.array(right["contrast_spectrum"], dtype=np.float64))
    finite = np.isfinite(log_ratio)
    if not finite.any():
        return float("nan")
    return float(np.sqrt(np.mean(log_ratio[finite] ** 2)))


def bright_structure_distance(left: dict, right: dict) -> float:
    return float(abs(left["bright_largest_share"] - right["bright_largest_share"])
                 + abs(left["bright_fragments_per_k"] - right["bright_fragments_per_k"]))


FAMILY_DISTANCES = {
    "W1": contrast_distance,
    "W2": bright_structure_distance,
    "W4": lambda left, right: direction_distance(left["w4"], right["w4"]),
    "W5": lambda left, right: transport_distance(left["w5"], right["w5"]),
}


def split_quarters(paths: list[Path]) -> list[list[Path]]:
    bounds = [round(len(paths) * i / 4) for i in range(5)]
    return [paths[bounds[i]:bounds[i + 1]] for i in range(4)]


def reference_frames(config: dict) -> tuple[list[Path], float]:
    """Reference frames with the arm's ref_window applied and caption frames dropped."""
    ref_dir = repo_root() / config["reference_dir"]
    ref_fps, caption_frames = flame.load_ref_meta(ref_dir)
    paths = flame.collect_frames(ref_dir)

    first, end = config["ref_window"] or (0, None)
    end = len(paths) if end is None else end
    return [p for i, p in enumerate(paths[first:end], first) if i not in caption_frames], ref_fps


def score_candidate(candidate_measured: dict, ref_measured: dict,
                    ceilings: dict, floors: dict, gated: list[str]) -> dict:
    """Closed fraction of the floor-to-ceiling gap and the ceiling relative score per family."""
    distances, gap_closed, scores = {}, {}, {}
    for family, distance_of in FAMILY_DISTANCES.items():
        d_arm = distance_of(candidate_measured, ref_measured)
        ceiling = ceilings[family]
        distances[family] = d_arm
        gap_closed[family] = (floors[family] - d_arm) / max(floors[family] - ceiling, 1e-6)
        scores[family] = 1.0 / (1.0 + max(0.0, d_arm / max(ceiling, 1e-6) - 1.0))

    match = float(np.mean([scores[family] for family in gated])) if gated else 0.0
    passed = all(gap_closed[family] >= GAP_CLOSED_PASS for family in gated) and match >= MATCH_PASS
    return {"d": distances, "gap_closed": gap_closed, "score": scores,
            "match": match, "pass": bool(gated) and passed}


def analyze_arm(out_dir: Path, arm: str, candidate: str, wind_set: list[str]) -> dict:
    """Distance of the arm's prep sequence to the reference against the reference's own spread."""
    config = REFERENCE_CONFIGS[arm]

    prep_dir = out_dir / f"{arm}_prep"
    floor_paths = flame.collect_frames(prep_dir)
    if not floor_paths:
        raise SystemExit(f"no prep frames found for {arm}: {prep_dir}")

    ref_paths, ref_fps = reference_frames(config)
    if len(ref_paths) < 8:
        raise SystemExit(f"reference sequence too short for {arm}: {len(ref_paths)} frames")

    column_width = flame.reference_column_width(ref_paths)
    floor_fps = BATCH_FRAMES_PER_SECOND / config["stride"]
    floor_measured = measure_wind(floor_paths, column_width, floor_fps, resample=config["resample"])
    ref_measured = measure_wind(ref_paths, column_width, ref_fps, resample=False)
    quarters = [measure_wind(paths, column_width, ref_fps, resample=False)
                for paths in split_quarters(ref_paths)]

    ceilings, floors, separations, gated = {}, {}, {}, []
    for family, distance_of in FAMILY_DISTANCES.items():
        pairs = [distance_of(quarters[i], quarters[j])
                 for i in range(len(quarters)) for j in range(i + 1, len(quarters))]
        ceiling = float(np.median(pairs))
        d_floor = distance_of(floor_measured, ref_measured)
        separation = (d_floor - ceiling) / max(ceiling, 1e-6)

        ceilings[family] = ceiling
        floors[family] = d_floor
        separations[family] = separation
        if separation > GATED_SEPARATION:
            gated.append(family)

    result = {"ceiling": ceilings, "floor": floors, "separation": separations, "gated": gated}

    candidate_prep = out_dir / f"{arm}_{candidate}_prep"
    if candidate_prep.is_dir():
        candidate_measured = measure_wind(flame.collect_frames(candidate_prep), column_width,
                                          floor_fps, resample=config["resample"])
        result["candidate"] = {
            "name": candidate,
            "wind_set": wind_set,
            **score_candidate(candidate_measured, ref_measured, ceilings, floors, gated),
        }

    return result


def main() -> None:
    args = parse_args()
    out_dir = Path(args.out_dir)
    if not out_dir.is_absolute():
        out_dir = repo_root() / out_dir

    if not args.skip_capture:
        capture_all(out_dir, args.arm, args.dood, args.candidate, args.wind_set)
    preprocess_sequences(out_dir, args.arm, args.candidate)

    arms = {}
    for arm in args.arm:
        arms[arm] = analyze_arm(out_dir, arm, args.candidate, args.wind_set)
        arm_json = out_dir / f"{arm}_n0.json"
        arm_json.write_text(json.dumps({"ok": True, "arms": {arm: arms[arm]}}))
        print(f"[wind_refmatch] {arm}: wrote {arm_json}", file=sys.stderr)

    print(json.dumps({"ok": True, "arms": arms}))


if __name__ == "__main__":
    main()
