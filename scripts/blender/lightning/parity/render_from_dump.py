"""Render the lightning addon headlessly from an engine `dump_lightning_debug` JSON.

Run inside Blender (xvfb, --gpu-backend vulkan):
    blender -noaudio --python render_from_dump.py -- <repo_root> <dump.json> <out_dir>
Writes blender_color.npy / blender_coverage.npy (top-down RGBA float32)
and prints a per-block comparison of the packed LightningUBO against the dumped one."""
import json
import sys
from pathlib import Path

argv = sys.argv[sys.argv.index("--") + 1:]
REPO_ROOT = Path(argv[0])
DUMP_JSON = Path(argv[1])
OUT_DIR = Path(argv[2])
OUT_DIR.mkdir(parents=True, exist_ok=True)
sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(REPO_ROOT / "log" / "blender_lightning_probe" / "site"))

import gpu
import numpy as np
from gpu_extras.batch import batch_for_shader
import thyllore_effect_core as fx

import blender_addon.effects.lightning as addon
from blender_addon.effects.lightning.draw_handler import LightningViewportRenderer, _load_shader

UBO_BLOCKS = (("model", 0, 16), ("inverse_model", 16, 16), ("core", 32, 4), ("glow", 36, 4), ("shape", 40, 4), ("inv_view_proj", 48, 16))
VIEWS = (("color", 0), ("coverage", 1))


def rows_from_columns(columns):
    return [[columns[c][r] for c in range(4)] for r in range(4)]


def column_major(columns):
    return [columns[c][r] for c in range(4) for r in range(4)]


def params_from_effect(effect):
    defaults = fx.lightning_preset_params("bolt")
    return {
        k: (int(v) if isinstance(defaults[k], int) and not isinstance(defaults[k], bool) else v)
        for k, v in effect.items() if k in defaults
    }


def compare_ubo(ubo_bytes, engine_ubo):
    ubo = np.frombuffer(ubo_bytes, dtype=np.float32)
    for name, offset, block_size in UBO_BLOCKS:
        theirs = np.array(engine_ubo[name], dtype=np.float32).reshape(-1)
        diff = float(np.max(np.abs(ubo[offset:offset + block_size] - theirs)))
        print(f"[ubo] {name}: max_abs_diff={diff:.3e}", flush=True)


def render_view(debug_view, frame, far_depth):
    renderer = LightningViewportRenderer()
    renderer.shader = _load_shader()
    renderer.batch = batch_for_shader(renderer.shader, "TRIS", {"pos": [(-1.0, -1.0), (3.0, -1.0), (-1.0, 3.0)]})
    tex = renderer.render(*frame, depth_tex=far_depth, debug_view=debug_view)
    arr = np.array(tex.read().to_list(), dtype=np.float32)[::-1]
    renderer.release()
    return arr


def main():
    addon.register()
    dump = json.loads(DUMP_JSON.read_text())
    projection = dump["projection"]
    width, height = (int(v) for v in projection["screen_size"])
    instance = dump["lightning_instances"][0]
    effect = instance["effect"]
    params = params_from_effect(effect)
    time = float(effect["time"])
    position = tuple(effect["position"])
    rotation = tuple(effect["rotation"])
    print(f"[dump] size={width}x{height} time={time} params={len(params)}", flush=True)

    view_cm = column_major(projection["view"])
    proj_cm = column_major(projection["proj"])
    ubo_bytes, segments_bytes, segment_count = fx.pack_lightning_ubo(params, time, position, rotation, view_cm, proj_cm)
    compare_ubo(ubo_bytes, instance["ubo"])

    far_depth = gpu.types.GPUTexture((1, 1), format="R32F", data=gpu.types.Buffer("FLOAT", 1, [0.0]))
    frame = (
        rows_from_columns(projection["view"]), rows_from_columns(projection["proj"]),
        tuple(dump["camera"]["position"]),
        params, time, position, rotation, width, height,
    )
    for view_name, debug_view in VIEWS:
        arr = render_view(debug_view, frame, far_depth)
        np.save(OUT_DIR / f"blender_{view_name}.npy", arr)
        print(f"[render] {view_name}: alpha>0 px={int((arr[..., 3] > 0).sum())}", flush=True)
    addon.unregister()
    print("RENDER_FROM_DUMP ok", flush=True)


main()
sys.exit(0)
