"""Tests for avatar_sidecar module: bone_specs, submesh_to_sidecar, write_submesh_sidecar, sidecar_stats."""
from __future__ import annotations

from types import SimpleNamespace

import pytest

from blender_addon.common.avatar_sidecar import (
    bone_specs,
    sidecar_stats,
    submesh_to_sidecar,
    write_submesh_sidecar,
)
from blender_addon.tests.test_spring_properties import FakeCollection


class _FakeHiddenItem:
    """Fake hidden triangle item with __getitem__, __setitem__, and get."""

    def __init__(self, material, triangles):
        self.material = material
        self._data = {"triangles": list(triangles)}

    def __getitem__(self, key):
        return self._data[key]

    def __setitem__(self, key, value):
        self._data[key] = value

    def get(self, key, default=None):
        return self._data.get(key, default)


def _hidden_item_factory(material, triangles):
    return _FakeHiddenItem(material, triangles)


def test_bone_specs_blender_to_engine():
    """bone_specs converts Blender (0,0,1)*2 to engine (0,2,0)."""
    names = ["root", "child"]
    parent_indices = [None, 0]
    world_heads = [(0.0, 0.0, 0.0), (0.0, 0.0, 2.0)]
    scale_length = 1.0

    specs = bone_specs(names, parent_indices, world_heads, scale_length)

    assert len(specs) == 2
    assert specs[0]["name"] == "root"
    assert specs[0]["parent"] is None
    assert specs[0]["rest_position"] == [0.0, 0.0, 0.0]
    assert specs[1]["name"] == "child"
    assert specs[1]["parent"] == 0
    assert specs[1]["rest_position"] == [0.0, 2.0, 0.0]


def test_bone_specs_scale_length():
    """bone_specs applies scale_length to convert to meters."""
    names = ["root", "child"]
    parent_indices = [None, 0]
    world_heads = [(0.0, 0.0, 0.0), (0.0, 0.0, 2.0)]
    scale_length = 0.5

    specs = bone_specs(names, parent_indices, world_heads, scale_length)

    assert specs[1]["rest_position"] == [0.0, 1.0, 0.0]


def test_submesh_to_sidecar():
    """submesh_to_sidecar builds roles and hidden_triangles from a submesh."""
    roles_collection = FakeCollection(lambda: SimpleNamespace(material="matA", role="shell"))
    roles_collection.add()

    hidden_collection = FakeCollection(lambda: _hidden_item_factory("matA", [0, 1, 2]))
    hidden_collection.add()

    submesh = SimpleNamespace(roles=roles_collection, hidden_triangles=hidden_collection)

    roles, hidden = submesh_to_sidecar("meshA", submesh)

    assert len(roles) == 1
    assert roles[0] == {"mesh": "meshA", "material": "matA", "role": "shell"}
    assert len(hidden) == 1
    assert hidden[0] == {"mesh": "meshA", "material": "matA", "triangles": [0, 1, 2]}


def test_submesh_to_sidecar_empty_triangles_skipped():
    """submesh_to_sidecar skips hidden triangle entries with empty triangles list."""
    roles_collection = FakeCollection(lambda: SimpleNamespace(material="matA", role="shell"))

    hidden_collection = FakeCollection(lambda: _hidden_item_factory("matA", []))
    hidden_collection.add()

    submesh = SimpleNamespace(roles=roles_collection, hidden_triangles=hidden_collection)

    _, hidden = submesh_to_sidecar("meshA", submesh)

    assert len(hidden) == 0


def test_write_submesh_sidecar_round_trip():
    """write_submesh_sidecar is idempotent: write then read back matches."""
    roles_in = [{"mesh": "meshA", "material": "matA", "role": "shell"}]
    hidden_in = [{"mesh": "meshA", "material": "matA", "triangles": [0, 1, 2]}]

    submesh = SimpleNamespace(
        roles=FakeCollection(lambda: SimpleNamespace(material="", role="")),
        hidden_triangles=FakeCollection(lambda: _hidden_item_factory("", [])),
    )

    write_submesh_sidecar(submesh, "meshA", roles_in, hidden_in)
    roles_out, hidden_out = submesh_to_sidecar("meshA", submesh)

    assert roles_out == roles_in
    assert hidden_out == hidden_in


def test_write_submesh_sidecar_filters_by_mesh():
    """write_submesh_sidecar only writes entries matching mesh_name."""
    roles_in = [
        {"mesh": "meshA", "material": "matA", "role": "shell"},
        {"mesh": "meshB", "material": "matB", "role": "inner"},
    ]
    hidden_in = [
        {"mesh": "meshA", "material": "matA", "triangles": [0, 1]},
        {"mesh": "meshB", "material": "matB", "triangles": [2, 3]},
    ]

    submesh = SimpleNamespace(
        roles=FakeCollection(lambda: SimpleNamespace(material="", role="")),
        hidden_triangles=FakeCollection(lambda: _hidden_item_factory("", [])),
    )

    write_submesh_sidecar(submesh, "meshA", roles_in, hidden_in)
    roles_out, hidden_out = submesh_to_sidecar("meshA", submesh)

    assert len(roles_out) == 1
    assert roles_out[0]["material"] == "matA"
    assert len(hidden_out) == 1
    assert hidden_out[0]["material"] == "matA"


def test_write_submesh_sidecar_idempotent():
    """write_submesh_sidecar is idempotent: writing twice gives the same result."""
    roles_in = [{"mesh": "meshA", "material": "matA", "role": "shell"}]
    hidden_in = [{"mesh": "meshA", "material": "matA", "triangles": [0, 1, 2]}]

    submesh = SimpleNamespace(
        roles=FakeCollection(lambda: SimpleNamespace(material="", role="")),
        hidden_triangles=FakeCollection(lambda: _hidden_item_factory("", [])),
    )

    write_submesh_sidecar(submesh, "meshA", roles_in, hidden_in)
    first = submesh_to_sidecar("meshA", submesh)

    write_submesh_sidecar(submesh, "meshA", roles_in, hidden_in)
    second = submesh_to_sidecar("meshA", submesh)

    assert first == second


def test_sidecar_stats():
    """sidecar_stats returns all 10 keys matching Rust AvatarStats."""
    spring = {
        "chains": [
            {"joints": [{"bone": "a"}, {"bone": "b"}]},
            {"joints": [{"bone": "c"}]},
        ],
        "colliders": [{"name": "c1"}, {"name": "c2"}, {"name": "c3"}],
    }

    stats = sidecar_stats(
        triangle_count=100,
        bone_count=50,
        material_count=4,
        mesh_count=5,
        skinned_mesh_count=2,
        morph_mesh_count=1,
        spring=spring,
    )

    assert stats["triangles"] == 100
    assert stats["bones"] == 50
    assert stats["materials"] == 4
    assert stats["meshes"] == 5
    assert stats["skinned_meshes"] == 2
    assert stats["morph_meshes"] == 1
    assert stats["spring_chains"] == 2
    assert stats["spring_transforms"] == 3  # 2 + 1 joints
    assert stats["spring_colliders"] == 3
    assert stats["texture_bytes"] == 0


def test_sidecar_stats_empty_spring():
    """sidecar_stats with empty spring has zero spring values."""
    stats = sidecar_stats(10, 5, 1, 1, 1, 0, {})

    assert stats["spring_chains"] == 0
    assert stats["spring_transforms"] == 0
    assert stats["spring_colliders"] == 0
