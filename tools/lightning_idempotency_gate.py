"""Verify the lightning effect renders deterministically and still evolves over time.

Captures the lightning probe scene at fixed lightning times and checks:
(a) t=0.02 twice -> every compared pixel matches bit for bit.
(b) t=0.02 with --batch-frames 30 and 60 -> every compared pixel matches bit for bit.
(c) t=0.02 vs t=0.045 -> more than 1000 compared pixels differ.

Every capture pins burst_start to 0, burst_jitter to 0 and reseed_period to 0.02 s. Without the
jitter pin the default 0.2 s jitter moves burst 0 of seed 0 to t=0.152 s and both times fall outside
it; with it both sit in the sustain plateau (attack 0.01 s, sustain 0.04 s) and still land in
different reseed windows (1 vs 2) and flicker windows (flicker_period 0.03: tick 0 vs 1), so the
strike geometry and brightness both change between them. The pins are applied after
--batch-lightning-set so they cannot be overridden away.

The imgui overlay (x < 300, 600 <= y <= 690 at 2560x1440) jitters between runs, so it is
excluded from every comparison.

    uv run --with numpy --with pillow python3 tools/lightning_idempotency_gate.py [--dood] [--batch-lightning-set core_intensity=12.0]

Exit code 0 = ran to completion (the JSON line on stdout holds pass/fail).
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
from engine_harness import dood_wrap, engine_env, engine_path, repo_root

HUD_MASK_MAX_X = 300
HUD_MASK_MIN_Y = 600
HUD_MASK_MAX_Y = 690
TIME_EVOLUTION_MIN_DIFF_PIXELS = 1000
BURST_TIMING_PINS = ["burst_start=0.0", "burst_jitter=0.0", "reseed_period=0.02"]
EARLY_BURST_TIME = 0.02
LATE_SUSTAIN_TIME = 0.045


def capture(scene: str, camera: str, frames: int, lightning_time: float,
            out_path: Path, dood: bool, lightning_set: list[str]) -> Path:
    """Run the engine once to take a batch screenshot at a fixed lightning time."""
    command = [
        str(engine_path()),
        "--batch-screenshot", str(out_path),
        "--batch-scene", scene,
        "--batch-frames", str(frames),
        "--batch-camera", camera,
        "--batch-lightning-time", str(lightning_time),
    ]
    for value in lightning_set + BURST_TIMING_PINS:
        command.extend(["--batch-lightning-set", value])
    if dood:
        command = dood_wrap(command)

    print(f"[lightning_idempotency_gate] capture {out_path.name}", file=sys.stderr)
    proc = subprocess.run(
        command, capture_output=True, text=True, timeout=600,
        cwd=str(repo_root()), env=engine_env(),
    )
    last_line = proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else ""
    report = json.loads(last_line) if last_line.startswith("{") else {"ok": False, "error": "no JSON"}
    if not report.get("ok"):
        raise SystemExit(f"capture failed ({out_path.name}): {report.get('error')}\n{proc.stderr[-2000:]}")
    if not out_path.is_file():
        raise SystemExit(f"capture reported ok but wrote no image: {out_path}")
    return out_path


def load_rgb(path: Path) -> np.ndarray:
    with Image.open(path) as image:
        return np.array(image.convert("RGB")).astype(np.int16)


def build_hud_mask(shape: tuple[int, ...]) -> np.ndarray:
    """True on the overlay pixels that must be skipped when comparing two shots."""
    height, width = shape[0], shape[1]
    mask = np.zeros((height, width), dtype=bool)
    mask[HUD_MASK_MIN_Y:min(HUD_MASK_MAX_Y + 1, height), :min(HUD_MASK_MAX_X, width)] = True
    return mask


def count_diff_pixels(first: np.ndarray, second: np.ndarray, hud_mask: np.ndarray) -> int:
    differs = np.any(first != second, axis=-1)
    return int(np.count_nonzero(differs & ~hud_mask))


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Verify the lightning effect renders deterministically and still evolves over time.")
    parser.add_argument("--scene", default="assets/scenes/lightning_probe.scene.ron")
    parser.add_argument("--camera", default="30,-8,14,0,3,0")
    parser.add_argument("--frames", type=int, default=30)
    parser.add_argument("--alt-frames", type=int, default=60,
                        help="second frame count used by the frame independence check")
    parser.add_argument("--out-dir", default="target/tmp_screens/lightning_idempotency_gate")
    parser.add_argument("--dood", action="store_true",
                        help="run the engine through the docker harness")
    parser.add_argument("--engine",
                        help="engine binary to run (defaults to THYLLORE_ENGINE, then release, then debug)")
    parser.add_argument("--batch-lightning-set", action="append", default=[], metavar="KEY=VALUE")
    args = parser.parse_args()

    if args.engine:
        os.environ["THYLLORE_ENGINE"] = args.engine
    print(f"[lightning_idempotency_gate] engine {engine_path()}", file=sys.stderr)

    out_dir = Path(args.out_dir)
    if not out_dir.is_absolute():
        out_dir = repo_root() / out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    scene_path = Path(args.scene)
    if not scene_path.is_absolute():
        scene_path = repo_root() / scene_path
    if not scene_path.is_file():
        raise SystemExit(f"scene not found: {scene_path}")
    scene = str(scene_path.relative_to(repo_root()))

    lightning_set = args.batch_lightning_set
    run_a = capture(scene, args.camera, args.frames, EARLY_BURST_TIME,
                    out_dir / f"t{EARLY_BURST_TIME}_frames{args.frames}_run1.png", args.dood, lightning_set)
    rerun_a = capture(scene, args.camera, args.frames, EARLY_BURST_TIME,
                      out_dir / f"t{EARLY_BURST_TIME}_frames{args.frames}_run2.png", args.dood, lightning_set)
    alt_frames = capture(scene, args.camera, args.alt_frames, EARLY_BURST_TIME,
                         out_dir / f"t{EARLY_BURST_TIME}_frames{args.alt_frames}.png", args.dood, lightning_set)
    later_time = capture(scene, args.camera, args.frames, LATE_SUSTAIN_TIME,
                         out_dir / f"t{LATE_SUSTAIN_TIME}_frames{args.frames}.png", args.dood, lightning_set)

    baseline = load_rgb(run_a)
    hud_mask = build_hud_mask(baseline.shape)

    repeat_diff_pixels = count_diff_pixels(baseline, load_rgb(rerun_a), hud_mask)
    frames_diff_pixels = count_diff_pixels(baseline, load_rgb(alt_frames), hud_mask)
    time_diff_pixels = count_diff_pixels(baseline, load_rgb(later_time), hud_mask)

    deterministic = repeat_diff_pixels == 0
    frame_independent = frames_diff_pixels == 0
    time_evolved = time_diff_pixels > TIME_EVOLUTION_MIN_DIFF_PIXELS

    result = {
        "ok": True,
        "pass": deterministic and frame_independent and time_evolved,
        "deterministic": deterministic,
        "repeat_diff_pixels": repeat_diff_pixels,
        "frame_independent": frame_independent,
        "frames_diff_pixels": frames_diff_pixels,
        "time_evolved": time_evolved,
        "time_diff_pixels": time_diff_pixels,
        "compared_pixels": int(np.count_nonzero(~hud_mask)),
        "hud_masked_pixels": int(np.count_nonzero(hud_mask)),
    }

    print(json.dumps(result))


if __name__ == "__main__":
    main()
