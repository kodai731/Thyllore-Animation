import bpy
import os
import sys
from mathutils import Vector

OUT_DIR = sys.argv[sys.argv.index("--") + 1]

BONES = [
    ("Hips", None, (0, 0, 0.95), (0, 0, 1.05)),
    ("Spine", "Hips", (0, 0, 1.05), (0, 0, 1.20)),
    ("Chest", "Spine", (0, 0, 1.20), (0, 0, 1.40)),
    ("Neck", "Chest", (0, 0, 1.40), (0, 0, 1.50)),
    ("Head", "Neck", (0, 0, 1.50), (0, 0, 1.70)),
    ("Shoulder.L", "Chest", (0.05, 0, 1.38), (0.15, 0, 1.38)),
    ("Upper_arm.L", "Shoulder.L", (0.15, 0, 1.38), (0.40, 0, 1.38)),
    ("Lower_arm.L", "Upper_arm.L", (0.40, 0, 1.38), (0.65, 0, 1.38)),
    ("Hand.L", "Lower_arm.L", (0.65, 0, 1.38), (0.75, 0, 1.38)),
    ("Shoulder.R", "Chest", (-0.05, 0, 1.38), (-0.15, 0, 1.38)),
    ("Upper_arm.R", "Shoulder.R", (-0.15, 0, 1.38), (-0.40, 0, 1.38)),
    ("Lower_arm.R", "Upper_arm.R", (-0.40, 0, 1.38), (-0.65, 0, 1.38)),
    ("Hand.R", "Lower_arm.R", (-0.65, 0, 1.38), (-0.75, 0, 1.38)),
    ("Upper_leg.L", "Hips", (0.10, 0, 0.95), (0.10, 0, 0.50)),
    ("Lower_leg.L", "Upper_leg.L", (0.10, 0, 0.50), (0.10, 0, 0.08)),
    ("Foot.L", "Lower_leg.L", (0.10, 0, 0.08), (0.10, -0.12, 0.0)),
    ("Upper_leg.R", "Hips", (-0.10, 0, 0.95), (-0.10, 0, 0.50)),
    ("Lower_leg.R", "Upper_leg.R", (-0.10, 0, 0.50), (-0.10, 0, 0.08)),
    ("Foot.R", "Lower_leg.R", (-0.10, 0, 0.08), (-0.10, -0.12, 0.0)),
    ("Left_braid_1", "Head", (0.08, 0.05, 1.60), (0.10, 0.08, 1.45)),
    ("Left_braid_2", "Left_braid_1", (0.10, 0.08, 1.45), (0.12, 0.10, 1.30)),
]

VISEMES = ["sil", "pp", "ff", "th", "dd", "kk", "ch", "ss", "nn", "rr", "aa", "e", "ih", "oh", "ou"]
SHAPE_KEYS = [f"vrc.v_{v}" for v in VISEMES] + ["eye_blink", "eye_blink_L", "eye_blink_R", "mouth_smile"]


def clear_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def build_armature():
    arm_data = bpy.data.armatures.new("Armature")
    arm_obj = bpy.data.objects.new("Armature", arm_data)
    bpy.context.collection.objects.link(arm_obj)
    bpy.context.view_layer.objects.active = arm_obj
    bpy.ops.object.mode_set(mode="EDIT")
    for name, parent, head, tail in BONES:
        bone = arm_data.edit_bones.new(name)
        bone.head = Vector(head)
        bone.tail = Vector(tail)
        if parent is not None:
            bone.parent = arm_data.edit_bones[parent]
    bpy.ops.object.mode_set(mode="OBJECT")
    return arm_obj


def build_body_mesh():
    bpy.ops.mesh.primitive_cube_add(size=1.0)
    body = bpy.context.active_object
    body.name = "Body"
    body.data.name = "Body"
    body.scale = (0.4, 0.15, 0.85)
    body.location = (0, 0, 0.85)
    bpy.ops.object.transform_apply(scale=True, location=True)
    return body


def assign_weights(body, arm_obj):
    bone_segments = [(name, Vector(head), Vector(tail)) for name, _, head, tail in BONES]
    for name, _, _ in bone_segments:
        body.vertex_groups.new(name=name)

    for vertex in body.data.vertices:
        best_name = None
        best_distance = None
        for name, head, tail in bone_segments:
            direction = tail - head
            t = max(0.0, min(1.0, (vertex.co - head).dot(direction) / direction.length_squared))
            distance = (head + direction * t - vertex.co).length
            if best_distance is None or distance < best_distance:
                best_name = name
                best_distance = distance
        body.vertex_groups[best_name].add([vertex.index], 1.0, "REPLACE")

    modifier = body.modifiers.new("Armature", "ARMATURE")
    modifier.object = arm_obj
    body.parent = arm_obj


def add_shape_keys(body):
    body.shape_key_add(name="Basis", from_mix=False)
    for index, name in enumerate(SHAPE_KEYS):
        key = body.shape_key_add(name=name, from_mix=False)
        key.value = 0.0
        key.data[0].co.z += 0.01 * (index + 1)


def export(arm_obj, body):
    fbx_path = os.path.join(OUT_DIR, "synthetic_avatar.fbx")
    bpy.ops.object.select_all(action="DESELECT")
    arm_obj.select_set(True)
    body.select_set(True)
    bpy.ops.export_scene.fbx(
        filepath=fbx_path,
        use_selection=True,
        add_leaf_bones=False,
        bake_anim=False,
        mesh_smooth_type="FACE",
    )
    print("EXPORTED", fbx_path)


clear_scene()
arm_obj = build_armature()
body = build_body_mesh()
assign_weights(body, arm_obj)
add_shape_keys(body)
export(arm_obj, body)
