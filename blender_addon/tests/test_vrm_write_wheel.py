"""vrm_write from the wheel: an unknown request key is rejected with 'unknown field'."""
from __future__ import annotations

import sys
import zipfile
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
WHEELS_DIR = REPO_ROOT / "blender_addon" / "wheels"


@pytest.fixture(scope="module")
def avatar_api(tmp_path_factory):
    wheels = sorted(WHEELS_DIR.glob("thyllore_avatar_api-*.whl"))
    if not wheels:
        pytest.skip("thyllore_avatar_api wheel not built (run scripts/collect_wheels.sh)")

    extract_dir = tmp_path_factory.mktemp("avatar_api_wheel")
    with zipfile.ZipFile(wheels[-1]) as archive:
        archive.extractall(extract_dir)

    sys.path.insert(0, str(extract_dir))
    try:
        import thyllore_avatar_api

        yield thyllore_avatar_api
    finally:
        sys.path.remove(str(extract_dir))


def test_vrm_write_rejects_an_unknown_request_key(avatar_api):
    with pytest.raises(ValueError, match="unknown field"):
        avatar_api.vrm_write({
            "glb": b"glTF",
            "sidecar_json": "{}",
            "meta": {"name": "x", "authors": ["y"]},
            "bogus": 1,
        })
