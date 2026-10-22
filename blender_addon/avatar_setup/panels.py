"""N-Panel UI for avatar setup (spring bones, submesh roles, colliders).

Lives in the View3D > Sidebar (N-Panel) under category "Thyllore Animation".
Two panels: the parent AvatarSetup panel and the Export sub-panel.
"""
from __future__ import annotations

import bpy
from bpy.types import Panel, UIList

from ..panels.main_panel import _CATEGORY
from .operators import target_armature
from .properties import SPRING_JOINT_PARAM_NAMES
from .ramp_operators import active_bone_name


class THYLLORE_UL_SpringChains(UIList):
    """UIList for spring bone chains: name, enabled checkbox, joint count."""

    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, prop_name):
        chain = item
        row = layout.row(align=True)
        row.prop(chain, "name", text="", emboss=False)
        row.prop(chain, "enabled", text="")
        row.label(text=f"{len(chain.joints)} joints")


class THYLLORE_PT_AvatarSetup(Panel):
    """Parent panel: shows the target armature name or a selection prompt."""

    bl_label = "Avatar Setup"
    bl_idname = "THYLLORE_PT_avatar_setup"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = _CATEGORY

    def draw(self, context):
        layout = self.layout
        arm = target_armature(context)
        if arm is None:
            layout.label(text="Select an armature or a skinned mesh", icon="ERROR")
            return
        layout.label(text=f"Armature: {arm.name}", icon="ARMATURE_DATA")


class THYLLORE_PT_AvatarSetupChains(Panel):
    """Chains sub-panel: chain list, joint editors, ramp form."""

    bl_label = "Chains"
    bl_idname = "THYLLORE_PT_avatar_setup_chains"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_parent_id = "THYLLORE_PT_avatar_setup"

    @staticmethod
    def _draw_chain_list(layout, arm):
        layout.operator("thyllore.add_spring_chains")
        layout.operator("thyllore.remove_spring_chain")
        settings = arm.data.thyllore_spring
        layout.template_list(
            "THYLLORE_UL_SpringChains", "", settings, "chains", settings, "active_chain_index"
        )

    @staticmethod
    def _draw_joints(layout, context, arm):
        settings = arm.data.thyllore_spring
        index = settings.active_chain_index
        chains = settings.chains
        if index < 0 or index >= len(chains):
            return
        chain = chains[index]
        if not SPRING_JOINT_PARAM_NAMES:
            return

        active_name = active_bone_name(context)
        joints = sorted(chain.joints, key=lambda joint: joint.bone != active_name)

        for joint in joints:
            box = layout.box()
            row = box.row()
            row.label(text=joint.bone or "(empty)")
            for name in SPRING_JOINT_PARAM_NAMES:
                row.prop(joint, name)

    @staticmethod
    def _draw_ramp(layout, context):
        if not SPRING_JOINT_PARAM_NAMES:
            return
        form = context.window_manager.thyllore_ramp
        box = layout.box()
        for end_name in ("root", "tip"):
            joint = getattr(form, end_name)
            row = box.row()
            row.label(text=end_name.capitalize())
            for name in SPRING_JOINT_PARAM_NAMES:
                row.prop(joint, name)

        box.prop(form, "shape")
        box.prop(form, "scope")
        box.operator("thyllore.load_ramp_from_chain")
        row = box.row(align=True)
        row.operator("thyllore.use_as_ramp_end", text="End=Root").end = "root"
        row.operator("thyllore.use_as_ramp_end", text="End=Tip").end = "tip"
        box.operator("thyllore.apply_ramp")

    def draw(self, context):
        layout = self.layout
        arm = target_armature(context)
        if arm is None:
            return
        self._draw_chain_list(layout, arm)
        self._draw_joints(layout, context, arm)
        self._draw_ramp(layout, context)


class THYLLORE_PT_AvatarSetupExport(Panel):
    """Export sub-panel: sidecar import/export buttons, collision mode, override count."""

    bl_label = "Export"
    bl_idname = "THYLLORE_PT_avatar_setup_export"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_parent_id = "THYLLORE_PT_avatar_setup"

    def draw(self, context):
        layout = self.layout
        arm = target_armature(context)

        layout.operator("thyllore.import_avatar_sidecar", icon="FILE_IMPORT")
        layout.operator("thyllore.export_for_unity", icon="FILE_EXPORT")

        if arm is not None:
            layout.prop(arm.data.thyllore_spring, "collision_mode")
            override_count = len(arm.data.thyllore_spring.humanoid_overrides)
            layout.label(text=f"Humanoid overrides: {override_count}")


UI_LISTS = (THYLLORE_UL_SpringChains,)

PANELS = (
    THYLLORE_PT_AvatarSetup,
    THYLLORE_PT_AvatarSetupChains,
    THYLLORE_PT_AvatarSetupExport,
)


def register() -> None:
    for cls in UI_LISTS:
        bpy.utils.register_class(cls)
    for cls in PANELS:
        bpy.utils.register_class(cls)


def unregister() -> None:
    for cls in reversed(PANELS):
        bpy.utils.unregister_class(cls)
    for cls in reversed(UI_LISTS):
        bpy.utils.unregister_class(cls)
