import os

import bpy

MODEL_PATH = os.environ["THYLLORE_AVATAR_MODEL"]


def remove_startup_objects():
    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj, do_unlink=True)


def import_model():
    bpy.ops.import_scene.fbx(filepath=MODEL_PATH)
    armature = next((o for o in bpy.context.scene.objects if o.type == "ARMATURE"), None)
    if armature is None:
        print(f"[avatar_setup/import_model] no armature in {MODEL_PATH}", flush=True)
        return
    for obj in bpy.context.scene.objects:
        obj.select_set(obj == armature)
    bpy.context.view_layer.objects.active = armature
    print(f"[avatar_setup/import_model] imported {MODEL_PATH}, active armature {armature.name}", flush=True)


def frame_all_views():
    for window in bpy.context.window_manager.windows:
        for area in window.screen.areas:
            if area.type != "VIEW_3D":
                continue
            region = next(r for r in area.regions if r.type == "WINDOW")
            with bpy.context.temp_override(window=window, area=area, region=region):
                bpy.ops.view3d.view_all()
    return None


remove_startup_objects()
import_model()
bpy.app.timers.register(frame_all_views, first_interval=0.5)
