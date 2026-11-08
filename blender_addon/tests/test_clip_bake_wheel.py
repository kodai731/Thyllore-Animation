"""bake_clip_file from the wheel: a role-named template lands on the inferred humanoid bones."""
from __future__ import annotations

from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
HIPS_SWAY = REPO_ROOT / "assets" / "templates" / "hips_sway.anim.ron"
IDENTITY_ROWS = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]


def tpose_specs(avatar_api, prefix: str, scale_length: float):
    heads_z_up = [
        ("Hips", None, (0.0, 0.0, 1.0)),
        ("Spine", 0, (0.0, 0.0, 1.1)),
        ("Chest", 1, (0.0, 0.0, 1.3)),
        ("Neck", 2, (0.0, 0.0, 1.5)),
        ("Head", 3, (0.0, 0.0, 1.6)),
        ("LeftUpperArm", 2, (0.18, 0.0, 1.45)),
        ("RightUpperArm", 2, (-0.18, 0.0, 1.45)),
        ("LeftUpperLeg", 0, (0.1, 0.0, 0.95)),
        ("LeftLowerLeg", 7, (0.1, 0.0, 0.5)),
        ("LeftFoot", 8, (0.1, 0.0, 0.0)),
        ("RightUpperLeg", 0, (-0.1, 0.0, 0.95)),
        ("RightLowerLeg", 10, (-0.1, 0.0, 0.5)),
        ("RightFoot", 11, (-0.1, 0.0, 0.0)),
    ]
    return avatar_api.bone_specs({
        "names": [prefix + role for role, _, _ in heads_z_up],
        "parents": [parent for _, parent, _ in heads_z_up],
        "armature_world": IDENTITY_ROWS,
        "rest_matrices": [_translated(head) for _, _, head in heads_z_up],
        "scale_length": scale_length,
    })


def _translated(head) -> list[list[float]]:
    return [[1.0, 0.0, 0.0, head[0]], [0.0, 1.0, 0.0, head[1]], [0.0, 0.0, 1.0, head[2]], [0.0, 0.0, 0.0, 1.0]]


def bake_request(avatar_api, prefix: str, scale_length: float, **extra) -> dict:
    specs = tpose_specs(avatar_api, prefix, scale_length)
    return {
        "bones": specs,
        "clip_ron": HIPS_SWAY.read_text(encoding="utf-8"),
        "fps": 2,
        "armature_world": IDENTITY_ROWS,
        "rest_matrices": [IDENTITY_ROWS for _ in specs],
        "scale_length": scale_length,
        **extra,
    }


def test_bake_clip_file_sways_prefixed_hips_by_the_hips_height(avatar_api):
    baked = avatar_api.bake_clip_file(bake_request(avatar_api, "mixamorig:", 1.0))

    assert baked["name"] == "hips_sway"
    assert baked["hips_bone"] == "mixamorig:Hips"
    assert baked["frame_times"] == pytest.approx([0.0, 0.5, 1.0, 1.5, 2.0])
    sideways = [offset[0] for offset in baked["hips_offsets"]]
    assert abs(sideways[1]) == pytest.approx(0.15, abs=1e-5)
    assert sideways[3] == pytest.approx(-sideways[1], abs=1e-5)
    assert {track["bone"] for track in baked["bone_rotations"]} >= {"mixamorig:Hips", "mixamorig:Head"}
    assert all(len(track["bases"]) == len(baked["frame_times"]) for track in baked["bone_rotations"])


def test_bake_clip_file_scales_the_sway_with_the_scene_unit(avatar_api):
    baked = avatar_api.bake_clip_file(bake_request(avatar_api, "", 0.01))

    assert abs(baked["hips_offsets"][1][0]) == pytest.approx(0.0015, abs=1e-6)
    assert abs(baked["hips_locations"][1][0]) == pytest.approx(0.15, abs=1e-5)


def test_bake_clip_file_reports_a_rig_without_feet(avatar_api):
    request = bake_request(avatar_api, "", 1.0)
    request["bones"] = request["bones"][:7]
    request["rest_matrices"] = request["rest_matrices"][:7]

    with pytest.raises(RuntimeError, match="feet"):
        avatar_api.bake_clip_file(request)


def test_bake_clip_file_rejects_an_unknown_request_key(avatar_api):
    with pytest.raises(ValueError, match="unknown field"):
        avatar_api.bake_clip_file(bake_request(avatar_api, "", 1.0, unknown_key="should fail"))
