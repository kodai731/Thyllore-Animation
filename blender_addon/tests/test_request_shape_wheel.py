"""Every remaining wheel entry point takes one request dict: an unknown key is rejected, the clean request runs."""
from __future__ import annotations

import pytest

SPRING = {"collision_mode": "vrm_compatible", "colliders": [], "collider_groups": [], "chains": []}
IDENTITY = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]
BONES = [
    {"name": "Hips", "parent": None, "rest_position": [0.0, 0.0, 0.0], "rest_frame": [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], "bind_matrix": IDENTITY},
]
TRIANGLE = {
    "positions": [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)],
    "normals": [(0.0, 0.0, 1.0)] * 3,
    "triangles": [(0, 1, 2)],
}


def _spring_and_bones() -> dict:
    return {"spring": SPRING, "bones": BONES}


def _bones() -> dict:
    return {"bones": BONES}


def _transfer_skin_weights() -> dict:
    return {"source": {**TRIANGLE, "weights": [[1.0], [1.0], [1.0]]}, "target": TRIANGLE, "max_distance": 1.0, "max_angle_deg": 90.0}


def _wire_sphere() -> dict:
    return {"center": (0.0, 0.0, 0.0), "radius": 1.0, "segments": 8}


def _wire_capsule() -> dict:
    return {"head": (0.0, 0.0, 0.0), "tail": (0.0, 1.0, 0.0), "radius": 1.0, "segments": 8}


def _spring_preview() -> dict:
    return {"spring": SPRING, "bones": BONES, "rest_locals": [IDENTITY]}


CASES = [
    ("spring_colliders_to_local", _spring_and_bones),
    ("spring_colliders_to_model", _spring_and_bones),
    ("has_fitted_body_colliders", _spring_and_bones),
    ("vrm_rest_clearance_warnings", _spring_and_bones),
    ("infer_humanoid_mapping", _bones),
    ("vrm_t_pose_warnings", _bones),
    ("transfer_skin_weights", _transfer_skin_weights),
    ("wire_sphere_segments", _wire_sphere),
    ("wire_capsule_segments", _wire_capsule),
    ("SpringPreview", _spring_preview),
]


@pytest.mark.parametrize(("entry_point", "builder"), CASES)
def test_request_rejects_unknown_field(avatar_api, entry_point, builder):
    request = builder()
    request["unknown_field"] = 1
    with pytest.raises(ValueError, match="unknown field"):
        getattr(avatar_api, entry_point)(request)


@pytest.mark.parametrize(("entry_point", "builder"), CASES)
def test_clean_request_is_accepted(avatar_api, entry_point, builder):
    getattr(avatar_api, entry_point)(builder())


def test_wire_sphere_request_draws_three_circles(avatar_api):
    segments = avatar_api.wire_sphere_segments(_wire_sphere())
    assert len(segments) == 3 * 8
