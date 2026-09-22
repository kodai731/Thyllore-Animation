"""Tests for blender_addon/common/slang_export.py (bpy-free, slangc-free)."""

from __future__ import annotations

import importlib.util
import os
import sys

_PATH = os.path.join(os.path.dirname(__file__), "..", "common", "slang_export.py")
_spec = importlib.util.spec_from_file_location("slang_export", _PATH)
_slang_export = importlib.util.module_from_spec(_spec)  # type: ignore[arg-type]
sys.modules["slang_export"] = _slang_export
_spec.loader.exec_module(_slang_export)  # type: ignore[attr-defined]

convert_to_blender_dialect = _slang_export.convert_to_blender_dialect
rewrite_sampler_uv = _slang_export.rewrite_sampler_uv

GENERATED = """#version 450
layout(row_major) uniform;
layout(row_major) buffer;

#line 5 0
struct WindUBO_0
{
    mat4x4 model_0;
    vec4  puffs_0[96];
};

layout(binding = 0, set = 1)
layout(std140) uniform block_WindUBO_0
{
    mat4x4 model_0;
    vec4  puffs_0[96];
}wind_0;

struct WindPush_0
{
    int mode_0;
    int debugView_0;
};

layout(push_constant)
layout(std430) uniform block_WindPush_0
{
    int mode_0;
    int debugView_0;
}push_0;

layout(binding = 1, set = 1)
uniform sampler2D sceneDepthSampler_0;

layout(rg16f)
layout(binding = 2, set = 1)
uniform image3D shadowVolumeImage_0;

layout(location = 0)
out vec4 entryPointParam_main_0;

layout(location = 0)
in vec2 fragTexCoord_1;

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

void main()
{
    int mode_0 = push_0.mode_0;
    vec4 c = texture((sceneDepthSampler_0), (fragTexCoord_1)) * wind_0.model_0[0];
    entryPointParam_main_0 = c;
}
"""


def test_strips_vulkan_declarations_and_demangles_bound_names():
    lines, bindings = convert_to_blender_dialect(GENERATED, ["outColor"])
    text = "\n".join(lines)

    assert lines[0] == "layout(row_major) uniform;"
    assert "#version" not in text and "#line" not in text and "buffer;" not in text
    assert "layout(binding" not in text and "layout(location" not in text and "local_size" not in text
    assert "struct WindUBO\n" in text and "struct WindPush\n" in text
    assert "block_" not in text
    assert "texture((sceneDepthSampler), (fragTexCoord)) * wind.model[0];" in text
    assert "outColor = c;" in text
    assert "int mode_0 = push.mode;" in text
    assert "    int mode;\n    int debugView;\n" in text
    assert "    mat4x4 model;\n    vec4  puffs[96];\n" in text


def test_collects_bindings():
    _, bindings = convert_to_blender_dialect(GENERATED, ["outColor"])
    assert bindings["ubos"] == [{"type": "WindUBO", "name": "wind", "binding": 0}]
    assert bindings["push_constants"] == [{"type": "WindPush", "name": "push", "members": ["int mode", "int debugView"]}]
    assert bindings["samplers"] == [{"name": "sceneDepthSampler", "binding": 1, "type": "FLOAT_2D"}]
    assert bindings["images"] == [{"name": "shadowVolumeImage", "binding": 2, "format": "RG16F"}]
    assert bindings["inputs"] == ["fragTexCoord"]
    assert bindings["outputs"] == ["outColor"]
    assert bindings["local_size"] == [8, 8, 1]


def test_rewrite_sampler_uv_handles_nested_calls():
    text = "a = texture((sceneColorSampler), (project(p, f(x, y)))).xyz + texture(sceneColorSampler, uv);"
    out = rewrite_sampler_uv(text, "sceneColorSampler", lambda uv: f"r({uv})")
    assert out == "a = texture(sceneColorSampler, r((project(p, f(x, y))))).xyz + texture(sceneColorSampler, r(uv));"
