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

from ..common.chain_bones import bone_specs, group_weights, submesh_triangles


def _root_world(arm, root_bone: str) -> tuple[float, float, float]:
    """World-space position of the root bone."""
    head = arm.data.bones[root_bone].head_local
    world = arm.matrix_world @ head
    return (world[0], world[1], world[2])


def _add_bones(context, arm, specs):
    """Add bones in edit mode from (name, parent, head, tail) specs."""
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
    joint_count: bpy.props.IntProperty(default=4, min=1)
    column_count: bpy.props.IntProperty(default=0, min=0)

    @classmethod
    def poll(cls, context):
        obj = context.active_object
        if obj is None or obj.type != "MESH":
            return False
        arm = None
        for mod in obj.modifiers:
            if mod.type == "ARMATURE":
                arm = mod.object
                break
        if arm is None:
            return False
        return _TML_AVAILABLE

    def execute(self, context):
        obj = context.active_object
        mesh = obj.data
        arm = None
        for mod in obj.modifiers:
            if mod.type == "ARMATURE":
                arm = mod.object
                break

        positions = [tuple(obj.matrix_world @ v.co) for v in mesh.vertices]

        mesh.calc_loop_triangles()
        triangle_vertices = []
        triangle_materials = []
        for lt in mesh.loop_triangles:
            triangle_vertices.extend(lt.vertices)
            triangle_materials.append(lt.material_index)
        triangles = submesh_triangles(triangle_vertices, triangle_materials, self.material_index)
        root_position = _root_world(arm, self.root_bone)
        up = (0.0, 0.0, 1.0)

        output = tml.generate_chains(
            positions,
            triangles,
            root_position,
            up,
            self.role,
            self.joint_count,
            self.column_count,
        )

        specs = bone_specs(output, self.root_bone)
        groups = group_weights(output, self.root_bone)

        try:
            _add_bones(context, arm, specs)
        except ValueError as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}

        context.view_layer.objects.active = obj
        _write_weights(obj, groups)

        self.report({"INFO"}, f"Generated {len(specs)} chain bones")
        return {"FINISHED"}
