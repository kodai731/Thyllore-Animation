"""Pure Python helpers for sidecar export/import (bpy-independent)."""
from __future__ import annotations

ATTACHMENT_TO_SIDECAR = {"AUTO": None, "CAP": "cap", "RING": "ring"}
ATTACHMENT_FROM_SIDECAR = {sidecar: enum for enum, sidecar in ATTACHMENT_TO_SIDECAR.items() if sidecar}


def attachment_to_sidecar(item) -> str | None:
    """The ``cap`` / ``ring`` a submesh row chose, None for Auto."""
    return ATTACHMENT_TO_SIDECAR[getattr(item, "attachment", "AUTO")]


def submesh_to_sidecar(mesh_name, submesh, material_loop_indices):
    """Build sidecar entries from a Blender submesh: (roles, hidden_triangles).

    Blender stores hidden triangles as ``mesh.loop_triangles`` indices; the sidecar numbers
    them inside the material's submesh, through ``material_loop_indices`` (see
    ``submesh_mask.material_loop_indices``). A material without a mapping is skipped."""
    roles = []
    for item in submesh.roles:
        entry = {"mesh": mesh_name, "material": item.material, "role": item.role}
        attachment = attachment_to_sidecar(item)
        if attachment is not None:
            entry["attachment"] = attachment
        roles.append(entry)
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
        item.attachment = ATTACHMENT_FROM_SIDECAR.get(entry.get("attachment"), "AUTO")
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
        item["triangles"] = triangles
