"""Generate chain bones on a hair or garment submesh and register them as spring chains."""
from __future__ import annotations

from mathutils import Vector

import bpy

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from ..common import coordinates
from ..common.chain_bone_display import apply_chain_bone_display
from ..common.chain_bones import (
    COVERED_GROUP,
    bone_specs,
    covered_group_vertices,
    group_weights,
    output_to_blender,
    settings_from_properties,
    vertex_tex_coords,
)
from ..common.skin_input import skin_input, skin_role_meshes
from ..common.spring_properties import (
    disabled_chain_names,
    remove_chains_generated_on,
    restore_disabled_chains,
    spring_settings_to_edit,
    write_sidecar_spring,
)
from ..common.submesh_mask import material_triangles
from .operators import armature_fit_bones, bound_meshes, target_armature
from .properties import CHAIN_GEN_UI_PARAMS

GENERATABLE_ROLES = ("hair", "garment")


def generatable_submeshes(arm) -> list[tuple[object, object]]:
    """(mesh object, role item) pairs whose submesh role can grow chain bones."""
    return [
        (mesh_obj, item)
        for mesh_obj in bound_meshes(arm)
        for item in mesh_obj.thyllore_submesh.roles
        if item.role in GENERATABLE_ROLES
    ]


def _material_slot_index(obj, material_name: str) -> int | None:
    for index, slot in enumerate(obj.material_slots):
        if slot.material is not None and slot.material.name == material_name:
            return index
    return None


def _root_engine_position(arm, root_bone: str, scale_length: float) -> list[float]:
    head = arm.data.bones[root_bone].head_local
    return [c * scale_length for c in coordinates.blender_to_engine_point(tuple(arm.matrix_world @ head))]


def _add_bones(context, arm, specs):
    """Add bones in edit mode from (name, parent, head, tail) specs in Blender world space."""
    for name, *_ in specs:
        if name in arm.data.bones:
            raise ValueError(f"Bone {name!r} already exists")
    to_armature = arm.matrix_world.inverted()
    context.view_layer.objects.active = arm
    bpy.ops.object.mode_set(mode="EDIT")
    try:
        edit_bones = arm.data.edit_bones
        for name, parent_name, head, tail in specs:
            bone = edit_bones.new(name)
            bone.head = to_armature @ Vector(head)
            bone.tail = to_armature @ Vector(tail)
            bone.parent = edit_bones[parent_name]
    finally:
        bpy.ops.object.mode_set(mode="OBJECT")


def _remove_bones(context, arm, bone_names):
    """Remove bones by name in edit mode; descendants of a removed bone are removed by Blender."""
    if not bone_names:
        return
    context.view_layer.objects.active = arm
    bpy.ops.object.mode_set(mode="EDIT")
    try:
        edit_bones = arm.data.edit_bones
        for name in bone_names:
            bone = edit_bones.get(name)
            if bone is not None:
                edit_bones.remove(bone)
    finally:
        bpy.ops.object.mode_set(mode="OBJECT")


def _remove_vertex_groups(obj, bone_names):
    for name in bone_names:
        group = obj.vertex_groups.get(name)
        if group is not None:
            obj.vertex_groups.remove(group)


def _write_weights(obj, groups):
    """Write vertex group weights: clear affected vertices from every bone group, then REPLACE."""
    affected_vertices: set[int] = set()
    for influences in groups.values():
        for v_idx, _ in influences:
            affected_vertices.add(v_idx)
    for vg in obj.vertex_groups:
        if vg.name == COVERED_GROUP:
            continue
        vg.remove(list(affected_vertices))
    for bone_name, influences in groups.items():
        vg = obj.vertex_groups.get(bone_name)
        if vg is None:
            vg = obj.vertex_groups.new(name=bone_name)
        for v_idx, weight in influences:
            vg.add([v_idx], weight, "REPLACE")


def _marked_covered_vertices(obj) -> list[int]:
    """The vertices of the covered group, or none when the group does not exist (the coverage is then measured)."""
    group = obj.vertex_groups.get(COVERED_GROUP)
    if group is None:
        return []
    pairs = [(v.index, [(g.group, g.weight) for g in v.groups]) for v in obj.data.vertices]
    return covered_group_vertices(pairs, group.index)


def _write_covered_group(obj, covered: list[int]) -> None:
    """Record the measured coverage so the user can repaint it; an existing group is left as it is."""
    if obj.vertex_groups.get(COVERED_GROUP) is not None:
        return
    group = obj.vertex_groups.new(name=COVERED_GROUP)
    if covered:
        group.add([int(v) for v in covered], 1.0, "REPLACE")


class THYLLORE_OT_GenerateChainBones(bpy.types.Operator):
    """Generate chain bones on one submesh of the rig; its role and the generation settings come from the rig"""

    bl_idname = "thyllore.generate_chain_bones"
    bl_label = "Generate"
    bl_options = {"REGISTER", "UNDO"}

    mesh: bpy.props.StringProperty(name="Mesh", options={"HIDDEN"})
    material: bpy.props.StringProperty(name="Material", options={"HIDDEN"})

    @classmethod
    def poll(cls, context):
        return _TML_AVAILABLE and target_armature(context) is not None

    def _resolve_submesh(self, arm):
        obj = bpy.data.objects.get(self.mesh)
        if obj is None or obj.type != "MESH" or obj not in bound_meshes(arm):
            raise ValueError(f"{self.mesh!r} is not a mesh bound to {arm.name!r}")
        role_item = next((item for item in obj.thyllore_submesh.roles if item.material == self.material), None)
        if role_item is None or role_item.role not in GENERATABLE_ROLES:
            raise ValueError(f"{self.material!r} on {obj.name!r} is not a hair or garment submesh")
        material_index = _material_slot_index(obj, self.material)
        if material_index is None:
            raise ValueError(f"{obj.name!r} has no material slot {self.material!r}")
        return obj, role_item.role, material_index

    def execute(self, context):
        arm = target_armature(context)
        try:
            obj, role, material_index = self._resolve_submesh(arm)
        except ValueError as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        generation = arm.data.thyllore_spring.chain_generation
        previous_active = context.active_object
        mesh = obj.data
        scale_length = context.scene.unit_settings.scale_length

        positions = [
            [c * scale_length for c in coordinates.blender_to_engine_point(tuple(obj.matrix_world @ v.co))]
            for v in mesh.vertices
        ]
        mesh.calc_loop_triangles()
        triangle_vertices = []
        triangle_materials = []
        for lt in mesh.loop_triangles:
            triangle_vertices.extend(lt.vertices)
            triangle_materials.append(lt.material_index)
        triangles = material_triangles(triangle_vertices, triangle_materials, material_index)[0]

        tex_coords: list[tuple[float, float]] = []
        if uv_layer := mesh.uv_layers.active:
            loop_vertices = [l.vertex_index for l in mesh.loops]
            loop_uvs = [tuple(d.uv) for d in uv_layer.data]
            tex_coords = vertex_tex_coords(loop_vertices, loop_uvs, len(mesh.vertices))

        skins = [skin_input(skin_obj, arm, scale_length) for skin_obj in skin_role_meshes(bound_meshes(arm))]
        fit_bones = armature_fit_bones(context, arm)
        try:
            root_name = tml.resolve_root_bone(fit_bones, generation.root_bone)
        except ValueError as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        root_index = arm.data.bones.find(root_name)

        target = f"{obj.name}/{self.material}"
        origin = {"mesh": obj.name, "material": self.material, "role": role}
        settings = arm.data.thyllore_spring
        replaced_bones = remove_chains_generated_on(settings, origin)
        _remove_bones(context, arm, replaced_bones)
        _remove_vertex_groups(obj, replaced_bones)
        marked = _marked_covered_vertices(obj)

        try:
            output = tml.generate_chains(
                positions,
                triangles,
                _root_engine_position(arm, root_name, scale_length),
                role,
                target,
                settings_from_properties(CHAIN_GEN_UI_PARAMS, generation),
                skins,
                root_index,
                tex_coords,
                marked,
            )
        except ValueError as e:
            self.report({"ERROR"}, f"generate_chains failed: {e}")
            return {"CANCELLED"}

        blender_output = output_to_blender(output, scale_length)
        specs = bone_specs(blender_output, root_name)
        groups = group_weights(blender_output, root_name, [bone.name for bone in arm.data.bones])

        try:
            _add_bones(context, arm, specs)
        except ValueError as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}

        context.view_layer.objects.active = obj
        _write_weights(obj, groups)
        if role == "garment":
            _write_covered_group(obj, output["covered"])
        apply_chain_bone_display(arm, [name for name, _, _, _ in specs], role)

        bones = armature_fit_bones(context, arm)
        try:
            spring = tml.register_generated_chains(
                tml.spring_colliders_to_model(spring_settings_to_edit(settings), bones),
                bones,
                skins,
                root_name,
                target,
                role,
                output["chains"],
                output.get("head_capsule"),
            )
        except ValueError as e:
            self.report({"ERROR"}, f"register_generated_chains failed: {e}")
            return {"CANCELLED"}
        disabled = disabled_chain_names(settings)
        write_sidecar_spring(settings, tml.spring_colliders_to_local(spring, bones))
        restore_disabled_chains(settings, disabled)

        context.view_layer.objects.active = previous_active
        collider_note = " with a head collider" if output.get("head_capsule") else ""
        covered_note = f", {len(output['covered'])} covered vertices" if output["covered"] else ""
        self.report(
            {"INFO"},
            f"Generated {len(specs)} chain bones in {len(output['chains'])} chains{collider_note}{covered_note}",
        )
        return {"FINISHED"}


def register() -> None:
    bpy.utils.register_class(THYLLORE_OT_GenerateChainBones)


def unregister() -> None:
    bpy.utils.unregister_class(THYLLORE_OT_GenerateChainBones)
