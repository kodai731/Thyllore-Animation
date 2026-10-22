"""Spring chain add/remove operators (thyllore.add_chains_under_bone, thyllore.remove_spring_chain)."""
from __future__ import annotations

import bpy

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from ..common.bone_collections import assign_bones_to_chain_collection
from ..common.spring_properties import (
    disabled_chain_names,
    restore_disabled_chains,
    spring_settings_to_edit,
    write_sidecar_spring,
)
from .operators import armature_bone_specs, target_armature

def assign_chain_bone_collection(arm) -> None:
    """Assign all chain joints' bones to the 'Thyllore Chains' collection."""
    settings = arm.data.thyllore_spring
    bone_names = []
    for chain in settings.chains:
        for joint in chain.joints:
            bone_names.append(joint.bone)
    assign_bones_to_chain_collection(arm, bone_names)


class THYLLORE_OT_AddChainsUnderBone(bpy.types.Operator):
    """Add a spring chain for every run of non-humanoid bones under a root bone."""

    bl_idname = "thyllore.add_chains_under_bone"
    bl_label = "Add Chains Under Bone"
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
            self.report({"ERROR"}, "thyllore_ml_core not available")
            return {"CANCELLED"}

        arm = target_armature(context)
        settings = arm.data.thyllore_spring

        try:
            sidecar = spring_settings_to_edit(settings)
            bones = armature_bone_specs(context, arm)
            output = tml.add_chains_under_bone(sidecar, bones, self.root_bone, self.min_joints)
        except ValueError as e:
            self.report({"ERROR"}, f"add_chains_under_bone failed: {e}")
            return {"CANCELLED"}

        disabled = disabled_chain_names(settings)

        write_sidecar_spring(settings, output)
        restore_disabled_chains(settings, disabled)

        assign_chain_bone_collection(arm)

        chain_count = len(output["chains"])
        self.report({"INFO"}, f"{chain_count} chains under '{self.root_bone}'")
        return {"FINISHED"}


class THYLLORE_OT_RemoveSpringChain(bpy.types.Operator):
    """Remove the active spring chain."""

    bl_idname = "thyllore.remove_spring_chain"
    bl_label = "Remove Spring Chain"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return target_armature(context) is not None

    def execute(self, context):
        arm = target_armature(context)
        settings = arm.data.thyllore_spring
        chains = settings.chains
        index = settings.active_chain_index

        if index < 0 or index >= len(chains):
            self.report({"ERROR"}, "Active chain index out of range")
            return {"CANCELLED"}

        chains.remove(index)

        if settings.active_chain_index >= len(chains):
            settings.active_chain_index = max(0, len(chains) - 1)

        self.report({"INFO"}, "Removed spring chain")
        return {"FINISHED"}


OPERATORS = (
    THYLLORE_OT_AddChainsUnderBone,
    THYLLORE_OT_RemoveSpringChain,
)


def register() -> None:
    for cls in OPERATORS:
        bpy.utils.register_class(cls)


def unregister() -> None:
    for cls in reversed(OPERATORS):
        bpy.utils.unregister_class(cls)
