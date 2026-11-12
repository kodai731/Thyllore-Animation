"""Tests for tools/chain_gen_eval.py pure functions."""

from __future__ import annotations

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from chain_gen_eval import reference_chains, generated_chains, reference_for_meshes, chain_heads, joint_metrics, weight_metrics, deform_metrics, evaluate, scale_position, REFERENCE_SPACE


def test_reference_chains_single_strand() -> None:
    """A single chain of 3 bones forms one chain of 3 names."""
    deleted_bones = [
        {"name": "Skirt_Back_1", "parent": "Hips", "head": [0.0, 0.9, -0.1], "tail": [0.0, 0.8, -0.2], "kind": "chain"},
        {"name": "Skirt_Back_2", "parent": "Skirt_Back_1", "head": [0.0, 0.8, -0.2], "tail": [0.0, 0.7, -0.3], "kind": "chain"},
        {"name": "Skirt_Back_3", "parent": "Skirt_Back_2", "head": [0.0, 0.7, -0.3], "tail": [0.0, 0.6, -0.4], "kind": "chain"},
    ]
    chains = reference_chains(deleted_bones)
    assert len(chains) == 1
    assert len(chains[0]) == 3
    assert chains[0][0] == "Skirt_Back_1"
    assert chains[0][1] == "Skirt_Back_2"
    assert chains[0][2] == "Skirt_Back_3"


def test_reference_chains_skips_single_kind() -> None:
    """Bones with kind=='single' are not included in chains."""
    deleted_bones = [
        {"name": "Skirt_Back_1", "parent": "Hips", "head": [0.0, 0.9, -0.1], "tail": [0.0, 0.8, -0.2], "kind": "chain"},
        {"name": "Skirt_Back_2", "parent": "Skirt_Back_1", "head": [0.0, 0.8, -0.2], "tail": [0.0, 0.7, -0.3], "kind": "single"},
    ]
    chains = reference_chains(deleted_bones)
    assert len(chains) == 0


def test_reference_chains_branch_from_hub() -> None:
    """A hub bone with several chain children yields one chain per branch and is itself excluded."""
    deleted_bones = [
        {"name": "Hair_root", "parent": "Head", "head": [0.0, 1.6, 0.0], "tail": [0.0, 1.7, 0.0], "kind": "chain"},
        {"name": "Hair_A1", "parent": "Hair_root", "head": [0.1, 1.7, 0.0], "tail": [0.1, 1.6, 0.0], "kind": "chain"},
        {"name": "Hair_A2", "parent": "Hair_A1", "head": [0.1, 1.6, 0.0], "tail": [0.1, 1.5, 0.0], "kind": "chain"},
        {"name": "Hair_B1", "parent": "Hair_root", "head": [-0.1, 1.7, 0.0], "tail": [-0.1, 1.6, 0.0], "kind": "chain"},
        {"name": "Hair_B2", "parent": "Hair_B1", "head": [-0.1, 1.6, 0.0], "tail": [-0.1, 1.5, 0.0], "kind": "chain"},
        {"name": "Hair_B3", "parent": "Hair_B2", "head": [-0.1, 1.5, 0.0], "tail": [-0.1, 1.4, 0.0], "kind": "chain"},
    ]
    chains = reference_chains(deleted_bones)
    assert sorted(chains) == [["Hair_A1", "Hair_A2"], ["Hair_B1", "Hair_B2", "Hair_B3"]]


def test_reference_for_meshes_keeps_only_chains_under_root_touching_mesh() -> None:
    """Breast bones weighting the skirt mesh hang from Chest and are dropped for a Hips generation."""
    ref_data = {
        "deleted_bones": [
            {"name": "Skirt_root", "parent": "Hips", "head": [0, 0.9, 0], "tail": [0, 0.8, 0], "kind": "chain"},
            {"name": "Skirt_A1", "parent": "Skirt_root", "head": [0, 0.8, 0], "tail": [0, 0.7, 0], "kind": "chain"},
            {"name": "Breast_1", "parent": "Chest", "head": [0, 1.0, 0], "tail": [0, 1.0, 0.1], "kind": "chain"},
            {"name": "Hair_A1", "parent": "Head", "head": [0, 1.5, 0], "tail": [0, 1.4, 0], "kind": "chain"},
        ],
        "vertex_weights": [
            {"mesh": "Cloth_skirt", "vertex": 0, "bone": "Skirt_A1", "weight": 1.0, "position": [0, 0.75, 0]},
            {"mesh": "Cloth_skirt", "vertex": 1, "bone": "Breast_1", "weight": 1.0, "position": [0, 1.0, 0]},
            {"mesh": "Hair_back", "vertex": 0, "bone": "Hair_A1", "weight": 1.0, "position": [0, 1.45, 0]},
        ],
    }
    bones, weights = reference_for_meshes(ref_data, {"Cloth_skirt"}, "Hips")
    assert sorted(b["name"] for b in bones) == ["Skirt_A1", "Skirt_root"]
    assert [w["bone"] for w in weights] == ["Skirt_A1"]


def test_generated_chains_single_column() -> None:
    """Bones with same role and column form one chain sorted by joint number."""
    bones = [
        {"name": "skirt_01_00", "parent": "", "head": [0.0, 0.9, -0.1]},
        {"name": "skirt_01_01", "parent": "skirt_01_00", "head": [0.0, 0.8, -0.2]},
        {"name": "skirt_01_02", "parent": "skirt_01_01", "head": [0.0, 0.7, -0.3]},
    ]
    chains = generated_chains(bones)
    assert len(chains) == 1
    assert len(chains[0]) == 3
    assert chains[0][0] == "skirt_01_00"
    assert chains[0][2] == "skirt_01_02"


def test_generated_chains_multiple_columns() -> None:
    """Bones with different columns form separate chains."""
    bones = [
        {"name": "skirt_01_00", "parent": "", "head": [0.0, 0.9, -0.1]},
        {"name": "skirt_01_01", "parent": "skirt_01_00", "head": [0.0, 0.8, -0.2]},
        {"name": "skirt_02_00", "parent": "", "head": [0.1, 0.9, -0.1]},
        {"name": "skirt_02_01", "parent": "skirt_02_00", "head": [0.1, 0.8, -0.2]},
    ]
    chains = generated_chains(bones)
    assert len(chains) == 2
    assert chains[0][0] == "skirt_01_00"
    assert chains[1][0] == "skirt_02_00"


def test_joint_metrics_perfect_match() -> None:
    """When generated joints match reference joints exactly, RMS and max are ~0."""
    ref_chains = [[(0.0, 0.9, -0.1), (0.0, 0.8, -0.2), (0.0, 0.7, -0.3)]]
    gen_chains = [[(0.0, 0.9, -0.1), (0.0, 0.8, -0.2), (0.0, 0.7, -0.3)]]

    metrics = joint_metrics(gen_chains, ref_chains)
    assert metrics["ref_chain_count"] == 1
    assert metrics["gen_chain_count"] == 1
    assert metrics["chain_count_diff"] == 0
    assert metrics["rms_distance"] < 1e-10
    assert metrics["max_distance"] < 1e-10
    assert metrics["miss_rate"] == 0.0


def test_joint_metrics_one_joint_offset() -> None:
    """When one generated joint is offset by 0.1, max distance is 0.1."""
    ref_chains = [[(0.0, 0.9, -0.1), (0.0, 0.8, -0.2), (0.0, 0.7, -0.3)]]
    gen_chains = [[(0.0, 0.9, -0.1), (0.0, 0.8, -0.2), (0.1, 0.7, -0.3)]]

    metrics = joint_metrics(gen_chains, ref_chains)
    assert metrics["max_distance"] == 0.1


def test_joint_metrics_miss_rate() -> None:
    """When a reference joint has no generated joint within miss_radius, it counts as missed."""
    ref_chains = [[(0.0, 0.9, -0.1), (0.0, 0.8, -0.2), (0.0, 0.7, -0.3)]]
    gen_chains = [[(0.0, 0.9, -0.1), (0.0, 0.8, -0.2)]]

    metrics = joint_metrics(gen_chains, ref_chains, miss_radius=0.03)
    assert metrics["gen_chain_count"] == 1
    assert metrics["chain_count_diff"] == 0
    assert abs(metrics["miss_rate"] - 1.0 / 3) < 1e-6


def test_joint_metrics_chain_count_diff() -> None:
    """When generated has more chains than reference, chain_count_diff is positive."""
    ref_chains = [[(0.0, 0.9, -0.1), (0.0, 0.8, -0.2)]]
    gen_chains = [
        [(0.0, 0.9, -0.1), (0.0, 0.8, -0.2)],
        [(0.1, 0.9, -0.1), (0.1, 0.8, -0.2)],
    ]

    metrics = joint_metrics(gen_chains, ref_chains)
    assert metrics["ref_chain_count"] == 1
    assert metrics["gen_chain_count"] == 2
    assert metrics["chain_count_diff"] == 1


def test_joint_metrics_correspondence() -> None:
    """Chain correspondence assigns each generated chain to the best-matching reference chain."""
    ref_chains = [
        [(0.0, 0.9, -0.1), (0.0, 0.8, -0.2)],
        [(1.0, 0.9, -0.1), (1.0, 0.8, -0.2)],
    ]
    gen_chains = [
        [(0.0, 0.9, -0.1), (0.0, 0.8, -0.2)],
        [(1.0, 0.9, -0.1), (1.0, 0.8, -0.2)],
    ]

    metrics = joint_metrics(gen_chains, ref_chains)
    corr = metrics["chain_correspondence"]
    assert len(corr) == 2
    assert corr[0]["matched_ref_chain"] == 0
    assert corr[1]["matched_ref_chain"] == 1


def test_joint_metrics_empty_generated() -> None:
    """When generated chains are empty, RMS is 0 and miss rate is 1.0."""
    ref_chains = [[(0.0, 0.9, -0.1), (0.0, 0.8, -0.2)]]
    gen_chains = []

    metrics = joint_metrics(gen_chains, ref_chains)
    assert metrics["gen_chain_count"] == 0
    assert metrics["chain_count_diff"] == -1
    assert metrics["rms_distance"] == 0.0
    assert metrics["miss_rate"] == 1.0


def test_joint_metrics_empty_reference() -> None:
    """When reference chains are empty, miss rate is 0.0."""
    ref_chains = []
    gen_chains = [[(0.0, 0.9, -0.1), (0.0, 0.8, -0.2)]]

    metrics = joint_metrics(gen_chains, ref_chains)
    assert metrics["ref_chain_count"] == 0
    assert metrics["chain_count_diff"] == 1
    assert metrics["miss_rate"] == 0.0


def test_weight_metrics_perfect_match() -> None:
    """When generated weights match reference weights exactly, agreement is 1.0 and L1 ≈ 0."""
    ref_weights = [
        {"mesh": "skirt", "vertex": 0, "bone": "Skirt_Back_1", "weight": 0.5, "position": [0.0, 0.9, -0.1]},
        {"mesh": "skirt", "vertex": 0, "bone": "Skirt_Back_2", "weight": 0.5, "position": [0.0, 0.9, -0.1]},
    ]
    gen_weights = [
        {"mesh": "skirt", "vertex": 0, "bone": "skirt_01_00", "weight": 0.5, "position": [0.0, 0.9, -0.1]},
        {"mesh": "skirt", "vertex": 0, "bone": "skirt_01_01", "weight": 0.5, "position": [0.0, 0.9, -0.1]},
    ]
    correspondence = [{"gen_chain": 0, "matched_ref_chain": 0}]
    gen_chain_names = [["skirt_01_00", "skirt_01_01"]]
    ref_chain_bones = [["Skirt_Back_1", "Skirt_Back_2"]]

    metrics = weight_metrics(gen_weights, ref_weights, correspondence, gen_chain_names, ref_chain_bones)
    assert metrics["dominant_chain_agreement"] == 1.0
    assert metrics["arc_weight_l1"] < 1e-6
    assert metrics["matched_vertices"] == 1
    assert metrics["unmatched_vertices"] == 0


def test_weight_metrics_unmatched_vertex() -> None:
    """When a generated vertex is beyond tolerance from any reference vertex, it counts as unmatched."""
    ref_weights = [
        {"mesh": "skirt", "vertex": 0, "bone": "Skirt_Back_1", "weight": 1.0, "position": [0.0, 0.9, -0.1]},
    ]
    gen_weights = [
        {"mesh": "skirt", "vertex": 0, "bone": "skirt_01_00", "weight": 1.0, "position": [0.0, 0.9, -0.1]},
        {"mesh": "skirt", "vertex": 1, "bone": "skirt_01_00", "weight": 1.0, "position": [1.0, 1.0, 1.0]},
    ]
    correspondence = [{"gen_chain": 0, "matched_ref_chain": 0}]
    gen_chain_names = [["skirt_01_00"]]
    ref_chain_bones = [["Skirt_Back_1"]]

    metrics = weight_metrics(gen_weights, ref_weights, correspondence, gen_chain_names, ref_chain_bones)
    assert metrics["matched_vertices"] == 1
    assert metrics["unmatched_vertices"] == 1


def test_deform_metrics_identical() -> None:
    """When generated and reference CSVs are identical, RMS is 0."""
    rows = [
        ["0", "0", "0.0", "0.9", "-0.1", "0.0", "0.9", "-0.1"],
        ["0", "1", "0.0", "0.8", "-0.2", "0.0", "0.8", "-0.2"],
    ]
    metrics = deform_metrics(rows, rows)
    assert metrics["rms_error"] == 0.0
    assert metrics["max_error"] == 0.0
    assert metrics["matched_vertices"] == 2


def test_deform_metrics_offset() -> None:
    """When one vertex is offset by 0.1, max error is 0.1."""
    ref_rows = [
        ["0", "0", "0.0", "0.9", "-0.1", "0.0", "0.9", "-0.1"],
        ["0", "1", "0.0", "0.8", "-0.2", "0.0", "0.8", "-0.2"],
    ]
    gen_rows = [
        ["0", "0", "0.0", "0.9", "-0.1", "0.0", "0.9", "-0.1"],
        ["0", "1", "0.0", "0.8", "-0.2", "0.1", "0.8", "-0.2"],
    ]
    metrics = deform_metrics(gen_rows, ref_rows)
    assert metrics["max_error"] == 0.1


def _reference_and_generated(space: str | None) -> tuple[dict, dict]:
    head_0 = [0.0, 0.9, -0.1]
    head_1 = [0.0, 0.8, -0.2]
    ref_data = {
        "deleted_bones": [
            {"name": "Skirt_Back_1", "parent": "Hips", "head": head_0, "tail": [0.0, 0.0, 0.0], "kind": "chain"},
            {"name": "Skirt_Back_2", "parent": "Skirt_Back_1", "head": head_1, "tail": [0.0, 0.0, 0.0], "kind": "chain"},
        ],
        "vertex_weights": [
            {"mesh": "skirt", "vertex": 0, "bone": "Skirt_Back_1", "weight": 0.5, "position": head_0},
            {"mesh": "skirt", "vertex": 0, "bone": "Skirt_Back_2", "weight": 0.5, "position": head_0},
        ],
    }
    if space is not None:
        ref_data["space"] = space
    gen_data = {
        "chain_gen": {
            "bones": [
                {"name": "skirt_01_00", "parent": "", "head": head_0},
                {"name": "skirt_01_01", "parent": "skirt_01_00", "head": head_1},
            ],
            "vertex_weights": [
                {"mesh": "skirt", "vertex": 0, "bone": "skirt_01_00", "weight": 0.5, "position": head_0},
                {"mesh": "skirt", "vertex": 0, "bone": "skirt_01_01", "weight": 0.5, "position": head_0},
            ],
        },
    }
    return ref_data, gen_data


def test_evaluate_matches_reference_written_in_engine_space() -> None:
    ref_data, gen_data = _reference_and_generated(REFERENCE_SPACE)

    output = evaluate(ref_data, gen_data, reference_scale=1.0, generated_scale=1.0)

    assert output["joint"]["rms_distance"] < 1e-10
    assert output["weight"]["matched_vertices"] == 1
    assert output["weight"]["unmatched_vertices"] == 0
    assert output["weight"]["dominant_chain_agreement"] == 1.0


def test_evaluate_rejects_reference_without_engine_space() -> None:
    import pytest

    for space in (None, "blender_z_up"):
        ref_data, gen_data = _reference_and_generated(space)
        with pytest.raises(ValueError):
            evaluate(ref_data, gen_data)
