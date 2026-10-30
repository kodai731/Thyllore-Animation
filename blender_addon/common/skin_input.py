"""Skin mesh input for the collider fit and the chain generation (engine metres, top influences)."""
from __future__ import annotations

from . import coordinates
from .collider_fit import top_influences


def skin_role_meshes(meshes) -> list:
    """The bound meshes that carry a 'skin' submesh role."""
    return [
        mesh_obj
        for mesh_obj in meshes
        if "skin" in [item.role for item in mesh_obj.thyllore_submesh.roles]
    ]


def skin_input(mesh_obj, arm, scale_length: float) -> dict:
    """Positions (engine metres) and top bone influences of one skin mesh, as the wheel's FitSkin."""
    group_to_bone_local = {
        i: arm.data.bones.find(vg.name)
        for i, vg in enumerate(mesh_obj.vertex_groups)
        if arm.data.bones.find(vg.name) >= 0
    }

    positions = []
    influences = []
    for v in mesh_obj.data.vertices:
        world_pos = tuple(mesh_obj.matrix_world @ v.co)
        engine_pos = coordinates.blender_to_engine_point(world_pos)
        positions.append([c * scale_length for c in engine_pos])

        vertex_groups = [(g.group, g.weight) for g in v.groups]
        influences.append(top_influences(vertex_groups, group_to_bone_local))

    return {"positions": positions, "influences": influences}


def dominant_bone_positions(skins: list[dict], bone_index: int) -> list[list[float]]:
    """The skin positions whose heaviest influence is ``bone_index``."""
    return [
        position
        for skin in skins
        for position, influences in zip(skin["positions"], skin["influences"])
        if influences and influences[0][0] == bone_index
    ]
