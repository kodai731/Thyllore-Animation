"""Tests for tools/chain_gen_quality.py on a synthetic two-column report."""

from __future__ import annotations

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from chain_gen_quality import centerline_gaps, chain_of_bone, quality_metrics


def build_head_ring(radius: float = 0.1, height: float = 1.6) -> list[list[float]]:
    ring = []
    for level in (height - 0.1, height, height + 0.1):
        for i in range(8):
            angle = 2 * math.pi * i / 8
            ring.append([radius * math.cos(angle), level, radius * math.sin(angle)])
    return ring


def build_report(first_joint_height: float = 1.6, tail_height: float = 1.2) -> dict:
    bones = []
    for column, x in ((0, 0.12), (1, -0.12)):
        for joint in range(3):
            head_height = first_joint_height - 0.1 * joint
            bones.append({
                "name": f"hair_{column:02d}_{joint:02d}",
                "parent": "Head" if joint == 0 else f"hair_{column:02d}_{joint - 1:02d}",
                "head": [x, head_height, 0.0],
                "tail": [x, tail_height, 0.0] if joint == 2 else None,
            })

    weights = []
    for column, x in ((0, 0.12), (1, -0.12)):
        for step in range(4):
            vertex = column * 10 + step
            weights.append({"mesh": "m:hair", "vertex": vertex, "component": column * 10,
                            "bone": f"hair_{column:02d}_01", "weight": 1.0,
                            "position": [x, 1.5 - 0.1 * step, 0.0]})
    for shell in range(3):
        weights.append({"mesh": "m:hair", "vertex": 100 + shell, "component": 100,
                        "bone": "Head", "weight": 1.0, "position": [0.0, 1.7, 0.0]})

    return {
        "target": "m/hair",
        "bones": bones,
        "vertex_weights": weights,
        "root_band": {"equator_height": 1.6, "height": 1.6, "center": [0.0, 1.6, 0.0]},
        "attachment_positions": build_head_ring(),
    }


def test_chain_of_bone_parses_generated_names_only() -> None:
    assert chain_of_bone("hair_03_02") == "hair_03"
    assert chain_of_bone("skirt_00_00") == "skirt_00"
    assert chain_of_bone("Head") is None
    assert chain_of_bone("mixamorig:Head") is None


def test_clean_report_satisfies_every_condition() -> None:
    metrics = quality_metrics(build_report())

    assert metrics["skull_intrusion_count"] == 0
    assert metrics["shell_rigid_ratio"] == 1.0
    assert metrics["shell_vertex_count"] == 3
    assert metrics["component_split_count"] == 0
    assert metrics["tail_reach_ratio"] == 1.0
    assert metrics["empty_column_count"] == 0
    assert metrics["chain_count"] == 2
    assert abs(metrics["root_height_diff_mean"]) < 1e-9


def test_joint_inside_the_skull_is_counted() -> None:
    report = build_report()
    report["bones"][0]["head"] = [0.0, 1.6, 0.0]

    metrics = quality_metrics(report)

    assert metrics["skull_intrusion_count"] == 1
    assert metrics["skull_intrusion_joints"] == ["hair_00_00"]


def test_shell_vertex_with_chain_weight_lowers_rigidity() -> None:
    report = build_report()
    report["vertex_weights"][-1]["bone"] = "hair_00_00"

    metrics = quality_metrics(report)

    assert abs(metrics["shell_rigid_ratio"] - 2 / 3) < 1e-9


def test_component_dominated_by_two_chains_is_a_split() -> None:
    report = build_report()
    report["vertex_weights"][3]["bone"] = "hair_01_01"

    metrics = quality_metrics(report)

    assert metrics["component_split_count"] == 1


def test_tail_short_of_the_lowest_vertex_is_a_miss() -> None:
    report = build_report(tail_height=1.4)

    metrics = quality_metrics(report)

    assert metrics["tail_reach_ratio"] == 0.0
    assert abs(metrics["tail_gap_max"] - 0.2) < 1e-9


def test_first_joint_below_the_equator_shows_in_root_height() -> None:
    metrics = quality_metrics(build_report(first_joint_height=1.55))

    assert abs(metrics["root_height_diff_mean"] + 0.05) < 1e-9


def test_column_without_vertices_is_empty() -> None:
    report = build_report()
    for entry in report["vertex_weights"]:
        if entry["bone"] == "hair_01_01":
            entry["bone"] = "hair_00_01"

    metrics = quality_metrics(report)

    assert metrics["empty_column_count"] == 1
    assert metrics["empty_columns"] == ["hair_01"]


def test_report_without_band_or_tails_degrades_to_none() -> None:
    report = build_report()
    report.pop("root_band")
    report.pop("attachment_positions")
    for bone in report["bones"]:
        bone.pop("tail")

    metrics = quality_metrics(report)

    assert metrics["skull_intrusion_count"] is None
    assert metrics["shell_rigid_ratio"] is None
    assert metrics["tail_reach_ratio"] is None
    assert metrics["component_split_count"] == 0


def test_centerline_gaps_vertical_chain_with_offset_centerline() -> None:
    """Vertical chain (heads (0,0,0),(0,-1,0) tail (0,-2,0)) with centerline at x=0.3 -> gap_max 0.3."""
    chains = {
        "hair_00": {
            "heads": [[0.0, 0.0, 0.0], [0.0, -1.0, 0.0]],
            "tail": [0.0, -2.0, 0.0],
        }
    }
    records = [
        {"component": (0, 0), "chain": 0, "points": [[0.3, 0.0, 0.0], [0.3, -1.0, 0.0], [0.3, -2.0, 0.0]]}
    ]
    result = centerline_gaps(chains, records, scale=1.0)

    assert abs(result["centerline_gap_max"] - 0.3) < 1e-9
    assert abs(result["centerline_gap_mean"] - 0.3) < 1e-9
    assert result["centerline_record_count"] == 1


def test_centerline_gaps_empty_records() -> None:
    """No records returns None values and count 0."""
    chains = {
        "hair_00": {
            "heads": [[0.0, 0.0, 0.0], [0.0, -1.0, 0.0]],
            "tail": [0.0, -2.0, 0.0],
        }
    }
    result = centerline_gaps(chains, [], scale=1.0)

    assert result["centerline_gap_max"] is None
    assert result["centerline_gap_mean"] is None
    assert result["centerline_record_count"] == 0


def test_centerline_gaps_chain_none_not_counted() -> None:
    """Record with chain=None is not counted."""
    chains = {
        "hair_00": {
            "heads": [[0.0, 0.0, 0.0], [0.0, -1.0, 0.0]],
            "tail": [0.0, -2.0, 0.0],
        }
    }
    records = [
        {"component": (0, 0), "chain": None, "points": [[0.3, 0.0, 0.0]]}
    ]
    result = centerline_gaps(chains, records, scale=1.0)

    assert result["centerline_gap_max"] is None
    assert result["centerline_gap_mean"] is None
    assert result["centerline_record_count"] == 0
