from __future__ import annotations

import pytest

SPRING = {"collision_mode": "vrm_compatible", "colliders": [], "collider_groups": [], "chains": []}
BONES = [{"name": "Hips", "parent": None, "rest_position": [0.0, 0.0, 0.0]}]


def _add_chains_under_bone(**extra) -> dict:
    return {"spring": SPRING, "bones": BONES, "root_bone": "Hips", **extra}


_JOINT = {"force": {"model": "vrm", "stiffness": 0.0, "gravity_power": 0.0}, "drag_force": 0.0, "gravity_dir": [0.0, -1.0, 0.0], "hit_radius": 0.0}


def _apply_ramp(**extra) -> dict:
    ramp = {"root": _JOINT, "tip": _JOINT, "shape": "linear"}
    return {"spring": SPRING, "bones": BONES, "chain_names": [], "ramp": ramp, **extra}


def _chains_in_scope(**extra) -> dict:
    return {"spring": SPRING, "bones": BONES, "chain_name": "c", "scope": "this_chain", **extra}


def _chains_under_bone(**extra) -> dict:
    return {"spring": SPRING, "bones": BONES, "bone_name": "Hips", **extra}


def _chains_with_radius(**extra) -> dict:
    return {"spring": SPRING, "bones": BONES, "mode": "none", **extra}


CASES = [
    ("add_chains_under_bone", _add_chains_under_bone),
    ("apply_ramp", _apply_ramp),
    ("chains_in_scope", _chains_in_scope),
    ("chains_under_bone", _chains_under_bone),
    ("chains_with_radius", _chains_with_radius),
]


@pytest.mark.parametrize(("func_name", "builder"), CASES)
def test_spring_edit_requests_reject_unknown_field(avatar_api, func_name, builder):
    request = builder()
    request["unknown_field"] = 1
    with pytest.raises(ValueError, match="unknown field"):
        getattr(avatar_api, func_name)(request)
