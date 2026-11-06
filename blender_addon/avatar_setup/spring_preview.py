"""Live swing preview: the engine's spring solver drives the chain bones on every frame change.

This file only reads Blender's bone matrices and writes the solver's ``matrix_basis`` back; the
rest-relative handling of the chain joints and the basis itself are the wheel's."""
from __future__ import annotations

import bpy
from bpy.app.handlers import persistent
from mathutils import Matrix

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from .. import _debuglog
from ..common.coordinates import blender_frame_to_engine_metres
from ..common.spring_properties import spring_settings_to_edit
from .operators import armature_bone_specs

_previews: dict[str, "_ArmaturePreview"] = {}


class _ArmaturePreview:
    def __init__(self, preview, spring_key: str, rest_key: str):
        self.preview = preview
        self.spring_key = spring_key
        self.rest_key = rest_key
        self.last_frame: int | None = None

    def matches(self, spring_key: str, rest_key: str) -> bool:
        return self.spring_key == spring_key and self.rest_key == rest_key


def invalidate_preview(armature_name: str) -> None:
    _previews.pop(armature_name, None)


def matrix_rows(matrix: Matrix) -> list[list[float]]:
    return [list(row) for row in matrix]


def _root_frame(arm, scale_length: float) -> Matrix:
    return Matrix(blender_frame_to_engine_metres(matrix_rows(arm.matrix_world), scale_length))


def rest_relative_to_parent(bone) -> Matrix:
    if bone.parent is None:
        return bone.matrix_local.copy()
    return bone.parent.matrix_local.inverted() @ bone.matrix_local


def _rest_locals(arm, root_frame: Matrix) -> list[list[list[float]]]:
    return [
        matrix_rows(root_frame @ bone.matrix_local if bone.parent is None else rest_relative_to_parent(bone))
        for bone in arm.data.bones
    ]


def _animated_local(pose_bone, root_frame: Matrix) -> Matrix:
    if pose_bone.parent is None:
        return root_frame @ pose_bone.matrix
    return pose_bone.parent.matrix.inverted() @ pose_bone.matrix


def _animated_locals(arm, root_frame: Matrix) -> list[list[list[float]]]:
    return [matrix_rows(_animated_local(arm.pose.bones[bone.name], root_frame)) for bone in arm.data.bones]


def _preview_for(arm, context) -> _ArmaturePreview | None:
    settings = arm.data.thyllore_spring
    spring = spring_settings_to_edit(settings)
    spring_key = repr(spring)
    rest_locals = _rest_locals(arm, _root_frame(arm, context.scene.unit_settings.scale_length))
    rest_key = repr([bone.name for bone in arm.data.bones]) + repr(rest_locals)

    cached = _previews.get(arm.name)
    if cached is not None and cached.matches(spring_key, rest_key):
        return cached

    preview = tml.SpringPreview(spring, armature_bone_specs(context, arm), rest_locals)
    cached = _ArmaturePreview(preview, spring_key, rest_key)
    _previews[arm.name] = cached
    return cached


def _step_armature(arm, context) -> None:
    cached = _preview_for(arm, context)
    if cached is None:
        return
    scene = context.scene
    animated = _animated_locals(arm, _root_frame(arm, scene.unit_settings.scale_length))

    frame = scene.frame_current
    if cached.last_frame is None or frame != cached.last_frame + 1:
        cached.preview.reset(animated)
    cached.last_frame = frame

    dt = scene.render.fps_base / scene.render.fps
    for joint in cached.preview.step(animated, dt):
        arm.pose.bones[joint["bone"]].matrix_basis = Matrix(joint["basis"])


def _previewed_armatures(scene):
    for obj in scene.objects:
        if obj.type != "ARMATURE":
            continue
        settings = obj.data.thyllore_spring
        if settings.preview_swing and len(settings.chains) > 0:
            yield obj


@persistent
def step_swing_preview(scene, _depsgraph=None) -> None:
    if not _TML_AVAILABLE:
        return
    for arm in _previewed_armatures(scene):
        try:
            _step_armature(arm, bpy.context)
        except (RuntimeError, ValueError, KeyError):
            _debuglog.log_exception(f"swing preview failed on {arm.name}")
            invalidate_preview(arm.name)


def register() -> None:
    if step_swing_preview not in bpy.app.handlers.frame_change_post:
        bpy.app.handlers.frame_change_post.append(step_swing_preview)


def unregister() -> None:
    if step_swing_preview in bpy.app.handlers.frame_change_post:
        bpy.app.handlers.frame_change_post.remove(step_swing_preview)
    _previews.clear()
