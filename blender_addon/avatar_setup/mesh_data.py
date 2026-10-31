"""Raw mesh arrays read from bpy for the submesh mask and the sidecar triangle numbering."""
from __future__ import annotations

from ..common.submesh_mask import material_loop_indices


def mesh_arrays(mesh_obj):
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


def material_loop_indices_of(mesh_obj) -> dict:
    """Loop-triangle indices per material name, the numbering the sidecar converts to and from."""
    _positions, _normals, _vertices, triangle_materials = mesh_arrays(mesh_obj)
    material_names = [slot.material.name if slot.material else None for slot in mesh_obj.material_slots]
    return material_loop_indices(triangle_materials, material_names)
