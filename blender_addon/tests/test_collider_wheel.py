"""fit_body_colliders and assign_collider_groups from the wheel: typed request shape."""
from __future__ import annotations

import pytest


def test_fit_body_colliders_rejects_unknown_request_field(avatar_api):
    """Request with an unknown key must be rejected by serde(deny_unknown_fields)."""
    request_with_unknown = {
        "spring": {"collision_mode": "vrm_compatible", "chains": [], "colliders": [], "collider_groups": []},
        "bones": [{"name": "Hips", "parent": None, "rest_position": [0.0, 0.0, 0.0], "bind_matrix": [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]}, {"name": "Spine", "parent": 0, "rest_position": [0.0, 0.1, 0.0], "bind_matrix": [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0.1, 0, 1]]}],
        "skins": [{"positions": [(0.05, 0.1, 0.0), (-0.05, 0.1, 0.0), (0.0, 0.1, 0.05)], "influences": [[(1, 1.0)], [(1, 1.0)], [(1, 1.0)]]}],
        "settings": {},
        "unknown_field": "should_fail",
    }
    with pytest.raises(ValueError, match="unknown field"):
        avatar_api.fit_body_colliders(request_with_unknown)

    clean_request = {k: v for k, v in request_with_unknown.items() if k != "unknown_field"}
    try:
        avatar_api.fit_body_colliders(clean_request)
    except Exception as e:
        assert "unknown field" not in str(e), f"clean request raised unknown-field error: {e}"


def test_assign_collider_groups_takes_one_request(avatar_api):
    """Empty spring and bones with one request dict returns a sidecar spring dict."""
    output = avatar_api.assign_collider_groups({
        "spring": {"collision_mode": "vrm_compatible", "chains": [], "colliders": [], "collider_groups": []},
        "bones": [{"name": "Hips", "parent": None, "rest_position": [0.0, 0.0, 0.0]}],
        "chain_names": [],
        "group_names": [],
    })
    assert isinstance(output, dict)
    assert "chains" in output
