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

from ..common import coordinates
from ..common.chain_bones import (
    bone_specs,
    group_weights,
    output_to_blender,
    settings_from_properties,
    submesh_triangles,
)
from ..common.effect_properties import build_param_properties

CHAIN_GEN_UI_PARAMS: list[dict] = tml.chain_gen_ui_params() if _TML_AVAILABLE else []


def _setting_annotations() -> dict[str, object]:
    annotations: dict[str, object] = {}
    for param in CHAIN_GEN_UI_PARAMS:
        annotations.update(build_param_properties({**param, "kind": "scalar"}))
    return annotations


def _armature_of(obj):
    for mod in obj.modifiers:
        if mod.type == "ARMATURE":
            return mod.object
    return None


def _root_engine_position(arm, root_bone: str) -> tuple[float, float, float]:
    head = arm.data.bones[root_bone].head_local
    return coordinates.blender_to_engine_point(tuple(arm.matrix_world @ head))


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


def _skin_engine_positions(skin_obj, root_bone):
    """Engine-space positions of the skin vertices whose heaviest vertex group is the root bone."""
    if skin_obj is None or skin_obj.type != "MESH":
        return []
    group_index = skin_obj.vertex_groups.find(root_bone)
    if group_index < 0:
        return []
    positions = []
    for v in skin_obj.data.vertices:
        if not v.groups:
            continue
        heaviest = max(v.groups, key=lambda g: g.weight)
        if heaviest.group == group_index and heaviest.weight > 0.0:
            positions.append(coordinates.blender_to_engine_point(tuple(skin_obj.matrix_world @ v.co)))
    return positions


class THYLLORE_OT_GenerateChainBones(bpy.types.Operator):
    """Generate chain bones from mesh surface."""

    bl_idname = "thyllore.generate_chain_bones"
    bl_label = "Generate Chain Bones"
    bl_options = {"REGISTER", "UNDO"}

    role: bpy.props.EnumProperty(
        items=[("hair", "Hair", ""), ("skirt", "Skirt", "")],
        default="hair",
    )
    material_index: bpy.props.IntProperty(default=0, min=0)
    root_bone: bpy.props.StringProperty(default="Head")
    skin_object: bpy.props.StringProperty(
        default="",
        description="Mesh object of the skin; its vertices driven by the root bone give the root band",
    )

    __annotations__.update(_setting_annotations())

    @classmethod
    def poll(cls, context):
        obj = context.active_object
        if obj is None or obj.type != "MESH":
            return False
        return _TML_AVAILABLE and _armature_of(obj) is not None

    def execute(self, context):
        obj = context.active_object
        mesh = obj.data
        arm = _armature_of(obj)

        positions = [
            coordinates.blender_to_engine_point(tuple(obj.matrix_world @ v.co)) for v in mesh.vertices
        ]
        mesh.calc_loop_triangles()
        triangle_vertices = []
        triangle_materials = []
        for lt in mesh.loop_triangles:
            triangle_vertices.extend(lt.vertices)
            triangle_materials.append(lt.material_index)
        triangles = submesh_triangles(triangle_vertices, triangle_materials, self.material_index)

        output = tml.generate_chains(
            positions,
            triangles,
            _root_engine_position(arm, self.root_bone),
            self.role,
            settings_from_properties(CHAIN_GEN_UI_PARAMS, self),
            _skin_engine_positions(bpy.data.objects.get(self.skin_object), self.root_bone),
        )

        blender_output = output_to_blender(output)
        specs = bone_specs(blender_output, self.root_bone)
        groups = group_weights(blender_output, self.root_bone)

        try:
            _add_bones(context, arm, specs)
        except ValueError as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}

        context.view_layer.objects.active = obj
        _write_weights(obj, groups)

        self.report({"INFO"}, f"Generated {len(specs)} chain bones")
        return {"FINISHED"}
