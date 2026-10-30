"""Skin weight transfer from source (skin) to target (garment) mesh.

Robust Skin Weights Transfer (Abdrashitov et al., SIGGRAPH Asia 2023).
Reference implementation: https://github.com/rin-23/RobustSkinWeightsTransferCode (MIT).
"""
from __future__ import annotations

import numpy as np
from numpy.typing import NDArray

DEGENERATE_AREA_EPSILON = 1e-12


def find_matched_vertices(
    distances: NDArray[np.floating],
    target_normals: NDArray[np.floating],
    source_normals: NDArray[np.floating],
    max_distance: float,
    max_angle_deg: float,
) -> NDArray[np.bool_]:
    target_unit = target_normals / np.linalg.norm(target_normals, axis=-1, keepdims=True)
    source_unit = source_normals / np.linalg.norm(source_normals, axis=-1, keepdims=True)
    cos_theta = np.clip(np.sum(target_unit * source_unit, axis=-1), -1.0, 1.0)
    angle_deg = np.degrees(np.arccos(cos_theta))

    within_distance = distances <= max_distance
    within_angle = (angle_deg <= max_angle_deg) | (angle_deg >= 180.0 - max_angle_deg)
    return within_distance & within_angle


def interpolate_weights(
    source_weights: NDArray[np.floating],
    face_vertex_indices: NDArray[np.integer],
    barycentric: NDArray[np.floating],
) -> NDArray[np.floating]:
    corner_weights = source_weights[face_vertex_indices]
    return np.einsum("vc,vcb->vb", barycentric, corner_weights)


def barycentric_coordinates(
    points: NDArray[np.floating],
    a: NDArray[np.floating],
    b: NDArray[np.floating],
    c: NDArray[np.floating],
) -> NDArray[np.floating]:
    ab = b - a
    ac = c - a
    n = np.cross(ab, ac)
    denom = np.sum(n * n, axis=-1)
    w1 = np.sum(np.cross(points - a, ac) * n, axis=-1) / denom
    w2 = np.sum(np.cross(ab, points - a) * n, axis=-1) / denom
    w0 = 1.0 - w1 - w2
    mask = denom <= DEGENERATE_AREA_EPSILON
    w0[mask] = 1.0
    w1[mask] = 0.0
    w2[mask] = 0.0
    return np.stack([w0, w1, w2], axis=-1)


def _triangle_double_areas(
    vertices: NDArray[np.floating],
    triangles: NDArray[np.integer],
) -> NDArray[np.floating]:
    corners = vertices[triangles]
    return np.linalg.norm(
        np.cross(corners[:, 1] - corners[:, 0], corners[:, 2] - corners[:, 0]), axis=-1
    )


def nondegenerate_triangles(
    vertices: NDArray[np.floating],
    triangles: NDArray[np.integer],
) -> NDArray[np.integer]:
    return triangles[_triangle_double_areas(vertices, triangles) > DEGENERATE_AREA_EPSILON]


def connected_component_labels(
    vertex_count: int,
    triangles: NDArray[np.integer],
) -> NDArray[np.integer]:
    """Each vertex's component, as the smallest vertex index it is connected to through triangles."""
    labels = np.arange(vertex_count)
    if len(triangles) == 0:
        return labels
    edge_a = np.concatenate([triangles[:, 0], triangles[:, 1], triangles[:, 2]])
    edge_b = np.concatenate([triangles[:, 1], triangles[:, 2], triangles[:, 0]])
    while True:
        lower = np.minimum(labels[edge_a], labels[edge_b])
        propagated = labels.copy()
        np.minimum.at(propagated, edge_a, lower)
        np.minimum.at(propagated, edge_b, lower)
        propagated = propagated[propagated]
        if np.array_equal(propagated, labels):
            return labels
        labels = propagated


def cotangent_laplacian(
    vertices: NDArray[np.floating],
    triangles: NDArray[np.integer],
) -> tuple[NDArray[np.integer], NDArray[np.integer], NDArray[np.floating]]:
    triangles = nondegenerate_triangles(vertices, triangles)
    corners = vertices[triangles]
    double_area = _triangle_double_areas(vertices, triangles)

    rows, cols, values = [], [], []
    for opposite, i, j in ((0, 1, 2), (1, 2, 0), (2, 0, 1)):
        to_i = corners[:, i] - corners[:, opposite]
        to_j = corners[:, j] - corners[:, opposite]
        half_cot = 0.5 * np.sum(to_i * to_j, axis=-1) / double_area
        vi, vj = triangles[:, i], triangles[:, j]
        rows += [vi, vj, vi, vj]
        cols += [vj, vi, vi, vj]
        values += [-half_cot, -half_cot, half_cot, half_cot]

    if not rows:
        return np.empty(0, np.int64), np.empty(0, np.int64), np.empty(0, np.float64)
    return np.concatenate(rows), np.concatenate(cols), np.concatenate(values)


def _multiply_sparse_matrix(
    rows: NDArray[np.integer],
    cols: NDArray[np.integer],
    values: NDArray[np.floating],
    x: NDArray[np.floating],
    size: int,
) -> NDArray[np.floating]:
    y = np.zeros(size, dtype=np.float64)
    np.add.at(y, rows, values * x[cols])
    return y


def _solve_conjugate_gradient(
    rows: NDArray[np.integer],
    cols: NDArray[np.integer],
    values: NDArray[np.floating],
    rhs: NDArray[np.floating],
    max_iterations: int,
    tolerance: float,
) -> NDArray[np.floating]:
    x = np.zeros_like(rhs)
    residual = rhs.copy()
    direction = residual.copy()
    residual_norm_sq = float(residual @ residual)

    for _ in range(max_iterations):
        if np.sqrt(residual_norm_sq) <= tolerance:
            break
        projected = _multiply_sparse_matrix(rows, cols, values, direction, len(rhs))
        alpha = residual_norm_sq / float(direction @ projected)
        x += alpha * direction
        residual -= alpha * projected
        next_norm_sq = float(residual @ residual)
        direction = residual + (next_norm_sq / residual_norm_sq) * direction
        residual_norm_sq = next_norm_sq

    return x


def _normalize_rows(weights: NDArray[np.floating]) -> NDArray[np.floating]:
    clipped = np.clip(weights, 0.0, 1.0)
    row_sums = clipped.sum(axis=1, keepdims=True)
    return np.divide(clipped, row_sums, out=clipped.copy(), where=row_sums > 0.0)


def _components_without_match(
    vertices: NDArray[np.floating],
    triangles: NDArray[np.integer],
    matched: NDArray[np.bool_],
) -> NDArray[np.bool_]:
    """Vertices whose component has no matched vertex: with no boundary to inpaint from, they keep the
    weights interpolated from the nearest source surface."""
    labels = connected_component_labels(len(vertices), nondegenerate_triangles(vertices, triangles))
    has_match = np.zeros(len(vertices), dtype=bool)
    has_match[labels[matched]] = True
    return ~has_match[labels]


def inpaint_weights(
    vertices: NDArray[np.floating],
    triangles: NDArray[np.integer],
    weights: NDArray[np.floating],
    matched: NDArray[np.bool_],
    max_iterations: int = 100,
    tolerance: float = 1e-6,
) -> NDArray[np.floating]:
    matched = matched | _components_without_match(vertices, triangles, matched)
    if np.all(matched):
        return weights

    rows, cols, values = cotangent_laplacian(vertices, triangles)
    unknown = np.flatnonzero(~matched)
    unknown_index = np.full(len(vertices), -1, dtype=np.int64)
    unknown_index[unknown] = np.arange(len(unknown))

    in_unknown_row = ~matched[rows]
    uu = in_unknown_row & ~matched[cols]
    uk = in_unknown_row & matched[cols]
    uu_rows, uu_cols, uu_values = unknown_index[rows[uu]], unknown_index[cols[uu]], values[uu]

    result = weights.astype(np.float64)
    for bone in range(weights.shape[1]):
        rhs = np.zeros(len(unknown), dtype=np.float64)
        np.add.at(rhs, unknown_index[rows[uk]], -values[uk] * weights[cols[uk], bone])
        result[unknown, bone] = _solve_conjugate_gradient(
            uu_rows, uu_cols, uu_values, rhs, max_iterations, tolerance
        )

    return _normalize_rows(result)


def transfer_skin_weights(
    target_vertices: NDArray[np.floating],
    target_triangles: NDArray[np.integer],
    target_normals: NDArray[np.floating],
    source_weights: NDArray[np.floating],
    source_normals: NDArray[np.floating],
    face_vertex_indices: NDArray[np.integer],
    barycentric: NDArray[np.floating],
    distances: NDArray[np.floating],
    max_distance: float,
    max_angle_deg: float,
    max_iterations: int = 100,
    tolerance: float = 1e-6,
) -> NDArray[np.floating]:
    matched = find_matched_vertices(
        distances, target_normals, source_normals, max_distance, max_angle_deg
    )
    weights = interpolate_weights(source_weights, face_vertex_indices, barycentric)
    return inpaint_weights(
        target_vertices, target_triangles, weights, matched, max_iterations, tolerance
    )
