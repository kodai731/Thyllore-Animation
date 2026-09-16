"""Headless render probe for the Lightning addon.

Run inside Blender (xvfb, --gpu-backend vulkan):
    blender -noaudio --python probe_render.py -- <repo_root> <out_dir>
Writes lightning_probe.npy (RGBA float32) and prints non-zero pixel count."""
import math
import sys
from pathlib import Path

argv = sys.argv[sys.argv.index("--") + 1:]
REPO_ROOT = Path(argv[0])
OUT_DIR = Path(argv[1])
OUT_DIR.mkdir(parents=True, exist_ok=True)
sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(REPO_ROOT / "log" / "blender_lightning_probe" / "site"))

import gpu
import mathutils
import numpy as np
import thyllore_effect_core as fx

from blender_addon.effects.lightning._common import coordinates
from blender_addon.effects.lightning.draw_handler import VIEWPORT_NEAR, LightningViewportRenderer, composite_tonemapped

SIZE = 512
CAMERA_POS = (0.0, 4.0, 14.0)
CAMERA_TARGET = (0.0, 4.0, 0.0)
PIXEL_PROJECTION = mathutils.Matrix(
    ((2.0 / SIZE, 0.0, 0.0, -1.0), (0.0, 2.0 / SIZE, 0.0, -1.0), (0.0, 0.0, -1.0, 0.0), (0.0, 0.0, 0.0, 1.0))
)


def build_view_matrix(eye, target):
    delta = [t - e for t, e in zip(target, eye)]
    length = math.sqrt(sum(c * c for c in delta))
    forward = tuple(c / length for c in delta)
    return coordinates.look_at_view_matrix(eye, forward, (0.0, 1.0, 0.0))


def main():
    params = dict(fx.lightning_preset_params("bolt"))
    time = 0.30
    position = (0.0, 8.0, 0.0)
    rotation = (1.0, 0.0, 0.0, 0.0)

    view = build_view_matrix(CAMERA_POS, CAMERA_TARGET)
    proj = coordinates.engine_projection(math.radians(60.0), 1.0, VIEWPORT_NEAR)

    offscreen = gpu.types.GPUOffScreen(SIZE, SIZE, format="RGBA32F")
    far_depth = gpu.types.GPUTexture((1, 1), format="R32F", data=gpu.types.Buffer("FLOAT", 1, [0.0]))
    renderer = LightningViewportRenderer()
    color_tex = renderer.render(view, proj, CAMERA_POS, params, time, position, rotation, SIZE, SIZE, depth_tex=far_depth)
    with offscreen.bind():
        gpu.state.active_framebuffer_get().clear(color=(0.0, 0.0, 0.0, 0.0))
        with gpu.matrix.push_pop():
            gpu.matrix.load_identity()
            gpu.matrix.load_projection_matrix(PIXEL_PROJECTION)
            composite_tonemapped(color_tex, SIZE, SIZE)
    arr = np.array(offscreen.texture_color.read().to_list(), dtype=np.float32)[::-1]
    renderer.release()
    offscreen.free()

    out_path = OUT_DIR / "lightning_probe.npy"
    np.save(out_path, arr)
    nonzero = int((arr[..., 3] > 0).sum())
    print(f"[probe] saved {out_path} shape={arr.shape} nonzero_alpha={nonzero}", flush=True)


main()
sys.exit(0)
