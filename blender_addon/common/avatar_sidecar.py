"""Pure Python helpers for sidecar export/import (bpy-independent)."""
from __future__ import annotations

from .coordinates import blender_to_engine_point


def bone_specs(names, parent_indices, world_heads, scale_length):
    """Build bone spec list from names, parent indices, and world-space head positions.

    Root bones have parent_index None (passed through as None); children have their
    parent's index in the bone list."""
    specs = []
    for name, parent_index, world_head in zip(names, parent_indices, world_heads):
        rest_position = [component * scale_length for component in blender_to_engine_point(world_head)]
        specs.append({
            "name": name,
            "parent": parent_index,
            "rest_position": list(rest_position),
        })
    return specs


def submesh_to_sidecar(mesh_name, submesh, material_loop_indices):
    """Build sidecar entries from a Blender submesh: (roles, hidden_triangles).

    Blender stores hidden triangles as ``mesh.loop_triangles`` indices; the sidecar numbers
    them inside the material's submesh, through ``material_loop_indices`` (see
    ``submesh_mask.material_loop_indices``). A material without a mapping is skipped."""
    roles = []
    for item in submesh.roles:
        roles.append({"mesh": mesh_name, "material": item.material, "role": item.role})
    hidden = []
    for item in submesh.hidden_triangles:
        loop_indices = material_loop_indices.get(item.material)
        if loop_indices is None:
            continue
        local_of_loop = {loop: local for local, loop in enumerate(loop_indices)}
        triangles = [local_of_loop[t] for t in item.get("triangles", []) if t in local_of_loop]
        if triangles:
            hidden.append({"mesh": mesh_name, "material": item.material, "triangles": triangles})
    return roles, hidden


def write_submesh_sidecar(submesh, mesh_name, roles, hidden, material_loop_indices):
    """Write sidecar submesh data into a Blender property group (idempotent).

    Sidecar triangle numbers are converted back to ``mesh.loop_triangles`` indices through
    ``material_loop_indices``; a material without a mapping is skipped."""
    submesh.roles.clear()
    for entry in roles:
        if entry["mesh"] != mesh_name:
            continue
        item = submesh.roles.add()
        item.material = entry["material"]
        item.role = entry["role"]
    submesh.hidden_triangles.clear()
    for entry in hidden:
        if entry["mesh"] != mesh_name:
            continue
        loop_indices = material_loop_indices.get(entry["material"])
        if loop_indices is None:
            continue
        triangles = [loop_indices[i] for i in entry.get("triangles", []) if i < len(loop_indices)]
        if not triangles:
            continue
        item = submesh.hidden_triangles.add()
        item.material = entry["material"]
        item.source = "user"
        item["triangles"] = triangles


def sidecar_stats(triangle_count, bone_count, material_count, mesh_count, skinned_mesh_count, morph_mesh_count, spring):
    """Build stats dict matching Rust AvatarStats fields."""
    chains = spring.get("chains", [])
    chain_count = len(chains)
    transform_count = sum(len(chain["joints"]) for chain in chains)
    collider_count = len(spring.get("colliders", []))
    return {
        "triangles": triangle_count,
        "bones": bone_count,
        "materials": material_count,
        "meshes": mesh_count,
        "skinned_meshes": skinned_mesh_count,
        "morph_meshes": morph_mesh_count,
        "spring_chains": chain_count,
        "spring_transforms": transform_count,
        "spring_colliders": collider_count,
        "texture_bytes": 0,
    }
