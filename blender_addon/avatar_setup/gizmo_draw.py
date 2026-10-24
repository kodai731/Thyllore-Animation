"""Spring chain gizmo draw handler."""
from __future__ import annotations

from mathutils import Vector

import bpy

try:
    import thyllore_ml_core as tml  # type: ignore

    _STYLE = tml.spring_gizmo_style()
    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _STYLE = None
    _TML_AVAILABLE = False

from ..common.spring_gizmo import capsule_segments, polyline_segments, wire_sphere_segments
from ..common.spring_properties import spring_settings_to_edit
from .operators import armature_bone_specs, target_armature

_handle: int | None = None


def _selected_bone_indices(arm):
    """Bone indices of selected pose bones."""
    return [arm.data.bones.find(pose_bone.name) for pose_bone in arm.pose.bones if pose_bone.select]


def _active_bone_index(arm):
    """Index of the active data bone, or None."""
    active = arm.data.bones.active
    return arm.data.bones.find(active.name) if active is not None else None


def _chain_segments(chains_by_name, chain_names, arm):
    """Build (color, [(start, end), ...]) pairs for chain lines."""
    if not chain_names:
        return None
    color = tuple(_STYLE["chain_color"])
    segments = []
    for chain_name in chain_names:
        chain = chains_by_name.get(chain_name)
        if chain is None:
            continue
        points = []
        for joint in chain.joints:
            pb = arm.pose.bones.get(joint.bone)
            if pb is not None:
                points.append(tuple(arm.matrix_world @ pb.head))
        if points:
            last_joint = chain.joints[-1]
            pb = arm.pose.bones.get(last_joint.bone)
            if pb is not None:
                points.append(tuple(arm.matrix_world @ pb.tail))
        segments.extend(polyline_segments(points))
    return (color, segments)


def _radius_segments(chains_by_name, chain_names, arm, scale_length):
    """Build (color, [(start, end), ...]) pairs for hit-radius wire spheres."""
    if not chain_names:
        return None
    color = tuple(_STYLE["chain_color"])
    segments = []
    circle_segments = _STYLE["chain_radius_segments"]
    for chain_name in chain_names:
        chain = chains_by_name.get(chain_name)
        if chain is None:
            continue
        for joint in chain.joints:
            pb = arm.pose.bones.get(joint.bone)
            if pb is not None:
                head = tuple(arm.matrix_world @ pb.head)
                radius = joint.hit_radius / scale_length
                segments.extend(wire_sphere_segments(head, radius, circle_segments))
    return (color, segments)


def _collider_segments(settings, arm):
    """Build (color, [(start, end), ...]) pairs for collider gizmos."""
    if not settings.show_collider_gizmos:
        return None
    color = tuple(_STYLE["collider_color"])
    segments = []
    circle_segments = _STYLE["circle_segments"]
    for collider in settings.colliders:
        pb = arm.pose.bones.get(collider.bone)
        if pb is None:
            continue
        world_matrix = arm.matrix_world @ pb.matrix
        scale_x = world_matrix.to_scale()[0]
        radius = collider.radius * scale_x
        offset = Vector(collider.offset)
        head = tuple(world_matrix @ offset)
        if collider.shape == "capsule" and collider.has_tail:
            tail = Vector(collider.tail)
            tail_world = tuple(world_matrix @ tail)
            segments.extend(capsule_segments(head, tail_world, radius, circle_segments))
        else:
            segments.extend(wire_sphere_segments(head, radius, circle_segments))
    return (color, segments)


def _draw_segments(shader, color, segments):
    """Draw line segments as a single batch."""
    from gpu_extras.batch import batch_for_shader

    if not segments:
        return
    verts = [v for seg in segments for v in seg]
    batch = batch_for_shader(shader, "LINES", {"pos": verts})
    shader.bind()
    shader.uniform_float("color", color + (1.0,))
    batch.draw(shader)


def _draw():
    """Draw handler callback: draw chain lines, radii, and colliders."""
    import gpu

    context = bpy.context
    arm = target_armature(context)
    if arm is None:
        return

    settings = arm.data.thyllore_spring
    bones = armature_bone_specs(context, arm)
    selected = _selected_bone_indices(arm)
    active = _active_bone_index(arm)

    spring = spring_settings_to_edit(settings)
    scale_length = context.scene.unit_settings.scale_length
    chains_result = tml.chains_to_draw(spring, bones, selected, active, settings.gizmo_mode)

    shader = gpu.shader.from_builtin("UNIFORM_COLOR")
    chains_by_name = {chain.name: chain for chain in settings.chains}

    pair = _chain_segments(chains_by_name, chains_result.get("lines", []), arm)
    if pair is not None:
        _draw_segments(shader, *pair)

    pair = _radius_segments(chains_by_name, chains_result.get("radii", []), arm, scale_length)
    if pair is not None:
        _draw_segments(shader, *pair)

    pair = _collider_segments(settings, arm)
    if pair is not None:
        _draw_segments(shader, *pair)


def register() -> None:
    """Register the draw handler (no-op if wheel is unavailable)."""
    global _handle
    if not _TML_AVAILABLE:
        return
    _handle = bpy.types.SpaceView3D.draw_handler_add(_draw, (), "WINDOW", "POST_VIEW")


def unregister() -> None:
    """Unregister the draw handler."""
    global _handle
    if _handle is not None:
        bpy.types.SpaceView3D.draw_handler_remove(_handle, "WINDOW")
        _handle = None
