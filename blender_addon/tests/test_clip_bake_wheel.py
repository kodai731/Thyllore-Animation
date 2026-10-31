"""bake_clip_file from the wheel: a role-named template lands on the inferred humanoid bones."""
from __future__ import annotations

import sys
import zipfile
from pathlib import Path

import pytest

from blender_addon.common.avatar_sidecar import bone_specs
from blender_addon.common.collider_fit import bone_rest_frame

REPO_ROOT = Path(__file__).resolve().parents[2]
WHEELS_DIR = REPO_ROOT / "blender_addon" / "wheels"
HIPS_SWAY = REPO_ROOT / "assets" / "templates" / "hips_sway.anim.ron"
IDENTITY_ROWS = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]


@pytest.fixture(scope="module")
def tml(tmp_path_factory):
    wheels = sorted(WHEELS_DIR.glob("thyllore_ml_core-*.whl"))
    if not wheels:
        pytest.skip("thyllore_ml_core wheel not built (run scripts/collect_wheels.sh)")

    extract_dir = tmp_path_factory.mktemp("tml_wheel")
    with zipfile.ZipFile(wheels[-1]) as archive:
        archive.extractall(extract_dir)

    sys.path.insert(0, str(extract_dir))
    try:
        import thyllore_ml_core

        yield thyllore_ml_core
    finally:
        sys.path.remove(str(extract_dir))


def tpose_specs(prefix: str, scale_length: float):
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
    names = [prefix + role for role, _, _ in heads_z_up]
    parents = [parent for _, parent, _ in heads_z_up]
    heads = [head for _, _, head in heads_z_up]
    rest_frames = [bone_rest_frame(IDENTITY_ROWS, IDENTITY_ROWS, scale_length) for _ in heads_z_up]
    return bone_specs(names, parents, heads, scale_length, rest_frames)


def test_bake_clip_file_sways_prefixed_hips_by_the_hips_height(tml):
    baked = tml.bake_clip_file(tpose_specs("mixamorig:", 1.0), HIPS_SWAY.read_text(encoding="utf-8"), 2)

    assert baked["name"] == "hips_sway"
    assert baked["hips_bone"] == "mixamorig:Hips"
    assert baked["frame_times"] == pytest.approx([0.0, 0.5, 1.0, 1.5, 2.0])
    sideways = [offset[0] for offset in baked["hips_offsets"]]
    assert abs(sideways[1]) == pytest.approx(0.15, abs=1e-5)
    assert sideways[3] == pytest.approx(-sideways[1], abs=1e-5)
    assert {track["bone"] for track in baked["bone_rotations"]} >= {"mixamorig:Hips", "mixamorig:Head"}


def test_bake_clip_file_scales_the_sway_with_the_scene_unit(tml):
    baked = tml.bake_clip_file(tpose_specs("", 0.01), HIPS_SWAY.read_text(encoding="utf-8"), 2)

    assert abs(baked["hips_offsets"][1][0]) == pytest.approx(0.0015, abs=1e-6)


def test_bake_clip_file_reports_a_rig_without_feet(tml):
    specs = tpose_specs("", 1.0)[:7]

    with pytest.raises(RuntimeError, match="feet"):
        tml.bake_clip_file(specs, HIPS_SWAY.read_text(encoding="utf-8"), 2)
