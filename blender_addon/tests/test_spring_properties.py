"""Tests for spring_properties module: round-trip, disabled chain exclusion, idempotent write."""
from __future__ import annotations

import pytest
from types import SimpleNamespace

from blender_addon.common.spring_properties import (
    configure_force_params,
    chain_with_joint,
    remove_chains_generated_on,
    copy_joint_values,
    disabled_chain_names,
    humanoid_overrides_to_dict,
    joint_values,
    restore_disabled_chains,
    spring_settings_to_edit,
    spring_settings_to_sidecar,
    write_sidecar_spring,
)


configure_force_params({
    "vrm": ("stiffness", "gravity_power"),
    "rest_relative": ("bending_relaxation", "gravity_scale"),
})


def _vrm_force(stiffness, gravity_power):
    return {"model": "vrm", "stiffness": stiffness, "gravity_power": gravity_power}


class _ItemProxy:
    """Thin proxy that delegates attribute access to an underlying item.

    Each instance is a distinct object, so ``is`` identity checks between two
    proxies wrapping the same item are always False — matching Blender RNA's
    behavior where each collection access returns a new wrapper."""

    __slots__ = ("_item",)

    def __init__(self, item):
        self._item = item

    def __getattr__(self, name):
        return getattr(self._item, name)

    def __setattr__(self, name, value):
        if name == "_item":
            super().__setattr__(name, value)
        else:
            setattr(self._item, name, value)


class FakeCollection:
    """Minimal fake collection with .add(), .clear(), and iteration.

    ``item_factory`` (callable, no args) controls what each .add() returns.
    Defaults to a plain SimpleNamespace."""

    def __init__(self, item_factory=None):
        self._items = []
        self._factory = item_factory or (lambda: SimpleNamespace())

    def add(self):
        item = self._factory()
        self._items.append(item)
        return item

    def clear(self):
        self._items.clear()

    def remove(self, index):
        del self._items[index]

    def __iter__(self):
        return (_ItemProxy(item) for item in self._items)

    def __len__(self):
        return len(self._items)

    def __getitem__(self, index):
        return _ItemProxy(self._items[index])


def _chain_item_factory():
    """Factory for chain items: each has joints and collider_groups as FakeCollections."""
    item = SimpleNamespace()
    item.joints = FakeCollection()
    item.collider_groups = FakeCollection()
    return item


def _collider_group_item_factory():
    """Factory for collider group items: each has colliders as FakeCollection."""
    item = SimpleNamespace()
    item.colliders = FakeCollection()
    return item


def _make_settings(chains_data, colliders_data, groups_data, collision_mode="vrm_compatible"):
    """Build a fake settings object from chain/collider/group data."""
    settings = SimpleNamespace()
    settings.collision_mode = collision_mode
    settings.chains = FakeCollection(_chain_item_factory)
    settings.colliders = FakeCollection()
    settings.collider_groups = FakeCollection(_collider_group_item_factory)

    for cd in chains_data:
        item = settings.chains.add()
        item.name = cd["name"]
        item.center = cd.get("center", "")
        item.enabled = cd.get("enabled", True)
        item.tail = cd.get("tail", "")
        origin = cd.get("origin") or {}
        item.origin_mesh = origin.get("mesh", "")
        item.origin_material = origin.get("material", "")
        item.origin_role = origin.get("role", "")
        for jd in cd["joints"]:
            j = item.joints.add()
            j.bone = jd["bone"]
            j.force_model = jd["force"]["model"]
            j.stiffness = jd["force"].get("stiffness", 0.0)
            j.gravity_power = jd["force"].get("gravity_power", 0.0)
            j.bending_relaxation = jd["force"].get("bending_relaxation", 0.0)
            j.gravity_scale = jd["force"].get("gravity_scale", 0.0)
            j.drag_force = jd["drag_force"]
            j.gravity_dir = jd["gravity_dir"]
            j.hit_radius = jd["hit_radius"]
        for cg in cd.get("collider_groups", []):
            entry = item.collider_groups.add()
            entry.name = cg

    for cld in colliders_data:
        c = settings.colliders.add()
        c.name = cld["name"]
        c.bone = cld["bone"]
        c.offset = cld["offset"]
        c.shape = cld["shape"]
        c.radius = cld["radius"]
        c.has_tail = cld.get("has_tail", False)
        c.tail = cld.get("tail", (0.0, 0.0, 0.0))
        c.origin = cld.get("origin", "authored")

    for gd in groups_data:
        g = settings.collider_groups.add()
        g.name = gd["name"]
        for cn in gd["colliders"]:
            entry = g.colliders.add()
            entry.name = cn

    return settings


TEST_CHAINS = [
    {
        "name": "chain_0",
        "center": "Hips",
        "enabled": True,
        "tail": "Spine3",
        "joints": [
            {
                "bone": "Hips",
                "force": _vrm_force(1.0, 1.0),
                "drag_force": 0.5,
                "gravity_dir": (0.0, -1.0, 0.0),
                "hit_radius": 0.05,
            },
            {
                "bone": "Spine",
                "force": _vrm_force(0.8, 0.5),
                "drag_force": 0.3,
                "gravity_dir": (0.0, -1.0, 0.0),
                "hit_radius": 0.03,
            },
        ],
        "collider_groups": ["body"],
    }
]

TEST_COLLIDERS = [
    {
        "name": "torso",
        "bone": "Hips",
        "offset": (0.0, 1.0, 0.0),
        "shape": "capsule",
        "radius": 0.2,
        "has_tail": True,
        "tail": (0.0, 3.0, 0.0),
        "origin": "authored",
    }
]

TEST_GROUPS = [{"name": "body", "colliders": ["torso"]}]


def _expected_sidecar():
    return {
        "collision_mode": "vrm_compatible",
        "chains": [
            {
                "name": "chain_0",
                "center": "Hips",
                "collider_groups": ["body"],
                "joints": [
                    {"bone": "Hips", "force": _vrm_force(1.0, 1.0), "drag_force": 0.5, "gravity_dir": (0.0, 0.0, 1.0), "hit_radius": 0.05},
                    {"bone": "Spine", "force": _vrm_force(0.8, 0.5), "drag_force": 0.3, "gravity_dir": (0.0, 0.0, 1.0), "hit_radius": 0.03},
                ],
                "tail": "Spine3",
                "origin": None,
            }
        ],
        "colliders": [
            {"name": "torso", "bone": "Hips", "offset": [0.0, 1.0, 0.0], "shape": "capsule", "radius": 0.2, "tail": [0.0, 3.0, 0.0], "origin": "authored"}
        ],
        "collider_groups": [{"name": "body", "colliders": ["torso"]}],
    }


def test_round_trip():
    """write_sidecar_spring -> spring_settings_to_sidecar round-trips to the original sidecar."""
    settings = _make_settings(TEST_CHAINS, TEST_COLLIDERS, TEST_GROUPS)
    expected = _expected_sidecar()

    sidecar = spring_settings_to_sidecar(settings)
    assert sidecar == expected

    write_sidecar_spring(settings, expected)
    round_tripped = spring_settings_to_sidecar(settings)

    assert round_tripped["collision_mode"] == expected["collision_mode"]
    assert len(round_tripped["chains"]) == len(expected["chains"])
    assert len(round_tripped["colliders"]) == len(expected["colliders"])
    assert len(round_tripped["collider_groups"]) == len(expected["collider_groups"])

    for rt_chain, exp_chain in zip(round_tripped["chains"], expected["chains"]):
        assert rt_chain["name"] == exp_chain["name"]
        assert rt_chain["center"] == exp_chain["center"]
        assert rt_chain["tail"] == exp_chain["tail"]
        assert rt_chain["collider_groups"] == exp_chain["collider_groups"]
        for rt_joint, exp_joint in zip(rt_chain["joints"], exp_chain["joints"]):
            assert rt_joint["bone"] == exp_joint["bone"]
            assert rt_joint["force"] == pytest.approx(exp_joint["force"])
            assert rt_joint["drag_force"] == pytest.approx(exp_joint["drag_force"])
            assert rt_joint["gravity_dir"] == pytest.approx(exp_joint["gravity_dir"])
            assert rt_joint["hit_radius"] == pytest.approx(exp_joint["hit_radius"])

    for rt_c, exp_c in zip(round_tripped["colliders"], expected["colliders"]):
        assert rt_c["name"] == exp_c["name"]
        assert rt_c["bone"] == exp_c["bone"]
        assert rt_c["offset"] == pytest.approx(exp_c["offset"])
        assert rt_c["shape"] == exp_c["shape"]
        assert rt_c["radius"] == pytest.approx(exp_c["radius"])
        assert rt_c["tail"] == pytest.approx(exp_c["tail"])
        assert rt_c["origin"] == exp_c["origin"]

    for rt_g, exp_g in zip(round_tripped["collider_groups"], expected["collider_groups"]):
        assert rt_g["name"] == exp_g["name"]
        assert rt_g["colliders"] == exp_g["colliders"]


def test_disabled_chain_excluded():
    """A chain with enabled=False is not included in the output."""
    chains = [
        {"name": "visible", "enabled": True, "joints": [{"bone": "Hips", "force": _vrm_force(1.0, 1.0), "drag_force": 0.5, "gravity_dir": (0.0, -1.0, 0.0), "hit_radius": 0.05}]},
        {"name": "hidden", "enabled": False, "joints": [{"bone": "Spine", "force": _vrm_force(1.0, 1.0), "drag_force": 0.5, "gravity_dir": (0.0, -1.0, 0.0), "hit_radius": 0.05}]},
    ]
    settings = _make_settings(chains, [], [])
    sidecar = spring_settings_to_sidecar(settings)

    assert len(sidecar["chains"]) == 1
    assert sidecar["chains"][0]["name"] == "visible"


def test_idempotent_double_write():
    """Writing the same sidecar twice does not increase item counts."""
    settings = _make_settings(TEST_CHAINS, TEST_COLLIDERS, TEST_GROUPS)
    expected = _expected_sidecar()

    write_sidecar_spring(settings, expected)
    count_chains_1 = len(settings.chains)
    count_colliders_1 = len(settings.colliders)
    count_groups_1 = len(settings.collider_groups)

    write_sidecar_spring(settings, expected)
    assert len(settings.chains) == count_chains_1 == 1
    assert len(settings.colliders) == count_colliders_1 == 1
    assert len(settings.collider_groups) == count_groups_1 == 1


def test_empty_center_and_tail_become_none():
    """Empty string center/tail in settings becomes None in sidecar."""
    chains = [{"name": "chain_0", "center": "", "enabled": True, "tail": "", "joints": [{"bone": "Hips", "force": _vrm_force(1.0, 1.0), "drag_force": 0.5, "gravity_dir": (0.0, -1.0, 0.0), "hit_radius": 0.05}]}]
    settings = _make_settings(chains, [], [])
    sidecar = spring_settings_to_sidecar(settings)

    assert sidecar["chains"][0]["center"] is None
    assert sidecar["chains"][0]["tail"] is None


def test_sphere_collider_no_tail():
    """A sphere collider (has_tail=False) has tail=None in the sidecar."""
    colliders = [{"name": "sphere_c", "bone": "Hips", "offset": (0.0, 1.0, 0.0), "shape": "sphere", "radius": 0.1, "has_tail": False, "tail": (0.0, 0.0, 0.0), "origin": "authored"}]
    settings = _make_settings([], colliders, [])
    sidecar = spring_settings_to_sidecar(settings)

    assert sidecar["colliders"][0]["tail"] is None


def test_humanoid_overrides_to_dict():
    """humanoid_overrides_to_dict reads role/bone pairs correctly."""
    settings = SimpleNamespace()
    settings.humanoid_overrides = FakeCollection()

    item1 = settings.humanoid_overrides.add()
    item1.role = "hips"
    item1.bone = "Hips_Bone"

    item2 = settings.humanoid_overrides.add()
    item2.role = "spine"
    item2.bone = "Spine_Bone"

    result = humanoid_overrides_to_dict(settings)
    assert result == {"hips": "Hips_Bone", "spine": "Spine_Bone"}


def test_edit_write_restore_keeps_disabled():
    """disabled chain settings -> edit dict -> write -> restore -> disabled stays.

    spring_settings_to_sidecar (export) drops disabled chains, so an operator that
    reads via spring_settings_to_edit and writes back must restore them."""
    chains = [
        {"name": "visible", "enabled": True, "joints": [{"bone": "Hips", "force": _vrm_force(1.0, 1.0), "drag_force": 0.5, "gravity_dir": (0.0, -1.0, 0.0), "hit_radius": 0.05}]},
        {"name": "hidden", "enabled": False, "joints": [{"bone": "Spine", "force": _vrm_force(1.0, 1.0), "drag_force": 0.5, "gravity_dir": (0.0, -1.0, 0.0), "hit_radius": 0.05}]},
    ]
    settings = _make_settings(chains, [], [])

    disabled = disabled_chain_names(settings)
    assert disabled == {"hidden"}

    sidecar = spring_settings_to_edit(settings)
    assert len(sidecar["chains"]) == 2

    write_sidecar_spring(settings, sidecar)
    restore_disabled_chains(settings, disabled)

    assert len(settings.chains) == 2
    names = [c.name for c in settings.chains]
    assert "visible" in names
    assert "hidden" in names
    for c in settings.chains:
        if c.name == "visible":
            assert c.enabled is True
        elif c.name == "hidden":
            assert c.enabled is False


def _joint_item(force_model, stiffness, gravity_power, bending_relaxation, gravity_scale, drag_force, gravity_dir, hit_radius):
    j = SimpleNamespace()
    j.force_model = force_model
    j.stiffness = stiffness
    j.gravity_power = gravity_power
    j.bending_relaxation = bending_relaxation
    j.gravity_scale = gravity_scale
    j.drag_force = drag_force
    j.gravity_dir = gravity_dir
    j.hit_radius = hit_radius
    return j


def test_joint_values():
    """joint_values reads the active model's force and the shared values with gravity_dir conversion."""
    j = _joint_item("vrm", 1.0, 1.0, 0.3, 2.0, 0.5, (0.0, -1.0, 0.0), 0.05)

    values = joint_values(j)

    assert values["force"] == {"model": "vrm", "stiffness": pytest.approx(1.0), "gravity_power": pytest.approx(1.0)}
    assert values["drag_force"] == pytest.approx(0.5)
    assert values["gravity_dir"] == (0.0, 0.0, 1.0)
    assert values["hit_radius"] == pytest.approx(0.05)


def test_joint_values_rest_relative_reads_its_own_magnitudes():
    j = _joint_item("rest_relative", 1.0, 1.0, 0.3, 2.0, 0.5, (0.0, -1.0, 0.0), 0.05)

    assert joint_values(j)["force"] == {
        "model": "rest_relative",
        "bending_relaxation": pytest.approx(0.3),
        "gravity_scale": pytest.approx(2.0),
    }


def test_copy_joint_values():
    """copy_joint_values copies the force model, every model's magnitudes and the shared values."""
    source = _joint_item("rest_relative", 2.0, 1.5, 0.4, 3.0, 0.8, (0.0, -1.0, 0.0), 0.1)
    target = _joint_item("vrm", 0.0, 0.0, 0.0, 0.0, 0.0, (0.0, 0.0, 0.0), 0.0)

    copy_joint_values(source, target)

    assert target.force_model == "rest_relative"
    assert target.stiffness == pytest.approx(2.0)
    assert target.gravity_power == pytest.approx(1.5)
    assert target.bending_relaxation == pytest.approx(0.4)
    assert target.gravity_scale == pytest.approx(3.0)
    assert target.drag_force == pytest.approx(0.8)
    assert target.gravity_dir == (0.0, -1.0, 0.0)
    assert target.hit_radius == pytest.approx(0.1)


def test_chain_with_joint_found():
    """chain_with_joint finds the chain containing a joint with the given bone name."""
    settings = _make_settings(
        [
            {"name": "chain_a", "joints": [{"bone": "Hips", "force": _vrm_force(1.0, 1.0), "drag_force": 0.5, "gravity_dir": (0.0, -1.0, 0.0), "hit_radius": 0.05}]},
            {"name": "chain_b", "joints": [{"bone": "Spine", "force": _vrm_force(1.0, 1.0), "drag_force": 0.5, "gravity_dir": (0.0, -1.0, 0.0), "hit_radius": 0.05}]},
        ],
        [],
        [],
    )

    chain = chain_with_joint(settings, "Spine")
    assert chain is not None
    assert chain.name == "chain_b"


def test_chain_with_joint_not_found():
    """chain_with_joint returns None when no joint matches the bone name."""
    settings = _make_settings(
        [
            {"name": "chain_a", "joints": [{"bone": "Hips", "force": _vrm_force(1.0, 1.0), "drag_force": 0.5, "gravity_dir": (0.0, -1.0, 0.0), "hit_radius": 0.05}]},
        ],
        [],
        [],
    )

    chain = chain_with_joint(settings, "Missing")
    assert chain is None


def test_generated_origin_round_trips_and_authored_is_none():
    """A chain's origin survives write_sidecar_spring -> spring_settings_to_sidecar; no origin -> None."""
    settings = _make_settings([], [], [])
    spring = {
        "collision_mode": "vrm_compatible",
        "colliders": [],
        "collider_groups": [],
        "chains": [
            {
                "name": "hair_hair_back_00",
                "center": None,
                "collider_groups": [],
                "joints": [{"bone": "hair_hair_back_00_00", "force": _vrm_force(1.0, 1.0), "drag_force": 0.5, "gravity_dir": (0.0, -1.0, 0.0), "hit_radius": 0.05}],
                "tail": None,
                "origin": {"mesh": "Hair_back", "material": "Hair", "role": "hair"},
            },
            {
                "name": "authored",
                "center": None,
                "collider_groups": [],
                "joints": [{"bone": "Spine", "force": _vrm_force(1.0, 1.0), "drag_force": 0.5, "gravity_dir": (0.0, -1.0, 0.0), "hit_radius": 0.05}],
                "tail": None,
            },
        ],
    }

    write_sidecar_spring(settings, spring)
    out = spring_settings_to_sidecar(settings)

    assert out["chains"][0]["origin"] == {"mesh": "Hair_back", "material": "Hair", "role": "hair"}
    assert out["chains"][1]["origin"] is None


def test_generated_chains_are_replaced_per_submesh():
    """remove_chains_generated_on drops only the chains generated on that submesh and names their bones."""
    settings = _make_settings(TEST_CHAINS, [], [])
    back = {"mesh": "Hair_back", "material": "Hair", "role": "hair"}
    side = {"mesh": "Hair_side", "material": "Hair", "role": "hair"}

    def joint(bone):
        return {"bone": bone, "force": _vrm_force(1.0, 0.0), "drag_force": 0.4, "gravity_dir": (0.0, -1.0, 0.0), "hit_radius": 0.01}

    spring = spring_settings_to_sidecar(settings)
    spring["chains"] += [
        {"name": "hair_hair_back_00", "center": None, "tail": None, "collider_groups": [], "origin": back,
         "joints": [joint("hair_hair_back_00_00"), joint("hair_hair_back_00_01")]},
        {"name": "hair_hair_back_01", "center": None, "tail": None, "collider_groups": [], "origin": back,
         "joints": [joint("hair_hair_back_01_00")]},
        {"name": "hair_hair_side_00", "center": None, "tail": None, "collider_groups": [], "origin": side,
         "joints": [joint("hair_hair_side_00_00")]},
    ]
    write_sidecar_spring(settings, spring)
    assert [c.name for c in settings.chains] == ["chain_0", "hair_hair_back_00", "hair_hair_back_01", "hair_hair_side_00"]
    assert settings.chains[1].origin_mesh == "Hair_back"

    removed = remove_chains_generated_on(settings, back)
    assert sorted(removed) == ["hair_hair_back_00_00", "hair_hair_back_00_01", "hair_hair_back_01_00"]
    assert [c.name for c in settings.chains] == ["chain_0", "hair_hair_side_00"]
    assert remove_chains_generated_on(settings, back) == []
