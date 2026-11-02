"""Spring overlay draw handler: the hit radii of every chain and the collider shapes.

Follows Blender's overlay convention: everything is drawn regardless of selection, the
active bone's chains are highlighted, and the viewport's Show Overlays switch hides it all.
"""
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

from ..common.spring_gizmo import capsule_segments, wire_sphere_segments
from ..common.spring_properties import spring_settings_to_edit
from .operators import armature_bone_specs, target_armature

_handle: int | None = None
INACTIVE_ALPHA = 0.35
ACTIVE_ALPHA = 1.0


def _active_bone_index(arm):
    """Index of the active data bone, or None."""
    active = arm.data.bones.active
    return arm.data.bones.find(active.name) if active is not None else None


def _radius_segments(chains, arm, scale_length):
    """Hit-radius wire spheres of every joint of the given chains."""
    segments = []
    circle_segments = _STYLE["chain_radius_segments"]
    for chain in chains:
        for joint in chain.joints:
            pb = arm.pose.bones.get(joint.bone)
            if pb is not None:
                head = tuple(arm.matrix_world @ pb.head)
                radius = joint.hit_radius / scale_length
                segments.extend(wire_sphere_segments(head, radius, circle_segments))
    return segments


def _highlighted_chain_names(context, arm, settings) -> set[str]:
    """Names of the chains holding the active bone."""
    bones = armature_bone_specs(context, arm)
    spring = spring_settings_to_edit(settings)
    return set(tml.chains_with_radius(spring, bones, _active_bone_index(arm), "all"))


def _draw_chain_radii(context, shader, arm, settings):
    highlighted = _highlighted_chain_names(context, arm, settings)
    scale_length = context.scene.unit_settings.scale_length
    color = tuple(_STYLE["chain_color"])

    inactive = [chain for chain in settings.chains if chain.name not in highlighted]
    active = [chain for chain in settings.chains if chain.name in highlighted]
    _draw_segments(shader, color + (INACTIVE_ALPHA,), _radius_segments(inactive, arm, scale_length))
    _draw_segments(shader, color + (ACTIVE_ALPHA,), _radius_segments(active, arm, scale_length))


def _collider_segments(settings, arm):
    """Wire shapes of every collider attached to a bone."""
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
    return segments


def _draw_segments(shader, color_rgba, segments):
    """Draw line segments as a single batch."""
    from gpu_extras.batch import batch_for_shader

    if not segments:
        return
    verts = [v for seg in segments for v in seg]
    batch = batch_for_shader(shader, "LINES", {"pos": verts})
    shader.bind()
    shader.uniform_float("color", color_rgba)
    batch.draw(shader)


def _overlays_shown(context) -> bool:
    space = context.space_data
    return space is not None and space.type == "VIEW_3D" and space.overlay.show_overlays


def _draw():
    """Draw handler callback; chain bones themselves are Blender's own stick bones."""
    import gpu

    context = bpy.context
    if not _overlays_shown(context):
        return
    arm = target_armature(context)
    if arm is None:
        return

    settings = arm.data.thyllore_spring
    shader = gpu.shader.from_builtin("UNIFORM_COLOR")
    gpu.state.blend_set("ALPHA")

    if settings.show_chain_radii:
        _draw_chain_radii(context, shader, arm, settings)
    if settings.show_collider_gizmos:
        collider_color = tuple(_STYLE["collider_color"]) + (ACTIVE_ALPHA,)
        _draw_segments(shader, collider_color, _collider_segments(settings, arm))

    gpu.state.blend_set("NONE")


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
