"""rename_legacy_chains: registered chains, orphan membership groups and stored bone references move to the new names."""
from __future__ import annotations

from types import SimpleNamespace

from blender_addon.common.chain_migration import rename_bone_references, rename_legacy_chains
from blender_addon.tests.test_spring_properties import FakeCollection


class _NamedCollection:
    def __init__(self, names):
        self._items = {name: SimpleNamespace(name=name) for name in names}

    def get(self, name):
        return self._items.get(name)

    def names(self) -> list[str]:
        return sorted(item.name for item in self._items.values())

    def __iter__(self):
        return iter(list(self._items.values()))


def _settings(chains, colliders):
    settings = SimpleNamespace(chains=FakeCollection(), colliders=FakeCollection())
    for name, origin, bones in chains:
        chain = settings.chains.add()
        chain.name = name
        chain.origin_mesh = origin["mesh"]
        chain.origin_material = origin["material"]
        chain.center = ""
        chain.tail = bones[-1]
        chain.joints = FakeCollection()
        for bone in bones:
            chain.joints.add().bone = bone
    for bone in colliders:
        settings.colliders.add().bone = bone
    return settings


def test_rename_legacy_chains_moves_bones_groups_and_collider_references(avatar_api, monkeypatch):
    monkeypatch.setattr("blender_addon.common.chain_migration.avatar_api", avatar_api)
    origin = {"mesh": "strip", "material": "hair"}
    bones = ["hair_strip_hair_00_00", "hair_strip_hair_00_01", "hair_strip_hair_00_01_end"]
    settings = _settings([("hair_strip_hair_00", origin, bones), ("strip_hair_01", origin, ["strip_hair_01_00"])], [bones[1]])
    arm = SimpleNamespace(data=SimpleNamespace(bones=_NamedCollection(bones), collections_all=_NamedCollection(["hair_strip_hair_00"])))
    obj = SimpleNamespace(vertex_groups=_NamedCollection(["thyllore_chain_hair_strip_hair_00", "thyllore_chain_hair_strip_hair_03", "other"]))

    renamed = rename_legacy_chains(arm, obj, settings, origin, "strip/hair")

    assert renamed == 2
    assert [chain.name for chain in settings.chains] == ["strip_hair_00", "strip_hair_01"]
    assert [joint.bone for joint in settings.chains[0].joints] == ["strip_hair_00_00", "strip_hair_00_01", "strip_hair_00_01_end"]
    assert settings.chains[0].tail == "strip_hair_00_01_end"
    assert settings.colliders[0].bone == "strip_hair_00_01"
    assert arm.data.bones.names() == ["strip_hair_00_00", "strip_hair_00_01", "strip_hair_00_01_end"]
    assert arm.data.collections_all.names() == ["strip_hair_00"]
    assert obj.vertex_groups.names() == ["other", "thyllore_chain_strip_hair_00", "thyllore_chain_strip_hair_03"]


def test_rename_bone_references_leaves_unrelated_names():
    settings = _settings([("tie", {"mesh": "", "material": ""}, ["tie_00", "tie_01"])], ["Head"])
    settings.chains[0].center = "Hips"

    rename_bone_references(settings, {"tie_01": "tie_01_new"})

    assert [joint.bone for joint in settings.chains[0].joints] == ["tie_00", "tie_01_new"]
    assert settings.chains[0].tail == "tie_01_new"
    assert settings.chains[0].center == "Hips"
    assert settings.colliders[0].bone == "Head"
