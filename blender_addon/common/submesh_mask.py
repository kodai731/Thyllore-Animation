"""Pure Python helpers for submesh mask operations (bpy-independent).

material_triangles extracts triangles belonging to one material from Blender loop data.
to_loop_triangle_indices maps material-local triangle indices back to loop_triangles indices.
mask_mesh_input builds the MaskMeshInput dict for one material's submesh from raw mesh arrays.
"""
from __future__ import annotations

from .coordinates import blender_to_engine_point


def material_triangles(
    triangle_vertices: list[int],
    triangle_materials: list[int],
    material_index: int,
) -> tuple[list[list[int]], list[int]]:
    """Extract triangles belonging to one material from Blender loop data.

    ``triangle_vertices`` is the flat list of vertex indices from
    ``mesh.loop_triangles`` (3 per triangle).  ``triangle_materials`` is the
    parallel list of ``material_index`` values.  Returns a pair:
    - triangles: only the triangles whose material matches ``material_index``,
      each as a 3-element list of vertex indices.
    - loop_triangle_indices: the indices into ``mesh.loop_triangles`` of those
      matching triangles (the original triangle indices).
    """
    result: list[list[int]] = []
    indices: list[int] = []
    for i in range(0, len(triangle_vertices), 3):
        if triangle_materials[i // 3] == material_index:
            result.append(
                [triangle_vertices[i], triangle_vertices[i + 1], triangle_vertices[i + 2]]
            )
            indices.append(i // 3)
    return result, indices


def to_loop_triangle_indices(hidden_local: list[int], loop_indices: list[int]) -> list[int]:
    """Map material-local triangle indices back to loop_triangles indices.

    ``hidden_local`` is a list of triangle indices within the material's submesh
    (as returned by the wheel's find_hidden_triangles).  ``loop_indices`` maps
    each material-local index to its index in the full ``mesh.loop_triangles``.
    Returns the corresponding loop_triangles indices."""
    return [loop_indices[i] for i in hidden_local]


def mask_mesh_input(
    vertex_positions: list[tuple[float, float, float]],
    vertex_normals: list[tuple[float, float, float]],
    triangle_vertices: list[int],
    triangle_materials: list[int],
    material_index: int,
    scale_length: float,
) -> tuple[dict, list[int]]:
    """Build the MaskMeshInput dict for one material's submesh from raw mesh arrays."""
    triangles, loop_indices = material_triangles(triangle_vertices, triangle_materials, material_index)

    positions = []
    for p in vertex_positions:
        engine_p = blender_to_engine_point(p)
        positions.append((engine_p[0] * scale_length, engine_p[1] * scale_length, engine_p[2] * scale_length))

    normals = []
    for n in vertex_normals:
        engine_n = blender_to_engine_point(n)
        length = (engine_n[0] ** 2 + engine_n[1] ** 2 + engine_n[2] ** 2) ** 0.5
        if length > 1e-9:
            engine_n = (engine_n[0] / length, engine_n[1] / length, engine_n[2] / length)
        normals.append(engine_n)

    return {"positions": positions, "normals": normals, "triangles": triangles}, loop_indices
