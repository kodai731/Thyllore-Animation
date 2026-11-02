"""Reset one stage of the chain generation on a submesh so the next Generate computes it again."""
from __future__ import annotations

import bpy

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from ..common.chain_bones import chain_of_membership_group
from ..common.spring_properties import generated_chain_bones, remove_chains_generated_on
from .chain_generation import remove_bones, remove_vertex_groups, resolve_submesh
from .operators import target_armature

RESET_STAGES = (
    ("assign", "Assign", "Forget which vertices belong to which chain, with the chain bones, weights and spring values"),
    ("place", "Place", "Remove the chain bones with their weights and spring values; the vertex assignment stays"),
    ("weight", "Weight", "Clear the chain bone weights; the assignment and the bones stay"),
)


def _clear_vertex_groups(obj, names: list[str]) -> None:
    every_vertex = [v.index for v in obj.data.vertices]
    for name in names:
        group = obj.vertex_groups.get(name)
        if group is not None:
            group.remove(every_vertex)


def _membership_groups_of_submesh(obj, role: str, target: str) -> list[str]:
    chains = {chain_of_membership_group(vg.name): vg.name for vg in obj.vertex_groups}
    chains.pop(None, None)
    return [chains[name] for name in tml.submesh_chain_names(role, target, list(chains))]


class THYLLORE_OT_ResetChainGeneration(bpy.types.Operator):
    """Reset a stage of the chain generation on one submesh; a higher stage also resets the stages below it"""

    bl_idname = "thyllore.reset_chain_generation"
    bl_label = "Reset"
    bl_options = {"REGISTER", "UNDO"}

    mesh: bpy.props.StringProperty(name="Mesh", options={"HIDDEN"})
    material: bpy.props.StringProperty(name="Material", options={"HIDDEN"})
    stage: bpy.props.EnumProperty(name="Stage", items=RESET_STAGES, default="place")

    @classmethod
    def poll(cls, context):
        return _TML_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        arm = target_armature(context)
        try:
            obj, role, _ = resolve_submesh(arm, self.mesh, self.material)
        except ValueError as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        settings = arm.data.thyllore_spring
        origin = {"mesh": obj.name, "material": self.material, "role": role}
        joint_bones = [bone for bones in generated_chain_bones(settings, origin).values() for bone in bones]

        if self.stage == "weight":
            _clear_vertex_groups(obj, joint_bones)
            self.report({"INFO"}, f"Cleared the weights of {len(joint_bones)} chain bones")
            return {"FINISHED"}

        remove_chains_generated_on(settings, origin)
        remove_bones(context, arm, joint_bones)
        remove_vertex_groups(obj, joint_bones)
        note = f"Removed {len(joint_bones)} chain bones"
        if self.stage == "assign":
            groups = _membership_groups_of_submesh(obj, role, f"{obj.name}/{self.material}")
            remove_vertex_groups(obj, groups)
            note += f" and {len(groups)} chain assignments"
        self.report({"INFO"}, note)
        return {"FINISHED"}


def register() -> None:
    bpy.utils.register_class(THYLLORE_OT_ResetChainGeneration)


def unregister() -> None:
    bpy.utils.unregister_class(THYLLORE_OT_ResetChainGeneration)
