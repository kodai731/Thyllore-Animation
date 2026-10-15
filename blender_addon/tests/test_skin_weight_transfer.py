"""Unit tests for skin_weight_transfer module."""
from __future__ import annotations

import numpy as np

from blender_addon.common.skin_weight_transfer import (
    cotangent_laplacian,
    find_matched_vertices,
    inpaint_weights,
    interpolate_weights,
)


def test_cotangent_laplacian_row_sum_zero_and_symmetric():
    """Square mesh of 2 triangles: Laplacian row sums are ~0 and matrix is symmetric."""
    vertices = np.array([[0.0, 0.0, 0.0],
                         [1.0, 0.0, 0.0],
                         [1.0, 1.0, 0.0],
                         [0.0, 1.0, 0.0]], dtype=np.float64)
    triangles = np.array([[0, 1, 2],
                          [0, 2, 3]], dtype=np.int64)

    rows, cols, vals = cotangent_laplacian(vertices, triangles)

    row_sums = np.zeros(4, dtype=np.float64)
    np.add.at(row_sums, rows, vals)
    np.testing.assert_allclose(row_sums, 0.0, atol=1e-10)

    off_diag = np.zeros((4, 4), dtype=np.float64)
    np.add.at(off_diag, (rows, cols), vals)
    np.testing.assert_allclose(off_diag, off_diag.T, atol=1e-10)


def test_inpaint_weights_monotone_decrease_and_row_sum_one():
    """Five vertices in a strip: left bone0=1, right bone1=1 fixed; middle 3 inpainted.

    bone0 weights should monotonically decrease from left to right, and each row
    should sum to ~1.
    """
    vertices = np.array([[0.0, 0.0, 0.0],
                         [1.0, 0.0, 0.0],
                         [2.0, 0.0, 0.0],
                         [3.0, 0.0, 0.0],
                         [4.0, 0.0, 0.0],
                         [0.0, -1.0, 0.0]], dtype=np.float64)
    triangles = np.array([[0, 1, 5],
                          [1, 2, 5],
                          [2, 3, 5],
                          [3, 4, 5]], dtype=np.int64)

    weights = np.zeros((6, 2), dtype=np.float64)
    weights[0, 0] = 1.0
    weights[4, 1] = 1.0

    matched = np.array([True, False, False, False, True, False], dtype=bool)

    result = inpaint_weights(vertices, triangles, weights, matched)

    row_sums = result.sum(axis=1)
    np.testing.assert_allclose(row_sums, 1.0, atol=1e-3)

    bone0 = result[:5, 0]
    for i in range(4):
        assert bone0[i] >= bone0[i + 1] - 1e-6


def test_find_matched_vertices_distance_and_angle():
    """Distance exceeded -> False; angle exceeded -> False; flipped normal -> True."""
    distances = np.array([0.01, 0.5, 0.01, 0.01], dtype=np.float64)

    target_normals = np.array([[1.0, 0.0, 0.0],
                               [0.0, 1.0, 0.0],
                               [0.0, 1.0, 0.0],
                               [-1.0, 0.0, 0.0]], dtype=np.float64)

    source_normals = np.array([[1.0, 0.0, 0.0],
                               [1.0, 0.0, 0.0],
                               [1.0, 0.0, 0.0],
                               [1.0, 0.0, 0.0]], dtype=np.float64)

    result = find_matched_vertices(distances, target_normals, source_normals,
                                   max_distance=0.1, max_angle_deg=30.0)

    assert result[0] == True
    assert result[1] == False
    assert result[2] == False
    assert result[3] == True


def test_interpolate_weights_barycentric_vertex_zero():
    """Barycentric (1, 0, 0) returns vertex 0's weights."""
    source_weights = np.array([[1.0, 0.0],
                               [0.0, 1.0],
                               [0.5, 0.5]], dtype=np.float64)

    face_vertex_indices = np.array([[0, 1, 2]], dtype=np.int64)
    barycentric = np.array([[1.0, 0.0, 0.0]], dtype=np.float64)

    result = interpolate_weights(source_weights, face_vertex_indices, barycentric)

    np.testing.assert_allclose(result, [[1.0, 0.0]])
