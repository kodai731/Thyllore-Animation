import bpy
import sys
from pathlib import Path


def parse_args():
    argv = sys.argv
    idx = argv.index("--") if "--" in argv else len(argv)
    args = argv[idx + 1:]
    if len(args) >= 2:
        return args[0], args[1]
    return None, None


def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()

    for block in list(bpy.data.meshes):
        bpy.data.meshes.remove(block)
    for block in list(bpy.data.armatures):
        bpy.data.armatures.remove(block)
    for block in list(bpy.data.materials):
        bpy.data.materials.remove(block)
    for block in list(bpy.data.actions):
        bpy.data.actions.remove(block)


def main():
    input_path, output_path = parse_args()
    if not input_path or not output_path:
        print("Usage: blender --background --python script.py -- <input.glb> <output.glb>")
        sys.exit(1)

    if not Path(input_path).exists():
        print(f"ERROR: input file not found: {input_path}")
        sys.exit(1)

    clear_scene()

    print(f"Importing: {input_path}")
    bpy.ops.import_scene.gltf(filepath=input_path)

    obj_count = len(bpy.data.objects)
    action_count = len(bpy.data.actions)
    armature_count = sum(1 for o in bpy.data.objects if o.type == "ARMATURE")
    print(f"Imported: {obj_count} objects, {armature_count} armatures, {action_count} actions")

    for action in bpy.data.actions:
        print(f"  Action '{action.name}': {len(action.fcurves)} fcurves, "
              f"range={action.frame_range[0]:.1f}-{action.frame_range[1]:.1f}")

    Path(output_path).parent.mkdir(parents=True, exist_ok=True)

    print(f"Exporting: {output_path}")
    bpy.ops.export_scene.gltf(
        filepath=output_path,
        export_format="GLB",
        export_animations=True,
        export_skins=True,
    )

    print("Roundtrip complete")


main()
