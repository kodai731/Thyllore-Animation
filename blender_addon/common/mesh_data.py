"""Raw mesh arrays read from bpy: positions, normals, triangles and vertex group weights."""
from __future__ import annotations

from .submesh_mask import material_loop_indices


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


def world_surface(mesh_obj) -> dict:
    """The mesh as a surface dict (world-space positions and normals, loop triangles) for the wheel."""
    positions, normals, triangle_vertices, _materials = mesh_arrays(mesh_obj)
    triangles = [triangle_vertices[i : i + 3] for i in range(0, len(triangle_vertices), 3)]
    return {"positions": positions, "normals": normals, "triangles": triangles}


def vertex_group_weight_rows(mesh_obj) -> list[list[float]]:
    """One weight row per vertex, one column per vertex group in ``mesh_obj.vertex_groups`` order."""
    group_count = len(mesh_obj.vertex_groups)
    rows = []
    for vertex in mesh_obj.data.vertices:
        row = [0.0] * group_count
        for group in vertex.groups:
            row[group.group] = group.weight
        rows.append(row)
    return rows


def material_loop_indices_of(mesh_obj) -> dict:
    """Loop-triangle indices per material name, the numbering the sidecar converts to and from."""
    _positions, _normals, _vertices, triangle_materials = mesh_arrays(mesh_obj)
    material_names = [slot.material.name if slot.material else None for slot in mesh_obj.material_slots]
    return material_loop_indices(triangle_materials, material_names)
