"""Tests for avatar_sidecar module: submesh_to_sidecar, write_submesh_sidecar."""
from __future__ import annotations

from types import SimpleNamespace

import pytest

from blender_addon.common.avatar_sidecar import (
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


def test_submesh_attachment_round_trips_and_auto_is_omitted():
    """A Cap / Ring row writes ``attachment`` and reads it back; an Auto row writes no key."""
    roles_in = [
        {"mesh": "meshA", "material": "hair", "role": "hair", "attachment": "ring"},
        {"mesh": "meshA", "material": "skirt", "role": "garment"},
    ]
    submesh = SimpleNamespace(
        roles=FakeCollection(lambda: SimpleNamespace(material="", role="", attachment="AUTO")),
        hidden_triangles=FakeCollection(),
    )

    write_submesh_sidecar(submesh, "meshA", roles_in, [], IDENTITY_LOOPS)
    roles_out, _ = submesh_to_sidecar("meshA", submesh, IDENTITY_LOOPS)

    assert [item.attachment for item in submesh.roles] == ["RING", "AUTO"]
    assert roles_out == roles_in


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
