"""Skin mesh input for the collider fit and the chain generation (engine metres, top influences, triangles)."""
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
    """Positions (engine metres), top bone influences and triangles of one skin mesh, as the wheel's FitSkin."""
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

    mesh_obj.data.calc_loop_triangles()
    triangles = [list(lt.vertices) for lt in mesh_obj.data.loop_triangles]

    return {"positions": positions, "influences": influences, "triangles": triangles}
