import bpy
import json
import os
import sys
from mathutils import Vector


def engine_to_blender(v):
    return (v[0], -v[2], v[1])


def clear_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def build_armature(bones_json):
    arm_data = bpy.data.armatures.new("Armature")
    arm_obj = bpy.data.objects.new("Armature", arm_data)
    bpy.context.collection.objects.link(arm_obj)
    bpy.context.view_layer.objects.active = arm_obj
    bpy.ops.object.mode_set(mode="EDIT")

    name_to_bone = {}
    for entry in bones_json:
        name = entry["name"]
        head = engine_to_blender(entry["head"])
        tail = engine_to_blender(entry["tail"])
        bone = arm_data.edit_bones.new(name)
        bone.head = Vector(head)
        bone.tail = Vector(tail)
        name_to_bone[name] = bone

    for entry in bones_json:
        parent_name = entry.get("parent")
        if parent_name is not None:
            name_to_bone[entry["name"]].parent = name_to_bone[parent_name]

    bpy.ops.object.mode_set(mode="OBJECT")
    return arm_obj


def export(arm_obj, output_dir):
    fbx_path = os.path.join(output_dir, "test_humanoid.fbx")
    bpy.ops.object.select_all(action="DESELECT")
    arm_obj.select_set(True)
    bpy.ops.export_scene.fbx(
        filepath=fbx_path,
        use_selection=True,
        add_leaf_bones=False,
        bake_anim=False,
    )
    print("EXPORTED", fbx_path)


def main():
    args = sys.argv[sys.argv.index("--") + 1:]
    json_path = args[0]
    output_dir = args[1]

    with open(json_path) as f:
        bones_json = json.load(f)

    clear_scene()
    arm_obj = build_armature(bones_json)
    export(arm_obj, output_dir)


if __name__ == "__main__":
    main()
