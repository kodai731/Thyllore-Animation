"""Reset one stage of the chain generation on a submesh so the next Generate computes it again."""
from __future__ import annotations

import bpy

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from ..common.chain_bone_display import remove_chain_collections
from ..common.chain_bones import chain_of_membership_group
from ..common.spring_properties import generated_chain_bones, remove_chains_generated_on
from .chain_bone_edit import remove_bones, remove_vertex_groups
from .chain_inputs import resolve_submesh
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


def remove_generated_chains(context, arm, obj, material: str, role: str) -> tuple[int, int]:
    """Drop the chains generated on one submesh with their bones, weights and display; returns
    (chain count, joint bone count)."""
    settings = arm.data.thyllore_spring
    origin = {"mesh": obj.name, "material": material, "role": role}
    chain_bones = generated_chain_bones(settings, origin)
    joint_bones = [bone for bones in chain_bones.values() for bone in bones]

    remove_chains_generated_on(settings, origin)
    remove_bones(context, arm, joint_bones)
    remove_vertex_groups(obj, joint_bones)
    remove_chain_collections(arm, sorted(chain_bones))
    return len(chain_bones), len(joint_bones)


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

        if self.stage == "weight":
            chain_bones = generated_chain_bones(settings, origin)
            joint_bones = [bone for bones in chain_bones.values() for bone in bones]
            _clear_vertex_groups(obj, joint_bones)
            self.report({"INFO"}, f"Cleared the weights of {len(joint_bones)} chain bones")
            return {"FINISHED"}

        _, joint_count = remove_generated_chains(context, arm, obj, self.material, role)
        note = f"Removed {joint_count} chain bones"
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
