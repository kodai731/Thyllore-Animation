"""Tests for blender_addon.common.bone_frames."""
from __future__ import annotations

from blender_addon.common.bone_frames import top_influences


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

