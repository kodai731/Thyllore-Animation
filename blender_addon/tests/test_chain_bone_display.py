"""Chain bones live in one bone collection per chain under 'Thyllore Chains', so Blender's collection select
buttons pick a chain's bones the standard way."""
from __future__ import annotations

from types import SimpleNamespace

from blender_addon.common.chain_bone_display import (
    CHAIN_COLLECTION_NAME,
    apply_chain_bone_display,
    apply_chain_display,
    remove_chain_collections,
)


class FakeCollection:
    def __init__(self, name, parent=None):
        self.name = name
        self.parent = parent
        self.bones = []

    def assign(self, bone):
        self.bones.append(bone.name)


class FakeCollections:
    def __init__(self):
        self.items = {}

    def get(self, name):
        return self.items.get(name)

    def new(self, name, parent=None):
        self.items[name] = FakeCollection(name, parent)
        return self.items[name]

    def remove(self, collection):
        del self.items[collection.name]


def _bone(name):
    return SimpleNamespace(name=name, display_type="OCTAHEDRAL", color=SimpleNamespace(palette="DEFAULT"))


def _armature(bone_names, chains=()):
    bones = {name: _bone(name) for name in bone_names}
    collections = FakeCollections()
    data = SimpleNamespace(
        collections=collections,
        collections_all=collections,
        bones=SimpleNamespace(get=bones.get),
        thyllore_spring=SimpleNamespace(chains=list(chains)),
    )
    return SimpleNamespace(data=data)


def test_each_chain_gets_its_own_collection_under_the_parent():
    arm = _armature(["hair_h_00_00", "hair_h_00_01", "hair_h_01_00"])

    apply_chain_bone_display(arm, "hair_h_00", ["hair_h_00_00", "hair_h_00_01"], "cap")
    apply_chain_bone_display(arm, "hair_h_01", ["hair_h_01_00", "missing"], "cap")

    collections = arm.data.collections.items
    assert set(collections) == {CHAIN_COLLECTION_NAME, "hair_h_00", "hair_h_01"}
    assert collections["hair_h_00"].parent is collections[CHAIN_COLLECTION_NAME]
    assert collections["hair_h_00"].bones == ["hair_h_00_00", "hair_h_00_01"]
    assert collections["hair_h_01"].bones == ["hair_h_01_00"]
    assert arm.data.bones.get("hair_h_00_01").display_type == "STICK"


def test_apply_chain_bone_display_reuses_an_existing_chain_collection():
    arm = _armature(["a_00", "a_01"])
    apply_chain_bone_display(arm, "a", ["a_00"], "ring")

    apply_chain_bone_display(arm, "a", ["a_01"], "ring")

    assert arm.data.collections.items["a"].bones == ["a_00", "a_01"]


def test_remove_chain_collections_drops_only_the_named_chains():
    arm = _armature(["a_00", "b_00"])
    apply_chain_bone_display(arm, "a", ["a_00"], "cap")
    apply_chain_bone_display(arm, "b", ["b_00"], "cap")

    remove_chain_collections(arm, ["a", "absent"])

    assert set(arm.data.collections.items) == {CHAIN_COLLECTION_NAME, "b"}


def test_apply_chain_display_names_collections_after_the_chains():
    chain = SimpleNamespace(name="skirt", origin_attachment="ring", joints=[SimpleNamespace(bone="skirt_00")])
    arm = _armature(["skirt_00"], [chain])

    apply_chain_display(arm)

    assert arm.data.collections.items["skirt"].bones == ["skirt_00"]
    assert arm.data.bones.get("skirt_00").color.palette == "THEME04"
