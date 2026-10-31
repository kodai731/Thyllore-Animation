"""Tests for tools/sidecar_diff.py — loaded via importlib, no Blender dependency."""
from __future__ import annotations

import importlib
import pathlib
import sys


def _load_module():
    """Load tools/sidecar_diff.py via importlib from the repo root."""
    repo_root = pathlib.Path(__file__).resolve().parents[2]
    tools_dir = repo_root / "tools"
    spec = importlib.util.spec_from_file_location("sidecar_diff", tools_dir / "sidecar_diff.py")
    mod = importlib.util.module_from_spec(spec)
    sys.modules["sidecar_diff"] = mod
    spec.loader.exec_module(mod)
    return mod


def _joint(bone, stiffness=1.0, model="vrm"):
    """Make a joint dict."""
    force = {"model": model, "stiffness": stiffness, "gravity_power": 1.0}
    if model == "rest_relative":
        force = {"model": model, "bending_relaxation": stiffness, "gravity_scale": 1.0}
    return {
        "bone": bone,
        "force": force,
        "drag_force": 0.5,
        "gravity_dir": [0.0, -1.0, 0.0],
        "hit_radius": 0.05,
    }


def _collider(name, radius=0.1, offset=(0.0, 0.0, 0.0), tail=None):
    """Make a collider dict."""
    return {
        "name": name,
        "bone": "Hips",
        "offset": list(offset),
        "shape": "capsule" if tail is not None else "sphere",
        "radius": radius,
        "tail": list(tail) if tail is not None else None,
        "origin": "fitted",
    }


def _chain(name, joints):
    """Make a chain dict."""
    return {"name": name, "joints": joints}


def test_identical_dicts():
    """Two identical sidecar dicts produce an empty diff."""
    mod = _load_module()
    sidecar = {
        "spring": {
            "chains": [_chain("chain_0", [_joint("Hips")])],
            "colliders": [],
        },
        "submesh_roles": [{"mesh": "m", "material": "Body", "role": "skin"}],
        "humanoid": {"Hips": "mixamorig:Hips"},
    }
    assert mod.diff_sidecars(sidecar, sidecar) == []


def test_joint_stiffness_f32_round_trip():
    """stiffness 0.5 vs 0.5000000059604645 (f32 written to JSON) is treated as equal."""
    mod = _load_module()
    engine = {
        "spring": {
            "chains": [_chain("chain_0", [_joint("Hips", stiffness=0.5)])],
            "colliders": [],
        },
    }
    addon = {
        "spring": {
            "chains": [_chain("chain_0", [_joint("Hips", stiffness=0.5000000059604645)])],
            "colliders": [],
        },
    }
    assert mod.diff_sidecars(engine, addon) == []


def test_bone_name_mismatch():
    """Different bone names in joints produce a diff."""
    mod = _load_module()
    engine = {
        "spring": {
            "chains": [_chain("chain_0", [_joint("Hips")])],
            "colliders": [],
        },
    }
    addon = {
        "spring": {
            "chains": [_chain("chain_0", [_joint("Spine")])],
            "colliders": [],
        },
    }
    diffs = mod.diff_sidecars(engine, addon)
    assert len(diffs) == 1
    assert "bone order mismatch" in diffs[0]


def test_collider_radius_diff():
    """collider radius differing by 0.002 (above default tolerance 1e-3) is a diff."""
    mod = _load_module()
    engine = {"spring": {"chains": [], "colliders": [_collider("c1", radius=0.1)]}}
    addon = {"spring": {"chains": [], "colliders": [_collider("c1", radius=0.102)]}}
    diffs = mod.diff_sidecars(engine, addon)
    assert len(diffs) == 1
    assert "radius" in diffs[0]


def test_collider_offset_and_tail_are_compared_in_model_space():
    """offset and tail share the sidecar's model space, so a shifted collider is a diff."""
    mod = _load_module()
    engine = {"spring": {"chains": [], "colliders": [_collider("c1", offset=(0.0, 0.1, 0.0), tail=(0.0, 0.3, 0.0))]}}
    same = {"spring": {"chains": [], "colliders": [_collider("c1", offset=(0.0, 0.1000001, 0.0), tail=(0.0, 0.3, 0.0))]}}
    moved = {"spring": {"chains": [], "colliders": [_collider("c1", offset=(0.0, 0.1, 0.0), tail=(0.0, 0.31, 0.0))]}}
    assert mod.diff_sidecars(engine, same) == []
    diffs = mod.diff_sidecars(engine, moved)
    assert len(diffs) == 1
    assert "tail" in diffs[0]


def test_joint_force_model_mismatch_is_a_diff():
    mod = _load_module()
    engine = {"spring": {"chains": [_chain("chain_0", [_joint("Hips")])], "colliders": []}}
    addon = {"spring": {"chains": [_chain("chain_0", [_joint("Hips", model="rest_relative")])], "colliders": []}}
    diffs = mod.diff_sidecars(engine, addon)
    assert len(diffs) == 1
    assert "force.model" in diffs[0]


def test_submesh_mesh_name_only_diff_is_match():
    """submesh_roles with different mesh names but same (material, role) sets match."""
    mod = _load_module()
    engine = {
        "submesh_roles": [
            {"mesh": "body", "material": "Body", "role": "skin"},
            {"mesh": "hair", "material": "Hair", "role": "hair"},
        ],
    }
    addon = {
        "submesh_roles": [
            {"mesh": "character_body", "material": "Body", "role": "skin"},
            {"mesh": "character_hair", "material": "Hair", "role": "hair"},
        ],
    }
    assert mod.diff_sidecars(engine, addon) == []


def test_humanoid_exact_match():
    """Identical humanoid dicts produce no diff."""
    mod = _load_module()
    sidecar = {"humanoid": {"Hips": "mixamorig:Hips", "Spine": "mixamorig:Spine"}}
    assert mod.diff_sidecars(sidecar, sidecar) == []


def test_humanoid_mismatch():
    """Different humanoid values produce a diff."""
    mod = _load_module()
    engine = {"humanoid": {"Hips": "mixamorig:Hips"}}
    addon = {"humanoid": {"Hips": "other:Hips"}}
    diffs = mod.diff_sidecars(engine, addon)
    assert len(diffs) == 1
    assert "humanoid" in diffs[0]
