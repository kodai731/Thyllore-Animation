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


def _spring_preview() -> dict:
    return {"spring": SPRING, "bones": BONES, "armature_world": IDENTITY, "rest_matrices": [IDENTITY], "scale_length": 1.0}


def _armature_local_points() -> dict:
    return {"armature_world": IDENTITY, "points": [(1.0, 2.0, 3.0)]}


def _bone_specs() -> dict:
    return {"names": ["Hips"], "parents": [None], "armature_world": IDENTITY, "rest_matrices": [IDENTITY], "scale_length": 1.0}


CASES = [
    ("spring_colliders_to_local", _spring_and_bones),
    ("spring_colliders_to_model", _spring_and_bones),
    ("has_fitted_body_colliders", _spring_and_bones),
    ("vrm_rest_clearance_warnings", _spring_and_bones),
    ("infer_humanoid_mapping", _bones),
    ("vrm_t_pose_warnings", _bones),
    ("transfer_skin_weights", _transfer_skin_weights),
    ("SpringPreview", _spring_preview),
    ("armature_local_points", _armature_local_points),
    ("bone_specs", _bone_specs),
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


def test_wire_shapes_take_plain_arguments_for_the_draw_handler(avatar_api):
    assert len(avatar_api.wire_sphere_segments((0.0, 0.0, 0.0), 1.0, 8)) == 3 * 8
    assert len(avatar_api.wire_capsule_segments((0.0, 0.0, 0.0), (0.0, 1.0, 0.0), 1.0, 8)) > 3 * 8
