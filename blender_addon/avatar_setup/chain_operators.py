"""Spring chain add/remove operators (thyllore.use_existing_bones_as_springs, thyllore.remove_spring_chain)."""
from __future__ import annotations

import bpy

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from ..common.chain_bone_display import apply_chain_display
from ..common.spring_properties import (
    active_chain,
    spring_settings_to_edit,
    write_sidecar_spring,
)
from ..common.logged_report import report
from .chain_inputs import GENERATABLE_ROLES
from .chain_reset import remove_generated_chains
from .operators import armature_bone_specs, bound_meshes, target_armature


class THYLLORE_OT_UseExistingBonesAsSprings(bpy.types.Operator):
    """Register every run of existing non-humanoid bones under the root bone as a spring chain; adds no bones"""

    bl_idname = "thyllore.use_existing_bones_as_springs"
    bl_label = "Use Existing Bones as Springs"
    bl_options = {"REGISTER", "UNDO"}

    root_bone: bpy.props.StringProperty(name="Root Bone", default="Head")
    min_joints: bpy.props.IntProperty(
        name="Min Joints",
        default=2,
        min=1,
        description="Shortest run of bones under the root that becomes a chain",
    )

    @classmethod
    def poll(cls, context):
        return _TML_AVAILABLE and target_armature(context) is not None

    def invoke(self, context, event):
        active = getattr(context, "active_bone", None)
        if active is not None:
            self.root_bone = active.name
        return self.execute(context)

    def execute(self, context):
        if not _TML_AVAILABLE:
            report(self, "ERROR", "thyllore_ml_core not available")
            return {"CANCELLED"}

        arm = target_armature(context)
        settings = arm.data.thyllore_spring

        try:
            sidecar = spring_settings_to_edit(settings)
            bones = armature_bone_specs(context, arm)
            output = tml.use_existing_bones_as_springs(sidecar, bones, self.root_bone, self.min_joints)
        except ValueError as e:
            report(self, "ERROR", f"use_existing_bones_as_springs failed: {e}")
            return {"CANCELLED"}

        write_sidecar_spring(settings, output)

        apply_chain_display(arm)

        chain_count = len(output["chains"])
        report(self, "INFO", f"{chain_count} chains under '{self.root_bone}'")
        return {"FINISHED"}


def generatable_roles_of_active_mesh(context, arm) -> list:
    """The hair / garment role items of the active object when it is a submesh of the rig, else empty."""
    obj = context.active_object
    if obj is None or obj.type != "MESH" or obj not in bound_meshes(arm):
        return []
    return [item for item in obj.thyllore_submesh.roles if item.role in GENERATABLE_ROLES]


class THYLLORE_OT_RemoveSpringChain(bpy.types.Operator):
    """Remove every chain generated on the active submesh, or the chain selected in the Chains list"""

    bl_idname = "thyllore.remove_spring_chain"
    bl_label = "Remove Spring Chain"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        arm = target_armature(context)
        if arm is None:
            return False
        return bool(generatable_roles_of_active_mesh(context, arm)) or active_chain(arm.data.thyllore_spring) is not None

    def execute(self, context):
        arm = target_armature(context)
        roles = generatable_roles_of_active_mesh(context, arm)
        if roles:
            return self.remove_submesh_chains(context, arm, roles)
        return self.remove_listed_chain(arm)

    def remove_submesh_chains(self, context, arm, roles):
        obj = context.active_object
        chain_count = 0
        joint_count = 0
        for item in roles:
            chains, joints = remove_generated_chains(context, arm, obj, item.material, item.role)
            chain_count += chains
            joint_count += joints

        if chain_count == 0:
            report(self, "ERROR", f"No generated chain on {obj.name}")
            return {"CANCELLED"}
        report(self, "INFO", f"Removed {chain_count} chains ({joint_count} chain bones) on {obj.name}")
        return {"FINISHED"}

    def remove_listed_chain(self, arm):
        settings = arm.data.thyllore_spring
        chains = settings.chains
        if active_chain(settings) is None:
            report(self, "ERROR", "Select a submesh or a chain in the Chains list")
            return {"CANCELLED"}

        chains.remove(settings.active_chain_index)
        if settings.active_chain_index >= len(chains):
            settings.active_chain_index = max(0, len(chains) - 1)

        report(self, "INFO", "Removed spring chain")
        return {"FINISHED"}


OPERATORS = (
    THYLLORE_OT_UseExistingBonesAsSprings,
    THYLLORE_OT_RemoveSpringChain,
)


def register() -> None:
    for cls in OPERATORS:
        bpy.utils.register_class(cls)


def unregister() -> None:
    for cls in reversed(OPERATORS):
        bpy.utils.unregister_class(cls)
