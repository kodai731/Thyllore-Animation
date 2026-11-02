import os
from pathlib import Path

import bpy

MODEL_PATH = os.environ["THYLLORE_AVATAR_MODEL"]
CLIP_PATH = Path(__file__).resolve().parents[3] / "assets" / "templates" / "hips_sway.anim.ron"
SCREENSHOT_DIR = os.environ.get("THYLLORE_SCREENSHOT_DIR", "")
SCREENSHOT_DELAY_SECONDS = float(os.environ.get("THYLLORE_SCREENSHOT_DELAY", "3"))
QUIT_AFTER_SCREENSHOT = os.environ.get("THYLLORE_SCREENSHOT_QUIT", "") == "1"
SIDEBAR_CATEGORY = bpy.types.THYLLORE_PT_avatar_setup.bl_category
CATEGORY_CYCLE_LIMIT = 16


def remove_startup_objects():
    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj, do_unlink=True)


def import_model():
    bpy.ops.import_scene.fbx(filepath=MODEL_PATH)
    armature = next((o for o in bpy.context.scene.objects if o.type == "ARMATURE"), None)
    if armature is None:
        print(f"[avatar_setup/setup_scene] no armature in {MODEL_PATH}", flush=True)
        return
    for obj in bpy.context.scene.objects:
        obj.select_set(obj == armature)
    bpy.context.view_layer.objects.active = armature
    armature.data.thyllore_vrm.source_model = Path(MODEL_PATH).stem
    bpy.ops.thyllore.reset_vrm_meta()
    print(f"[avatar_setup/setup_scene] imported {MODEL_PATH}, active armature {armature.name}", flush=True)


def load_default_clip():
    if bpy.context.view_layer.objects.active is None:
        return
    result = bpy.ops.thyllore.load_engine_clip(filepath=str(CLIP_PATH))
    print(f"[avatar_setup/setup_scene] clip {CLIP_PATH.name}: {result}", flush=True)


def find_view3d():
    for window in bpy.context.window_manager.windows:
        for area in window.screen.areas:
            if area.type == "VIEW_3D":
                return window, area
    return None


def region_of(area, region_type):
    return next(r for r in area.regions if r.type == region_type)


def frame_model_and_open_sidebar():
    window, area = find_view3d()
    with bpy.context.temp_override(window=window, area=area, region=region_of(area, "WINDOW")):
        bpy.ops.view3d.view_all()
    area.spaces.active.show_region_ui = True
    return None


def cycle_sidebar_to_avatar_setup(remaining=CATEGORY_CYCLE_LIMIT):
    window, area = find_view3d()
    sidebar = region_of(area, "UI")
    if sidebar.active_panel_category == SIDEBAR_CATEGORY or remaining == 0:
        print(f"[avatar_setup/setup_scene] sidebar tab: {sidebar.active_panel_category}", flush=True)
        return None
    tab_x = sidebar.x + sidebar.width - 8
    tab_y = sidebar.y + sidebar.height - 60
    window.event_simulate(type="MOUSEMOVE", value="NOTHING", x=tab_x, y=tab_y)
    window.event_simulate(type="WHEELDOWNMOUSE", value="PRESS", x=tab_x, y=tab_y, ctrl=True)
    bpy.app.timers.register(lambda: cycle_sidebar_to_avatar_setup(remaining - 1), first_interval=0.2)
    return None


def take_screenshot():
    window, _ = find_view3d()
    path = str(Path(SCREENSHOT_DIR) / "avatar_setup.png")
    with bpy.context.temp_override(window=window):
        bpy.ops.screen.screenshot(filepath=path)
    print(f"[avatar_setup/setup_scene] screenshot -> {path}", flush=True)
    if QUIT_AFTER_SCREENSHOT:
        bpy.ops.wm.quit_blender()
    return None


bpy.context.preferences.view.show_splash = False
remove_startup_objects()
import_model()
load_default_clip()
bpy.app.timers.register(frame_model_and_open_sidebar, first_interval=0.5)
if bpy.app.use_event_simulate:
    bpy.app.timers.register(cycle_sidebar_to_avatar_setup, first_interval=1.0)
if SCREENSHOT_DIR:
    bpy.app.timers.register(take_screenshot, first_interval=SCREENSHOT_DELAY_SECONDS)
