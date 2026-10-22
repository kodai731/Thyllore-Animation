"""generate_chains from the wheel: bone specs, weights, settings table and the Blender-space conversion."""
from __future__ import annotations

import math
import sys
import zipfile
from pathlib import Path

import pytest

from blender_addon.common.chain_bones import (
    bone_specs,
    group_weights,
    output_to_blender,
    vertex_tex_coords,
)
from blender_addon.common.coordinates import blender_to_engine_point, engine_to_blender_point

WHEELS_DIR = Path(__file__).resolve().parents[1] / "wheels"


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


HEAD = [(0.2, 0.1, 0.0), (-0.2, 0.1, 0.0), (0.0, 0.1, 0.2), (0.0, 0.1, -0.2)]


def _strand_mesh(azimuths, levels, top, bottom, flare):
    positions = []
    triangles = []
    for azimuth in azimuths:
        base = len(positions)
        for level in range(levels):
            y = top + (bottom - top) * level / (levels - 1)
            radius = 0.1 + flare * level
            for offset in (-0.02, 0.02):
                angle = azimuth + offset
                positions.append((radius * math.cos(angle), y, radius * math.sin(angle)))
        for level in range(levels - 1):
            a = base + 2 * level
            triangles.append([a, a + 1, a + 2])
            triangles.append([a + 1, a + 3, a + 2])
    return positions, triangles


def test_settings_table_lists_defaults(tml):
    params = tml.chain_gen_ui_params()

    assert [p["name"] for p in params] == [
        "joint_count",
        "column_count",
        "strand_merge_distance",
        "root_blend_width",
        "column_blend_width",
        "root_band_offset",
    ]
    assert params[0]["default"] == 4
    assert params[0]["label"] == "Joint Count"


def test_unknown_setting_is_rejected(tml):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)

    with pytest.raises(Exception, match="unknown chain generation setting"):
        tml.generate_chains(positions, triangles, (0.0, 0.0, 0.0), "hair", "strip/hair", {"joints": 4})


def test_bone_specs_chain_joints_root_to_tip(tml):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)
    output = tml.generate_chains(
        positions, triangles, (0.0, 0.0, 0.0), "hair", "strip/hair", {"joint_count": 4, "column_count": 1}, HEAD
    )

    specs = bone_specs(output, "Head")

    assert [name for name, _, _, _ in specs] == [
        "hair_strip_hair_00_00",
        "hair_strip_hair_00_01",
        "hair_strip_hair_00_02",
        "hair_strip_hair_00_03",
    ]
    assert [parent for _, parent, _, _ in specs] == [
        "Head",
        "hair_strip_hair_00_00",
        "hair_strip_hair_00_01",
        "hair_strip_hair_00_02",
    ]
    for (_, _, _, tail), (_, _, next_head, _) in zip(specs, specs[1:]):
        assert tail == next_head
    assert specs[0][2][1] > specs[-1][2][1]


def test_group_weights_sum_to_one_per_vertex(tml):
    positions, triangles = _strand_mesh((0.5, 3.5), 9, -0.05, -0.45, 0.01)
    output = tml.generate_chains(positions, triangles, (0.0, 0.0, 0.0), "hair", "strip/hair", {"joint_count": 4}, HEAD)

    totals: dict[int, float] = {}
    for influences in group_weights(output, "Head").values():
        for vertex, weight in influences:
            totals[vertex] = totals.get(vertex, 0.0) + weight

    assert len(totals) == len(positions)
    assert all(abs(total - 1.0) < 1e-5 for total in totals.values())


def test_blender_space_input_converted_at_the_boundary_matches_engine_input(tml):
    positions_engine, triangles = _strand_mesh((0.5, 3.5), 9, -0.05, -0.45, 0.01)
    positions_blender = [engine_to_blender_point(p) for p in positions_engine]
    settings = {"joint_count": 4}

    engine = tml.generate_chains(positions_engine, triangles, (0.0, 0.0, 0.0), "hair", "strip/hair", settings, HEAD)
    converted = tml.generate_chains(
        [blender_to_engine_point(p) for p in positions_blender],
        triangles,
        blender_to_engine_point((0.0, 0.0, 0.0)),
        "hair",
        "strip/hair",
        settings,
        [blender_to_engine_point(engine_to_blender_point(p)) for p in HEAD],
    )

    assert len(engine["chains"]) == len(converted["chains"]) == 2
    for chain_engine, chain_converted in zip(engine["chains"], converted["chains"]):
        for joint_engine, joint_converted in zip(chain_engine["joints"], chain_converted["joints"]):
            assert joint_engine["name"] == joint_converted["name"]
            assert max(abs(a - b) for a, b in zip(joint_engine["head"], joint_converted["head"])) < 1e-6
    assert engine["weights"] == converted["weights"]


def test_output_to_blender_converts_heads_and_tails(tml):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)
    output = tml.generate_chains(positions, triangles, (0.0, 0.0, 0.0), "hair", "strip/hair", {"column_count": 1}, HEAD)

    blender = output_to_blender(output)

    for chain_engine, chain_blender in zip(output["chains"], blender["chains"]):
        assert chain_blender["tail"] == engine_to_blender_point(chain_engine["tail"])
        for joint_engine, joint_blender in zip(chain_engine["joints"], chain_blender["joints"]):
            assert joint_blender["head"] == engine_to_blender_point(joint_engine["head"])
            assert blender_to_engine_point(joint_blender["head"]) == tuple(joint_engine["head"])
    assert blender["weights"] == output["weights"]


def test_vertex_tex_coords():
    loop_vertices = [0, 1, 2, 1, 3, 2]
    loop_uvs = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.5, 0.5), (0.0, 1.0), (0.5, 0.5)]
    result = vertex_tex_coords(loop_vertices, loop_uvs, 4)

    assert result[0] == (0.0, 0.0)
    assert result[1] == (1.0, 0.0)
    assert result[2] == (1.0, 1.0)
    assert result[3] == (0.0, 1.0)


def test_vertex_tex_coords_uncovered_vertex():
    loop_vertices = [0, 1, 0]
    loop_uvs = [(0.25, 0.75), (0.5, 0.5), (0.0, 0.0)]
    result = vertex_tex_coords(loop_vertices, loop_uvs, 3)

    assert result[0] == (0.25, 0.75)
    assert result[1] == (0.5, 0.5)
    assert result[2] == (0.0, 0.0)


def test_vertex_tex_coords_first_loop_uv_zero_later_loops_half():
    """First loop has UV (0.0, 0.0), later loops have (0.5, 0.5).
    Vertex 2 is uncovered — it stays at (0.0, 0.0) from initialization."""
    loop_vertices = [0, 1, 0]
    loop_uvs = [(0.0, 0.0), (0.5, 0.5), (0.0, 0.0)]
    result = vertex_tex_coords(loop_vertices, loop_uvs, 3)

    assert result[0] == (0.0, 0.0)
    assert result[1] == (0.5, 0.5)
    assert result[2] == (0.0, 0.0)


def test_generate_chains_with_tex_coords(tml):
    top, bottom = -0.1, -0.5
    positions, triangles = _strand_mesh((0.0, 2.1, 4.2), 5, top, bottom, 0.0)
    tex_coords = [(0.0, (p[1] - bottom) / (top - bottom)) for p in positions]

    output = tml.generate_chains(
        positions,
        triangles,
        (0.0, 0.0, 0.0),
        "hair",
        "strip/hair",
        {"joint_count": 4},
        HEAD,
        tex_coords,
    )

    assert len(output["chains"]) == 3
    for chain in output["chains"]:
        assert len(chain["joints"]) == 4
