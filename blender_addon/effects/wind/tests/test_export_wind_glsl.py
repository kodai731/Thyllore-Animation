"""Exporter smoke for the wind addon: runs slangc through scripts/blender/wind/export_glsl.py."""

import json
import os
import subprocess
import sys
import tempfile

import pytest

SCRIPT = os.path.join(os.path.dirname(__file__), "..", "..", "..", "..", "scripts", "blender", "wind", "export_glsl.py")
REPO_ROOT = os.path.join(os.path.dirname(__file__), "..", "..", "..", "..")


def _run_exporter(tmp_path: str) -> dict[str, tuple[str, dict]]:
    out_dir = os.path.join(tmp_path, "shaders")
    result = subprocess.run(
        [sys.executable, SCRIPT, "--repo-root", REPO_ROOT, "--out", out_dir],
        capture_output=True, text=True,
    )
    assert result.returncode == 0, f"exporter failed: {result.stderr}"
    exported = {}
    for stem in ("wind_resolve", "wind_shadow_bake", "wind_upsample"):
        with open(os.path.join(out_dir, f"{stem}.glsl")) as f:
            glsl_text = f.read()
        with open(os.path.join(out_dir, f"{stem}.bindings.json")) as f:
            bindings = json.load(f)
        exported[stem] = (glsl_text, bindings)
    return exported


@pytest.fixture(scope="module")
def exported():
    with tempfile.TemporaryDirectory() as tmp:
        yield _run_exporter(tmp)


@pytest.mark.parametrize("stem", ["wind_resolve", "wind_shadow_bake", "wind_upsample"])
def test_no_vulkan_artifacts(exported, stem):
    glsl_text, _ = exported[stem]
    assert glsl_text.splitlines()[0] == "layout(row_major) uniform;"
    for forbidden in ("#version", "#line", "#include", "layout(binding", "layout(location", "layout(push_constant", "block_"):
        assert forbidden not in glsl_text, f"{stem} still contains {forbidden}"
    assert "void main()" in glsl_text


def test_resolve_bindings(exported):
    _, bindings = exported["wind_resolve"]
    assert {s["name"]: s["type"] for s in bindings["samplers"]} == {"sceneDepthSampler": "FLOAT_2D", "shadowVolumeSampler": "FLOAT_3D"}
    assert {u["name"]: u["type"] for u in bindings["ubos"]} == {"frame": "FrameUBO", "wind": "WindUBO"}
    assert bindings["push_constants"] == [{"type": "WindPush", "name": "push", "members": ["int mode", "int stepCount", "int debugView"]}]
    assert bindings["inputs"] == ["fragTexCoord"]
    assert bindings["outputs"] == ["outColor"]


def test_resolve_body_uses_plain_names(exported):
    glsl_text, _ = exported["wind_resolve"]
    assert "push.debugView" in glsl_text
    assert "wind." in glsl_text and "frame." in glsl_text
    assert "struct WindUBO\n" in glsl_text and "struct FrameUBO\n" in glsl_text and "struct WindPush\n" in glsl_text


def test_bake_bindings(exported):
    _, bindings = exported["wind_shadow_bake"]
    assert bindings["images"] == [{"name": "shadowVolumeImage", "binding": 1, "format": "RG16F"}]
    assert bindings["local_size"] == [8, 8, 1]
    assert bindings["shadow_volume_size"] == [48 * 4, 48, 64]
    assert {u["name"] for u in bindings["ubos"]} == {"frame", "wind"}


def test_upsample_bindings(exported):
    _, bindings = exported["wind_upsample"]
    assert [s["name"] for s in bindings["samplers"]] == ["windColorSampler", "sceneDepthSampler"]
    assert bindings["outputs"] == ["outColor"]
