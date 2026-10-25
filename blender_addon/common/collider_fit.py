"""Pure Python helpers for body collider fit (bpy-independent)."""
from __future__ import annotations

from .coordinates import _mat4_mul, blender_frame_to_engine_matrix


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


def bone_bind_matrix(armature_world_rows, bone_local_rows, scale_length: float) -> list[list[float]]:
    """The bone's rest frame in engine space (row-major), as the engine reads it from the exported FBX.

    The armature object's world matrix carries the Z-up to Y-up conversion the FBX exporter and
    importer place on it, so the frame is converted on its output side only; conjugating it would
    rotate the bone's own axes a second time."""
    world = _mat4_mul(armature_world_rows, bone_local_rows)
    return scaled_bind_matrix(blender_frame_to_engine_matrix(world), scale_length)
