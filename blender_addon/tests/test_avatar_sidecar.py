"""Tests for avatar_sidecar module: bone_specs, submesh_to_sidecar, write_submesh_sidecar."""
from __future__ import annotations

from types import SimpleNamespace

import pytest

from blender_addon.common.avatar_sidecar import (
    bone_specs,
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


IDENTITY_LOOPS = {"matA": [0, 1, 2, 3], "matB": [0, 1, 2, 3]}


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

    roles, hidden = submesh_to_sidecar("meshA", submesh, IDENTITY_LOOPS)

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

    _, hidden = submesh_to_sidecar("meshA", submesh, IDENTITY_LOOPS)

    assert len(hidden) == 0


def test_write_submesh_sidecar_round_trip():
    """write_submesh_sidecar is idempotent: write then read back matches."""
    roles_in = [{"mesh": "meshA", "material": "matA", "role": "shell"}]
    hidden_in = [{"mesh": "meshA", "material": "matA", "triangles": [0, 1, 2]}]

    submesh = SimpleNamespace(
        roles=FakeCollection(lambda: SimpleNamespace(material="", role="")),
        hidden_triangles=FakeCollection(lambda: _hidden_item_factory("", [])),
    )

    write_submesh_sidecar(submesh, "meshA", roles_in, hidden_in, IDENTITY_LOOPS)
    roles_out, hidden_out = submesh_to_sidecar("meshA", submesh, IDENTITY_LOOPS)

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

    write_submesh_sidecar(submesh, "meshA", roles_in, hidden_in, IDENTITY_LOOPS)
    roles_out, hidden_out = submesh_to_sidecar("meshA", submesh, IDENTITY_LOOPS)

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

    write_submesh_sidecar(submesh, "meshA", roles_in, hidden_in, IDENTITY_LOOPS)
    first = submesh_to_sidecar("meshA", submesh, IDENTITY_LOOPS)

    write_submesh_sidecar(submesh, "meshA", roles_in, hidden_in, IDENTITY_LOOPS)
    second = submesh_to_sidecar("meshA", submesh, IDENTITY_LOOPS)

    assert first == second


def test_submesh_to_sidecar_numbers_triangles_inside_the_material():
    """A loop_triangles index is written as the triangle's index inside its material submesh."""
    roles_collection = FakeCollection(lambda: SimpleNamespace(material="", role=""))
    hidden_collection = FakeCollection(lambda: _hidden_item_factory("Skin", [1, 3]))
    hidden_collection.add()
    submesh = SimpleNamespace(roles=roles_collection, hidden_triangles=hidden_collection)

    _, hidden = submesh_to_sidecar("Body", submesh, {"Skin": [1, 3], "Cloth": [0, 2]})

    assert hidden == [{"mesh": "Body", "material": "Skin", "triangles": [0, 1]}]


def test_write_submesh_sidecar_restores_loop_triangle_indices():
    """Sidecar triangle numbers come back as loop_triangles indices of the material."""
    hidden_in = [{"mesh": "Body", "material": "Skin", "triangles": [0, 1, 9]}]
    submesh = SimpleNamespace(
        roles=FakeCollection(lambda: SimpleNamespace(material="", role="")),
        hidden_triangles=FakeCollection(lambda: _hidden_item_factory("", [])),
    )

    write_submesh_sidecar(submesh, "Body", [], hidden_in, {"Skin": [1, 3]})

    assert [item.get("triangles") for item in submesh.hidden_triangles] == [[1, 3]]
