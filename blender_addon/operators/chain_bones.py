"""Chain bone generation operator."""
from __future__ import annotations

from mathutils import Vector

import bpy

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from ..avatar_setup.operators import armature_fit_bones, bound_meshes
from ..common import coordinates
from ..common.armature import armature_of
from ..common.bone_collections import assign_bones_to_chain_collection
from ..common.chain_bones import (
    bone_specs,
    group_weights,
    output_to_blender,
    settings_from_properties,
    vertex_tex_coords,
)
from ..common.effect_properties import build_param_properties
from ..common.skin_input import dominant_bone_positions, skin_input, skin_role_meshes
from ..common.spring_properties import (
    disabled_chain_names,
    remove_chains_generated_on,
    restore_disabled_chains,
    spring_settings_to_edit,
    write_sidecar_spring,
)
from ..common.submesh_mask import material_triangles

CHAIN_GEN_UI_PARAMS: list[dict] = tml.chain_gen_ui_params() if _TML_AVAILABLE else []


def _setting_annotations() -> dict[str, object]:
    annotations: dict[str, object] = {}
    for param in CHAIN_GEN_UI_PARAMS:
        annotations.update(build_param_properties({**param, "kind": "scalar"}))
    return annotations


def _material_name(obj, material_index: int) -> str:
    slots = obj.material_slots
    if material_index < len(slots) and slots[material_index].material is not None:
        return slots[material_index].material.name
    return str(material_index)


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
    """Write vertex group weights: clear affected vertices from ALL groups, then REPLACE."""
    affected_vertices: set[int] = set()
    for influences in groups.values():
        for v_idx, _ in influences:
            affected_vertices.add(v_idx)
    for vg in obj.vertex_groups:
        vg.remove(list(affected_vertices))
    for bone_name, influences in groups.items():
        vg = obj.vertex_groups.get(bone_name)
        if vg is None:
            vg = obj.vertex_groups.new(name=bone_name)
        for v_idx, weight in influences:
            vg.add([v_idx], weight, "REPLACE")


class THYLLORE_OT_GenerateChainBones(bpy.types.Operator):
    """Generate chain bones from mesh surface."""

    bl_idname = "thyllore.generate_chain_bones"
    bl_label = "Generate Chain Bones"
    bl_options = {"REGISTER", "UNDO"}

    role: bpy.props.EnumProperty(
        items=[("hair", "Hair", ""), ("garment", "Garment", "")],
        default="hair",
    )
    material_index: bpy.props.IntProperty(default=0, min=0)
    root_bone: bpy.props.StringProperty(default="Head")

    __annotations__.update(_setting_annotations())

    @classmethod
    def poll(cls, context):
        obj = context.active_object
        if obj is None or obj.type != "MESH":
            return False
        return _TML_AVAILABLE and armature_of(obj) is not None

    def execute(self, context):
        obj = context.active_object
        mesh = obj.data
        arm = armature_of(obj)
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
        triangles = material_triangles(triangle_vertices, triangle_materials, self.material_index)[0]

        tex_coords: list[tuple[float, float]] = []
        if uv_layer := mesh.uv_layers.active:
            loop_vertices = [l.vertex_index for l in mesh.loops]
            loop_uvs = [tuple(d.uv) for d in uv_layer.data]
            tex_coords = vertex_tex_coords(loop_vertices, loop_uvs, len(mesh.vertices))

        skins = [skin_input(skin_obj, arm, scale_length) for skin_obj in skin_role_meshes(bound_meshes(arm))]
        root_index = arm.data.bones.find(self.root_bone)
        if root_index < 0:
            self.report({"ERROR"}, f"Root bone {self.root_bone!r} not found")
            return {"CANCELLED"}

        material_name = _material_name(obj, self.material_index)
        target = f"{obj.name}/{material_name}"
        origin = {"mesh": obj.name, "material": material_name, "role": self.role}
        settings = arm.data.thyllore_spring
        replaced_bones = remove_chains_generated_on(settings, origin)
        _remove_bones(context, arm, replaced_bones)
        _remove_vertex_groups(obj, replaced_bones)

        output = tml.generate_chains(
            positions,
            triangles,
            _root_engine_position(arm, self.root_bone, scale_length),
            self.role,
            target,
            settings_from_properties(CHAIN_GEN_UI_PARAMS, self),
            dominant_bone_positions(skins, root_index),
            tex_coords,
        )

        blender_output = output_to_blender(output, scale_length)
        specs = bone_specs(blender_output, self.root_bone)
        groups = group_weights(blender_output, self.root_bone)

        try:
            _add_bones(context, arm, specs)
        except ValueError as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}

        context.view_layer.objects.active = obj
        _write_weights(obj, groups)
        assign_bones_to_chain_collection(arm, [name for name, _, _, _ in specs])

        try:
            spring = tml.register_generated_chains(
                spring_settings_to_edit(settings),
                armature_fit_bones(context, arm),
                skins,
                self.root_bone,
                target,
                self.role,
                output["chains"],
                output.get("head_capsule"),
            )
        except ValueError as e:
            self.report({"ERROR"}, f"register_generated_chains failed: {e}")
            return {"CANCELLED"}
        disabled = disabled_chain_names(settings)
        write_sidecar_spring(settings, spring)
        restore_disabled_chains(settings, disabled)

        collider_note = " with a head collider" if output.get("head_capsule") else ""
        self.report({"INFO"}, f"Generated {len(specs)} chain bones in {len(output['chains'])} chains{collider_note}")
        return {"FINISHED"}
