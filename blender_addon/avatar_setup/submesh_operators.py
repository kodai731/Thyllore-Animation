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
from ..common.logged_report import report
from .mesh_data import mesh_arrays
from .operators import bound_meshes, target_armature
from .submesh_roles import refresh_submesh_roles


def _material_index(collection, material: str) -> int | None:
    """Find the index of an entry for ``material`` in collection, or None."""
    return next((i for i, item in enumerate(collection) if item.material == material), None)


class THYLLORE_OT_GuessSubmeshRoles(bpy.types.Operator):
    """Guess the role of every material again, keeping the rows the user set by hand."""

    bl_idname = "thyllore.guess_submesh_roles"
    bl_label = "Guess Submesh Roles"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return _TML_AVAILABLE and target_armature(context) is not None

    def execute(self, context):
        arm = target_armature(context)
        if arm is None:
            report(self, "ERROR", "No target armature")
            return {"CANCELLED"}

        meshes = bound_meshes(arm)
        if not meshes:
            report(self, "WARNING", "No bound meshes found")
            return {"CANCELLED"}

        count = 0
        for mesh_obj in meshes:
            refresh_submesh_roles(mesh_obj)
            for item in mesh_obj.thyllore_submesh.roles:
                if item.source == "user":
                    continue
                item.role = tml.guess_submesh_role(mesh_obj.name, item.material)
                item.source = "guessed"
                count += 1

        report(self, "INFO", f"Guessed roles for {count} materials")
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
            report(self, "ERROR", "No target armature")
            return {"CANCELLED"}

        meshes = bound_meshes(arm)
        if not meshes:
            report(self, "WARNING", "No bound meshes found")
            return {"CANCELLED"}

        skin_materials, garment_materials = _materials_by_role(meshes)

        if not garment_materials:
            report(self, "WARNING", "No garment materials found — cancelled")
            return {"CANCELLED"}

        scale_length = context.scene.unit_settings.scale_length
        params = {name: getattr(self, name) for name in HIDDEN_MASK_DEFAULTS}

        garment_inputs: list[dict] = []
        for g_mesh_obj, g_material_index in garment_materials:
            garment_input, loop_indices = mask_mesh_input(
                *mesh_arrays(g_mesh_obj), g_material_index, scale_length
            )
            if not loop_indices:
                continue
            garment_inputs.append(garment_input)

        total_hidden = 0
        for skin_mesh_obj, skin_material_index in skin_materials:
            body_input, loop_indices = mask_mesh_input(
                *mesh_arrays(skin_mesh_obj), skin_material_index, scale_length
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

        report(self, "INFO", f"Hidden {total_hidden} skin triangles")
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
