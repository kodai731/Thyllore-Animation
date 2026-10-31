import argparse
import importlib.util
import json
import os
import re
import sys

import bpy

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))


def load_coordinates_module():
    path = os.path.join(REPO_ROOT, "blender_addon", "common", "coordinates.py")
    spec = importlib.util.spec_from_file_location("thyllore_coordinates", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


blender_to_engine_point = load_coordinates_module().blender_to_engine_point

REFERENCE_SPACE = "engine_y_up_m"


def engine_position(vector):
    return [round(v, 6) for v in blender_to_engine_point(tuple(vector))]


def parse_args():
    argv = sys.argv
    idx = argv.index("--") if "--" in argv else len(argv)
    args = argv[idx + 1:]
    parser = argparse.ArgumentParser()
    parser.add_argument("input_fbx")
    parser.add_argument("--humanoid-ron", default=None)
    parser.add_argument("--out-dir", default=None)
    return parser.parse_args(args)


def import_fbx(path):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=path)


def find_armature():
    for obj in bpy.data.objects:
        if obj.type == "ARMATURE":
            return obj
    return None


def find_meshes():
    return [obj for obj in bpy.data.objects if obj.type == "MESH"]


UNITY_ROLE_NAMES = [
    "Hips", "LeftUpperLeg", "RightUpperLeg", "LeftLowerLeg", "RightLowerLeg",
    "LeftFoot", "RightFoot", "Spine", "Chest", "UpperChest", "Neck", "Head",
    "LeftShoulder", "RightShoulder", "LeftUpperArm", "RightUpperArm",
    "LeftLowerArm", "RightLowerArm", "LeftHand", "RightHand",
    "LeftToes", "RightToes", "LeftEye", "RightEye", "Jaw",
    "LeftThumbProximal", "RightThumbProximal", "LeftThumbIntermediate",
    "RightThumbIntermediate", "LeftThumbDistal", "RightThumbDistal",
    "LeftIndexProximal", "RightIndexProximal", "LeftIndexIntermediate",
    "RightIndexIntermediate", "LeftIndexDistal", "RightIndexDistal",
    "LeftMiddleProximal", "RightMiddleProximal", "LeftMiddleIntermediate",
    "RightMiddleIntermediate", "LeftMiddleDistal", "RightMiddleDistal",
    "LeftRingProximal", "RightRingProximal", "LeftRingIntermediate",
    "RightRingIntermediate", "LeftRingDistal", "RightRingDistal",
    "LeftLittleProximal", "RightLittleProximal", "LeftLittleIntermediate",
    "RightLittleIntermediate", "LeftLittleDistal", "RightLittleDistal",
]


def parse_humanoid_ron(path):
    text = open(path, encoding="utf-8").read()
    roles = {}
    for m in re.finditer(r'(\w+):\s*"([^"]*)"', text):
        role = m.group(1)
        bone_name = m.group(2)
        roles[role] = bone_name
    return roles


def normalize(name):
    s = name.lower().strip()
    s = re.sub(r"^(mixamorig:|mixamo:)", "", s)
    s = s.replace("-", " ").replace("_", " ").replace(".", " ")
    return s.strip()


def infer_humanoid_bones(bone_names):
    normalized = [normalize(b) for b in bone_names]
    role_to_bone = {}
    used = set()
    for role in UNITY_ROLE_NAMES:
        role_norm = normalize(role)
        best = None
        for i, n in enumerate(normalized):
            if i in used:
                continue
            if n == role_norm:
                best = i
                break
        else:
            for i, n in enumerate(normalized):
                if i in used:
                    continue
                if n.endswith(role_norm):
                    best = i
                    break
        if best is not None:
            role_to_bone[role] = bone_names[best]
            used.add(best)
    return list(role_to_bone.values())


def resolve_humanoid_bones(input_path, ron_path_arg):
    arm = find_armature()
    all_bone_names = [b.name for b in arm.data.bones]

    if ron_path_arg:
        ron_path = ron_path_arg
    else:
        stem = os.path.splitext(os.path.basename(input_path))[0]
        ron_path = os.path.join(os.path.dirname(input_path), stem + ".humanoid.ron")

    if os.path.isfile(ron_path):
        roles = parse_humanoid_ron(ron_path)
        return list(roles.values())

    return infer_humanoid_bones(all_bone_names)


def build_bone_map(armature_obj):
    by_name = {}
    for bone in armature_obj.data.bones:
        by_name[bone.name] = bone
    return by_name


def find_humanoid_ancestor(bone, humanoid_set):
    current = bone
    while current is not None:
        if current.name in humanoid_set:
            return current
        current = current.parent
    return None


def collect_deleted_bones(armature_obj, humanoid_set):
    deleted = []
    for bone in armature_obj.data.bones:
        if bone.name not in humanoid_set:
            has_secondary_parent = bone.parent is not None and bone.parent.name not in humanoid_set
            has_secondary_child = any(c.name not in humanoid_set for c in bone.children)
            if has_secondary_parent or has_secondary_child:
                kind = "chain"
            else:
                kind = "single"
            deleted.append({
                "name": bone.name,
                "parent": bone.parent.name if bone.parent else None,
                "head": engine_position(armature_obj.matrix_world @ bone.head_local),
                "tail": engine_position(armature_obj.matrix_world @ bone.tail_local),
                "kind": kind,
            })
    return deleted


def collect_vertex_weights(mesh_obj, deleted_bone_names):
    entries = []
    for vert in mesh_obj.data.vertices:
        for vg in vert.groups:
            if vg.group < len(mesh_obj.vertex_groups):
                vg_name = mesh_obj.vertex_groups[vg.group].name
                if vg_name in deleted_bone_names:
                    entries.append({
                        "mesh": mesh_obj.name,
                        "vertex": vert.index,
                        "bone": vg_name,
                        "weight": round(vg.weight, 6),
                        "position": engine_position(mesh_obj.matrix_world @ vert.co),
                    })
    return entries


def redistribute_weights(mesh_obj, deleted_bone_names, humanoid_set, bone_map):
    deleted_set = set(deleted_bone_names)
    if not deleted_set:
        return

    ancestor_map = {}
    for bone_name in deleted_set:
        bone = bone_map.get(bone_name)
        if bone is None:
            continue
        ancestor = find_humanoid_ancestor(bone, humanoid_set)
        if ancestor is not None:
            ancestor_map[bone_name] = ancestor.name

    group_names = {vg.index: vg.name for vg in mesh_obj.vertex_groups}
    vertex_weights = {}
    for vert in mesh_obj.data.vertices:
        vertex_weights[vert.index] = {group_names[g.group]: g.weight for g in vert.groups}

    for weights in vertex_weights.values():
        for bone_name in list(weights):
            if bone_name in deleted_set:
                w = weights.pop(bone_name)
                target = ancestor_map.get(bone_name)
                if target is not None:
                    weights[target] = weights.get(target, 0.0) + w
        total = sum(weights.values())
        if total > 1e-9:
            for bone_name in weights:
                weights[bone_name] /= total

    for vg in list(mesh_obj.vertex_groups):
        mesh_obj.vertex_groups.remove(vg)

    for vi, weights in vertex_weights.items():
        for bone_name, weight in weights.items():
            if weight <= 0.0:
                continue
            target_vg = mesh_obj.vertex_groups.get(bone_name)
            if target_vg is None:
                target_vg = mesh_obj.vertex_groups.new(name=bone_name)
            target_vg.add([vi], weight, "ADD")


def delete_bones(armature_obj, humanoid_set):
    to_delete = [b.name for b in armature_obj.data.bones if b.name not in humanoid_set]
    if not to_delete:
        return

    bpy.context.view_layer.objects.active = armature_obj
    bpy.ops.object.mode_set(mode="EDIT")
    edit_bones = armature_obj.data.edit_bones
    for name in to_delete:
        edit_bones.remove(edit_bones[name])
    bpy.ops.object.mode_set(mode="OBJECT")


def write_fbx(meshes, armature_obj, output_path):
    for obj in bpy.data.objects:
        obj.select_set(False)
    for m in meshes:
        m.select_set(True)
    armature_obj.select_set(True)
    bpy.context.view_layer.objects.active = armature_obj
    bpy.ops.export_scene.fbx(
        filepath=output_path,
        use_selection=True,
        global_scale=1.0,
        apply_unit_scale=False,
        bake_anim=False,
        add_leaf_bones=False,
        use_mesh_modifiers=True,
    )


def write_reference_json(deleted_bones, vertex_weights, humanoid_bone_names, output_path):
    data = {
        "space": REFERENCE_SPACE,
        "deleted_bones": sorted(deleted_bones, key=lambda b: b["name"]),
        "vertex_weights": sorted(vertex_weights, key=lambda e: (e["mesh"], e["vertex"], e["bone"])),
        "humanoid_bones": sorted(humanoid_bone_names),
    }
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False, sort_keys=True)


def main():
    args = parse_args()
    input_path = os.path.abspath(args.input_fbx)
    out_dir = args.out_dir or os.path.dirname(input_path)
    os.makedirs(out_dir, exist_ok=True)

    stem = os.path.splitext(os.path.basename(input_path))[0]
    output_fbx = os.path.join(out_dir, stem + "_humanoid_only.fbx")
    output_json = os.path.join(out_dir, stem + "_secondary_reference.json")

    import_fbx(input_path)

    armature_obj = find_armature()
    meshes = find_meshes()

    humanoid_bone_names = resolve_humanoid_bones(input_path, args.humanoid_ron)
    humanoid_set = set(humanoid_bone_names)

    bone_map = build_bone_map(armature_obj)
    deleted_bone_names = [b.name for b in armature_obj.data.bones if b.name not in humanoid_set]

    deleted_bones = collect_deleted_bones(armature_obj, humanoid_set)

    vertex_weights = []
    for mesh_obj in meshes:
        vertex_weights.extend(collect_vertex_weights(mesh_obj, deleted_bone_names))

    for mesh_obj in meshes:
        redistribute_weights(mesh_obj, deleted_bone_names, humanoid_set, bone_map)

    delete_bones(armature_obj, humanoid_set)

    write_fbx(meshes, armature_obj, output_fbx)

    write_reference_json(deleted_bones, vertex_weights, humanoid_bone_names, output_json)


main()
