"""``coordinates.C`` must equal the Blender-to-engine constant of ``thyllore_math_core``.

The wheel exposes the row-major rows of ``BLENDER_TO_ENGINE`` so the addon's pure-Python
copy cannot drift from the engine. The wheel is loaded from ``blender_addon/wheels/``.
"""
from __future__ import annotations

import sys
import zipfile
from pathlib import Path

import pytest

from blender_addon.common.coordinates import C, C_INV, _mat4_mul

WHEELS_DIR = Path(__file__).resolve().parents[1] / "wheels"


@pytest.fixture(scope="module")
def effect_core(tmp_path_factory):
    wheels = sorted(WHEELS_DIR.glob("thyllore_effect_core-*.whl"))
    if not wheels:
        pytest.skip("thyllore_effect_core wheel not built (run scripts/collect_wheels.sh)")
    extract_dir = tmp_path_factory.mktemp("effect_core_wheel")
    with zipfile.ZipFile(wheels[-1]) as archive:
        archive.extractall(extract_dir)

    sys.path.insert(0, str(extract_dir))
    try:
        import thyllore_effect_core

        yield thyllore_effect_core
    finally:
        sys.path.remove(str(extract_dir))


def test_c_matches_wheel_constant(effect_core):
    assert [list(row) for row in effect_core.blender_to_engine_matrix()] == C


def test_c_inv_is_the_inverse_of_c():
    identity = [[1.0 if r == c else 0.0 for c in range(4)] for r in range(4)]
    assert _mat4_mul(C, C_INV) == identity
