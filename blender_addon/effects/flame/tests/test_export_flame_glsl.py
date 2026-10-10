"""Exporter smoke for the flame addon: runs slangc through scripts/blender/flame/export_glsl.py."""

import json
import os
import subprocess
import sys
import tempfile

import pytest

SCRIPT = os.path.join(os.path.dirname(__file__), "..", "..", "..", "..", "scripts", "blender", "flame", "export_glsl.py")
REPO_ROOT = os.path.join(os.path.dirname(__file__), "..", "..", "..", "..")


def _run_exporter(tmp_path: str) -> tuple[str, dict]:
    out_dir = os.path.join(tmp_path, "shaders")
    result = subprocess.run(
        [sys.executable, SCRIPT, "--repo-root", REPO_ROOT, "--out", out_dir],
        capture_output=True, text=True,
    )
    assert result.returncode == 0, f"exporter failed: {result.stderr}"
    with open(os.path.join(out_dir, "flame_resolve.glsl")) as f:
        glsl_text = f.read()
    with open(os.path.join(out_dir, "flame_resolve.bindings.json")) as f:
        bindings = json.load(f)
    return glsl_text, bindings


@pytest.fixture(scope="module")
def exported():
    with tempfile.TemporaryDirectory() as tmp:
        yield _run_exporter(tmp)


def test_no_vulkan_artifacts(exported):
    glsl_text, _ = exported
    assert glsl_text.splitlines()[0] == "layout(row_major) uniform;"
    for forbidden in ("#version", "#line", "#include", "layout(binding", "layout(location", "layout(push_constant", "block_"):
        assert forbidden not in glsl_text, f"output still contains {forbidden}"
    assert "void main()" in glsl_text
    assert "inverse(" in glsl_text


def test_bindings(exported):
    _, bindings = exported
    assert {s["name"] for s in bindings["samplers"]} == {"flameHistorySampler", "flameSdfSampler", "sceneDepthSampler"}
    assert {u["name"]: u["type"] for u in bindings["ubos"]} == {"frame": "FrameUBO", "flame": "FlameUBO"}
    assert bindings["push_constants"] == [{"type": "FlamePush", "name": "push", "members": ["int mode", "int stepCount", "int debugView"]}]
    assert bindings["inputs"] == ["fragTexCoord"]
    assert bindings["outputs"] == ["outColor", "outHistory"]


def test_body_uses_plain_names(exported):
    glsl_text, _ = exported
    assert "push.mode" in glsl_text and "push.debugView" in glsl_text
    assert "flame." in glsl_text and "frame." in glsl_text
    assert "outColor" in glsl_text and "outHistory" in glsl_text
