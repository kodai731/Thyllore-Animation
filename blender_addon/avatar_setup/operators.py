"""Avatar sidecar import / export operators (thyllore.import_avatar_sidecar, thyllore.export_for_unity)."""
from __future__ import annotations

from pathlib import Path

import bpy
from bpy_extras.io_utils import ExportHelper, ImportHelper

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from ..common import coordinates
from ..common.avatar_sidecar import bone_specs, sidecar_stats, submesh_to_sidecar, write_submesh_sidecar
from ..common.collider_fit import scaled_bind_matrix
from ..common.spring_properties import humanoid_overrides_to_dict, spring_settings_to_sidecar, write_sidecar_spring
from ..common.armature import armature_of


def bound_meshes(arm) -> list:
    """Collect mesh objects that reference arm via an ARMATURE modifier."""
    meshes = []
    for obj in bpy.data.objects:
        if obj.type != "MESH":
            continue
        for mod in obj.modifiers:
            if mod.type == "ARMATURE" and mod.object is arm:
                meshes.append(obj)
                break
    return meshes


def _collect_bone_data(arm):
    """Collect bone names, parent indices, and world-space head positions from arm.data.bones."""
    bones = arm.data.bones
    names = []
    parent_indices = []
    world_heads = []
    for bone in bones:
        names.append(bone.name)
        if bone.parent is not None:
            parent_indices.append(bones.find(bone.parent.name))
        else:
            parent_indices.append(None)
        world_heads.append(tuple(arm.matrix_world @ bone.head_local))
    return names, parent_indices, world_heads


def target_armature(context):
    """Resolve the target armature from context.active_object.

    Returns the active object if it is an ARMATURE, or its armature modifier's
    target if it is a MESH bound to one. Returns None otherwise."""
    obj = context.active_object
    if obj is None:
        return None
    if obj.type == "ARMATURE":
        return obj
    if obj.type == "MESH":
        return armature_of(obj)
    return None


def armature_bone_specs(context, arm):
    """Collect bone data from arm and build bone specs with context scale_length."""
    names, parent_indices, world_heads = _collect_bone_data(arm)
    scale_length = context.scene.unit_settings.scale_length
    return bone_specs(names, parent_indices, world_heads, scale_length)


def armature_fit_bones(context, arm):
    """Collect bone data from arm and build bone specs with bind_matrix for collider fit."""
    specs = armature_bone_specs(context, arm)
    scale_length = context.scene.unit_settings.scale_length
    for spec, bone in zip(specs, arm.data.bones):
        world_matrix = arm.matrix_world @ bone.matrix_local
        engine_matrix = coordinates.blender_to_engine_matrix(world_matrix)
        rows = [[engine_matrix[i][j] for j in range(4)] for i in range(4)]
        spec["bind_matrix"] = scaled_bind_matrix(rows, scale_length)
    return specs


def _select_only(context, objects):
    """Deselect everything and select only the given objects."""
    for o in bpy.data.objects:
        o.select_set(False)
    for o in objects:
        o.select_set(True)


def _restore_selection(context, selected, active):
    """Restore selection state from a saved (selected, active) tuple."""
    for o in bpy.data.objects:
        o.select_set(o in selected)
    context.view_layer.objects.active = active


def _mesh_counts(meshes):
    """Count total triangles, unique material names, and morph-enabled meshes.

    Returns (triangle_count, material_names, morph_mesh_count)."""
    triangle_count = 0
    material_names = set()
    morph_mesh_count = 0
    for mesh_obj in meshes:
        mesh_obj.data.calc_loop_triangles()
        loop_tris = mesh_obj.data.loop_triangles
        triangle_count += len(loop_tris)
        for slot in mesh_obj.material_slots:
            if slot.material is not None:
                material_names.add(slot.material.name)
        if mesh_obj.data.shape_keys is not None:
            morph_mesh_count += 1
    return triangle_count, material_names, morph_mesh_count


def _sidecar_build_input(context, arm, meshes):
    """Build the input dict for tml.sidecar_build from arm and meshes."""
    bones = armature_bone_specs(context, arm)

    spring = spring_settings_to_sidecar(arm.data.thyllore_spring)

    all_roles = []
    all_hidden = []
    for mesh_obj in meshes:
        roles, hidden = submesh_to_sidecar(mesh_obj.name, mesh_obj.thyllore_submesh)
        all_roles.extend(roles)
        all_hidden.extend(hidden)

    triangle_count, material_names, morph_mesh_count = _mesh_counts(meshes)

    stats = sidecar_stats(
        triangle_count=triangle_count,
        bone_count=len(bones),
        material_count=len(material_names),
        mesh_count=len(meshes),
        skinned_mesh_count=len(meshes),
        morph_mesh_count=morph_mesh_count,
        spring=spring,
    )

    humanoid_overrides = humanoid_overrides_to_dict(arm.data.thyllore_spring)

    return {
        "bones": bones,
        "humanoid_overrides": humanoid_overrides,
        "spring": spring,
        "submesh_roles": all_roles,
        "hidden_triangles": all_hidden,
        "materials": {},
        "stats": stats,
    }


class THYLLORE_OT_ImportAvatarSidecar(bpy.types.Operator, ImportHelper):
    """Import avatar sidecar JSON and apply spring/submesh data to the active rig."""

    bl_idname = "thyllore.import_avatar_sidecar"
    bl_label = "Import Avatar Sidecar"
    bl_options = {"REGISTER", "UNDO"}

    filename_ext = ".avatar.json"
    filter_glob: bpy.props.StringProperty(default="*.avatar.json", options={"HIDDEN"})

    @classmethod
    def poll(cls, context):
        return _TML_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        if not _TML_AVAILABLE:
            self.report({"ERROR"}, "thyllore_ml_core not available")
            return {"CANCELLED"}

        arm = target_armature(context)

        try:
            with open(self.filepath, encoding="utf-8") as f:
                text = f.read()
        except OSError as e:
            self.report({"ERROR"}, f"Cannot read file: {e}")
            return {"CANCELLED"}

        try:
            sidecar = tml.sidecar_read(text)
        except (RuntimeError, ValueError) as e:
            self.report({"ERROR"}, f"Invalid sidecar file: {e}")
            return {"CANCELLED"}

        write_sidecar_spring(arm.data.thyllore_spring, sidecar["spring"])

        meshes = bound_meshes(arm)
        for mesh_obj in meshes:
            write_submesh_sidecar(
                mesh_obj.thyllore_submesh,
                mesh_obj.name,
                sidecar["submesh_roles"],
                sidecar["hidden_triangles"],
            )

        chain_count = len(sidecar["spring"].get("chains", []))
        collider_count = len(sidecar["spring"].get("colliders", []))
        self.report({"INFO"}, f"Imported {chain_count} chains, {collider_count} colliders")

        return {"FINISHED"}


class THYLLORE_OT_ExportForUnity(bpy.types.Operator, ExportHelper):
    """Export FBX + avatar sidecar JSON for Unity."""

    bl_idname = "thyllore.export_for_unity"
    bl_label = "Export for Unity (FBX + sidecar)"
    bl_options = {"REGISTER", "UNDO"}

    filename_ext = ".fbx"

    @classmethod
    def poll(cls, context):
        return _TML_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        if not _TML_AVAILABLE:
            self.report({"ERROR"}, "thyllore_ml_core not available")
            return {"CANCELLED"}

        arm = target_armature(context)
        meshes = bound_meshes(arm)

        selected_before = frozenset(o for o in context.selected_objects)
        active_before = context.view_layer.objects.active

        sidecar_input = _sidecar_build_input(context, arm, meshes)

        try:
            json_text = tml.sidecar_build(sidecar_input)
        except ValueError as e:
            self.report({"ERROR"}, f"Sidecar build failed: {e}")
            return {"CANCELLED"}

        _select_only(context, [arm] + meshes)
        context.view_layer.objects.active = arm

        bpy.ops.export_scene.fbx(
            filepath=self.filepath,
            use_selection=True,
            global_scale=1.0,
            apply_unit_scale=False,
            bake_anim=False,
            add_leaf_bones=False,
            use_mesh_modifiers=True,
        )

        _restore_selection(context, selected_before, active_before)

        fbx_path = Path(self.filepath)
        sidecar_path = fbx_path.with_suffix(".avatar.json")
        try:
            with open(sidecar_path, "w", encoding="utf-8") as f:
                f.write(json_text)
        except OSError as e:
            self.report({"ERROR"}, f"Cannot write sidecar file: {e}")
            return {"CANCELLED"}

        result = tml.infer_humanoid_mapping(sidecar_input["bones"])
        unresolved = result.get("unresolved", [])
        if unresolved:
            self.report({"WARNING"}, f"Humanoid mapping unresolved: {len(unresolved)} roles")

        self.report({"INFO"}, f"Exported FBX and sidecar to {sidecar_path}")
        return {"FINISHED"}


OPERATORS = (
    THYLLORE_OT_ImportAvatarSidecar,
    THYLLORE_OT_ExportForUnity,
)


def register() -> None:
    for cls in OPERATORS:
        bpy.utils.register_class(cls)


def unregister() -> None:
    for cls in reversed(OPERATORS):
        bpy.utils.unregister_class(cls)
