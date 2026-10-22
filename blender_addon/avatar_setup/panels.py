"""N-Panel UI for avatar setup (spring bones, submesh roles, colliders).

Lives in the View3D > Sidebar (N-Panel) under category "Thyllore Animation".
Two panels: the parent AvatarSetup panel and the Export sub-panel.
"""
from __future__ import annotations

import bpy
from bpy.types import Panel

from ..panels.main_panel import _CATEGORY
from .operators import _target_armature


class THYLLORE_PT_AvatarSetup(Panel):
    """Parent panel: shows the target armature name or a selection prompt."""

    bl_label = "Avatar Setup"
    bl_idname = "THYLLORE_PT_avatar_setup"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = _CATEGORY

    def draw(self, context):
        layout = self.layout
        arm = _target_armature(context)
        if arm is None:
            layout.label(text="Select an armature or a skinned mesh", icon="ERROR")
            return
        layout.label(text=f"Armature: {arm.name}", icon="ARMATURE_DATA")


class THYLLORE_PT_AvatarSetupExport(Panel):
    """Export sub-panel: sidecar import/export buttons, collision mode, override count."""

    bl_label = "Export"
    bl_idname = "THYLLORE_PT_avatar_setup_export"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_parent_id = "THYLLORE_PT_avatar_setup"

    def draw(self, context):
        layout = self.layout
        arm = _target_armature(context)

        layout.operator("thyllore.import_avatar_sidecar", icon="FILE_IMPORT")
        layout.operator("thyllore.export_for_unity", icon="FILE_EXPORT")

        if arm is not None:
            layout.prop(arm.data.thyllore_spring, "collision_mode")
            override_count = len(arm.data.thyllore_spring.humanoid_overrides)
            layout.label(text=f"Humanoid overrides: {override_count}")


PANELS = (
    THYLLORE_PT_AvatarSetup,
    THYLLORE_PT_AvatarSetupExport,
)


def register() -> None:
    for cls in PANELS:
        bpy.utils.register_class(cls)


def unregister() -> None:
    for cls in reversed(PANELS):
        bpy.utils.unregister_class(cls)
