"""Unit tests for effect_properties module."""
from __future__ import annotations

from blender_addon.common.effect_properties import (
    convert_offsets_to_blender,
    convert_offsets_to_engine,
    group_params_by_owner,
    offset_param_names,
)


def test_group_used_when_present_and_non_empty():
    """When 'group' is present and non-empty, it is used as the key."""
    params = [
        {"name": "a", "group": "crystal", "owner": "frame"},
        {"name": "b", "group": "crystal", "owner": "frame"},
        {"name": "c", "group": "wave", "owner": "frame"},
    ]
    result = group_params_by_owner(params)
    assert result == [("crystal", ["a", "b"]), ("wave", ["c"])]


def test_owner_used_when_group_missing():
    """When 'group' is missing, 'owner' is used as fallback."""
    params = [
        {"name": "a", "owner": "frame"},
        {"name": "b", "owner": "style"},
    ]
    result = group_params_by_owner(params)
    assert result == [("frame", ["a"]), ("style", ["b"])]


def test_owner_used_when_group_empty():
    """When 'group' is empty string, 'owner' is used as fallback."""
    params = [
        {"name": "a", "group": "", "owner": "frame"},
        {"name": "b", "group": "", "owner": "style"},
    ]
    result = group_params_by_owner(params)
    assert result == [("frame", ["a"]), ("style", ["b"])]


def test_default_owner_is_frame():
    """When both 'group' and 'owner' are missing, 'frame' is the default."""
    params = [
        {"name": "a"},
        {"name": "b"},
    ]
    result = group_params_by_owner(params)
    assert result == [("frame", ["a", "b"])]


def test_mixed_group_and_owner():
    """Mixed: some params have group, others fall back to owner."""
    params = [
        {"name": "a", "group": "crystal", "owner": "frame"},
        {"name": "b", "owner": "frame"},
        {"name": "c"},
    ]
    result = group_params_by_owner(params)
    assert result == [("crystal", ["a"]), ("frame", ["b", "c"])]


def test_offset_param_names():
    """offset_param_names returns names of parameters with kind 'offset'."""
    params = [
        {"name": "intensity", "kind": "scalar"},
        {"name": "end_offset", "kind": "offset"},
        {"name": "source", "kind": "enum"},
    ]
    result = offset_param_names(params)
    assert result == ["end_offset"]


def test_convert_offsets_to_blender():
    """Engine (0, -8.5, 0) converts to Blender (0, 0, -8.5)."""
    values = {"end_offset": [0.0, -8.5, 0.0], "intensity": 1.0}
    result = convert_offsets_to_blender(values, ["end_offset"])
    assert result["end_offset"] == [0.0, 0.0, -8.5]
    assert result["intensity"] == 1.0


def test_convert_offsets_to_engine():
    """Blender (0, 0, -8.5) converts back to engine (0, -8.5, 0)."""
    values = {"end_offset": [0.0, 0.0, -8.5], "intensity": 1.0}
    result = convert_offsets_to_engine(values, ["end_offset"])
    assert result["end_offset"] == [0.0, -8.5, 0.0]
    assert result["intensity"] == 1.0


def test_round_trip():
    """Engine -> Blender -> engine round trip returns original values."""
    engine_values = {"end_offset": [0.0, -8.5, 0.0], "intensity": 1.0}
    blender_values = convert_offsets_to_blender(engine_values, ["end_offset"])
    back_to_engine = convert_offsets_to_engine(blender_values, ["end_offset"])
    assert back_to_engine["end_offset"] == [0.0, -8.5, 0.0]
    assert back_to_engine["intensity"] == 1.0


def test_non_offset_values_unchanged():
    """Non-offset values remain unchanged through conversion."""
    values = {"end_offset": [0.0, -8.5, 0.0], "intensity": 2.5, "duration": 3.0}
    blender = convert_offsets_to_blender(values, ["end_offset"])
    assert blender["intensity"] == 2.5
    assert blender["duration"] == 3.0
    engine = convert_offsets_to_engine(blender, ["end_offset"])
    assert engine["intensity"] == 2.5
    assert engine["duration"] == 3.0
