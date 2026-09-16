"""Capture the lightning effect from Blender's 3D Viewport as screenshots and video.

Run inside Blender (xvfb, --gpu-backend vulkan):
    blender -noaudio --python capture_viewport.py -- <repo_root> <out_dir>

Writes:
    <out_dir>/frames/frame_NNNN.png   — one PNG per frame
    <out_dir>/lightning_screenshot.png — copy of frame 1 (screenshot feature)
    <out_dir>/lightning_viewport.mp4  — MP4 video from encode_frames_to_video"""

import math
import shutil
import sys
from pathlib import Path

argv = sys.argv[sys.argv.index("--") + 1:]
REPO_ROOT = Path(argv[0])
OUT_DIR = Path(argv[1])
OUT_DIR.mkdir(parents=True, exist_ok=True)
sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(REPO_ROOT / "log" / "blender_lightning_probe" / "site"))

import bpy
from mathutils import Euler
from blender_addon.common import viewport_recording
import blender_addon.effects.lightning as addon

addon.register()

for obj in list(bpy.data.objects):
    if obj.type == "MESH":
        bpy.data.objects.remove(obj, do_unlink=True)

bpy.ops.thyllore.lightning_add()
lightning = bpy.context.active_object
lightning.location = (0.0, 0.0, 8.0)
lightning.thyllore_lightning.end_offset = (0.0, -8.5, 0.0)
lightning.thyllore_lightning.preset = "bolt"

window = bpy.context.window
area = next((a for a in window.screen.areas if a.type == "VIEW_3D"), None)

if area is None:
    print("ERROR: no VIEW_3D area found", flush=True)
    sys.exit(1)

space = area.spaces.active
region = next((r for r in area.regions if r.type == "WINDOW"), None)
if region is None:
    print("ERROR: no WINDOW region found in VIEW_3D area", flush=True)
    sys.exit(1)

space.region_3d.view_perspective = "PERSP"
space.region_3d.view_rotation = Euler((math.radians(90.0), 0.0, 0.0)).to_quaternion()
space.region_3d.view_location = (0.0, 4.0, 0.0)
space.region_3d.view_distance = 14.0

scene = bpy.context.scene
scene.frame_start = 1
scene.frame_end = 48
scene.render.fps = 24
scene.render.fps_base = 1.0
frames = list(range(scene.frame_start, scene.frame_end + 1))
frames_dir = OUT_DIR / "frames"
frames_dir.mkdir(parents=True, exist_ok=True)
paths = [frames_dir / f"frame_{frame:04d}.png" for frame in frames]

print(f"Capturing {len(frames)} frames (frame {scene.frame_start}-{scene.frame_end}, fps={scene.render.fps})", flush=True)

for frame, path in zip(frames, paths):
    scene.frame_set(frame)
    bpy.ops.wm.redraw_timer(type="DRAW_WIN_SWAP", iterations=2)
    with bpy.context.temp_override(window=window, area=area, region=region):
        bpy.ops.screen.screenshot_area(filepath=str(path))

screenshot_path = OUT_DIR / "lightning_screenshot.png"
shutil.copy(paths[0], screenshot_path)
print(f"Screenshot saved: {screenshot_path}", flush=True)

video_path = str(OUT_DIR / "lightning_viewport.mp4")
viewport_recording.encode_frames_to_video(paths, video_path, scene.render.fps, scene.render.fps_base)
print(f"Video encoded: {video_path}", flush=True)

print(f"Frames captured: {len(frames)}", flush=True)
print(f"MP4 path: {video_path}", flush=True)
