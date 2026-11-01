"""Bake an engine humanoid clip (.anim.ron) onto the active rig (thyllore.load_engine_clip).

The wheel retargets the clip the way the engine plays a template; every frame then lands in the
armature's action, so the swing preview and the timeline see the same motion as the engine."""
from __future__ import annotations

import bpy
from bpy_extras.io_utils import ImportHelper
from mathutils import Quaternion, Vector

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from .. import _debuglog
from ..common.coordinates import engine_to_blender_point
from ..common.logged_report import report
from .operators import armature_bone_specs, target_armature
from .spring_preview import ENGINE_AXES, rest_relative_to_parent, write_joint_rotation


def scene_fps(scene) -> int:
    return round(scene.render.fps / scene.render.fps_base)


def _replace_action(arm, name: str):
    stale = bpy.data.actions.get(name)
    if stale is not None:
        bpy.data.actions.remove(stale)
    animation_data = arm.animation_data or arm.animation_data_create()
    animation_data.action = None


def _hips_location(arm, bone, offset_metres, scale_length: float) -> Vector:
    world_delta = Vector(engine_to_blender_point(offset_metres)) / scale_length
    armature_delta = arm.matrix_world.to_3x3().inverted() @ world_delta
    return bone.matrix_local.to_3x3().inverted() @ armature_delta


def _write_frame(arm, baked: dict, frame_index: int, frame: int, root_rotation_inv: Quaternion, scale_length: float) -> None:
    for track in baked["bone_rotations"]:
        bone = arm.data.bones[track["bone"]]
        rotation = Quaternion(track["rotations"][frame_index])
        if bone.parent is None:
            rotation = root_rotation_inv @ rotation
        write_joint_rotation(arm, rest_relative_to_parent(bone), bone.name, rotation)
        arm.pose.bones[bone.name].keyframe_insert("rotation_quaternion", frame=frame)

    hips = arm.data.bones[baked["hips_bone"]]
    hips_pose = arm.pose.bones[hips.name]
    hips_pose.location = _hips_location(arm, hips, baked["hips_offsets"][frame_index], scale_length)
    hips_pose.keyframe_insert("location", frame=frame)


def bake_clip_onto_armature(context, arm, clip_text: str) -> dict:
    """Bake clip_text onto arm as an action named after the clip; returns the wheel's baked clip."""
    scene = context.scene
    scale_length = scene.unit_settings.scale_length
    baked = tml.bake_clip_file(armature_bone_specs(context, arm), clip_text, scene_fps(scene))

    _replace_action(arm, baked["name"])
    for track in baked["bone_rotations"]:
        arm.pose.bones[track["bone"]].rotation_mode = "QUATERNION"

    root_rotation_inv = (ENGINE_AXES @ arm.matrix_world).to_3x3().normalized().to_quaternion().inverted()
    first_frame = scene.frame_start
    for frame_index in range(len(baked["frame_times"])):
        _write_frame(arm, baked, frame_index, first_frame + frame_index, root_rotation_inv, scale_length)

    arm.animation_data.action.name = baked["name"]
    scene.frame_end = first_frame + len(baked["frame_times"]) - 1
    scene.frame_set(first_frame)
    return baked


class THYLLORE_OT_LoadEngineClip(bpy.types.Operator, ImportHelper):
    """Bake an engine humanoid clip (.anim.ron) onto the active rig as its action"""

    bl_idname = "thyllore.load_engine_clip"
    bl_label = "Load Engine Clip"
    bl_options = {"REGISTER", "UNDO"}

    filename_ext = ".anim.ron"
    filter_glob: bpy.props.StringProperty(default="*.anim.ron", options={"HIDDEN"})

    @classmethod
    def poll(cls, context):
        return _TML_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        arm = target_armature(context)
        try:
            with open(self.filepath, encoding="utf-8") as f:
                clip_text = f.read()
        except OSError as e:
            report(self, "ERROR", f"Cannot read file: {e}")
            return {"CANCELLED"}

        try:
            baked = bake_clip_onto_armature(context, arm, clip_text)
        except (RuntimeError, ValueError, KeyError) as e:
            report(self, "ERROR", f"Clip bake failed: {e}")
            return {"CANCELLED"}

        report(self, "INFO", f"Baked {baked['name']}: {len(baked['frame_times'])} frames at {baked['fps']} fps")
        return {"FINISHED"}


OPERATORS = (THYLLORE_OT_LoadEngineClip,)


def register() -> None:
    """Debug builds only: addon users have no engine clips to load."""
    if not _debuglog.is_debug_enabled():
        return
    for cls in OPERATORS:
        bpy.utils.register_class(cls)


def unregister() -> None:
    if not _debuglog.is_debug_enabled():
        return
    for cls in reversed(OPERATORS):
        bpy.utils.unregister_class(cls)
