"""Generate chain bones on a hair or garment submesh and register them as spring chains."""
from __future__ import annotations

from dataclasses import dataclass

import bpy

try:
    import thyllore_avatar_api as avatar_api  # type: ignore

    _AVATAR_API_AVAILABLE = True
except ImportError:
    avatar_api = None  # type: ignore
    _AVATAR_API_AVAILABLE = False

from ..common.chain_bone_display import apply_chain_bone_display, remove_chain_collections
from ..common.chain_bones import (
    bone_specs,
    group_weights,
    mismatched_vertices,
    output_of_new_chains,
    output_to_blender,
    settings_from_properties,
)
from ..common.skin_input import skin_input, skin_role_meshes
from ..common.spring_properties import (
    generated_chain_bones,
    remove_chains_named,
    spring_settings_to_edit,
    write_sidecar_spring,
)
from .chain_bone_edit import (
    add_bones,
    remove_bones,
    remove_vertex_groups,
    update_hit_radii,
    write_covered_group,
    write_membership_groups,
    write_weights,
)
from .chain_inputs import (
    DEFAULT_ROOT_BY_ROLE,
    body_collider_inputs,
    engine_point,
    given_chains,
    humanoid_bone_indices,
    marked_covered_vertices,
    read_back_root,
    resolve_submesh,
    root_engine_position,
    submesh_tex_coords,
    submesh_triangles,
    vertex_group_weights,
)
from .collider_operators import default_fit_settings, fit_body_colliders
from .operators import armature_fit_bones, bound_meshes, target_armature
from .properties import CHAIN_GEN_UI_PARAMS


@dataclass
class GenerationRun:
    """One generation on one submesh: what was read from the rig before the wheel runs."""

    obj: object
    role: str
    target: str
    scale_length: float
    fit_bones: list
    root_name: str
    registered: dict
    marked: list
    given: list
    read_back: set
    skins: list
    fitted_note: str = ""


def _ensure_body_colliders(context, arm, settings, fit_bones) -> str:
    if avatar_api.has_fitted_body_colliders(spring_settings_to_edit(settings), fit_bones):
        return ""
    return f", fitted {fit_body_colliders(context, arm, default_fit_settings())} body colliders first"


def _read_run(context, arm, obj, role: str, material: str) -> GenerationRun:
    settings = arm.data.thyllore_spring
    generation = settings.chain_generation
    scale_length = context.scene.unit_settings.scale_length
    fit_bones = armature_fit_bones(context, arm)
    root_name = avatar_api.resolve_root_bone(fit_bones, generation.root_bone.strip() or DEFAULT_ROOT_BY_ROLE[role])
    fitted_note = _ensure_body_colliders(context, arm, settings, fit_bones)

    registered = generated_chain_bones(settings, {"mesh": obj.name, "material": material})
    given = given_chains(obj, arm, registered, scale_length)
    read_back = {chain["name"] for chain in given if "joints" in chain}
    if not generation.root_bone.strip():
        root_name = read_back_root(arm, registered, given) or root_name
    return GenerationRun(
        obj=obj,
        role=role,
        target=f"{obj.name}/{material}",
        scale_length=scale_length,
        fit_bones=fit_bones,
        root_name=root_name,
        registered=registered,
        marked=marked_covered_vertices(obj),
        given=given,
        read_back=read_back,
        skins=[skin_input(skin_obj, arm, scale_length) for skin_obj in skin_role_meshes(bound_meshes(arm))],
        fitted_note=fitted_note,
    )


def _generate(context, arm, run: GenerationRun, material_index: int) -> dict:
    settings = arm.data.thyllore_spring
    mesh = run.obj.data
    body_bones = armature_fit_bones(context, arm)
    attachment = {"AUTO": None, "CAP": "cap", "RING": "ring"}[settings.chain_generation.attachment]
    return avatar_api.generate_chains(
        [engine_point(run.obj, v.co, run.scale_length) for v in mesh.vertices],
        submesh_triangles(mesh, material_index),
        root_engine_position(arm, run.root_name, run.scale_length),
        run.target,
        settings_from_properties(CHAIN_GEN_UI_PARAMS, settings.chain_generation),
        run.skins,
        arm.data.bones.find(run.root_name),
        submesh_tex_coords(mesh),
        run.marked,
        body_collider_inputs(arm, settings, body_bones),
        humanoid_bone_indices(arm, settings, body_bones),
        body_bones,
        run.given,
        attachment=attachment,
    )


def _register_new_chains(context, arm, run: GenerationRun, new_chains: list[dict], head_capsule, attachment: str) -> None:
    settings = arm.data.thyllore_spring
    bones = armature_fit_bones(context, arm)
    spring = avatar_api.register_generated_chains(
        avatar_api.spring_colliders_to_model(spring_settings_to_edit(settings), bones),
        bones,
        run.skins,
        run.root_name,
        run.target,
        attachment,
        new_chains,
        head_capsule,
    )
    write_sidecar_spring(settings, avatar_api.spring_colliders_to_local(spring, bones))


def _apply_output(context, arm, run: GenerationRun, output: dict) -> dict:
    """Write the generated chains into the rig and the mesh; the counts the report names."""
    settings = arm.data.thyllore_spring
    obj = run.obj
    blender_output = output_to_blender(output, run.scale_length)
    specs = bone_specs(output_of_new_chains(blender_output, run.read_back), run.root_name)
    groups = group_weights(blender_output, run.root_name, [bone.name for bone in arm.data.bones])
    generated_names = {chain["name"] for chain in output["chains"]}
    stale_names = {name for name in run.registered if name not in generated_names}
    stale_bones = [bone for name in stale_names for bone in run.registered[name]]
    new_chains = [chain for chain in output["chains"] if chain["name"] not in run.read_back]
    group_names = {vg.index: vg.name for vg in obj.vertex_groups}
    rewritten = mismatched_vertices(
        output["chains"], vertex_group_weights(obj), group_names, {chain["name"] for chain in new_chains}
    )

    add_bones(context, arm, specs)
    remove_bones(context, arm, stale_bones)
    remove_vertex_groups(obj, stale_bones)
    remove_chains_named(settings, stale_names)
    remove_chain_collections(arm, sorted(stale_names))

    context.view_layer.objects.active = obj
    write_weights(obj, groups, rewritten, {bone.name for bone in arm.data.bones})
    write_membership_groups(obj, output["chains"], run.given)
    rethickened = update_hit_radii(settings, output["chains"], run.read_back)
    if run.role == "garment":
        write_covered_group(obj, output["covered"], run.marked)
    for chain in output["chains"]:
        apply_chain_bone_display(arm, chain["name"], [joint["name"] for joint in chain["joints"]], output["attachment"])
    if new_chains:
        _register_new_chains(context, arm, run, new_chains, output.get("head_capsule"), output["attachment"])
    return {"bones": len(specs), "rewritten": len(rewritten), "rethickened": rethickened}


def _report_line(run: GenerationRun, output: dict, counts: dict) -> str:
    support = output.get("support")
    notes = [
        f", kept the bones and spring values of {len(run.read_back)} chains" if run.read_back else "",
        f", re-measured the hit radius of {counts['rethickened']} joints" if counts["rethickened"] else "",
        " with a head collider" if output.get("head_capsule") else "",
        f", {len(output['covered'])} covered vertices" if output["covered"] else "",
        f", hanging from {run.root_name} at height {support['boundary_height']:.3f}" if support else "",
        run.fitted_note,
    ]
    return (
        f"Generated {counts['bones']} chain bones in {len(output['chains'])} chains, "
        f"weighted {counts['rewritten']} vertices{''.join(notes)}"
    )


class THYLLORE_OT_GenerateChainBones(bpy.types.Operator):
    """Generate chain bones on one submesh of the rig; its role and the generation settings come from the rig"""

    bl_idname = "thyllore.generate_chain_bones"
    bl_label = "Generate"
    bl_options = {"REGISTER", "UNDO"}

    mesh: bpy.props.StringProperty(name="Mesh", options={"HIDDEN"})
    material: bpy.props.StringProperty(name="Material", options={"HIDDEN"})

    @classmethod
    def poll(cls, context):
        return _AVATAR_API_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        arm = target_armature(context)
        previous_active = context.active_object
        try:
            obj, role, material_index = resolve_submesh(arm, self.mesh, self.material)
            run = _read_run(context, arm, obj, role, self.material)
            output = _generate(context, arm, run, material_index)
            support = output.get("support")
            if support is not None and not arm.data.thyllore_spring.chain_generation.root_bone.strip():
                run.root_name = arm.data.bones[support["bone"]].name
            counts = _apply_output(context, arm, run, output)
        except ValueError as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}

        context.view_layer.objects.active = previous_active
        self.report({"INFO"}, _report_line(run, output, counts))
        return {"FINISHED"}


def register() -> None:
    bpy.utils.register_class(THYLLORE_OT_GenerateChainBones)


def unregister() -> None:
    bpy.utils.unregister_class(THYLLORE_OT_GenerateChainBones)
