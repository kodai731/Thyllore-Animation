"""generate_chains from the wheel: bone specs, weights, settings table and the Blender-space conversion."""
from __future__ import annotations

import math
import sys
import zipfile
from pathlib import Path

import pytest

from blender_addon.common.chain_bones import (
    bone_specs,
    chain_of_membership_group,
    given_chains_from_groups,
    group_vertices,
    group_weights,
    joints_from_bones,
    membership_group_name,
    mismatched_vertices,
    output_of_new_chains,
    output_to_blender,
    vertex_tex_coords,
)
from blender_addon.common.coordinates import blender_to_engine_point, engine_to_blender_point

WHEELS_DIR = Path(__file__).resolve().parents[1] / "wheels"


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


HEAD_POSITIONS = [(0.2, 0.1, 0.0), (-0.2, 0.1, 0.0), (0.0, 0.1, 0.2), (0.0, 0.1, -0.2)]
ROOT_BONE = 0


def _head_skins(positions=HEAD_POSITIONS) -> list[dict]:
    return [{"positions": list(positions), "influences": [[(ROOT_BONE, 1.0)]] * len(positions)}]


HEAD = _head_skins()


def _generate(avatar_api, positions, triangles, target, settings, **inputs):
    """Build the GenerateChainsRequest dict and call avatar_api.generate_chains."""
    request = {
        "positions": positions,
        "triangles": triangles,
        "root_position": (0.0, 0.0, 0.0),
        "target": target,
        "settings": settings,
        **inputs,
    }
    return avatar_api.generate_chains(request)


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


def test_settings_table_lists_defaults(avatar_api):
    params = avatar_api.chain_gen_ui_params()

    assert [p["name"] for p in params] == [
        "joint_count",
        "column_count",
        "strand_merge_distance",
        "root_blend_width",
        "column_blend_width",
        "azimuth_root_band_offset",
        "card_root_rigid_length",
        "garment_fit_gap",
        "garment_support_ratio",
    ]
    assert params[0]["default"] == 4
    assert params[0]["label"] == "Joint Count"


def test_chain_roles_lists_generatable_roles_with_their_default_root(avatar_api):
    roles = avatar_api.chain_roles()

    assert len(roles) == 2
    assert roles[0] == {"role": "garment", "default_root": "Hips"}
    assert roles[1] == {"role": "hair", "default_root": "Head"}


def test_unknown_setting_is_rejected(avatar_api):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)

    with pytest.raises(Exception, match="unknown chain generation setting"):
        _generate(avatar_api, positions, triangles, "strip/hair", {"joints": 4})


def test_bone_specs_chain_joints_root_to_tip(avatar_api):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)
    output = _generate(avatar_api, positions, triangles, "strip/hair", {"joint_count": 4, "column_count": 1}, skins=HEAD, attachment="cap")

    specs = bone_specs(output, "Head")

    assert [name for name, _, _, _ in specs] == [
        "strip_hair_00_00",
        "strip_hair_00_01",
        "strip_hair_00_02",
        "strip_hair_00_03",
    ]
    assert [parent for _, parent, _, _ in specs] == [
        "Head",
        "strip_hair_00_00",
        "strip_hair_00_01",
        "strip_hair_00_02",
    ]
    for (_, _, _, tail), (_, _, next_head, _) in zip(specs, specs[1:]):
        assert tail == next_head
    assert specs[0][2][1] > specs[-1][2][1]


def test_group_weights_sum_to_one_per_vertex(avatar_api):
    positions, triangles = _strand_mesh((0.5, 3.5), 9, -0.05, -0.45, 0.01)
    output = _generate(avatar_api, positions, triangles, "strip/hair", {"joint_count": 4}, skins=HEAD, attachment="cap")

    totals: dict[int, float] = {}
    for influences in group_weights(output, "Head").values():
        for vertex, weight in influences:
            totals[vertex] = totals.get(vertex, 0.0) + weight

    assert len(totals) == len(positions)
    assert all(abs(total - 1.0) < 1e-5 for total in totals.values())


def test_blender_space_input_converted_at_the_boundary_matches_engine_input(avatar_api):
    positions_engine, triangles = _strand_mesh((0.5, 3.5), 9, -0.05, -0.45, 0.01)
    positions_blender = [engine_to_blender_point(p) for p in positions_engine]
    settings = {"joint_count": 4}

    engine = _generate(avatar_api, positions_engine, triangles, "strip/hair", settings, skins=HEAD, attachment="cap")
    converted = _generate(
        avatar_api,
        [blender_to_engine_point(p) for p in positions_blender],
        triangles,
        "strip/hair",
        settings,
        skins=_head_skins([blender_to_engine_point(engine_to_blender_point(p)) for p in HEAD_POSITIONS]),
        attachment="cap",
    )

    assert len(engine["chains"]) == len(converted["chains"]) == 2
    for chain_engine, chain_converted in zip(engine["chains"], converted["chains"]):
        for joint_engine, joint_converted in zip(chain_engine["joints"], chain_converted["joints"]):
            assert joint_engine["name"] == joint_converted["name"]
            assert max(abs(a - b) for a, b in zip(joint_engine["head"], joint_converted["head"])) < 1e-6
    assert engine["weights"] == converted["weights"]


def test_output_to_blender_converts_heads_and_tails(avatar_api):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)
    output = _generate(avatar_api, positions, triangles, "strip/hair", {"column_count": 1}, skins=HEAD, attachment="cap")

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


def test_generate_chains_with_tex_coords(avatar_api):
    top, bottom = -0.1, -0.5
    positions, triangles = _strand_mesh((0.0, 2.1, 4.2), 5, top, bottom, 0.0)
    tex_coords = [(0.0, (p[1] - bottom) / (top - bottom)) for p in positions]

    output = _generate(
        avatar_api,
        positions,
        triangles,
        "strip/hair",
        {"joint_count": 4},
        skins=HEAD,
        root_bone=ROOT_BONE,
        tex_coords=tex_coords,
        attachment="cap",
    )

    assert len(output["chains"]) == 3
    for chain in output["chains"]:
        assert len(chain["joints"]) == 4


def test_generate_chains_reports_the_attachment_it_used(avatar_api):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)

    output = _generate(avatar_api, positions, triangles, "strip/skirt", {"column_count": 1}, attachment="ring")

    assert output["attachment"] == "ring"


def _fit_bones(names_parents_heads):
    bones = []
    for name, parent, head in names_parents_heads:
        bind = [[1.0, 0.0, 0.0, head[0]], [0.0, 1.0, 0.0, head[1]], [0.0, 0.0, 1.0, head[2]], [0.0, 0.0, 0.0, 1.0]]
        bones.append({"name": name, "parent": parent, "rest_position": list(head), "bind_matrix": bind})
    return bones


def test_generate_chains_measures_joint_radii_and_fits_a_head_capsule(avatar_api):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)
    output = _generate(
        avatar_api,
        positions,
        triangles,
        "strip/hair",
        {"column_count": 1},
        skins=HEAD,
        humanoid_bones={"Head": ROOT_BONE},
        bones=[{"name": "Head", "parent": None, "rest_position": [0.0, 0.0, 0.0]}],
    )

    for chain in output["chains"]:
        assert all(math.isfinite(joint["hit_radius"]) for joint in chain["joints"])
    capsule = output["head_capsule"]
    assert capsule is not None
    assert capsule["radius"] > 0.0
    assert capsule["top"][1] >= capsule["bottom"][1]


def test_register_generated_chains_binds_the_head_capsule_to_every_chain(avatar_api):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)
    output = _generate(
        avatar_api,
        positions,
        triangles,
        "strip/hair",
        {"column_count": 1},
        skins=HEAD,
        humanoid_bones={"Head": ROOT_BONE},
        bones=[{"name": "Head", "parent": None, "rest_position": [0.0, 0.0, 0.0]}],
    )
    joint_names = [joint["name"] for chain in output["chains"] for joint in chain["joints"]]
    bones = _fit_bones(
        [("Hips", None, (0.0, -1.0, 0.0)), ("Head", 0, (0.0, 0.0, 0.0))]
        + [(name, 1 + i, (0.0, -0.1 * (i + 1), 0.0)) for i, name in enumerate(joint_names)]
    )
    empty = {"collision_mode": "vrm_compatible", "colliders": [], "collider_groups": [], "chains": []}

    spring = avatar_api.register_generated_chains({
        "spring": empty, "bones": bones, "skins": [], "root_bone": "Head",
        "target": "strip/hair", "attachment": "cap", "chains": output["chains"],
        "head_capsule": output["head_capsule"],
    })

    assert [c["name"] for c in spring["chains"]] == [c["name"] for c in output["chains"]]
    assert [c["bone"] for c in spring["colliders"]] == ["Head"]
    assert spring["colliders"][0]["shape"] == "capsule"
    assert [g["name"] for g in spring["collider_groups"]] == ["Fitted Body"]
    for chain, generated in zip(spring["chains"], output["chains"]):
        assert chain["collider_groups"] == ["Fitted Body"]
        assert chain["origin"] == {"mesh": "strip", "material": "hair", "attachment": "cap"}
        assert [j["hit_radius"] for j in chain["joints"]] == [j["hit_radius"] for j in generated["joints"]]

    again = avatar_api.register_generated_chains({
        "spring": spring, "bones": bones, "skins": [], "root_bone": "Head",
        "target": "strip/hair", "attachment": "cap", "chains": output["chains"],
        "head_capsule": output["head_capsule"],
    })
    assert len(again["chains"]) == len(spring["chains"])
    assert len(again["colliders"]) == 1


def test_register_generated_chains_rejects_unknown_attachment(avatar_api):
    empty = {"collision_mode": "vrm_compatible", "colliders": [], "collider_groups": [], "chains": []}
    bones = _fit_bones([("Head", None, (0.0, 0.0, 0.0))])

    with pytest.raises(ValueError):
        avatar_api.register_generated_chains({
            "spring": empty, "bones": bones, "skins": [], "root_bone": "Head",
            "target": "strip/hair", "attachment": "band", "chains": [],
        })


def test_group_weights_resolves_root_and_skin_bone_targets():
    output = {
        "weights": [
            {"vertex": 0, "influences": [("root", 1.0)]},
            {"vertex": 1, "influences": [({"bone": 2}, 0.75), ("strip_hair_00_00", 0.25)]},
        ]
    }

    groups = group_weights(output, "Head", ["Hips", "Spine", "Chest", "Head"])

    assert groups == {
        "Head": [(0, 1.0)],
        "Chest": [(1, 0.75)],
        "strip_hair_00_00": [(1, 0.25)],
    }


def test_group_vertices_reads_only_weighted_members():
    pairs = [(0, [(2, 1.0)]), (1, [(2, 0.0)]), (2, [(1, 1.0)]), (3, [(1, 0.5), (2, 0.25)])]

    assert group_vertices(pairs, 2) == [0, 3]


def test_generate_chains_marked_covered_vertices_leave_the_chain(avatar_api):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)
    marked = [v for v, p in enumerate(positions) if p[1] > -0.25]

    output = _generate(
        avatar_api,
        positions,
        triangles,
        "strip/skirt",
        {"joint_count": 2, "column_count": 1},
        covered_vertices=marked,
    )

    assert sorted(output["covered"]) == marked
    assert not set(output["chains"][0]["vertices"]) & set(marked)
    for entry in output["weights"]:
        if entry["vertex"] in marked:
            assert entry["influences"] == [({"bone": 0}, 1.0)]


def _ring_mesh(radius_at, top, bottom, levels, around=8):
    positions = []
    triangles = []
    for level in range(levels):
        y = top + (bottom - top) * level / (levels - 1)
        radius = radius_at(y)
        for i in range(around):
            angle = 2 * math.pi * i / around
            positions.append((radius * math.cos(angle), y, radius * math.sin(angle)))
    for level in range(levels - 1):
        base = level * around
        for i in range(around):
            nxt = (i + 1) % around
            triangles.append([base + i, base + nxt, base + around + i])
            triangles.append([base + nxt, base + around + nxt, base + around + i])
    return positions, triangles


def test_generate_chains_hangs_a_flared_dress_from_the_supporting_collider(avatar_api):
    positions, triangles = _ring_mesh(lambda y: 0.11 if y > -0.4 else 0.2, -0.1, -0.9, 11)
    pelvis = {"bone": 3, "head": [0.0, 0.0, 0.0], "tail": [0.0, -0.7, 0.0], "radius": 0.1}
    arm = {"bone": 4, "head": [0.0, 0.0, 0.0], "tail": [0.9, 0.0, 0.0], "radius": 0.3}
    bones = [
        {"name": "Root", "parent": None, "rest_position": [0.0, 0.0, 0.0]},
        {"name": "A", "parent": 0, "rest_position": [0.0, 0.0, 0.0]},
        {"name": "B", "parent": 0, "rest_position": [0.0, 0.0, 0.0]},
        {"name": "Hips", "parent": 0, "rest_position": [0.0, -0.1, 0.0]},
        {"name": "LeftArm", "parent": 0, "rest_position": [0.0, -0.2, 0.0]},
    ]

    output = _generate(
        avatar_api,
        positions,
        triangles,
        "dress/top",
        {"joint_count": 3, "column_count": 1, "garment_fit_gap": 0.015},
        skins=[],
        root_bone=0,
        tex_coords=[],
        body_colliders=[pelvis, arm],
        humanoid_bones={"Hips": 3, "LeftUpperArm": 4},
        bones=bones,
    )

    support = output["support"]
    assert support["bone"] == 3
    assert -0.42 < support["boundary_height"] < -0.33
    covered = set(output["covered"])
    assert covered == {v for v, p in enumerate(positions) if p[1] >= support["boundary_height"]}
    assert not covered & set(output["chains"][0]["vertices"])
    for entry in output["weights"]:
        if entry["vertex"] in covered:
            assert entry["influences"] == [({"bone": 3}, 1.0)]


def test_membership_group_name_round_trips_and_rejects_other_groups():
    assert chain_of_membership_group(membership_group_name("strip_hair_03")) == "strip_hair_03"
    assert chain_of_membership_group("strip_hair_03_00") is None
    assert chain_of_membership_group("thyllore_covered") is None


def test_membership_group_prefix_is_built_and_parsed_in_chain_bones_only():
    addon_root = Path(__file__).resolve().parents[1]
    offenders = [
        path.relative_to(addon_root)
        for path in addon_root.rglob("*.py")
        if "tests" not in path.parts and "thyllore_chain_" in path.read_text() and path.name != "chain_bones.py"
    ]
    assert offenders == []


def test_given_chains_from_groups_lists_weighted_membership_groups_only():
    pairs = [(0, [(0, 1.0)]), (1, [(0, 1.0), (2, 0.0)]), (2, [(1, 1.0)]), (3, [(2, 1.0)])]
    groups = [("thyllore_chain_strip_hair_00", 0), ("Head", 1), ("thyllore_chain_strip_hair_01", 2), ("thyllore_chain_strip_hair_02", 3)]

    assert given_chains_from_groups(pairs, groups) == [
        {"name": "strip_hair_00", "vertices": [0, 1]},
        {"name": "strip_hair_01", "vertices": [3]},
    ]


def test_generate_chains_with_given_memberships_skips_the_split(avatar_api):
    positions, triangles = _strand_mesh((0.5, 3.5), 9, -0.05, -0.45, 0.01)
    automatic = _generate(avatar_api, positions, triangles, "strip/hair", {"joint_count": 4}, skins=HEAD, attachment="cap")
    assert len(automatic["chains"]) == 2
    members = [chain["vertices"] for chain in automatic["chains"]]
    given = [
        {"name": "strip_hair_07", "vertices": members[0] + members[1]},
        {"name": "other_mesh_00", "vertices": members[1]},
    ]

    output = _generate(avatar_api, positions, triangles, "strip/hair", {"joint_count": 4}, skins=HEAD, given_chains=given, attachment="cap")

    assert [chain["name"] for chain in output["chains"]] == ["strip_hair_07"]
    assert output["chains"][0]["vertices"] == sorted(members[0] + members[1])
    assert [joint["name"] for joint in output["chains"][0]["joints"]][0] == "strip_hair_07_00"
    assert len(output["chains"][0]["joints"]) == 4


def test_joints_from_bones_are_the_heads_then_the_last_tail():
    bones = [((0.0, 1.0, 0.0), (0.0, 0.8, 0.0)), ((0.0, 0.8, 0.0), (0.0, 0.5, 0.1))]

    assert joints_from_bones(bones) == [(0.0, 1.0, 0.0), (0.0, 0.8, 0.0), (0.0, 0.5, 0.1)]


def test_output_of_new_chains_drops_the_read_back_chains():
    output = {"chains": [{"name": "a"}, {"name": "b"}, {"name": "c"}], "weights": []}

    assert output_of_new_chains(output, {"b"}) == {"chains": [{"name": "a"}, {"name": "c"}], "weights": []}


def test_submesh_chain_names_keeps_this_submesh_only(avatar_api):
    names = ["strip_hair_00", "strip_hair_12", "other_hair_00", "strip_hair_00_01"]

    assert avatar_api.submesh_chain_names("strip/hair", names) == ["strip_hair_00", "strip_hair_12"]


def test_generate_chains_with_given_joints_places_the_bones_there(avatar_api):
    positions, triangles = _strand_mesh((0.5,), 9, -0.05, -0.45, 0.01)
    heads_then_tail = [(0.1, -0.05, 0.0), (0.1, -0.2, 0.0), (0.1, -0.45, 0.0)]
    given = [{"name": "strip_hair_00", "vertices": list(range(len(positions))), "joints": heads_then_tail}]

    output = _generate(avatar_api, positions, triangles, "strip/hair", {"joint_count": 4}, skins=HEAD, given_chains=given, attachment="cap")

    chain = output["chains"][0]
    placed = [tuple(joint["head"]) for joint in chain["joints"]] + [tuple(chain["tail"])]
    assert len(placed) == len(heads_then_tail)
    assert all(abs(a - b) < 1e-6 for point, expected in zip(placed, heads_then_tail) for a, b in zip(point, expected))


def test_mismatched_vertices_follow_the_membership_rules():
    chains = [
        {"name": "a", "vertices": [0, 1, 2, 3, 7], "joints": [{"name": "a_00"}, {"name": "a_01"}]},
        {"name": "b", "vertices": [4], "joints": [{"name": "b_00"}]},
    ]
    group_names = {0: "a_00", 1: "a_01", 2: "b_00", 3: "Head", 4: "Spine"}
    weights = [
        (0, [(0, 0.5), (1, 0.5)]),
        (1, [(3, 1.0)]),
        (2, [(0, 0.5), (2, 0.5)]),
        (3, [(0, 0.3), (3, 0.7)]),
        (4, [(2, 1.0)]),
        (5, [(2, 0.2), (4, 0.8)]),
        (6, [(4, 1.0)]),
        (7, [(2, 0.2), (4, 0.8)]),
    ]

    assert mismatched_vertices(chains, weights, group_names, set()) == {1, 2, 5, 7}
    assert mismatched_vertices(chains, weights, group_names, {"b"}) == {1, 2, 4, 5, 7}


def test_generate_chains_rejects_an_unknown_request_key(avatar_api):
    positions = [(0.0, -0.1, 0.0), (0.0, -0.3, 0.0)]
    triangles = [[0, 1, 0]]
    request = {
        "positions": positions,
        "triangles": triangles,
        "root_position": (0.0, 0.0, 0.0),
        "target": "strip/hair",
        "settings": {"joint_count": 2},
        "skins": [{"positions": positions, "influences": [[(0, 1.0)], [(0, 1.0)]]}],
        "root_bone": 0,
        "given_chain": [],  # misspelled key — should be "given_chains"
    }
    with pytest.raises(Exception):
        avatar_api.generate_chains(request)
