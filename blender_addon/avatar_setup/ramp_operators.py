"""Spring ramp operators (thyllore.apply_ramp, thyllore.load_ramp_from_chain, thyllore.use_as_ramp_end)."""
from __future__ import annotations

import bpy

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from ..common.spring_properties import (
    chain_with_joint,
    copy_joint_values,
    disabled_chain_names,
    joint_for_bone,
    joint_values,
    restore_disabled_chains,
    spring_settings_to_edit,
    write_sidecar_spring,
)
from .operators import armature_bone_specs, target_armature


def active_bone_name(context):
    """Get the name of the active pose bone, falling back to arm.data.bones.active."""
    bone = context.active_pose_bone
    if bone is not None:
        return bone.bone.name
    arm = target_armature(context)
    if arm is not None and arm.data.bones.active is not None:
        return arm.data.bones.active.name
    return None


def _target_chain_names(settings, spring, bones, bone_name, scope):
    """Determine which chain names to apply ramp to based on active bone and scope."""
    chain = chain_with_joint(settings, bone_name)
    if chain is not None:
        return tml.chains_in_scope(spring, bones, chain.name, scope)
    return tml.chains_under_bone(spring, bones, bone_name)


class THYLLORE_OT_ApplyRamp(bpy.types.Operator):
    """Apply ramp interpolation to spring chains from form root/tip values."""

    bl_idname = "thyllore.apply_ramp"
    bl_label = "Apply Ramp"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return _TML_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        arm = target_armature(context)
        settings = arm.data.thyllore_spring
        form = context.window_manager.thyllore_ramp

        bone_name = active_bone_name(context)
        if bone_name is None:
            self.report({"ERROR"}, "No active pose bone")
            return {"CANCELLED"}

        spring = spring_settings_to_edit(settings)
        bones = armature_bone_specs(context, arm)

        try:
            names = _target_chain_names(settings, spring, bones, bone_name, form.scope)
        except ValueError as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}

        if not names:
            self.report({"WARNING"}, f"No chains found for bone '{bone_name}'")
            return {"CANCELLED"}

        disabled = disabled_chain_names(settings)

        try:
            output = tml.apply_ramp(
                spring,
                bones,
                names,
                {
                    "root": joint_values(form.root),
                    "tip": joint_values(form.tip),
                    "shape": form.shape,
                },
            )
        except ValueError as e:
            self.report({"ERROR"}, f"apply_ramp failed: {e}")
            return {"CANCELLED"}

        write_sidecar_spring(settings, output)
        restore_disabled_chains(settings, disabled)

        joint_count = sum(len(c.joints) for c in settings.chains if c.name in names)
        self.report({"INFO"}, f"Applied ramp to {len(names)} chains ({joint_count} joints)")
        return {"FINISHED"}


class THYLLORE_OT_LoadRampFromChain(bpy.types.Operator):
    """Load ramp root/tip from the active chain's first/last joint."""

    bl_idname = "thyllore.load_ramp_from_chain"
    bl_label = "Load Ramp From Chain"
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

        chain = chains[index]
        if len(chain.joints) == 0:
            self.report({"ERROR"}, "Chain has no joints")
            return {"CANCELLED"}

        form = context.window_manager.thyllore_ramp

        copy_joint_values(chain.joints[0], form.root)
        copy_joint_values(chain.joints[-1], form.tip)

        self.report({"INFO"}, f"Loaded ramp from chain '{chain.name}'")
        return {"FINISHED"}


class THYLLORE_OT_UseAsRampEnd(bpy.types.Operator):
    """Use active bone's joint as ramp root or tip."""

    bl_idname = "thyllore.use_as_ramp_end"
    bl_label = "Use As Ramp End"
    bl_options = {"REGISTER", "UNDO"}

    end: bpy.props.EnumProperty(
        name="End",
        items=[("root", "Root", ""), ("tip", "Tip", "")],
        default="root",
    )

    @classmethod
    def poll(cls, context):
        return target_armature(context) is not None

    def execute(self, context):
        arm = target_armature(context)
        settings = arm.data.thyllore_spring

        bone_name = active_bone_name(context)
        if bone_name is None:
            self.report({"ERROR"}, "No active pose bone")
            return {"CANCELLED"}

        source_joint = joint_for_bone(settings, bone_name)
        if source_joint is None:
            self.report({"ERROR"}, f"No spring joint found for bone '{bone_name}'")
            return {"CANCELLED"}

        form = context.window_manager.thyllore_ramp
        target = form.root if self.end == "root" else form.tip

        copy_joint_values(source_joint, target)

        self.report({"INFO"}, f"Copied joint from '{bone_name}' to ramp {self.end}")
        return {"FINISHED"}


OPERATORS = (
    THYLLORE_OT_ApplyRamp,
    THYLLORE_OT_LoadRampFromChain,
    THYLLORE_OT_UseAsRampEnd,
)


def register() -> None:
    for cls in OPERATORS:
        bpy.utils.register_class(cls)


def unregister() -> None:
    for cls in reversed(OPERATORS):
        bpy.utils.unregister_class(cls)
