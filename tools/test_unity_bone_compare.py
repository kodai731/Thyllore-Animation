"""Tests for tools/unity_bone_compare.py compare_bone_positions pure function."""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from unity_bone_compare import compare_bone_positions


def _engine_dump(bones: list[dict], times: list[float] | None = None) -> dict:
    """Build a minimal engine dump with given bones at each time."""
    if times is None:
        times = [0.0, 1.0]
    poses = []
    for t in times:
        pose_bones = []
        for b in bones:
            entry = {
                "id": b["id"],
                "name": b["name"],
                "role": b.get("role"),
                "world_position": list(b["world_position"]),
            }
            pose_bones.append(entry)
        poses.append({"time": t, "bones": pose_bones})
    return {"poses": poses}


def _unity_pose(bones_by_time: dict[float, dict[str, list]]) -> dict:
    """Build a minimal Unity pose JSON."""
    poses = []
    for t, bones in sorted(bones_by_time.items()):
        poses.append({"time": t, "bones": bones})
    return {"poses": poses}


def test_x_inverted_match_is_ok() -> None:
    """When Unity x is negated to match engine, all differences are zero -> ok."""
    bones = [
        {"id": 1, "name": "Hips", "role": "Hips", "world_position": (0.0, 0.95, 0.0)},
        {"id": 2, "name": "Spine", "role": "Spine", "world_position": (0.0, 1.05, 0.0)},
    ]
    engine = _engine_dump(bones)

    unity = _unity_pose({
        0.0: {"Hips": [0.0, 0.95, 0.0], "Spine": [0.0, 1.05, 0.0]},
        1.0: {"Hips": [0.0, 0.95, 0.0], "Spine": [0.0, 1.05, 0.0]},
    })

    result = compare_bone_positions(engine, unity, 0.01)
    assert result["ok"] is True, f"Expected ok=True, got {json.dumps(result)}"
    assert result["humanoid_max_ratio"] == 0.0


def test_humanoid_bone_2_percent_off_is_ng() -> None:
    """A humanoid bone (with role) that is 2% off hips height should fail."""
    bones = [
        {"id": 1, "name": "Hips", "role": "Hips", "world_position": (0.0, 0.95, 0.0)},
        {"id": 2, "name": "Spine", "role": "Spine", "world_position": (0.0, 1.05, 0.0)},
    ]
    engine = _engine_dump(bones)

    unity = _unity_pose({
        0.0: {"Hips": [0.0, 0.95, 0.0], "Spine": [-0.02, 1.05, 0.0]},
        1.0: {"Hips": [0.0, 0.95, 0.0], "Spine": [-0.02, 1.05, 0.0]},
    })

    result = compare_bone_positions(engine, unity, 0.01)
    assert result["ok"] is False, f"Expected ok=False (2% off), got {json.dumps(result)}"
    assert result["humanoid_worst_bone"] == "Spine"


def test_extra_only_offset_is_ng() -> None:
    """Only extra bones (role=null) are offset beyond tolerance -> ng because extra bones count."""
    bones = [
        {"id": 1, "name": "Hips", "role": "Hips", "world_position": (0.0, 0.95, 0.0)},
        {"id": 2, "name": "Spine", "role": "Spine", "world_position": (0.0, 1.05, 0.0)},
        {"id": 3, "name": "Skirt_Back_1", "role": None, "world_position": (0.0, 0.9, -0.1)},
    ]
    engine = _engine_dump(bones)

    unity = _unity_pose({
        0.0: {
            "Hips": [0.0, 0.95, 0.0],
            "Spine": [0.0, 1.05, 0.0],
            "Skirt_Back_1": [1.0, 0.9, -0.1],  # x=1.0 -> after negation: -1.0, diff=1.0
        },
        1.0: {
            "Hips": [0.0, 0.95, 0.0],
            "Spine": [0.0, 1.05, 0.0],
            "Skirt_Back_1": [1.0, 0.9, -0.1],
        },
    })

    result = compare_bone_positions(engine, unity, 0.01)
    assert result["ok"] is False, f"Expected ok=False (extra bones exceed tolerance), got {json.dumps(result)}"
    assert result["humanoid_max_ratio"] == 0.0
    assert result["extra_worst_bone"] == "Skirt_Back_1"


def test_missing_humanoid_bone_is_ng() -> None:
    """When a humanoid bone (with role) is missing from Unity output, ok=False and
    missing_humanoid_bones contains the bone name."""
    bones = [
        {"id": 1, "name": "Hips", "role": "Hips", "world_position": (0.0, 0.95, 0.0)},
        {"id": 2, "name": "Spine", "role": "Spine", "world_position": (0.0, 1.05, 0.0)},
    ]
    engine = _engine_dump(bones)

    unity = _unity_pose({
        0.0: {"Hips": [0.0, 0.95, 0.0]},
        1.0: {"Hips": [0.0, 0.95, 0.0]},
    })

    result = compare_bone_positions(engine, unity, 0.01)
    assert result["ok"] is False, f"Expected ok=False (missing Spine), got {json.dumps(result)}"
    assert "Spine" in result["missing_humanoid_bones"], \
        f"Expected 'Spine' in missing_humanoid_bones, got {result['missing_humanoid_bones']}"
    assert len(result["missing_humanoid_bones"]) == 1, \
        f"Expected exactly 1 missing bone, got {result['missing_humanoid_bones']}"
