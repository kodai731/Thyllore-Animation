"""Keep every bound mesh's submesh role list in step with its material slots.

Mirrors the engine's ``refresh_submesh_roles``: a material that has a row keeps it, a new
material gets a guessed row, a removed material loses its row. Runs after every depsgraph
update and file load so the list is filled as soon as a model is imported."""
from __future__ import annotations

import bpy
from bpy.app.handlers import persistent

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from .operators import bound_meshes


def material_names(mesh_obj) -> list[str]:
    return [slot.material.name for slot in mesh_obj.material_slots if slot.material is not None]


def add_guessed_role(submesh, mesh_name: str, material: str) -> None:
    item = submesh.roles.add()
    item.material = material
    item.role = tml.guess_submesh_role(mesh_name, material)
    item.source = "guessed"


def refresh_submesh_roles(mesh_obj) -> int:
    """Add guessed rows for new materials and drop rows of removed ones; returns the row count changed."""
    submesh = mesh_obj.thyllore_submesh
    materials = material_names(mesh_obj)
    listed = [item.material for item in submesh.roles]

    changed = 0
    for index in reversed(range(len(listed))):
        if listed[index] not in materials:
            submesh.roles.remove(index)
            changed += 1
    for material in materials:
        if material not in listed:
            add_guessed_role(submesh, mesh_obj.name, material)
            changed += 1
    return changed


def bound_mesh_objects():
    for obj in bpy.data.objects:
        if obj.type == "ARMATURE":
            yield from bound_meshes(obj)


@persistent
def refresh_all_submesh_roles(*_args) -> None:
    if not _TML_AVAILABLE:
        return
    for mesh_obj in bound_mesh_objects():
        refresh_submesh_roles(mesh_obj)


def register() -> None:
    for handlers in (bpy.app.handlers.depsgraph_update_post, bpy.app.handlers.load_post):
        if refresh_all_submesh_roles not in handlers:
            handlers.append(refresh_all_submesh_roles)


def unregister() -> None:
    for handlers in (bpy.app.handlers.depsgraph_update_post, bpy.app.handlers.load_post):
        if refresh_all_submesh_roles in handlers:
            handlers.remove(refresh_all_submesh_roles)
