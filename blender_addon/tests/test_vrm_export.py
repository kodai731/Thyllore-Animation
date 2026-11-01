"""Tests for vrm_export: the meta dict the wheel takes and the T-pose check."""
from __future__ import annotations

from types import SimpleNamespace

from blender_addon.common.vrm_export import t_pose_warnings, vrm_meta_to_dict


def _meta(**overrides):
    fields = dict(
        name=" Avatar ",
        authors="alice, bob,",
        version="",
        copyright_information="",
        contact_information="",
        other_license_url="",
        avatar_permission="onlyAuthor",
        commercial_usage="personalNonProfit",
        credit_notation="required",
        modification="prohibited",
        allow_excessively_violent_usage=False,
        allow_excessively_sexual_usage=False,
        allow_political_or_religious_usage=False,
        allow_antisocial_or_hate_usage=False,
        allow_redistribution=False,
    )
    fields.update(overrides)
    return SimpleNamespace(**fields)


def test_meta_dict_splits_authors_and_drops_empty_texts():
    meta = vrm_meta_to_dict(_meta())

    assert meta["name"] == "Avatar"
    assert meta["authors"] == ["alice", "bob"]
    assert "version" not in meta
    assert meta["avatar_permission"] == "onlyAuthor"
    assert meta["allow_redistribution"] is False


def test_meta_dict_keeps_filled_texts():
    meta = vrm_meta_to_dict(_meta(version="1.2", contact_information="x@example.com"))

    assert meta["version"] == "1.2"
    assert meta["contact_information"] == "x@example.com"


HUMANOID = {
    "LeftUpperArm": "L_arm",
    "LeftHand": "L_hand",
    "RightUpperArm": "R_arm",
    "RightHand": "R_hand",
    "LeftUpperLeg": "L_leg",
    "LeftFoot": "L_foot",
}


def _heads(arm_tip_y=1.4, foot=(0.1, 0.1, 0.0)):
    return {
        "L_arm": (0.2, 1.4, 0.0),
        "L_hand": (0.7, arm_tip_y, 0.0),
        "R_arm": (-0.2, 1.4, 0.0),
        "R_hand": (-0.7, 1.4, 0.0),
        "L_leg": (0.1, 0.9, 0.0),
        "L_foot": foot,
    }


def test_t_pose_passes_for_level_arms_and_straight_legs():
    assert t_pose_warnings(HUMANOID, _heads()) == []


def test_t_pose_warns_for_a_pose_arms():
    warnings = t_pose_warnings(HUMANOID, _heads(arm_tip_y=1.0))

    assert len(warnings) == 1
    assert warnings[0].startswith("LeftUpperArm")


def test_t_pose_warns_for_a_bent_leg():
    warnings = t_pose_warnings(HUMANOID, _heads(foot=(0.5, 0.3, 0.0)))

    assert len(warnings) == 1
    assert warnings[0].startswith("LeftUpperLeg")


def test_t_pose_skips_unmapped_limbs():
    assert t_pose_warnings({}, _heads()) == []
