"""vrm_write from the wheel: an unknown request key is rejected with 'unknown field'."""
from __future__ import annotations

import pytest


def test_vrm_write_rejects_an_unknown_request_key(avatar_api):
    with pytest.raises(ValueError, match="unknown field"):
        avatar_api.vrm_write({
            "glb": b"glTF",
            "sidecar_json": "{}",
            "meta": {"name": "x", "authors": ["y"]},
            "bogus": 1,
        })
