import bpy
import json
import sys
import os
import mathutils


def clear_scene():
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)

    for collection in bpy.data.collections:
        bpy.data.collections.remove(collection)


def import_model(model_path):
    ext = os.path.splitext(model_path)[1].lower()

    if ext == '.fbx':
        bpy.ops.import_scene.fbx(filepath=model_path)
    elif ext in ('.glb', '.gltf'):
        bpy.ops.import_scene.gltf(filepath=model_path)
    else:
        raise ValueError(f"Unsupported format: {ext}")


def compute_mesh_aabb():
    depsgraph = bpy.context.evaluated_depsgraph_get()

    all_min = [float('inf')] * 3
    all_max = [float('-inf')] * 3
    vertex_count = 0

    for obj in depsgraph.objects:
        if obj.type != 'MESH':
            continue

        eval_obj = obj.evaluated_get(depsgraph)
        mesh = eval_obj.to_mesh()

        if mesh is None:
            continue

        world_matrix = eval_obj.matrix_world

        for vert in mesh.vertices:
            world_pos = world_matrix @ vert.co
            for i in range(3):
                all_min[i] = min(all_min[i], world_pos[i])
                all_max[i] = max(all_max[i], world_pos[i])
            vertex_count += 1

        eval_obj.to_mesh_clear()

    if vertex_count == 0:
        return [0, 0, 0], [0, 0, 0]

    return all_min, all_max


def get_animation_info():
    scene = bpy.context.scene

    if scene.animation_data and scene.animation_data.action:
        action = scene.animation_data.action
        frame_start = int(action.frame_range[0])
        frame_end = int(action.frame_range[1])
        return frame_start, frame_end

    for obj in bpy.data.objects:
        if obj.animation_data and obj.animation_data.action:
            action = obj.animation_data.action
            frame_start = int(action.frame_range[0])
            frame_end = int(action.frame_range[1])
            return frame_start, frame_end

    for obj in bpy.data.objects:
        if obj.type == 'ARMATURE' and obj.animation_data and obj.animation_data.action:
            action = obj.animation_data.action
            frame_start = int(action.frame_range[0])
            frame_end = int(action.frame_range[1])
            return frame_start, frame_end

    return scene.frame_start, scene.frame_end


def main():
    argv = sys.argv
    separator_index = argv.index('--') if '--' in argv else -1
    if separator_index < 0 or separator_index + 2 >= len(argv):
        print("Usage: blender --background --python script.py -- <model_path> <output_json_path>")
        sys.exit(1)

    model_path = argv[separator_index + 1]
    output_path = argv[separator_index + 2]

    model_path = os.path.abspath(model_path)
    output_path = os.path.abspath(output_path)

    clear_scene()
    import_model(model_path)

    frame_start, frame_end = get_animation_info()
    fps = bpy.context.scene.render.fps
    mid_frame = (frame_start + frame_end) // 2

    frames_data = []

    bpy.context.scene.frame_set(frame_start)
    bounds_min, bounds_max = compute_mesh_aabb()
    frames_data.append({
        "frame": frame_start,
        "bounds_min": [round(v, 6) for v in bounds_min],
        "bounds_max": [round(v, 6) for v in bounds_max],
    })

    bpy.context.scene.frame_set(mid_frame)
    bounds_min, bounds_max = compute_mesh_aabb()
    frames_data.append({
        "frame": mid_frame,
        "bounds_min": [round(v, 6) for v in bounds_min],
        "bounds_max": [round(v, 6) for v in bounds_max],
    })

    result = {
        "model_path": model_path,
        "frames": frames_data,
        "fps": fps,
        "frame_start": frame_start,
        "frame_end": frame_end,
    }

    with open(output_path, 'w') as f:
        json.dump(result, f, indent=2)

    print(f"Bounds data written to: {output_path}")


if __name__ == '__main__':
    main()
