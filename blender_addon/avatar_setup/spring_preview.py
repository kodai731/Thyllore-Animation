"""Live swing preview: the engine's spring solver drives the chain bones on every frame change.

Blender's parent-relative bone transforms are handed to the wheel as-is; only root bones carry the
armature's world transform converted to engine axes and metres, so the solver works in engine space
and the parent-relative rotations it returns go straight back into ``matrix_basis``."""
from __future__ import annotations

import bpy
from bpy.app.handlers import persistent
from mathutils import Matrix, Quaternion

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from .. import _debuglog
from ..common.coordinates import C
from ..common.spring_properties import spring_settings_to_edit
from .operators import armature_bone_specs

ENGINE_AXES = Matrix(C)

_previews: dict[str, "_ArmaturePreview"] = {}


class _ArmaturePreview:
    def __init__(self, preview, spring_key: str, bone_count: int):
        self.preview = preview
        self.spring_key = spring_key
        self.bone_count = bone_count
        self.last_frame: int | None = None

    def matches(self, spring_key: str, bone_count: int) -> bool:
        return self.spring_key == spring_key and self.bone_count == bone_count


def invalidate_preview(armature_name: str) -> None:
    _previews.pop(armature_name, None)


def _root_frame(arm, scale_length: float) -> Matrix:
    return Matrix.Scale(scale_length, 4) @ ENGINE_AXES @ arm.matrix_world


def _matrix_rows(matrix: Matrix) -> list[list[float]]:
    return [list(row) for row in matrix]


def _rest_locals(arm, root_frame: Matrix) -> list[list[list[float]]]:
    locals_ = []
    for bone in arm.data.bones:
        if bone.parent is None:
            locals_.append(_matrix_rows(root_frame @ bone.matrix_local))
        else:
            locals_.append(_matrix_rows(bone.parent.matrix_local.inverted() @ bone.matrix_local))
    return locals_


def _animated_local(pose_bone, root_frame: Matrix) -> Matrix:
    if pose_bone.parent is None:
        return root_frame @ pose_bone.matrix
    return pose_bone.parent.matrix.inverted() @ pose_bone.matrix


def _animated_locals(arm, root_frame: Matrix) -> list[Matrix]:
    return [_animated_local(arm.pose.bones[bone.name], root_frame) for bone in arm.data.bones]


def _rest_relative_to_parent(bone) -> Matrix:
    if bone.parent is None:
        return bone.matrix_local.copy()
    return bone.parent.matrix_local.inverted() @ bone.matrix_local


def _write_joint_rotation(arm, animated_local: Matrix, bone_name: str, rotation) -> None:
    pose_bone = arm.pose.bones[bone_name]
    translation = Matrix.Translation(animated_local.to_translation())
    scale = Matrix.Diagonal(animated_local.to_scale()).to_4x4()
    swung_local = translation @ Quaternion(rotation).to_matrix().to_4x4() @ scale
    pose_bone.matrix_basis = _rest_relative_to_parent(pose_bone.bone).inverted() @ swung_local


def _preview_for(arm, context) -> _ArmaturePreview | None:
    settings = arm.data.thyllore_spring
    spring = spring_settings_to_edit(settings)
    spring_key = repr(spring)
    bone_count = len(arm.data.bones)

    cached = _previews.get(arm.name)
    if cached is not None and cached.matches(spring_key, bone_count):
        return cached

    scale_length = context.scene.unit_settings.scale_length
    preview = tml.SpringPreview(spring, armature_bone_specs(context, arm), _rest_locals(arm, _root_frame(arm, scale_length)))
    cached = _ArmaturePreview(preview, spring_key, bone_count)
    _previews[arm.name] = cached
    return cached


def _step_armature(arm, context) -> None:
    cached = _preview_for(arm, context)
    if cached is None:
        return
    scene = context.scene
    root_frame = _root_frame(arm, scene.unit_settings.scale_length)
    animated = _animated_locals(arm, root_frame)
    rows = [_matrix_rows(local) for local in animated]

    frame = scene.frame_current
    if cached.last_frame is None or frame != cached.last_frame + 1:
        cached.preview.reset(rows)
    cached.last_frame = frame

    dt = scene.render.fps_base / scene.render.fps
    bone_index = {bone.name: index for index, bone in enumerate(arm.data.bones)}
    for joint in cached.preview.step(rows, dt):
        _write_joint_rotation(arm, animated[bone_index[joint["bone"]]], joint["bone"], joint["rotation"])


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
