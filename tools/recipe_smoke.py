"""Smoke test: bake each motion recipe and verify the output has real movement.

For each recipe (default: wave.json, hands_on_hips_tilt.json, bow.json from
crates/thyllore-avatar-core/tests/data/recipes/) this script:
  1. Creates a fresh output directory target/recipe_smoke/<recipe name>/
  2. Copies the test rig (mixamo.fbx.txt -> mixamo.fbx) into it
  3. Writes a scene.ron pointing at that rig
  4. Runs the engine once with --batch-anim-edit recipe=<path> --batch-play
     --batch-anim-dump --batch-anim-dump-tracks --batch-screenshot-sequence
  5. Checks the anim dump via check_anim_dump (pure function)
  6. Measures motion from the screenshot sequence via measure_motion (pure function)

    uv run --with numpy --with pillow python3 tools/recipe_smoke.py [--dood] [--recipe <path>]...

Exit code 0 = ran to completion (the JSON line on stdout holds ok/fail).
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

sys.path.insert(0, str(Path(__file__).resolve().parent))
from engine_harness import dood_wrap, engine_env, engine_path, repo_root

RECIPES_DIR = "crates/thyllore-avatar-core/tests/data/recipes"
RIG_SRC = "crates/thyllore-avatar-core/tests/data/rigs/mixamo.fbx.txt"
DEFAULT_RECIPES = ["wave.json", "hands_on_hips_tilt.json", "bow.json"]

MOTION_MIN_MEAN_ABS_DIFF = 0.5


def run_engine_once(out_dir: Path, scene_path: Path, recipe_path: Path, dood: bool) -> subprocess.CompletedProcess[str]:
    """Run the engine once with batch mode to bake the recipe and capture frames."""
    command = [
        str(engine_path()),
        "--batch-scene", str(scene_path),
        "--batch-anim-edit", f"recipe={recipe_path}",
        "--batch-play",
        "--batch-anim-dump", str(out_dir / "anim.json"),
        "--batch-anim-dump-tracks",
        "--batch-camera", "0,5,3.2,0,0.9,0",
        "--batch-screenshot-sequence", f"{out_dir}/seq,6,30",
        "--batch-frames", "1",
    ]
    if dood:
        command = dood_wrap(command)

    print(f"[recipe_smoke] engine {out_dir.name}", file=sys.stderr)
    return subprocess.run(
        command, capture_output=True, text=True, timeout=600,
        cwd=str(repo_root()), env=engine_env(),
    )


def check_anim_dump(dump: dict, recipe: dict) -> dict:
    """Pure function: validate the anim dump against the recipe.

    Returns {"clip_found", "scheduled", "missing_roles", "key_count_mismatches", "ok"}.
    """
    clip_name = recipe["name"]
    fps = recipe["fps"]
    duration_seconds = recipe["duration_seconds"]
    expected_key_count = round(fps * duration_seconds) + 1

    recipe_roles = {role for pose in recipe.get("poses", []) for role in pose.get("rotations", {})}

    matching_clip = next((clip for clip in dump.get("clips", []) if clip.get("name") == clip_name), None)
    if matching_clip is None or matching_clip.get("bone_track_count", 0) < 1:
        return {
            "clip_found": matching_clip is not None,
            "scheduled": False,
            "missing_roles": sorted(recipe_roles),
            "key_count_mismatches": [],
            "ok": False,
        }

    clip_id = matching_clip["id"]
    scheduled = any(
        entry.get("source_id") == clip_id
        for model in dump.get("models", [])
        for entry in model.get("schedule", [])
    )

    role_tracks = [track for track in matching_clip.get("bone_tracks", []) if track.get("role") in recipe_roles]
    missing_roles = sorted(recipe_roles - {track["role"] for track in role_tracks})

    key_count_mismatches = []
    for track in role_tracks:
        actual = len(track["curves"]["rot_x"])
        if actual != expected_key_count:
            key_count_mismatches.append({
                "role": track["role"],
                "expected": expected_key_count,
                "actual": actual,
            })

    return {
        "clip_found": True,
        "scheduled": scheduled,
        "missing_roles": missing_roles,
        "key_count_mismatches": key_count_mismatches,
        "ok": scheduled and not missing_roles and not key_count_mismatches,
    }


def measure_motion(frames: list[np.ndarray]) -> float:
    """Pure function: mean absolute difference between frame 1 and frames 2..5.

    Each frame is converted to grayscale float. A rectangle covering x 0.35-0.63
    and y 0.10-0.55 of the image is extracted. Frame 0 is skipped (imgui state
    differs). Returns the maximum mean absolute difference between frame 1 and
    each of frames 2..5.
    """
    if len(frames) < 3:
        return 0.0

    def extract_roi(img: np.ndarray) -> np.ndarray:
        h, w = img.shape[:2]
        y0, y1 = int(h * 0.10), int(h * 0.55)
        x0, x1 = int(w * 0.35), int(w * 0.63)
        return np.mean(img[y0:y1, x0:x1], axis=-1).astype(np.float32)

    roi_1 = extract_roi(frames[1])
    diffs = []
    for i in range(2, min(6, len(frames))):
        roi_i = extract_roi(frames[i])
        mae = float(np.abs(roi_i - roi_1).mean())
        diffs.append(mae)

    return max(diffs) if diffs else 0.0


def process_recipe(recipe_path: Path, dood: bool) -> dict:
    """Process a single recipe: run engine, check dump, measure motion."""
    recipe_name = recipe_path.stem
    out_dir = repo_root() / "target" / "recipe_smoke" / recipe_name
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    rig_src = repo_root() / RIG_SRC
    rig_dst = out_dir / "mixamo.fbx"
    shutil.copy(rig_src, rig_dst)

    scene_path = out_dir / "scene.ron"
    scene_content = f"""(
    version: 8,
    model: (
        path: "{rig_dst}",
    ),
)
"""
    scene_path.write_text(scene_content)

    proc = run_engine_once(out_dir, scene_path, recipe_path, dood)
    last_line = proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else ""
    report = json.loads(last_line) if last_line.startswith("{") else {"ok": False, "error": "no JSON"}

    if not report.get("ok"):
        return {
            "recipe": recipe_name,
            "dump": None,
            "motion": 0.0,
            "ok": False,
            "error": (proc.stderr + "\n" + proc.stdout).splitlines()[-10:],
        }

    dump_path = out_dir / "anim.json"
    if not dump_path.is_file():
        return {
            "recipe": recipe_name,
            "dump": None,
            "motion": 0.0,
            "ok": False,
            "error": "anim.json not written",
        }

    with open(dump_path) as f:
        dump = json.load(f)

    with open(recipe_path) as f:
        recipe = json.load(f)

    dump_check = check_anim_dump(dump, recipe)

    seq_dir = out_dir / "seq"
    frames = []
    if seq_dir.is_dir():
        for path in sorted(seq_dir.glob("*.png")):
            with Image.open(path) as img:
                frames.append(np.array(img.convert("RGB")))

    motion = measure_motion(frames) if len(frames) >= 3 else 0.0

    return {
        "recipe": recipe_name,
        "dump": dump_check,
        "motion": round(motion, 4),
        "ok": dump_check["ok"] and motion > MOTION_MIN_MEAN_ABS_DIFF,
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Smoke test: bake each motion recipe and verify the output has real movement.")
    parser.add_argument("--recipe", action="append", default=[],
                        help="path to a recipe JSON file (default: wave.json, hands_on_hips_tilt.json, bow.json)")
    parser.add_argument("--dood", action="store_true",
                        help="run the engine through the docker harness")
    args = parser.parse_args()

    recipes = args.recipe or [f"{RECIPES_DIR}/{name}" for name in DEFAULT_RECIPES]
    results = []

    for recipe_file in recipes:
        recipe_path = Path(recipe_file)
        if not recipe_path.is_absolute():
            recipe_path = repo_root() / recipe_path
        if not recipe_path.is_file():
            results.append({
                "recipe": recipe_file,
                "dump": None,
                "motion": 0.0,
                "ok": False,
                "error": f"recipe not found: {recipe_path}",
            })
            continue

        result = process_recipe(recipe_path, args.dood)
        results.append(result)

    overall = {"ok": all(r["ok"] for r in results), "results": results}
    print(json.dumps(overall))


if __name__ == "__main__":
    main()
