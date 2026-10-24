"""Pure Python helpers for body collider fit (bpy-independent)."""
from __future__ import annotations


def top_influences(
    vertex_groups: list[tuple[int, float]],
    group_to_bone: dict[int, int],
    limit: int = 4,
) -> list[tuple[int, float]]:
    """Top influences for one vertex.

    ``vertex_groups`` is the list of (group_index, weight) pairs from Blender's
    vertex.groups.  ``group_to_bone`` maps each vertex group index to a bone index.
    Groups not in the map and zero-weight entries are dropped.  Returns at most
    ``limit`` (bone_index, weight) pairs sorted by weight descending."""
    filtered = [
        (group_to_bone[g], w)
        for g, w in vertex_groups
        if w > 0.0 and g in group_to_bone
    ]
    filtered.sort(key=lambda x: -x[1])
    return filtered[:limit]


def scaled_bind_matrix(rows: list[list[float]], scale_length: float) -> list[list[float]]:
    """Scale the translation components of a row-major 4x4 matrix.

    Only the translation part (rows[i][3] for i < 3) is multiplied by ``scale_length``;
    the rest of the matrix is unchanged."""
    result = [row[:] for row in rows]
    for i in range(3):
        result[i][3] *= scale_length
    return result
