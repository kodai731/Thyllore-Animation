"""generate_chains from the wheel: bone specs, weights, settings table and the Blender-space conversion."""
from __future__ import annotations

import math
import sys
import zipfile
from pathlib import Path

import pytest

from blender_addon.common.chain_bones import (
    bone_specs,
    covered_group_vertices,
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


HEAD_POSITIONS = [(0.2, 0.1, 0.0), (-0.2, 0.1, 0.0), (0.0, 0.1, 0.2), (0.0, 0.1, -0.2)]
ROOT_BONE = 0


def _head_skins(positions=HEAD_POSITIONS) -> list[dict]:
    return [{"positions": list(positions), "influences": [[(ROOT_BONE, 1.0)]] * len(positions)}]


HEAD = _head_skins()


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
        "azimuth_root_band_offset",
        "card_root_rigid_length",
        "garment_cover_distance",
        "garment_covered_band_ratio",
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
        _head_skins([blender_to_engine_point(engine_to_blender_point(p)) for p in HEAD_POSITIONS]),
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
        ROOT_BONE,
        tex_coords,
    )

    assert len(output["chains"]) == 3
    for chain in output["chains"]:
        assert len(chain["joints"]) == 4


def _fit_bones(names_parents_heads):
    bones = []
    for name, parent, head in names_parents_heads:
        bind = [[1.0, 0.0, 0.0, head[0]], [0.0, 1.0, 0.0, head[1]], [0.0, 0.0, 1.0, head[2]], [0.0, 0.0, 0.0, 1.0]]
        bones.append({"name": name, "parent": parent, "rest_position": list(head), "bind_matrix": bind})
    return bones


def test_generate_chains_measures_joint_radii_and_fits_a_head_capsule(tml):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)
    output = tml.generate_chains(positions, triangles, (0.0, 0.0, 0.0), "hair", "strip/hair", {"column_count": 1}, HEAD)

    for chain in output["chains"]:
        assert all(math.isfinite(joint["hit_radius"]) for joint in chain["joints"])
    capsule = output["head_capsule"]
    assert capsule is not None
    assert capsule["radius"] > 0.0
    assert capsule["top"][1] >= capsule["bottom"][1]


def test_register_generated_chains_binds_the_head_capsule_to_every_chain(tml):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)
    output = tml.generate_chains(positions, triangles, (0.0, 0.0, 0.0), "hair", "strip/hair", {"column_count": 1}, HEAD)
    joint_names = [joint["name"] for chain in output["chains"] for joint in chain["joints"]]
    bones = _fit_bones(
        [("Hips", None, (0.0, -1.0, 0.0)), ("Head", 0, (0.0, 0.0, 0.0))]
        + [(name, 1 + i, (0.0, -0.1 * (i + 1), 0.0)) for i, name in enumerate(joint_names)]
    )
    empty = {"collision_mode": "vrm_compatible", "colliders": [], "collider_groups": [], "chains": []}

    spring = tml.register_generated_chains(
        empty, bones, [], "Head", "strip/hair", "hair", output["chains"], output["head_capsule"]
    )

    assert [c["name"] for c in spring["chains"]] == [c["name"] for c in output["chains"]]
    assert [c["bone"] for c in spring["colliders"]] == ["Head"]
    assert spring["colliders"][0]["shape"] == "capsule"
    assert [g["name"] for g in spring["collider_groups"]] == ["Fitted Body"]
    for chain, generated in zip(spring["chains"], output["chains"]):
        assert chain["collider_groups"] == ["Fitted Body"]
        assert chain["origin"] == {"mesh": "strip", "material": "hair", "role": "hair"}
        assert [j["hit_radius"] for j in chain["joints"]] == [j["hit_radius"] for j in generated["joints"]]

    again = tml.register_generated_chains(
        spring, bones, [], "Head", "strip/hair", "hair", output["chains"], output["head_capsule"]
    )
    assert len(again["chains"]) == len(spring["chains"])
    assert len(again["colliders"]) == 1


def test_group_weights_resolves_root_and_skin_bone_targets():
    output = {
        "weights": [
            {"vertex": 0, "influences": [("root", 1.0)]},
            {"vertex": 1, "influences": [({"bone": 2}, 0.75), ("hair_strip_hair_00_00", 0.25)]},
        ]
    }

    groups = group_weights(output, "Head", ["Hips", "Spine", "Chest", "Head"])

    assert groups == {
        "Head": [(0, 1.0)],
        "Chest": [(1, 0.75)],
        "hair_strip_hair_00_00": [(1, 0.25)],
    }


def test_covered_group_vertices_reads_only_weighted_members():
    pairs = [(0, [(2, 1.0)]), (1, [(2, 0.0)]), (2, [(1, 1.0)]), (3, [(1, 0.5), (2, 0.25)])]

    assert covered_group_vertices(pairs, 2) == [0, 3]


def test_generate_chains_marked_covered_vertices_leave_the_chain(tml):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)
    marked = [v for v, p in enumerate(positions) if p[1] > -0.25]

    output = tml.generate_chains(
        positions,
        triangles,
        (0.0, 0.0, 0.0),
        "garment",
        "strip/skirt",
        {"joint_count": 2, "column_count": 1},
        None,
        0,
        [],
        marked,
    )

    assert sorted(output["covered"]) == marked
    assert not set(output["chains"][0]["vertices"]) & set(marked)
    for entry in output["weights"]:
        if entry["vertex"] in marked:
            assert entry["influences"] == [({"bone": 0}, 1.0)]
