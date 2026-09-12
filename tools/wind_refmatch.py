"""Capture the wind tornado reference-matching arms and prepare them for analysis.

Each arm pairs a reference footage directory with a camera. For every arm the tool captures a
background (wind density set to 0) used to detect the viewport, then captures the frame sequence
that starts at the arm's wind time twice: the shaded colour into `<capture>/color/` and the wind
coverage debug view into `<capture>/coverage/`, which supplies the mask the shape measures need.

The engine advances wind time by 1/60 s per batch frame, so the sequence starts at
`wind_time_start` by rendering `wind_time_start * 60` frames before the first capture, and a stride
of 2 yields 30 fps.

    uv run --with numpy --with pillow --with scipy python3 tools/wind_refmatch.py [--dood] [--arm far]

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
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).resolve().parent))
from engine_harness import dood_wrap, engine_env, engine_path, repo_root

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
import flame_ref_match as flame
from wind_ref_match import measure_shape, s1_distance, s2_distance, s3_distance

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
    },
    "inside_tunnel": {
        "reference_dir": "assets/textures/wind/storm_ref_seq/shot_18_41.78s",
        "ref_window": None,
        "camera": "0,-85,1.2,0,1.5,0",
        "wind_time_start": 1.25,
        "frames": 40,
        "stride": 2,
    },
    "far": {
        "reference_dir": "assets/textures/wind/castle_ref_seq",
        "ref_window": None,
        "camera": "80,25,4,0,0.3,0",
        "wind_time_start": 1.25,
        "frames": 40,
        "stride": 1,
    },
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Wind reference matching capture tool.")
    parser.add_argument("--out-dir", default="target/tmp_screens/wind_refmatch",
                        help="output directory")
    parser.add_argument("--dood", action="store_true",
                        help="run the engine through the docker harness")
    parser.add_argument("--skip-capture", action="store_true",
                        help="analyze the sequences already present in the output directory")
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


def capture_sequence(out_dir: Path, config: dict, dood: bool,
                     wind_set: list[str], capture_name: str) -> None:
    """Capture the arm's colour and coverage sequences starting at its wind time."""
    start_frame = round(config["wind_time_start"] * BATCH_FRAMES_PER_SECOND)

    color_dir = out_dir / capture_name / "color"
    color_sequence = f"{color_dir},{config['frames']},{config['stride']}"
    command = [
        str(engine_path()),
        "--batch-screenshot-sequence", color_sequence,
        "--batch-scene", SCENE,
        "--batch-camera", config["camera"],
        "--batch-frames", str(start_frame),
    ]
    for value in wind_set:
        command.extend(["--batch-wind-set", value])
    run_engine(command, f"sequence {capture_name} color", dood)

    coverage_dir = out_dir / capture_name / "coverage"
    coverage_sequence = f"{coverage_dir},{config['frames']},{config['stride']}"
    command = [
        str(engine_path()),
        "--batch-screenshot-sequence", coverage_sequence,
        "--batch-scene", SCENE,
        "--batch-camera", config["camera"],
        "--batch-frames", str(start_frame),
        "--batch-wind-debug-view", "coverage",
    ]
    for value in wind_set:
        command.extend(["--batch-wind-set", value])
    run_engine(command, f"sequence {capture_name} coverage", dood)


def capture_all(out_dir: Path, arms: list[str], dood: bool, candidate: str,
                wind_set: list[str]) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    for arm in arms:
        config = REFERENCE_CONFIGS[arm]
        capture_background(out_dir, arm, config, dood, [])
        capture_sequence(out_dir, config, dood, [], arm)
        if wind_set:
            candidate_arm = f"{arm}_{candidate}"
            capture_background(out_dir, candidate_arm, config, dood, wind_set)
            capture_sequence(out_dir, config, dood, wind_set, candidate_arm)


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


COVERAGE_MASK_LEVEL = 25
REFERENCE_MASK_LEVEL = 127


def load_render_fields(color_dir: Path, coverage_dir: Path,
                       viewport: tuple[int, int, int, int]) -> list[tuple[np.ndarray, np.ndarray]]:
    """Viewport cropped luminance paired with the coverage mask, one entry per captured frame."""
    x0, y0, x1, y1 = viewport
    color_paths = sorted(color_dir.glob("frame_*.png"))
    coverage_paths = sorted(coverage_dir.glob("frame_*.png"))
    if not color_paths:
        raise SystemExit(f"no frames captured: {color_dir}")
    if len(color_paths) != len(coverage_paths):
        raise SystemExit(f"color/coverage frame count differs: {len(color_paths)} vs {len(coverage_paths)}")

    fields = []
    for color_path, coverage_path in zip(color_paths, coverage_paths):
        with Image.open(color_path) as image:
            color = np.asarray(image.convert("RGB"), dtype=np.float64)[y0:y1, x0:x1]
        with Image.open(coverage_path) as image:
            coverage = np.asarray(image.convert("RGB"), dtype=np.float64)[y0:y1, x0:x1]
        fields.append((flame.luminance(color).astype(np.float32),
                       flame.luminance(coverage) > COVERAGE_MASK_LEVEL))
    return fields


def load_reference_fields(reference_dir: Path,
                          ref_window: tuple[int, int | None] | None
                          ) -> list[tuple[np.ndarray, np.ndarray]]:
    """Reference luminance paired with the mask_NNN.png written next to each frame."""
    frame_paths = sorted(reference_dir.glob("frame_*.png"))
    first, end = ref_window or (0, None)
    end = len(frame_paths) if end is None else end

    fields = []
    for frame_path in frame_paths[first:end]:
        mask_path = frame_path.with_name(frame_path.name.replace("frame_", "mask_"))
        if not mask_path.is_file():
            raise SystemExit(f"reference mask missing: {mask_path} "
                             "(run scripts/wind_ref_extract.py masks)")
        with Image.open(frame_path) as image:
            rgb = np.asarray(image.convert("RGB"), dtype=np.float64)
        with Image.open(mask_path) as image:
            mask = np.asarray(image.convert("L"))
        fields.append((flame.luminance(rgb).astype(np.float32), mask > REFERENCE_MASK_LEVEL))
    return fields


GAP_CLOSED_PASS = 0.5
BLUR_SIGMA = 8.0
BLUR_GAP_FRACTION = 0.5
S2_ONLY_ARMS = {"far"}
S2_ONLY_FAMILIES = ["S2"]

FAMILY_DISTANCES = {
    "S1": lambda a, b: s1_distance(a["s1"], b["s1"]),
    "S2": lambda a, b: s2_distance(a["s2"], b["s2"]),
    "S3": lambda a, b: s3_distance(a["s3"], b["s3"]),
}


def split_quarters(items: list) -> list[list]:
    bounds = [round(len(items) * i / 4) for i in range(5)]
    return [items[bounds[i]:bounds[i + 1]] for i in range(4)]


def score_candidate(candidate_measured: dict, ref_measured: dict,
                    ceilings: dict, floors: dict, gated: list[str]) -> dict:
    """Closed fraction of the floor-to-ceiling gap and the ceiling relative score per family."""
    distances, gap_closed, scores = {}, {}, {}
    for family, ceiling in ceilings.items():
        d_arm = FAMILY_DISTANCES[family](candidate_measured, ref_measured)
        distances[family] = d_arm
        gap_closed[family] = (floors[family] - d_arm) / max(floors[family] - ceiling, 1e-6)
        scores[family] = 1.0 / (1.0 + max(0.0, d_arm / max(ceiling, 1e-6) - 1.0))

    match = float(np.mean([scores[family] for family in gated])) if gated else 0.0
    passed = all(gap_closed[family] >= GAP_CLOSED_PASS for family in gated)
    return {"d": distances, "gap_closed": gap_closed, "score": scores,
            "match": match, "pass": bool(gated) and passed}


def blurred_fields(fields: list[tuple[np.ndarray, np.ndarray]],
                   sigma: float) -> list[tuple[np.ndarray, np.ndarray]]:
    """Gaussian blurred luminance paired with the blurred mask thresholded back to binary."""
    return [(ndimage.gaussian_filter(lum, sigma).astype(np.float32),
             ndimage.gaussian_filter(mask.astype(np.float32), sigma) > 0.5)
            for lum, mask in fields]


def field_preview(field: tuple[np.ndarray, np.ndarray]) -> Image.Image:
    """Normalized luminance with the mask boundary drawn in red."""
    lum, mask = field
    low, high = float(lum.min()), float(lum.max())
    scaled = (lum - low) / max(high - low, 1e-6) * 255.0
    rgb = np.repeat(np.clip(scaled, 0.0, 255.0).astype(np.uint8)[:, :, None], 3, axis=2)

    boundary = mask ^ ndimage.binary_erosion(mask)
    rgb[boundary] = (255, 0, 0)
    return Image.fromarray(rgb)


def write_contact_sheet(out_dir: Path, arm: str,
                        ref_fields: list[tuple[np.ndarray, np.ndarray]],
                        floor_fields: list[tuple[np.ndarray, np.ndarray]],
                        candidate_fields: list[tuple[np.ndarray, np.ndarray]] | None) -> Path:
    """Middle frame of each sequence side by side, reference first."""
    sequences = [ref_fields, floor_fields]
    if candidate_fields:
        sequences.append(candidate_fields)

    previews = [field_preview(fields[len(fields) // 2]) for fields in sequences]
    width = sum(preview.width for preview in previews)
    height = max(preview.height for preview in previews)

    sheet = Image.new("RGB", (width, height))
    offset = 0
    for preview in previews:
        sheet.paste(preview, (offset, 0))
        offset += preview.width

    sheet_path = out_dir / f"{arm}_contact.png"
    sheet.save(sheet_path)
    return sheet_path


def analyze_arm(out_dir: Path, arm: str, candidate: str, wind_set: list[str]) -> dict:
    """Distance of the arm's rendered sequence to the reference against the reference's own spread."""
    config = REFERENCE_CONFIGS[arm]

    viewport = detect_viewport_from_background(background_path(out_dir, arm))
    floor_fields = load_render_fields(out_dir / arm / "color",
                                      out_dir / arm / "coverage",
                                      viewport)
    ref_fields = load_reference_fields(repo_root() / config["reference_dir"],
                                       config["ref_window"])

    if len(ref_fields) < 8:
        raise SystemExit(f"reference sequence too short for {arm}: {len(ref_fields)} frames")

    ref_measured = measure_shape(ref_fields)
    quarters = [measure_shape(quarter) for quarter in split_quarters(ref_fields)]
    floor_measured = measure_shape(floor_fields)

    blurred_measured = measure_shape(blurred_fields(ref_fields, BLUR_SIGMA))

    ceilings, floors, blur_gaps = {}, {}, {}
    for family, distance_of in FAMILY_DISTANCES.items():
        pairs = [distance_of(quarters[i], quarters[j])
                 for i in range(len(quarters)) for j in range(i + 1, len(quarters))]
        ceiling = float(np.median(pairs))

        ceilings[family] = ceiling
        floors[family] = distance_of(floor_measured, ref_measured)
        blur_gaps[family] = distance_of(blurred_measured, ref_measured) - ceiling

    families = S2_ONLY_FAMILIES if arm in S2_ONLY_ARMS else list(FAMILY_DISTANCES)
    gated = [family for family in families
             if blur_gaps[family] > BLUR_GAP_FRACTION * ceilings[family]]

    result = {"ceiling": ceilings, "floor": floors, "blur_gap": blur_gaps, "gated": gated}

    candidate_color_dir = out_dir / f"{arm}_{candidate}" / "color"
    candidate_coverage_dir = out_dir / f"{arm}_{candidate}" / "coverage"
    candidate_fields = None
    if candidate_color_dir.is_dir():
        candidate_fields = load_render_fields(candidate_color_dir, candidate_coverage_dir, viewport)
        result["candidate"] = {
            "name": candidate,
            "wind_set": wind_set,
            **score_candidate(measure_shape(candidate_fields), ref_measured,
                              ceilings, floors, gated),
        }

    result["contact_sheet"] = str(write_contact_sheet(out_dir, arm, ref_fields,
                                                      floor_fields, candidate_fields))
    return result


def main() -> None:
    args = parse_args()
    out_dir = Path(args.out_dir)
    if not out_dir.is_absolute():
        out_dir = repo_root() / out_dir

    if not args.skip_capture:
        capture_all(out_dir, args.arm, args.dood, args.candidate, args.wind_set)

    arms = {}
    for arm in args.arm:
        arms[arm] = analyze_arm(out_dir, arm, args.candidate, args.wind_set)
        arm_json = out_dir / f"{arm}_n0.json"
        arm_json.write_text(json.dumps({"ok": True, "arms": {arm: arms[arm]}}))
        print(f"[wind_refmatch] {arm}: wrote {arm_json}", file=sys.stderr)

    print(json.dumps({"ok": True, "arms": arms}))


if __name__ == "__main__":
    main()
