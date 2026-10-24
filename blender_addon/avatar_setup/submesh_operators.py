"""Submesh role and hidden mask operators (thyllore.guess_submesh_roles, thyllore.hide_covered_skin)."""
from __future__ import annotations

import bpy

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from ..common.submesh_mask import mask_mesh_input, material_triangles, to_loop_triangle_indices
from .operators import bound_meshes, target_armature


def _material_index(collection, material: str) -> int | None:
    """Find the index of an entry for ``material`` in collection, or None."""
    return next((i for i, item in enumerate(collection) if item.material == material), None)


class THYLLORE_OT_GuessSubmeshRoles(bpy.types.Operator):
    """Guess submesh roles from mesh/material names."""

    bl_idname = "thyllore.guess_submesh_roles"
    bl_label = "Guess Submesh Roles"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return _TML_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        arm = target_armature(context)
        if arm is None:
            self.report({"ERROR"}, "No target armature")
            return {"CANCELLED"}

        meshes = bound_meshes(arm)
        if not meshes:
            self.report({"WARNING"}, "No bound meshes found")
            return {"CANCELLED"}

        count = 0
        for mesh_obj in meshes:
            submesh = mesh_obj.thyllore_submesh
            for slot in mesh_obj.material_slots:
                if slot.material is None:
                    continue
                material_name = slot.material.name
                role = tml.guess_submesh_role(mesh_obj.name, material_name)

                existing_index = _material_index(submesh.roles, material_name)
                if existing_index is not None and submesh.roles[existing_index].source == "user":
                    continue

                if existing_index is not None:
                    item = submesh.roles[existing_index]
                    item.role = role
                    item.source = "guessed"
                else:
                    item = submesh.roles.add()
                    item.material = material_name
                    item.role = role
                    item.source = "guessed"

                count += 1

        self.report({"INFO"}, f"Guessed roles for {count} materials")
        return {"FINISHED"}


HIDDEN_MASK_DEFAULTS = tml.hidden_mask_defaults() if _TML_AVAILABLE else {}


def _setting_annotations() -> dict[str, object]:
    """Blender property annotations for THYLLORE_OT_HideCoveredSkin from wheel defaults."""
    import bpy

    return {
        "cover_distance": bpy.props.FloatProperty(
            name="Cover Distance", default=HIDDEN_MASK_DEFAULTS.get("cover_distance", 0.05)
        ),
        "cone_angle_deg": bpy.props.FloatProperty(
            name="Cone Angle (deg)", default=HIDDEN_MASK_DEFAULTS.get("cone_angle_deg", 120.0)
        ),
        "hidden_ratio": bpy.props.FloatProperty(
            name="Hidden Ratio", default=HIDDEN_MASK_DEFAULTS.get("hidden_ratio", 0.5)
        ),
        "ray_count": bpy.props.IntProperty(
            name="Ray Count", default=HIDDEN_MASK_DEFAULTS.get("ray_count", 8)
        ),
    }


class THYLLORE_OT_HideCoveredSkin(bpy.types.Operator):
    """Hide skin triangles covered by garment (thyllore.hide_covered_skin)."""

    bl_idname = "thyllore.hide_covered_skin"
    bl_label = "Hide Covered Skin"
    bl_options = {"REGISTER", "UNDO"}

    __annotations__ = _setting_annotations()

    @classmethod
    def poll(cls, context):
        return _TML_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        arm = target_armature(context)
        if arm is None:
            self.report({"ERROR"}, "No target armature")
            return {"CANCELLED"}

        meshes = bound_meshes(arm)
        if not meshes:
            self.report({"WARNING"}, "No bound meshes found")
            return {"CANCELLED"}

        skin_materials, garment_materials = _materials_by_role(meshes)

        if not garment_materials:
            self.report({"WARNING"}, "No garment materials found — cancelled")
            return {"CANCELLED"}

        scale_length = context.scene.unit_settings.scale_length
        params = {name: getattr(self, name) for name in HIDDEN_MASK_DEFAULTS}

        garment_inputs: list[dict] = []
        for g_mesh_obj, g_material_index in garment_materials:
            garment_input, loop_indices = mask_mesh_input(
                *_mesh_arrays(g_mesh_obj), g_material_index, scale_length
            )
            if not loop_indices:
                continue
            garment_inputs.append(garment_input)

        total_hidden = 0
        for skin_mesh_obj, skin_material_index in skin_materials:
            body_input, loop_indices = mask_mesh_input(
                *_mesh_arrays(skin_mesh_obj), skin_material_index, scale_length
            )
            if not loop_indices:
                continue

            hidden_local = tml.find_hidden_triangles(body_input, garment_inputs, params)
            loop_hidden = to_loop_triangle_indices(hidden_local, loop_indices)

            submesh = skin_mesh_obj.thyllore_submesh
            material_name = skin_mesh_obj.material_slots[skin_material_index].material.name
            existing_index = _material_index(submesh.hidden_triangles, material_name)
            if existing_index is not None:
                item = submesh.hidden_triangles[existing_index]
                item["triangles"] = loop_hidden
            else:
                item = submesh.hidden_triangles.add()
                item.material = material_name
                item["triangles"] = loop_hidden

            total_hidden += len(loop_hidden)

        self.report({"INFO"}, f"Hidden {total_hidden} skin triangles")
        return {"FINISHED"}


def _materials_by_role(meshes):
    """Return (skin_materials, garment_materials) lists of (mesh_obj, material_index) tuples."""
    skin_materials: list[tuple] = []
    garment_materials: list[tuple] = []
    for mesh_obj in meshes:
        submesh = mesh_obj.thyllore_submesh
        for material_index, slot in enumerate(mesh_obj.material_slots):
            if slot.material is None:
                continue
            role_index = _material_index(submesh.roles, slot.material.name)
            if role_index is None:
                continue
            role = submesh.roles[role_index].role
            if role == "skin":
                skin_materials.append((mesh_obj, material_index))
            elif role == "garment":
                garment_materials.append((mesh_obj, material_index))
    return skin_materials, garment_materials


def _mesh_arrays(mesh_obj):
    """Collect raw mesh arrays from bpy for mask_mesh_input.

    Returns (vertex_positions, vertex_normals, triangle_vertices, triangle_materials)
    where positions and normals are in Blender world space."""
    mesh = mesh_obj.data
    matrix_world = mesh_obj.matrix_world
    vertex_positions = [tuple(matrix_world @ v.co) for v in mesh.vertices]
    vertex_normals = [tuple((matrix_world.to_3x3() @ v.normal).normalized()) for v in mesh.vertices]
    mesh.calc_loop_triangles()
    triangle_vertices = []
    triangle_materials = []
    for lt in mesh.loop_triangles:
        triangle_vertices.extend(lt.vertices)
        triangle_materials.append(lt.material_index)
    return vertex_positions, vertex_normals, triangle_vertices, triangle_materials


OPERATORS = (
    THYLLORE_OT_GuessSubmeshRoles,
    THYLLORE_OT_HideCoveredSkin,
)


def register() -> None:
    for cls in OPERATORS:
        bpy.utils.register_class(cls)


def unregister() -> None:
    for cls in reversed(OPERATORS):
        bpy.utils.unregister_class(cls)
