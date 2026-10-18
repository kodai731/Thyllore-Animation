"""Reference-free quality metrics of a chain generation report (the `chain_gen` block of --batch-anim-dump).

These need no hand-made rig: they check the physical and topological conditions a generated hair rig must
satisfy on any model (joints outside the skull, a rigid shell above the root band, no strand split across
columns, tails that reach the strand ends, columns that hold vertices). The tool reports numbers only;
the thresholds are the user's to set.

All positions are in the dump's model space with +Y up; `scale` brings them to meters.
"""

from __future__ import annotations

import math
from collections import defaultdict

import numpy as np
from scipy.spatial import ConvexHull, Delaunay

UP_AXIS = 1


def chain_of_bone(bone_name: str) -> str | None:
    """`hair_03_02` -> `hair_03`; a bone without the `<role>_<column>_<joint>` shape has no chain."""
    parts = bone_name.rsplit("_", 2)
    if len(parts) != 3 or not (parts[1].isdigit() and parts[2].isdigit()):
        return None
    return f"{parts[0]}_{parts[1]}"


def group_chains(bones: list[dict], scale: float) -> dict[str, dict]:
    """Per chain: joint heads in order, the tail, and the mean joint spacing."""
    chains: dict[str, dict] = {}
    for bone in bones:
        chain = chain_of_bone(bone["name"])
        if chain is None:
            continue
        entry = chains.setdefault(chain, {"heads": [], "tail": None})
        entry["heads"].append(np.asarray(bone["head"], dtype=float) * scale)
        if bone.get("tail") is not None:
            entry["tail"] = np.asarray(bone["tail"], dtype=float) * scale

    for entry in chains.values():
        points = entry["heads"] + ([entry["tail"]] if entry["tail"] is not None else [])
        segments = [np.linalg.norm(b - a) for a, b in zip(points, points[1:])]
        entry["joint_length"] = float(np.mean(segments)) if segments else 0.0
    return chains


def group_vertices(vertex_weights: list[dict], scale: float) -> dict[tuple[str, int], dict]:
    """Per (mesh, vertex): position, component and the weight of every bone."""
    vertices: dict[tuple[str, int], dict] = {}
    for entry in vertex_weights:
        key = (entry["mesh"], entry["vertex"])
        record = vertices.setdefault(
            key,
            {
                "position": np.asarray(entry["position"], dtype=float) * scale,
                "component": (entry["mesh"], entry.get("component", entry["vertex"])),
                "weights": defaultdict(float),
            },
        )
        record["weights"][entry["bone"]] += entry["weight"]
    return vertices


def dominant_chain(weights: dict[str, float]) -> str | None:
    """The chain of the heaviest bone, or None when the heaviest bone is the root."""
    bone = max(weights, key=weights.get)
    return chain_of_bone(bone)


def is_root_only(weights: dict[str, float], tolerance: float) -> bool:
    chain_weight = sum(w for bone, w in weights.items() if chain_of_bone(bone) is not None)
    return chain_weight <= tolerance


def skull_intrusions(chains: dict[str, dict], attachment: np.ndarray) -> dict:
    """Joint heads strictly inside the convex hull of the skin the root bone drives."""
    if len(attachment) < 4:
        return {"skull_intrusion_count": None, "skull_intrusion_joints": []}
    hull = Delaunay(attachment[ConvexHull(attachment).vertices])
    intruding = []
    for chain, entry in chains.items():
        for joint, head in enumerate(entry["heads"]):
            if hull.find_simplex(head[None, :])[0] >= 0:
                intruding.append(f"{chain}_{joint:02d}")
    return {"skull_intrusion_count": len(intruding), "skull_intrusion_joints": intruding}


def shell_rigidity(vertices: dict, band_height: float, tolerance: float) -> dict:
    """Share of vertices above the root band whose weight is the root bone alone."""
    above = [v for v in vertices.values() if v["position"][UP_AXIS] > band_height]
    rigid = sum(1 for v in above if is_root_only(v["weights"], tolerance))
    return {
        "shell_vertex_count": len(above),
        "shell_rigid_ratio": rigid / len(above) if above else None,
    }


def component_splits(vertices: dict) -> dict:
    """Connected components whose chained vertices are dominated by more than one chain."""
    chains_by_component: dict[tuple, set[str]] = defaultdict(set)
    for v in vertices.values():
        chain = dominant_chain(v["weights"])
        if chain is not None:
            chains_by_component[v["component"]].add(chain)
    split = [key for key, chains in chains_by_component.items() if len(chains) > 1]
    return {
        "chained_component_count": len(chains_by_component),
        "component_split_count": len(split),
    }


def tail_reach(chains: dict[str, dict], vertices: dict) -> dict:
    """Chains whose tail ends within one joint length (along up) of the lowest vertex they dominate."""
    lowest: dict[str, float] = {}
    for v in vertices.values():
        chain = dominant_chain(v["weights"])
        if chain is None:
            continue
        height = v["position"][UP_AXIS]
        lowest[chain] = min(lowest.get(chain, height), height)

    gaps = {}
    for chain, entry in chains.items():
        if entry["tail"] is None or chain not in lowest:
            continue
        gaps[chain] = float(entry["tail"][UP_AXIS] - lowest[chain])
    reached = sum(1 for chain, gap in gaps.items() if gap <= chains[chain]["joint_length"])
    return {
        "tail_reach_ratio": reached / len(gaps) if gaps else None,
        "tail_gap_max": max(gaps.values()) if gaps else None,
        "tail_gaps": gaps,
    }


def root_height(chains: dict[str, dict], equator_height: float) -> dict:
    """First joint height relative to the equator; the design expects it to equal `root_band_offset`."""
    diffs = [float(entry["heads"][0][UP_AXIS] - equator_height) for entry in chains.values() if entry["heads"]]
    if not diffs:
        return {"root_height_diff_mean": None, "root_height_diff_max_abs": None}
    return {
        "root_height_diff_mean": float(np.mean(diffs)),
        "root_height_diff_max_abs": float(np.max(np.abs(diffs))),
    }


def empty_columns(chains: dict[str, dict], vertices: dict, min_fraction: float) -> dict:
    """Chains that dominate fewer than `min_fraction` of all chained vertices."""
    counts = {chain: 0 for chain in chains}
    for v in vertices.values():
        chain = dominant_chain(v["weights"])
        if chain in counts:
            counts[chain] += 1
    total = sum(counts.values())
    empty = [chain for chain, count in counts.items() if total == 0 or count < min_fraction * total]
    return {
        "chain_count": len(chains),
        "chained_vertex_count": total,
        "empty_column_count": len(empty),
        "empty_columns": empty,
        "column_vertex_counts": counts,
    }


def quality_metrics(chain_gen: dict, scale: float = 1.0, empty_fraction: float = 0.05,
                    root_only_tolerance: float = 1e-6) -> dict:
    """Every reference-free metric of one chain_gen report; None marks a metric its inputs cannot give."""
    chains = group_chains(chain_gen.get("bones", []), scale)
    vertices = group_vertices(chain_gen.get("vertex_weights", []), scale)
    attachment = np.asarray(chain_gen.get("attachment_positions", []), dtype=float).reshape(-1, 3) * scale
    band = chain_gen.get("root_band")

    out = {}
    out.update(skull_intrusions(chains, attachment))
    out.update(component_splits(vertices))
    out.update(tail_reach(chains, vertices))
    out.update(empty_columns(chains, vertices, empty_fraction))
    if band is not None:
        out.update(shell_rigidity(vertices, band["height"] * scale, root_only_tolerance))
        out.update(root_height(chains, band["equator_height"] * scale))
        out["root_band_height"] = band["height"] * scale
        out["equator_height"] = band["equator_height"] * scale
    else:
        out.update({"shell_vertex_count": None, "shell_rigid_ratio": None,
                    "root_height_diff_mean": None, "root_height_diff_max_abs": None})
    return out


SUMMARY_KEYS = (
    "skull_intrusion_count",
    "shell_rigid_ratio",
    "component_split_count",
    "tail_reach_ratio",
    "tail_gap_max",
    "root_height_diff_mean",
    "empty_column_count",
    "chain_count",
    "chained_vertex_count",
    "shell_vertex_count",
    "chained_component_count",
)


def format_value(value) -> str:
    if value is None:
        return "n/a"
    if isinstance(value, float):
        return f"{value:.4f}" if not math.isnan(value) else "nan"
    return str(value)


def quality_markdown_rows(metrics: dict) -> list[str]:
    return [f"| {key} | {format_value(metrics.get(key))} |" for key in SUMMARY_KEYS]
