import math
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, "/workspace")
from blender_addon.common.coordinates import orbit_camera

ENGINE_DEFAULT_CAMERA = {"yaw_deg": 45.0, "pitch_deg": math.degrees(math.asin(5.0 / math.sqrt(75.0))), "distance": math.sqrt(75.0), "pivot": (0.0, 0.0, 0.0)}
ENGINE_DEFAULT_FOV_Y_DEG = 45.0
ENGINE_DEFAULT_LIGHT_POSITION = (1.0, 1.0, 2.0)


def engine_to_blender_point(p):
    return (p[0], -p[2], p[1])


for obj in list(bpy.data.objects):
    if obj.type == "MESH":
        bpy.data.objects.remove(obj, do_unlink=True)

position, direction, _ = orbit_camera(**ENGINE_DEFAULT_CAMERA)
camera = bpy.data.objects["Camera"]
camera.location = engine_to_blender_point(position)
camera.rotation_euler = (Vector(engine_to_blender_point(ENGINE_DEFAULT_CAMERA["pivot"])) - camera.location).to_track_quat("-Z", "Y").to_euler()
camera.data.sensor_fit = "VERTICAL"
camera.data.angle_y = math.radians(ENGINE_DEFAULT_FOV_Y_DEG)

light = next(o for o in bpy.data.objects if o.type == "LIGHT")
light.location = engine_to_blender_point(ENGINE_DEFAULT_LIGHT_POSITION)

bpy.ops.thyllore.wind_add()
wind = bpy.context.active_object
wind.location = (0.0, 0.0, 0.0)

for screen in bpy.data.screens:
    for area in screen.areas:
        if area.type == "VIEW_3D":
            area.spaces.active.region_3d.view_perspective = "CAMERA"

bpy.context.scene.frame_end = 120
bpy.ops.wm.save_as_mainfile(filepath=sys.argv[sys.argv.index("--") + 1])
print("[make_wind_blend] saved", [o.name for o in bpy.data.objects], flush=True)
