"""Skin weight transfer operator."""
from __future__ import annotations

import bpy
import numpy as np
from mathutils.bvhtree import BVHTree

from ..common.skin_weight_transfer import (
    barycentric_coordinates,
    transfer_skin_weights,
)


def _mesh_arrays(obj):
    """Return world-space vertices, normals, and loop triangles from rest mesh."""
    mesh = obj.data
    n_verts = len(mesh.vertices)
    m = np.array(obj.matrix_world, dtype=np.float64)
    co_buf = np.empty(n_verts * 3, dtype=np.float32)
    mesh.vertices.foreach_get("co", co_buf)
    vertices = np.asarray(co_buf, dtype=np.float64).reshape(-1, 3) @ m[:3, :3].T + m[:3, 3]
    no_buf = np.empty(n_verts * 3, dtype=np.float32)
    mesh.vertex_normals.foreach_get("vector", no_buf)
    normals_raw = np.asarray(no_buf, dtype=np.float64).reshape(-1, 3)
    normal_matrix = np.linalg.inv(m[:3, :3]).T
    normals = normals_raw @ normal_matrix.T
    norms = np.linalg.norm(normals, axis=-1, keepdims=True)
    norms[norms < 1e-12] = 1.0
    normals /= norms
    mesh.calc_loop_triangles()
    loop_triangles = np.array([lt.vertices[:] for lt in mesh.loop_triangles], dtype=np.int64)
    return vertices, normals, loop_triangles


def _weight_matrix(obj):
    """Build weight matrix (V x B) from vertex groups, columns in vertex_groups order."""
    n_verts = len(obj.data.vertices)
    n_bones = len(obj.vertex_groups)
    weights = np.zeros((n_verts, n_bones), dtype=np.float64)
    for v in obj.data.vertices:
        for g in v.groups:
            weights[v.index, g.group] = g.weight
    return weights


def _nearest_triangles(bvh, vertices):
    """Find nearest triangle index, distance, and hit point for each vertex via BVH."""
    n = len(vertices)
    tri_indices = np.zeros(n, dtype=np.int64)
    distances = np.zeros(n, dtype=np.float64)
    hit_points = np.empty((n, 3), dtype=np.float64)
    for i in range(n):
        location, _normal, face_index, distance = bvh.find_nearest(vertices[i])
        if face_index is None:
            distances[i] = float("inf")
            hit_points[i] = vertices[i]
        else:
            tri_indices[i] = face_index
            distances[i] = distance
            hit_points[i] = location
    return tri_indices, distances, hit_points


def _write_vertex_groups(obj, weights, source_vg_names):
    """Write weights to target vertex groups (create if missing, replace existing)."""
    all_vertices = list(range(len(obj.data.vertices)))
    for bone_idx, name in enumerate(source_vg_names):
        vg = obj.vertex_groups.get(name)
        if vg is None:
            vg = obj.vertex_groups.new(name=name)
        else:
            vg.remove(all_vertices)
        for v_idx, w in enumerate(weights[:, bone_idx]):
            if w >= 1e-4:
                vg.add([v_idx], w, "REPLACE")


def _ensure_armature_modifier(obj, source_obj):
    """Add armature modifier to target if source has one and target does not."""
    source_mod = None
    for mod in source_obj.modifiers:
        if mod.type == "ARMATURE":
            source_mod = mod
            break
    if source_mod is None:
        return
    for mod in obj.modifiers:
        if mod.type == "ARMATURE":
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
        if obj is None or obj.type != "MESH":
            return False
        if not obj.vertex_groups:
            return False
        selected = [o for o in context.selected_objects if o != obj and o.type == "MESH"]
        return len(selected) >= 1

    def execute(self, context):
        source_obj = context.active_object
        source_vg_names = [vg.name for vg in source_obj.vertex_groups]
        source_vertices, source_normals, source_triangles = _mesh_arrays(source_obj)
        source_weights = _weight_matrix(source_obj)
        diag = np.linalg.norm(source_vertices.max(0) - source_vertices.min(0))
        max_distance = diag * self.max_distance_ratio
        bvh = BVHTree.FromPolygons(source_vertices, source_triangles)

        n_transferred = 0
        for target_obj in context.selected_objects:
            if target_obj == source_obj or target_obj.type != "MESH":
                continue
            target_vertices, target_normals, target_triangles = _mesh_arrays(target_obj)
            tri_indices, distances, hit_points = _nearest_triangles(bvh, target_vertices)
            face_verts = source_vertices[source_triangles[tri_indices]]
            barycentric = barycentric_coordinates(hit_points, face_verts[:, 0], face_verts[:, 1], face_verts[:, 2])
            source_hit_normals = source_normals[source_triangles[tri_indices]]
            interpolated_normals = np.einsum("vc,vcb->vb", barycentric, source_hit_normals)
            weights = transfer_skin_weights(
                target_vertices,
                target_triangles,
                target_normals,
                source_weights,
                interpolated_normals,
                source_triangles[tri_indices],
                barycentric,
                distances,
                max_distance,
                self.max_angle_deg,
                self.max_iterations,
                self.tolerance,
            )
            _write_vertex_groups(target_obj, weights, source_vg_names)
            _ensure_armature_modifier(target_obj, source_obj)
            n_transferred += 1

        self.report({"INFO"}, f"Transferred weights to {n_transferred} meshes")
        return {"FINISHED"}
