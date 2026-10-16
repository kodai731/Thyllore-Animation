"""generate_chains from the wheel: bone specs, weights, and Blender Z-up parity with the engine's Y-up."""
from __future__ import annotations

import math
import sys
import zipfile
from pathlib import Path

import pytest

from blender_addon.common.chain_bones import bone_specs, group_weights

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


def _yup_to_zup(p):
    return (p[0], -p[2], p[1])


def _zup_to_yup(p):
    return (p[0], p[2], -p[1])


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


def test_bone_specs_chain_joints_root_to_tip(tml):
    positions, triangles = _strand_mesh((0.0,), 5, -0.1, -0.5, 0.0)
    output = tml.generate_chains(positions, triangles, (0.0, 0.0, 0.0), (0.0, 1.0, 0.0), "hair", 4, 1)

    specs = bone_specs(output, "Head")

    assert [name for name, _, _, _ in specs] == ["hair_00_00", "hair_00_01", "hair_00_02", "hair_00_03"]
    assert [parent for _, parent, _, _ in specs] == ["Head", "hair_00_00", "hair_00_01", "hair_00_02"]
    for (_, _, _, tail), (_, _, next_head, _) in zip(specs, specs[1:]):
        assert tail == next_head
    assert specs[0][2][1] > specs[-1][2][1]


def test_group_weights_sum_to_one_per_vertex(tml):
    positions, triangles = _strand_mesh((0.5, 3.5), 9, -0.05, -0.45, 0.01)
    output = tml.generate_chains(positions, triangles, (0.0, 0.0, 0.0), (0.0, 1.0, 0.0), "hair", 4, 0)

    totals: dict[int, float] = {}
    for influences in group_weights(output, "Head").values():
        for vertex, weight in influences:
            totals[vertex] = totals.get(vertex, 0.0) + weight

    assert len(totals) == len(positions)
    assert all(abs(total - 1.0) < 1e-5 for total in totals.values())


def test_zup_input_matches_yup_input(tml):
    positions_yup, triangles = _strand_mesh((0.5, 3.5), 9, -0.05, -0.45, 0.01)
    positions_zup = [_yup_to_zup(p) for p in positions_yup]

    yup = tml.generate_chains(positions_yup, triangles, (0.0, 0.0, 0.0), (0.0, 1.0, 0.0), "hair", 4, 0)
    zup = tml.generate_chains(positions_zup, triangles, (0.0, 0.0, 0.0), (0.0, 0.0, 1.0), "hair", 4, 0)

    assert len(yup["chains"]) == len(zup["chains"]) == 2
    for chain_yup, chain_zup in zip(yup["chains"], zup["chains"]):
        for joint_yup, joint_zup in zip(chain_yup["joints"], chain_zup["joints"]):
            assert joint_yup["name"] == joint_zup["name"]
            converted = _zup_to_yup(joint_zup["head"])
            assert max(abs(a - b) for a, b in zip(joint_yup["head"], converted)) < 1e-5
    for weight_yup, weight_zup in zip(yup["weights"], zup["weights"]):
        assert weight_yup["vertex"] == weight_zup["vertex"]
        pairs = zip(weight_yup["influences"], weight_zup["influences"])
        for (target_yup, w_yup), (target_zup, w_zup) in pairs:
            assert target_yup == target_zup
            assert abs(w_yup - w_zup) < 1e-6
