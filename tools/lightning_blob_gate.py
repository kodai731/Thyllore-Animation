"""Verify no bright pixels appear outside the lightning segment band mask.

For each lightning segment in a dump, project it to screen using model->view->proj and build
a band mask from the projected segment with thickness max(r0,r1)*glow_ratio. Dilate the mask,
then count pixels outside it where the image is brighter than background + margin. Zero
violations means pass.

The imgui overlay (x < 300, 600 <= y <= 690 at 2560x1440) is excluded, as in
lightning_idempotency_gate.py.

    uv run --with numpy --with pillow python3 tools/lightning_blob_gate.py \
        --screenshot log/lightning/lightning_debug_1234.png \
        --dump log/lightning/lightning_debug_1234.json \
        --background path/to/background.png

    uv run --with numpy --with pillow python3 tools/lightning_blob_gate.py --self-check

    uv run --with numpy --with pillow python3 tools/lightning_blob_gate.py [--dood] [--batch-lightning-set core_intensity=12.0]

Exit code 0 = ran to completion (the JSON line on stdout holds pass/fail).
"""

from __future__ import annotations

import argparse
import json
import math
import os
import subprocess
import sys
import time
from pathlib import Path

import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
from engine_harness import dood_wrap, engine_env, engine_path, repo_root
from viewport import detect_viewport_from_background, trim_uniform_frame

HUD_MASK_MAX_X = 300
HUD_MASK_MIN_Y = 600
HUD_MASK_MAX_Y = 690
DEFAULT_DILATE_PX = 16
DEFAULT_LUMINANCE_MARGIN = 10.0
AXIS_SAMPLES = 128
BURST_TIMING_PINS = ["burst_start=0.0", "burst_jitter=0.0"]
BACKGROUND_LIGHTNING_OFF = ["core_intensity=0.0", "glow_intensity=0.0"]


def rows_from_columns(columns: list[list[float]]) -> np.ndarray:
    return np.array([[columns[c][r] for c in range(4)] for r in range(4)], dtype=np.float64)


def project_segment(
    model_view: np.ndarray,
    proj: np.ndarray,
    a: np.ndarray,
    b: np.ndarray,
    radius: float,
    screen_width: float,
    screen_height: float,
    viewport: tuple[int, int, int, int] | None = None,
) -> list[tuple[float, float, float]]:
    """(x, y, pixel_radius) samples along the segment axis, in screen_size pixels."""
    focal_y = abs(proj[1, 1])
    x0, y0, x1, y1 = viewport or (0, 0, int(screen_width), int(screen_height))
    points = []
    for t in np.linspace(0.0, 1.0, AXIS_SAMPLES):
        local = a[:3] + t * (b[:3] - a[:3])
        view_pos = model_view @ np.append(local, 1.0)
        clip = proj @ view_pos
        if clip[3] <= 0:
            continue
        sx = (clip[0] / clip[3] * 0.5 + 0.5) * (x1 - x0) + x0
        sy = (clip[1] / clip[3] * 0.5 + 0.5) * (y1 - y0) + y0
        pixel_radius = radius * focal_y * 0.5 * screen_height / clip[3]
        points.append((float(sx), float(sy), float(pixel_radius)))
    return points


def project_dump_segments(dump: dict, width: int, height: int,
                          viewport: tuple[int, int, int, int] | None = None) -> list[tuple[float, float, float]]:
    """Every segment sample of every instance, in image pixels."""
    projection = dump["projection"]
    view = rows_from_columns(projection["view"])
    proj = rows_from_columns(projection["proj"])
    screen_width, screen_height = (float(v) for v in projection["screen_size"])

    points = []
    for instance in dump["lightning_instances"]:
        model_view = view @ rows_from_columns(instance["ubo"]["model"])
        glow_ratio = float(instance["ubo"]["shape"][0])
        for segment in instance["segments"]:
            a = np.array(segment["a_r0"], dtype=np.float64)
            b = np.array(segment["b_r1"], dtype=np.float64)
            radius = max(a[3], b[3]) * glow_ratio
            points.extend(project_segment(model_view, proj, a, b, radius, screen_width, screen_height, viewport))
    return points


def draw_band_mask(points: list[tuple[float, float, float]], width: int, height: int,
                   dilate_px: int) -> np.ndarray:
    """Union of discs of radius pixel_radius + dilate_px around every axis sample."""
    mask = np.zeros((height, width), dtype=bool)
    for sx, sy, pixel_radius in points:
        reach = pixel_radius + dilate_px
        clip_min_x = max(math.floor(sx - reach), 0)
        clip_max_x = min(math.floor(sx + reach) + 1, width)
        clip_min_y = max(math.floor(sy - reach), 0)
        clip_max_y = min(math.floor(sy + reach) + 1, height)
        if clip_min_x >= clip_max_x or clip_min_y >= clip_max_y:
            continue
        y, x = np.ogrid[clip_min_y:clip_max_y, clip_min_x:clip_max_x]
        mask[clip_min_y:clip_max_y, clip_min_x:clip_max_x] |= (x - sx) ** 2 + (y - sy) ** 2 <= reach * reach
    return mask


def build_hud_mask(width: int, height: int) -> np.ndarray:
    """True on the overlay pixels that must be skipped."""
    mask = np.zeros((height, width), dtype=bool)
    mask[HUD_MASK_MIN_Y:min(HUD_MASK_MAX_Y + 1, height), :min(HUD_MASK_MAX_X, width)] = True
    return mask


def luminance(image_rgb: np.ndarray) -> np.ndarray:
    rgb = image_rgb.astype(np.float32)
    return 0.2126 * rgb[..., 0] + 0.7152 * rgb[..., 1] + 0.0722 * rgb[..., 2]


def count_blob_violations(
    image_rgb: np.ndarray,
    background_rgb: np.ndarray,
    dump: dict,
    dilate_px: int,
    luminance_margin: float,
    background_path: Path | None = None,
    viewport: tuple[int, int, int, int] | None = None,
) -> dict:
    height, width = image_rgb.shape[:2]
    if background_rgb.shape != image_rgb.shape:
        raise SystemExit(f"background size {background_rgb.shape} != screenshot size {image_rgb.shape}")

    if background_path is not None and viewport is None:
        viewport = trim_uniform_frame(background_rgb, detect_viewport_from_background(background_path))
        vx, vy = viewport[2] - viewport[0], viewport[3] - viewport[1]
        sx, sy = dump["projection"]["screen_size"]
        if abs(vx - sx) > 4 or abs(vy - sy) > 4:
            raise SystemExit(
                f"viewport size ({vx}, {vy}) from background does not match "
                f"dump screen_size ({sx}, {sy})")

    points = project_dump_segments(dump, width, height, viewport)
    band_mask = draw_band_mask(points, width, height, 0)
    dilated_mask = draw_band_mask(points, width, height, dilate_px)
    hud_mask = build_hud_mask(width, height)

    outside = ~dilated_mask & ~hud_mask
    brighter = luminance(image_rgb) - luminance(background_rgb) >= luminance_margin
    return {
        "violations": int(np.count_nonzero(outside & brighter)),
        "band_pixels": int(np.count_nonzero(band_mask)),
        "dilated_band_pixels": int(np.count_nonzero(dilated_mask)),
        "compared_pixels": int(np.count_nonzero(outside)),
        "hud_masked_pixels": int(np.count_nonzero(hud_mask)),
    }


def build_synthetic_dump(width: int, height: int) -> dict:
    """One horizontal segment 10 units in front of an identity camera (Vulkan reverse-Z proj)."""
    identity = np.eye(4).tolist()
    focal = 2.0
    proj_rows = np.array([
        [focal * height / width, 0.0, 0.0, 0.0],
        [0.0, -focal, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.1],
        [0.0, 0.0, -1.0, 0.0],
    ])
    return {
        "projection": {"view": identity, "proj": proj_rows.T.tolist(), "screen_size": [width, height]},
        "lightning_instances": [{
            "ubo": {"model": identity, "shape": [4.0, 0.5, 3.0, 0.0]},
            "segments": [{"a_r0": [-3.0, 0.0, -10.0, 0.05], "b_r1": [3.0, 0.0, -10.0, 0.05]}],
        }],
    }


def draw_synthetic_screenshot(dump: dict, width: int, height: int,
                              blob_center: tuple[int, int] | None,
                              viewport: tuple[int, int, int, int] | None = None) -> np.ndarray:
    image = np.full((height, width, 3), 40, dtype=np.uint8)
    inside_band = draw_band_mask(project_dump_segments(dump, width, height, viewport), width, height, 0)
    image[inside_band] = 230
    if blob_center is not None:
        y, x = np.ogrid[:height, :width]
        image[(x - blob_center[0]) ** 2 + (y - blob_center[1]) ** 2 <= 30 ** 2] = 200
    return image


def run_self_check() -> dict:
    width, height = 640, 360
    dump = build_synthetic_dump(width, height)
    background = np.full((height, width, 3), 40, dtype=np.uint8)
    cases = {
        "inside_band_only": (draw_synthetic_screenshot(dump, width, height, None), lambda v: v == 0),
        "disk_outside_band": (draw_synthetic_screenshot(dump, width, height, (320, 80)), lambda v: v > 0),
    }

    offset_x, offset_y = 258, 23
    large_width, large_height = 900, 400
    viewport = (offset_x, offset_y, offset_x + width, offset_y + height)
    large_background = np.full((large_height, large_width, 3), 40, dtype=np.uint8)
    offset_image = draw_synthetic_screenshot(dump, large_width, large_height, None, viewport)
    cases["offset_viewport"] = (offset_image, lambda v: v == 0)

    results = {}
    for name, (image, expectation) in cases.items():
        bg = large_background if name == "offset_viewport" else background
        vp = viewport if name == "offset_viewport" else None
        report = count_blob_violations(image, bg, dump, DEFAULT_DILATE_PX, DEFAULT_LUMINANCE_MARGIN, viewport=vp)
        report["match"] = expectation(report["violations"])
        results[name] = report
    return {"ok": True, "pass": all(r["match"] for r in results.values()), **results}


def load_rgb(path: Path) -> np.ndarray:
    if not path.is_file():
        raise SystemExit(f"not found: {path}")
    with Image.open(path) as image:
        return np.array(image.convert("RGB"))


def resolve_path(value: str) -> Path:
    path = Path(value)
    return path if path.is_absolute() else repo_root() / path


def find_dump_written_since(started_at: float) -> Path:
    directory = repo_root() / "log" / "lightning"
    fresh = [path for path in directory.glob("lightning_debug_*.json") if path.stat().st_mtime >= started_at]
    if not fresh:
        raise SystemExit(f"engine wrote no lightning_debug_*.json into {directory}")
    return max(fresh, key=lambda path: path.stat().st_mtime)


def run_engine(out_path: Path, scene: str, camera: str, frames: int, lightning_time: float,
               dood: bool, lightning_set: list[str], extra_args: list[str]) -> Path:
    command = [
        str(engine_path()),
        "--batch-screenshot", str(out_path),
        "--batch-scene", scene,
        "--batch-frames", str(frames),
        "--batch-camera", camera,
        "--batch-lightning-time", str(lightning_time),
        *extra_args,
    ]
    for value in lightning_set + BURST_TIMING_PINS:
        command.extend(["--batch-lightning-set", value])
    if dood:
        command = dood_wrap(command)

    print(f"[lightning_blob_gate] capture {out_path.name}", file=sys.stderr)
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


def capture(scene: str, camera: str, frames: int, lightning_time: float,
            out_dir: Path, dood: bool, lightning_set: list[str]) -> tuple[Path, Path, Path]:
    """Screenshot + dump of one frame, then the same frame with the lightning switched off."""
    started_at = time.time()
    screenshot_path = run_engine(
        out_dir / f"screenshot_t{lightning_time}.png", scene, camera, frames, lightning_time,
        dood, lightning_set, ["--batch-debug-action", "dump_lightning_debug"],
    )
    dump_path = find_dump_written_since(started_at)
    background_path = run_engine(
        out_dir / f"background_t{lightning_time}.png", scene, camera, frames, lightning_time,
        dood, lightning_set + BACKGROUND_LIGHTNING_OFF, [],
    )
    return screenshot_path, background_path, dump_path


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Verify no bright pixels appear outside the lightning segment band mask.")
    parser.add_argument("--self-check", action="store_true",
                        help="run the pure judgement on synthetic images instead of captured files")
    parser.add_argument("--screenshot", help="lightning screenshot PNG")
    parser.add_argument("--dump", help="dump_lightning_debug JSON of the same frame")
    parser.add_argument("--background", help="same frame without lightning")
    parser.add_argument("--dilate-px", type=int, default=DEFAULT_DILATE_PX)
    parser.add_argument("--luminance-margin", type=float, default=DEFAULT_LUMINANCE_MARGIN)
    parser.add_argument("--scene", default="assets/scenes/lightning_probe.scene.ron")
    parser.add_argument("--camera", default="30,-8,14,0,3,0")
    parser.add_argument("--frames", type=int, default=1)
    parser.add_argument("--lightning-time", type=float, default=0.02)
    parser.add_argument("--out-dir", default="target/tmp_screens/lightning_blob_gate")
    parser.add_argument("--dood", action="store_true",
                        help="run the engine through the docker harness")
    parser.add_argument("--engine",
                        help="engine binary to run (defaults to THYLLORE_ENGINE, then release, then debug)")
    parser.add_argument("--batch-lightning-set", action="append", default=[], metavar="KEY=VALUE")
    args = parser.parse_args()

    if args.self_check:
        print(json.dumps(run_self_check()))
        return

    if args.screenshot and args.dump and args.background:
        dump = json.loads(resolve_path(args.dump).read_text())
        background_path = resolve_path(args.background)
        report = count_blob_violations(
            load_rgb(resolve_path(args.screenshot)), load_rgb(background_path), dump,
            args.dilate_px, args.luminance_margin, background_path=background_path,
        )
        print(json.dumps({"ok": True, "pass": report["violations"] == 0, **report}))
        return

    if args.engine:
        os.environ["THYLLORE_ENGINE"] = args.engine
    print(f"[lightning_blob_gate] engine {engine_path()}", file=sys.stderr)

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

    screenshot_path, background_path, dump_path = capture(
        scene, args.camera, args.frames, args.lightning_time,
        out_dir, args.dood, args.batch_lightning_set,
    )

    dump = json.loads(dump_path.read_text())
    report = count_blob_violations(
        load_rgb(screenshot_path), load_rgb(background_path), dump,
        args.dilate_px, args.luminance_margin, background_path=background_path,
    )
    print(json.dumps({"ok": True, "pass": report["violations"] == 0, **report}))


if __name__ == "__main__":
    main()
