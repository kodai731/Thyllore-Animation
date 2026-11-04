"""Submesh role operators (thyllore.guess_submesh_roles)."""
from __future__ import annotations

import bpy

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from ..common.logged_report import report
from .operators import bound_meshes, target_armature
from .submesh_roles import refresh_submesh_roles


class THYLLORE_OT_GuessSubmeshRoles(bpy.types.Operator):
    """Guess the role of every material again, keeping the rows the user set by hand."""

    bl_idname = "thyllore.guess_submesh_roles"
    bl_label = "Guess Submesh Roles"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return _TML_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        arm = target_armature(context)
        if arm is None:
            report(self, "ERROR", "No target armature")
            return {"CANCELLED"}

        meshes = bound_meshes(arm)
        if not meshes:
            report(self, "WARNING", "No bound meshes found")
            return {"CANCELLED"}

        count = 0
        for mesh_obj in meshes:
            refresh_submesh_roles(mesh_obj)
            for item in mesh_obj.thyllore_submesh.roles:
                if item.source == "user":
                    continue
                item.role = tml.guess_submesh_role(mesh_obj.name, item.material)
                item.source = "guessed"
                count += 1

        report(self, "INFO", f"Guessed roles for {count} materials")
        return {"FINISHED"}


OPERATORS = (
    THYLLORE_OT_GuessSubmeshRoles,
)


def register() -> None:
    for cls in OPERATORS:
        bpy.utils.register_class(cls)


def unregister() -> None:
    for cls in reversed(OPERATORS):
        bpy.utils.unregister_class(cls)
