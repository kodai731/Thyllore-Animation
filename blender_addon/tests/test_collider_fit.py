"""Tests for blender_addon.common.collider_fit."""
from __future__ import annotations

from blender_addon.common.collider_fit import bone_bind_matrix, scaled_bind_matrix, top_influences


def test_top_influences_basic():
    groups = [(0, 1.0)]
    group_to_bone = {0: 2}
    assert top_influences(groups, group_to_bone) == [(2, 1.0)]


def test_top_influences_sorted_by_weight_descending():
    groups = [(0, 0.3), (1, 0.7), (2, 0.5)]
    group_to_bone = {0: 10, 1: 20, 2: 30}
    assert top_influences(groups, group_to_bone) == [(20, 0.7), (30, 0.5), (10, 0.3)]


def test_top_influences_limit():
    groups = [(0, 0.4), (1, 0.6), (2, 0.8), (3, 0.9), (4, 0.1)]
    group_to_bone = {i: i for i in range(5)}
    assert top_influences(groups, group_to_bone, limit=2) == [(3, 0.9), (2, 0.8)]


def test_top_influences_drops_unknown_group():
    groups = [(0, 1.0), (99, 0.5)]
    group_to_bone = {0: 5}
    assert top_influences(groups, group_to_bone) == [(5, 1.0)]


def test_top_influences_drops_zero_weight():
    groups = [(0, 0.0), (1, 1.0)]
    group_to_bone = {0: 0, 1: 1}
    assert top_influences(groups, group_to_bone) == [(1, 1.0)]


def test_top_influences_empty():
    assert top_influences([], {}) == []


def test_scaled_bind_matrix_identity():
    identity = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
    result = scaled_bind_matrix(identity, 2.0)
    assert result == identity


def test_scaled_bind_matrix_translation():
    m = [
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 2.0],
        [0.0, 0.0, 1.0, 3.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
    result = scaled_bind_matrix(m, 2.0)
    assert result[0][3] == 2.0
    assert result[1][3] == 4.0
    assert result[2][3] == 6.0
    assert result[3] == [0.0, 0.0, 0.0, 1.0]


def test_scaled_bind_matrix_does_not_modify_original():
    m = [
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 2.0],
        [0.0, 0.0, 1.0, 3.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
    scaled_bind_matrix(m, 2.0)
    assert m[0][3] == 1.0
    assert m[1][3] == 2.0
    assert m[2][3] == 3.0


def _identity():
    return [[1.0 if i == j else 0.0 for j in range(4)] for i in range(4)]


def _rotation_x_90():
    return [[1.0, 0.0, 0.0, 0.0], [0.0, 0.0, -1.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]]


def _translated(rows, t):
    out = [row[:] for row in rows]
    for i in range(3):
        out[i][3] = t[i]
    return out


def test_bone_bind_matrix_fbx_import_armature_keeps_the_bone_axes():
    """An imported FBX armature carries the Z-up conversion; a bone with identity rotation stays identity."""
    bone_local = _translated(_identity(), (0.0, 1.55, -0.03))
    bind = bone_bind_matrix(_rotation_x_90(), bone_local, 1.0)
    for i in range(3):
        for j in range(3):
            assert abs(bind[i][j] - (1.0 if i == j else 0.0)) < 1e-9
    assert [round(bind[i][3], 6) for i in range(3)] == [0.0, 1.55, -0.03]


def test_bone_bind_matrix_native_armature_gets_the_axis_conversion_once():
    """A Blender-native armature (identity world) exports with the Z-up to Y-up rotation on its node."""
    bind = bone_bind_matrix(_identity(), _translated(_identity(), (0.0, 0.0, 2.0)), 1.0)
    expected = [[1.0, 0.0, 0.0, 0.0], [0.0, 0.0, 1.0, 2.0], [0.0, -1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]]
    assert [[round(v, 6) for v in row] for row in bind] == expected


def test_bone_bind_matrix_scales_translation_only():
    bind = bone_bind_matrix(_identity(), _translated(_identity(), (0.0, 0.0, 2.0)), 0.01)
    assert round(bind[1][3], 6) == 0.02
    assert bind[1][2] == 1.0
