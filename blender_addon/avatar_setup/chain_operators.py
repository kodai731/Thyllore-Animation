"""Spring chain add/remove operators (thyllore.add_spring_chains, thyllore.remove_spring_chain)."""
from __future__ import annotations

import bpy

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from ..common.spring_properties import (
    disabled_chain_names,
    restore_disabled_chains,
    spring_settings_to_edit,
    write_sidecar_spring,
)
from .operators import armature_bone_specs, target_armature


class THYLLORE_OT_AddSpringChains(bpy.types.Operator):
    """Add spring chains from bones matching a prefix."""

    bl_idname = "thyllore.add_spring_chains"
    bl_label = "Add Spring Chains"
    bl_options = {"REGISTER", "UNDO"}

    prefix: bpy.props.StringProperty(default="hair_")

    @classmethod
    def poll(cls, context):
        return _TML_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        if not _TML_AVAILABLE:
            self.report({"ERROR"}, "thyllore_ml_core not available")
            return {"CANCELLED"}

        arm = target_armature(context)
        settings = arm.data.thyllore_spring

        try:
            sidecar = spring_settings_to_edit(settings)
            bones = armature_bone_specs(context, arm)
            output = tml.chains_from_prefix(sidecar, bones, self.prefix)
        except ValueError as e:
            self.report({"ERROR"}, f"chains_from_prefix failed: {e}")
            return {"CANCELLED"}

        disabled = disabled_chain_names(settings)

        write_sidecar_spring(settings, output)
        restore_disabled_chains(settings, disabled)

        collection = arm.data.collections.get("Thyllore Chains")
        if collection is None:
            collection = arm.data.collections.new("Thyllore Chains")

        for chain in settings.chains:
            for joint in chain.joints:
                bone = arm.data.bones.get(joint.bone)
                if bone is not None:
                    collection.assign(bone)

        chain_count = len(output["chains"])
        self.report({"INFO"}, f"Added {chain_count} chains with prefix '{self.prefix}'")
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
    THYLLORE_OT_AddSpringChains,
    THYLLORE_OT_RemoveSpringChain,
)


def register() -> None:
    for cls in OPERATORS:
        bpy.utils.register_class(cls)


def unregister() -> None:
    for cls in reversed(OPERATORS):
        bpy.utils.unregister_class(cls)
