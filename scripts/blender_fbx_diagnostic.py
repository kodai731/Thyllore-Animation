import bpy
import json
import os
import sys
from pathlib import Path

def parse_args():
    argv = sys.argv
    idx = argv.index("--") if "--" in argv else len(argv)
    args = argv[idx + 1:]
    if len(args) >= 2:
        return args[0], args[1]
    return None, None


def import_fbx(fbx_path):
    for obj in bpy.data.objects:
        obj.select_set(True)
    bpy.ops.object.delete()

    for block in list(bpy.data.meshes):
        bpy.data.meshes.remove(block)
    for block in list(bpy.data.armatures):
        bpy.data.armatures.remove(block)

    try:
        bpy.ops.import_scene.fbx(filepath=fbx_path)
        return {"success": True, "imported_from": fbx_path}
    except Exception as e:
        return {"success": False, "error": str(e)}


def collect_object_info():
    objects = []
    for obj in bpy.data.objects:
        entry = {
            "name": obj.name,
            "type": obj.type,
            "parent": obj.parent.name if obj.parent else None,
            "location": [round(v, 6) for v in obj.location],
            "rotation_euler": [round(v, 6) for v in obj.rotation_euler],
            "scale": [round(v, 6) for v in obj.scale],
        }

        if obj.type == "ARMATURE" and obj.data:
            bones = []
            for bone in obj.data.bones:
                bones.append({
                    "name": bone.name,
                    "parent": bone.parent.name if bone.parent else None,
                })
            entry["bones"] = bones

        objects.append(entry)
    return objects


def collect_actions_info():
    actions = []
    for action in bpy.data.actions:
        fcurves = []
        for fc in action.fcurves:
            kf_frames = [round(kp.co[0], 2) for kp in list(fc.keyframe_points)[:5]]
            fcurves.append({
                "data_path": fc.data_path,
                "array_index": fc.array_index,
                "keyframe_count": len(fc.keyframe_points),
                "first_keyframes": kf_frames,
            })
        actions.append({
            "name": action.name,
            "frame_range": [round(v, 2) for v in action.frame_range],
            "fcurve_count": len(action.fcurves),
            "fcurves": fcurves,
        })
    return actions


def collect_animation_data_info():
    results = []
    for obj in bpy.data.objects:
        entry = {
            "object_name": obj.name,
            "object_type": obj.type,
            "has_animation_data": obj.animation_data is not None,
            "active_action": None,
            "nla_track_count": 0,
        }

        if obj.animation_data:
            anim = obj.animation_data
            if anim.action:
                entry["active_action"] = anim.action.name
            entry["nla_track_count"] = len(anim.nla_tracks)

        results.append(entry)
    return results


def check_animation_playback():
    scene = bpy.context.scene
    frame_a = int(scene.frame_start)
    frame_b = min(frame_a + 10, int(scene.frame_end))

    def snapshot():
        matrices = {}
        for obj in bpy.data.objects:
            matrices[obj.name] = [list(row) for row in obj.matrix_world]
        return matrices

    scene.frame_set(frame_a)
    bpy.context.view_layer.update()
    snap_a = snapshot()

    scene.frame_set(frame_b)
    bpy.context.view_layer.update()
    snap_b = snapshot()

    scene.frame_set(frame_a)

    moved = []
    static = []
    for name in snap_a:
        if name in snap_b and snap_a[name] != snap_b[name]:
            moved.append(name)
        else:
            static.append(name)

    return {
        "test_frames": [frame_a, frame_b],
        "objects_that_moved": moved,
        "objects_that_stayed": static,
    }


def main():
    fbx_path, output_path = parse_args()
    if not fbx_path or not output_path:
        print("Usage: blender --background --python script.py -- <fbx_path> <output_path>")
        return

    print("=== Blender FBX Diagnostic ===")
    print(f"FBX: {fbx_path}")

    if not Path(fbx_path).exists():
        print(f"ERROR: FBX not found at {fbx_path}")
        return

    import_result = import_fbx(fbx_path)
    print(f"Import: {import_result}")

    diagnostic = {
        "fbx_path": fbx_path,
        "import_result": import_result,
        "scene_info": {
            "frame_start": bpy.context.scene.frame_start,
            "frame_end": bpy.context.scene.frame_end,
            "fps": bpy.context.scene.render.fps,
        },
        "objects": collect_object_info(),
        "actions": collect_actions_info(),
        "animation_data": collect_animation_data_info(),
        "playback_test": check_animation_playback(),
    }

    obj_types = {}
    for o in diagnostic["objects"]:
        t = o["type"]
        obj_types[t] = obj_types.get(t, 0) + 1

    animated_count = sum(1 for a in diagnostic["animation_data"] if a["has_animation_data"])
    action_count = sum(1 for a in diagnostic["animation_data"] if a["active_action"])

    diagnostic["summary"] = {
        "total_objects": len(diagnostic["objects"]),
        "object_types": obj_types,
        "total_actions": len(diagnostic["actions"]),
        "total_fcurves": sum(a["fcurve_count"] for a in diagnostic["actions"]),
        "objects_with_animation_data": animated_count,
        "objects_with_active_action": action_count,
        "moved": diagnostic["playback_test"]["objects_that_moved"],
        "static": diagnostic["playback_test"]["objects_that_stayed"],
    }

    Path(output_path).parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(diagnostic, f, indent=2, ensure_ascii=False)

    s = diagnostic["summary"]
    print(f"\nTotal objects: {s['total_objects']}")
    print(f"Object types: {s['object_types']}")
    print(f"Total actions: {s['total_actions']}")
    print(f"Total FCurves: {s['total_fcurves']}")
    print(f"Objects with animation: {s['objects_with_animation_data']}")
    print(f"Objects with action: {s['objects_with_active_action']}")
    print(f"Moved (frame test): {s['moved']}")
    print(f"Static (frame test): {s['static']}")
    print(f"\nSaved to: {output_path}")


main()
