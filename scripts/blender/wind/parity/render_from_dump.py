"""Render the wind addon headlessly from an engine `dump_wind_debug` JSON.

Run inside Blender (xvfb, --gpu-backend vulkan):
    blender -noaudio --python render_from_dump.py -- <repo_root> <dump.json> <out_dir>
Writes blender_color.npy / blender_coverage.npy / blender_knots.npy (top-down RGBA float32)
and prints a per-block comparison of the packed WindUBO against the dumped one."""
import json
import sys
from pathlib import Path

argv = sys.argv[sys.argv.index("--") + 1:]
REPO_ROOT = Path(argv[0])
DUMP_JSON = Path(argv[1])
OUT_DIR = Path(argv[2])
OUT_DIR.mkdir(parents=True, exist_ok=True)
sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(REPO_ROOT / "log" / "blender_wind_probe" / "site"))

import gpu
import numpy as np
from gpu_extras.batch import batch_for_shader
import thyllore_effect_core as fx

import blender_addon.effects.wind as addon
from blender_addon.effects.wind import draw_handler
from blender_addon.effects.wind.draw_handler import WindViewportRenderer
from blender_addon.effects.wind.wind_shader import specialization_key

UBO_BLOCKS = ("shape", "wall", "optics", "albedo", "lighting", "streak", "streak2", "eddy", "eddy2", "puff_params")
UBO_MATRIX_FLOATS = 32
VIEWS = (("color", 0), ("coverage", 1), ("knots", 2))


def rows_from_columns(columns):
    return [[columns[c][r] for c in range(4)] for r in range(4)]


def column_major(columns):
    return [columns[c][r] for c in range(4) for r in range(4)]


def params_from_effect(effect):
    defaults = fx.wind_preset_params("column")
    return {
        k: (int(v) if isinstance(defaults[k], int) and not isinstance(defaults[k], bool) else v)
        for k, v in effect.items() if k in defaults
    }


def compare_ubo(ubo_bytes, engine_ubo):
    ubo = np.frombuffer(ubo_bytes, dtype=np.float32)
    offset = UBO_MATRIX_FLOATS
    for name in UBO_BLOCKS:
        theirs = np.array(engine_ubo[name], dtype=np.float32)
        diff = float(np.max(np.abs(ubo[offset:offset + 4] - theirs)))
        print(f"[ubo] {name}: max_abs_diff={diff:.3e}", flush=True)
        offset += 4
    puff_count = int(engine_ubo["puff_params"][0])
    if puff_count:
        ours = ubo[offset:offset + 4 * puff_count].reshape(-1, 4)
        theirs = np.array(engine_ubo["puffs"], dtype=np.float32)[:puff_count]
        print(f"[ubo] puffs({puff_count}): max_abs_diff={float(np.max(np.abs(ours - theirs))):.3e}", flush=True)


def render_view(spec, frame, far_depth):
    renderer = WindViewportRenderer()
    renderer.shader = draw_handler._load_shader(spec)
    renderer.batch = batch_for_shader(renderer.shader, "TRIS", {"pos": [(-1.0, -1.0), (3.0, -1.0), (-1.0, 3.0)]})
    renderer.shader_key = specialization_key(draw_handler.CLOSED_FORM_SPECIALIZATION)
    tex = renderer.render(*frame, depth_tex=far_depth)
    arr = np.array(tex.read().to_list(), dtype=np.float32)[::-1]
    renderer.release()
    return arr


def main():
    addon.register()
    dump = json.loads(DUMP_JSON.read_text())
    projection = dump["projection"]
    width, height = (int(v) for v in projection["screen_size"])
    instance = dump["wind_instances"][0]
    effect = instance["effect"]
    params = params_from_effect(effect)
    time = float(effect["time"])
    position = tuple(effect["position"])
    rotation = tuple(effect["rotation"])
    print(f"[dump] size={width}x{height} time={time} params={len(params)}", flush=True)

    compare_ubo(
        fx.pack_wind_ubo(params, time, position, rotation, column_major(projection["view"]), column_major(projection["proj"])),
        instance["ubo"],
    )

    far_depth = gpu.types.GPUTexture((1, 1), format="R32F", data=gpu.types.Buffer("FLOAT", 1, [0.0]))
    frame = (
        rows_from_columns(projection["view"]), rows_from_columns(projection["proj"]),
        tuple(dump["camera"]["position"]), tuple(dump["light"]["position"]),
        params, time, position, rotation, width, height,
    )
    for view_name, debug_view in VIEWS:
        arr = render_view({"mode": 0, "debugView": debug_view}, frame, far_depth)
        np.save(OUT_DIR / f"blender_{view_name}.npy", arr)
        print(f"[render] {view_name}: alpha>0 px={int((arr[..., 3] > 0).sum())}", flush=True)
    addon.unregister()
    print("RENDER_FROM_DUMP ok", flush=True)


main()
sys.exit(0)
