"""Hidden skin mask: hide skin triangles covered by garments on every mesh bound to an armature, or show them again."""
from __future__ import annotations

import bpy

try:
    import thyllore_avatar_api as avatar_api  # type: ignore

    _AVATAR_API_AVAILABLE = True
except ImportError:
    avatar_api = None  # type: ignore
    _AVATAR_API_AVAILABLE = False

from .. import _debuglog
from ..common.mesh_data import mesh_arrays
from ..common.submesh_mask import mask_mesh_input, to_loop_triangle_indices
from ..common.rig_snapshot import bound_meshes


def armature_object(armature_data):
    """The object that uses ``armature_data``, or None."""
    return next(
        (obj for obj in bpy.data.objects if obj.type == "ARMATURE" and obj.data is armature_data),
        None,
    )


def is_hidden_mask_applied(arm) -> bool:
    return any(
        len(item.get("triangles", [])) > 0
        for mesh_obj in bound_meshes(arm)
        for item in mesh_obj.thyllore_submesh.hidden_triangles
    )


def clear_hidden_mask(arm) -> None:
    for mesh_obj in bound_meshes(arm):
        mesh_obj.thyllore_submesh.hidden_triangles.clear()


def apply_hidden_mask(arm, params: dict, scale_length: float) -> int:
    """Recompute the hidden triangles of every skin material from the garment materials; returns the hidden count."""
    if not _AVATAR_API_AVAILABLE:
        return 0
    skin_materials, garment_materials = materials_by_role(bound_meshes(arm))
    if not garment_materials:
        _debuglog.log_report("WARNING", "hidden_mask", "No garment materials found — nothing hidden")
        clear_hidden_mask(arm)
        return 0

    garment_inputs = []
    for mesh_obj, material_index in garment_materials:
        garment_input, loop_indices = mask_mesh_input(*mesh_arrays(mesh_obj), material_index, scale_length)
        if loop_indices:
            garment_inputs.append(garment_input)

    clear_hidden_mask(arm)
    total_hidden = 0
    for mesh_obj, material_index in skin_materials:
        body_input, loop_indices = mask_mesh_input(*mesh_arrays(mesh_obj), material_index, scale_length)
        if not loop_indices:
            continue
        hidden_local = avatar_api.find_hidden_triangles({"body": body_input, "garments": garment_inputs, "params": params})
        loop_hidden = to_loop_triangle_indices(hidden_local, loop_indices)
        if not loop_hidden:
            continue

        item = mesh_obj.thyllore_submesh.hidden_triangles.add()
        item.material = mesh_obj.material_slots[material_index].material.name
        item["triangles"] = loop_hidden
        total_hidden += len(loop_hidden)

    _debuglog.log_report("INFO", "hidden_mask", f"Hidden {total_hidden} skin triangles")
    return total_hidden


def materials_by_role(meshes) -> tuple[list[tuple], list[tuple]]:
    """(skin, garment) lists of (mesh_obj, material_index) from the submesh roles."""
    skin_materials: list[tuple] = []
    garment_materials: list[tuple] = []
    for mesh_obj in meshes:
        roles = {item.material: item.role for item in mesh_obj.thyllore_submesh.roles}
        for material_index, slot in enumerate(mesh_obj.material_slots):
            if slot.material is None:
                continue
            role = roles.get(slot.material.name)
            if role == "skin":
                skin_materials.append((mesh_obj, material_index))
            elif role == "garment":
                garment_materials.append((mesh_obj, material_index))
    return skin_materials, garment_materials
