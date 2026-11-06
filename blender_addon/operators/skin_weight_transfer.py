"""Skin weight transfer operator: the wheel transfers, this file reads and writes Blender data."""
from __future__ import annotations

import bpy

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

from ..common.mesh_data import vertex_group_weight_rows, world_surface

MIN_WRITTEN_WEIGHT = 1e-4


def _bounds_diagonal(positions) -> float:
    lower = [min(p[axis] for p in positions) for axis in range(3)]
    upper = [max(p[axis] for p in positions) for axis in range(3)]
    return sum((hi - lo) ** 2 for lo, hi in zip(lower, upper)) ** 0.5


def _write_vertex_groups(obj, weights, source_vg_names):
    """Write weights to target vertex groups (create if missing, replace existing)."""
    all_vertices = list(range(len(obj.data.vertices)))
    for bone_idx, name in enumerate(source_vg_names):
        vg = obj.vertex_groups.get(name)
        if vg is None:
            vg = obj.vertex_groups.new(name=name)
        else:
            vg.remove(all_vertices)
        for v_idx, row in enumerate(weights):
            if row[bone_idx] >= MIN_WRITTEN_WEIGHT:
                vg.add([v_idx], row[bone_idx], "REPLACE")


def _ensure_armature_modifier(obj, source_obj):
    """Add armature modifier to target if source has one and target does not."""
    source_mod = next((mod for mod in source_obj.modifiers if mod.type == "ARMATURE"), None)
    if source_mod is None:
        return
    if any(mod.type == "ARMATURE" for mod in obj.modifiers):
        return
    mod = obj.modifiers.new(name="Armature", type="ARMATURE")
    mod.object = source_mod.object


class THYLLORE_OT_TransferSkinWeights(bpy.types.Operator):
    """Transfer skin weights from active mesh to selected meshes."""

    bl_idname = "thyllore.transfer_skin_weights"
    bl_label = "Transfer Skin Weights"
    bl_options = {"REGISTER", "UNDO"}

    max_distance_ratio: bpy.props.FloatProperty(default=0.05, min=0.0)
    max_angle_deg: bpy.props.FloatProperty(default=30.0, min=0.0, max=90.0)
    max_iterations: bpy.props.IntProperty(default=200, min=1)
    tolerance: bpy.props.FloatProperty(default=1e-6, min=0.0)

    @classmethod
    def poll(cls, context):
        obj = context.active_object
        if not _TML_AVAILABLE or obj is None or obj.type != "MESH":
            return False
        if not obj.vertex_groups:
            return False
        selected = [o for o in context.selected_objects if o != obj and o.type == "MESH"]
        return len(selected) >= 1

    def execute(self, context):
        source_obj = context.active_object
        source_vg_names = [vg.name for vg in source_obj.vertex_groups]
        source = world_surface(source_obj)
        source["weights"] = vertex_group_weight_rows(source_obj)
        max_distance = _bounds_diagonal(source["positions"]) * self.max_distance_ratio

        n_transferred = 0
        for target_obj in context.selected_objects:
            if target_obj == source_obj or target_obj.type != "MESH":
                continue
            try:
                weights = tml.transfer_skin_weights(
                    source,
                    world_surface(target_obj),
                    max_distance,
                    self.max_angle_deg,
                    self.max_iterations,
                    self.tolerance,
                )
            except ValueError as e:
                self.report({"ERROR"}, f"transfer_skin_weights failed: {e}")
                return {"CANCELLED"}
            _write_vertex_groups(target_obj, weights, source_vg_names)
            _ensure_armature_modifier(target_obj, source_obj)
            n_transferred += 1

        self.report({"INFO"}, f"Transferred weights to {n_transferred} meshes")
        return {"FINISHED"}
