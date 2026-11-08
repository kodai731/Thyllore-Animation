"""Live swing preview: the engine's spring solver drives the chain bones on every frame change.

This file only reads Blender's bone matrices and writes the solver's ``matrix_basis`` back; the
parent-relative locals, the rest-relative handling of the chain joints and the basis itself are the wheel's."""
from __future__ import annotations

import bpy
from bpy.app.handlers import persistent
from mathutils import Matrix

try:
    import thyllore_avatar_api as avatar_api  # type: ignore

    _AVATAR_API_AVAILABLE = True
except ImportError:
    avatar_api = None  # type: ignore
    _AVATAR_API_AVAILABLE = False

from .. import _debuglog
from ..common.rig_snapshot import bone_specs, matrix_rows, rest_matrices
from ..common.spring_properties import spring_settings_to_edit

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


def _pose_request(arm) -> dict:
    return {
        "armature_world": matrix_rows(arm.matrix_world),
        "pose_matrices": [matrix_rows(arm.pose.bones[bone.name].matrix) for bone in arm.data.bones],
    }


def _preview_for(arm, context) -> _ArmaturePreview | None:
    """The preview of the armature, rebuilt only when the spring settings or the rest pose changed; the bone
    specs derive from the rest pose, so they are read only on a rebuild."""
    settings = arm.data.thyllore_spring
    spring = spring_settings_to_edit(settings)
    spring_key = repr(spring)
    armature_world = matrix_rows(arm.matrix_world)
    rest_rows = rest_matrices(arm)
    scale_length = context.scene.unit_settings.scale_length
    bone_hierarchy = [(bone.name, bone.parent.name if bone.parent else None) for bone in arm.data.bones]
    rest_key = repr((armature_world, rest_rows, bone_hierarchy, scale_length))

    cached = _previews.get(arm.name)
    if cached is not None and cached.matches(spring_key, rest_key):
        return cached

    preview = avatar_api.SpringPreview({
        "spring": spring,
        "bones": bone_specs(arm, armature_world, rest_rows, scale_length),
        "armature_world": armature_world,
        "rest_matrices": rest_rows,
        "scale_length": scale_length,
    })
    cached = _ArmaturePreview(preview, spring_key, rest_key)
    _previews[arm.name] = cached
    return cached


def _step_armature(arm, context) -> None:
    cached = _preview_for(arm, context)
    if cached is None:
        return
    scene = context.scene
    pose = _pose_request(arm)

    frame = scene.frame_current
    if cached.last_frame is None or frame != cached.last_frame + 1:
        cached.preview.reset(pose)
    cached.last_frame = frame

    dt = scene.render.fps_base / scene.render.fps
    for joint in cached.preview.step({**pose, "dt": dt}):
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
    if not _AVATAR_API_AVAILABLE:
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
