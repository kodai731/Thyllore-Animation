import tempfile

import pytest
import tomllib

from unity_muscle_table import build_slope_table, write_toml, unity_curve_attribute


def load_generated_muscles(muscles, rest_muscles, probes):
    slopes = build_slope_table(probes)
    with tempfile.NamedTemporaryFile(suffix=".toml", delete=True) as f:
        write_toml(f.name, muscles, rest_muscles, slopes)
        d = tomllib.load(open(f.name, "rb"))
    return d["muscle"]


FIXTURE_JSON = {
    "muscles": [
        {"index": 0, "name": "Spine Front-Back", "bone": "Spine", "dof": 2, "min": -40.0, "max": 40.0},
        {"index": 1, "name": "Chest Twist", "bone": "Chest", "dof": 0, "min": -20.0, "max": 20.0},
    ],
    "rest_muscles": [0.5, -0.3],
    "probes": [
        {"bone": "Spine", "axis": "x", "angle": 20, "deltas": [{"index": 0, "delta": 0.4}]},
        {"bone": "Spine", "axis": "x", "angle": -20, "deltas": [{"index": 0, "delta": -0.36}]},
        {"bone": "Chest", "axis": "y", "angle": 20, "deltas": [{"index": 1, "delta": 0.005}]},
        {"bone": "Chest", "axis": "y", "angle": -20, "deltas": [{"index": 1, "delta": -0.004}]},
    ],
}


def test_rest_values():
    m = load_generated_muscles(FIXTURE_JSON["muscles"], FIXTURE_JSON["rest_muscles"], FIXTURE_JSON["probes"])
    assert len(m) == 2
    assert m[0]["rest"] == pytest.approx(0.5, abs=1e-6)
    assert m[1]["rest"] == pytest.approx(-0.3, abs=1e-6)


def test_positive_negative_slopes():
    m = load_generated_muscles(FIXTURE_JSON["muscles"], FIXTURE_JSON["rest_muscles"], FIXTURE_JSON["probes"])
    assert len(m[0]["slopes"]) == 1
    s = m[0]["slopes"][0]
    assert s["role"] == "Spine"
    assert s["axis"] == "x"
    assert s["positive"] == pytest.approx(0.02, abs=1e-6)
    assert s["negative"] == pytest.approx(0.018, abs=1e-6)


def test_below_threshold_dropped():
    m = load_generated_muscles(FIXTURE_JSON["muscles"], FIXTURE_JSON["rest_muscles"], FIXTURE_JSON["probes"])
    assert len(m[1]["slopes"]) == 0


def test_muscle_count_and_fields():
    m = load_generated_muscles(FIXTURE_JSON["muscles"], FIXTURE_JSON["rest_muscles"], FIXTURE_JSON["probes"])
    assert len(m) == 2
    for entry in m:
        assert "index" in entry
        assert "name" in entry
        assert "role" in entry
        assert "dof" in entry
        assert "min" in entry
        assert "max" in entry
        assert "rest" in entry


def test_slopes_role_axis_order():
    probes = [
        {"bone": "Chest", "axis": "y", "angle": 20, "deltas": [{"index": 0, "delta": 0.2}]},
        {"bone": "Chest", "axis": "y", "angle": -20, "deltas": [{"index": 0, "delta": -0.2}]},
        {"bone": "Spine", "axis": "x", "angle": 20, "deltas": [{"index": 0, "delta": 0.4}]},
        {"bone": "Spine", "axis": "x", "angle": -20, "deltas": [{"index": 0, "delta": -0.4}]},
        {"bone": "Chest", "axis": "x", "angle": 20, "deltas": [{"index": 0, "delta": 0.1}]},
        {"bone": "Chest", "axis": "x", "angle": -20, "deltas": [{"index": 0, "delta": -0.1}]},
    ]
    muscles = [
        {"index": 0, "name": "Test", "bone": "Hips", "dof": 2, "min": -40.0, "max": 40.0},
    ]
    m = load_generated_muscles(muscles, [0.0], probes)
    assert len(m[0]["slopes"]) == 3
    assert (m[0]["slopes"][0]["role"], m[0]["slopes"][0]["axis"]) == ("Chest", "x")
    assert (m[0]["slopes"][1]["role"], m[0]["slopes"][1]["axis"]) == ("Chest", "y")
    assert (m[0]["slopes"][2]["role"], m[0]["slopes"][2]["axis"]) == ("Spine", "x")


def test_curve_attribute_for_fingers_and_body():
    assert unity_curve_attribute("Left Thumb 1 Stretched") == "LeftHand.Thumb.1 Stretched"
    assert unity_curve_attribute("Left Index Spread") == "LeftHand.Index.Spread"
    assert unity_curve_attribute("Right Little 3 Stretched") == "RightHand.Little.3 Stretched"
    assert unity_curve_attribute("Spine Front-Back") == "Spine Front-Back"
