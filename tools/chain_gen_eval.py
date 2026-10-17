"""Measure generated chain bones against the secondary bones of a hand-rigged model.

    python3 tools/chain_gen_eval.py --reference <secondary_reference.json> --generated <anim_dump.json> --out <dir>
        [--reference-scale 1.0] [--generated-scale 1.0] [--miss-radius 0.03] [--match-tolerance 0.002]
        [--deform-generated <csv> --deform-reference <csv>]

The reference is written by scripts/blender/strip_secondary_bones.py in engine coordinates (Y-up, "space"
field); this tool knows no other space. The scales bring both sides to meters. Writes <out>/chain_gen_eval.json and chain_gen_eval.md with
joint, weight and (when both CSVs are given) deformation metrics. It reports numbers only; the thresholds are
the user's to set.

Procedure:
1. Run strip_secondary_bones.py on the purchased model (the user runs it) to get <name>_humanoid_only.fbx and
    <name>_secondary_reference.json.
2. Load the copy in the engine with submesh_role=<role>:<mesh>[/<material>] and
    generate_chain_bones=<mesh>[/<material>],<root_bone>[,<setting>=<value>...], and keep --batch-anim-dump.
    Without the material the whole mesh is one generation target.
3. For the deformation RMS, give both models the same spring values and colliders and the same sway clip,
    and take dump_skinned_vertices=<csv>,30,<mesh>/<material> from each.
4. Run this tool on the outputs.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sys
from pathlib import Path

import numpy as np


def reference_chains(deleted_bones: list[dict]) -> list[list[str]]:
    """Build chains from reference deleted_bones.

    Only kind=="chain" bones are included. A chain starts at a bone whose parent is not a chain bone or
    is a hub with several chain children (Hair_root -> 22 strands), and follows single children to the
    tip or the next hub. Hubs themselves are not part of any chain.
    Returns list of chains, each chain is a list of bone names in order from root to tip.
    """
    chain_bones = [b for b in deleted_bones if b.get("kind") == "chain"]
    bone_by_name = {b["name"]: b for b in chain_bones}
    children_by_parent: dict[str, list[str]] = {}
    for b in chain_bones:
        children_by_parent.setdefault(b.get("parent"), []).append(b["name"])

    def is_hub(name: str) -> bool:
        return len(children_by_parent.get(name, [])) > 1

    heads = [
        b["name"]
        for b in chain_bones
        if b.get("parent") not in bone_by_name or is_hub(b["parent"])
    ]

    chains = []
    for head_name in heads:
        chain = [head_name]
        while not is_hub(chain[-1]):
            children = children_by_parent.get(chain[-1], [])
            if not children:
                break
            chain.append(children[0])
        if len(chain) >= 2:
            chains.append(chain)

    return chains


def generated_chains(bones: list[dict]) -> list[list[str]]:
    """Build chains from generated bones.

    Groups bones by role_column (from name format <role>_<column>_<joint>) and sorts by joint number.
    Returns list of chains, each chain is a list of bone names in order from root to tip.
    """
    groups: dict[tuple[str, str], list[tuple[int, str]]] = {}
    for b in bones:
        name = b["name"]
        parts = name.split("_")
        if len(parts) < 3:
            continue
        role = parts[0]
        column = parts[1]
        try:
            joint_num = int(parts[2])
        except ValueError:
            continue
        key = (role, column)
        groups.setdefault(key, []).append((joint_num, name))

    chains = []
    for key in sorted(groups.keys()):
        entries = sorted(groups[key], key=lambda e: e[0])
        chain = [e[1] for e in entries]
        if len(chain) >= 1:
            chains.append(chain)

    return chains


def chain_heads(chains: list[list[str]], head_by_name: dict[str, tuple]) -> list[list[tuple]]:
    """Convert name chains to position chains using a bone name -> head position lookup."""
    return [[head_by_name[name] for name in chain] for chain in chains]


def _dist(a: tuple, b: tuple) -> float:
    return math.sqrt(sum((x - y) ** 2 for x, y in zip(a, b)))


REFERENCE_SPACE = "engine_y_up_m"


def scale_position(position: list, scale: float) -> tuple:
    """Scale an engine Y-up position by the given factor."""
    return (position[0] * scale, position[1] * scale, position[2] * scale)


def nearest_indices(query: np.ndarray, points: np.ndarray, chunk: int = 1024) -> tuple[np.ndarray, np.ndarray]:
    """For each row in query, find the index of the nearest row in points by Euclidean distance.

    Returns (indices, distances) where indices[i] is the index of the nearest point to query[i],
    and distances[i] is the corresponding distance. Processes query in chunks to avoid large
    intermediate arrays."""
    n = len(query)
    indices = np.empty(n, dtype=np.intp)
    distances = np.empty(n, dtype=np.float64)
    for start in range(0, n, chunk):
        end = min(start + chunk, n)
        q = query[start:end]
        dists = ((q[:, None, :] - points[None, :, :]) ** 2).sum(-1)
        best = np.argmin(dists, axis=1)
        indices[start:end] = best
        distances[start:end] = np.sqrt(np.min(dists, axis=1))
    return indices, distances


def group_vertex_weights(entries: list[dict]) -> tuple[list[tuple[str, int]], list[dict[str, float]], np.ndarray]:
    """Group vertex weight entries by (mesh, vertex).

    Returns (keys, weights_list, positions_array) where keys[i] is the (mesh, vertex) key,
    weights_list[i] is a dict of bone->weight, and positions_array[i] is the position as a numpy array."""
    data: dict[tuple[str, int], tuple[dict[str, float], list[float]]] = {}
    for entry in entries:
        key = (entry["mesh"], entry["vertex"])
        if key not in data:
            data[key] = ({}, list(entry["position"]))
        data[key][0][entry["bone"]] = entry["weight"]
    keys = list(data.keys())
    weights_list = [data[k][0] for k in keys]
    positions_array = np.array([data[k][1] for k in keys], dtype=np.float64)
    return keys, weights_list, positions_array


def dominant_chain(weights: dict[str, float], bone_to_chain: dict[str, int]) -> int | None:
    """Find the chain with the highest total weight."""
    best_chain = -1
    max_weight = -1.0
    for bone, weight in weights.items():
        if bone in bone_to_chain and weight > max_weight:
            max_weight = weight
            best_chain = bone_to_chain[bone]
    return best_chain if best_chain >= 0 else None


def weight_metrics(gen_weights: list[dict], ref_weights: list[dict], correspondence: list[dict],
                   gen_chain_names: list[list[str]], ref_chain_bones: list[list[str]],
                   match_tolerance: float = 0.002) -> dict:
    """Compute weight-level metrics between generated and reference vertex weights.

    Matches generated vertices to reference vertices by nearest neighbor within tolerance,
    computes dominant chain agreement and arc weight L1 for matched vertices.
    """
    if not gen_weights:
        return {
            "matched_vertices": 0,
            "unmatched_vertices": 0,
            "dominant_chain_agreement": 0.0,
            "arc_weight_l1": 0.0,
        }

    corr_map = {c["gen_chain"]: c["matched_ref_chain"] for c in correspondence}

    ref_bone_to_chain: dict[str, int] = {}
    for ci, bones in enumerate(ref_chain_bones):
        for bone in bones:
            ref_bone_to_chain[bone] = ci

    gen_bone_to_chain: dict[str, int] = {}
    for ci, names in enumerate(gen_chain_names):
        for name in names:
            gen_bone_to_chain[name] = ci

    ref_keys, ref_weights_list, ref_positions = group_vertex_weights(ref_weights)
    gen_keys, gen_weights_list, gen_positions = group_vertex_weights(gen_weights)

    best_indices, best_dists = nearest_indices(gen_positions, ref_positions)

    matched_count = 0
    unmatched_count = 0
    agreement_count = 0
    total_l1 = 0.0
    l1_count = 0

    for i, (gen_weights_dict, dist) in enumerate(zip(gen_weights_list, best_dists)):
        if dist > match_tolerance:
            unmatched_count += 1
            continue

        matched_count += 1
        ref_weights_dict = ref_weights_list[best_indices[i]]

        gen_dc = dominant_chain(gen_weights_dict, gen_bone_to_chain)
        if gen_dc is None:
            continue

        ref_chain_i = corr_map.get(gen_dc)
        if ref_chain_i is None:
            continue

        ref_dc = dominant_chain(ref_weights_dict, ref_bone_to_chain)
        if ref_dc is None:
            continue

        if ref_chain_i == ref_dc:
            agreement_count += 1

            gen_t = _compute_arc_position(gen_weights_dict, gen_bone_to_chain, gen_dc, gen_chain_names)
            ref_t = _compute_arc_position(ref_weights_dict, ref_bone_to_chain, ref_dc, ref_chain_bones)
            if gen_t is not None and ref_t is not None:
                total_l1 += abs(gen_t - ref_t)
                l1_count += 1

    dominant_chain_agreement = agreement_count / matched_count if matched_count > 0 else 0.0
    arc_weight_l1 = total_l1 / l1_count if l1_count > 0 else 0.0

    return {
        "matched_vertices": matched_count,
        "unmatched_vertices": unmatched_count,
        "dominant_chain_agreement": round(dominant_chain_agreement, 6),
        "arc_weight_l1": round(arc_weight_l1, 6),
    }


def _compute_arc_position(weights: dict[str, float], bone_to_chain: dict[str, int],
                          chain_id: int, chain_bones: list[list[str]]) -> float | None:
    """Compute arc position t = Σ w_j·(j/(n-1)) / Σ w_j for bones in a chain."""
    bones = chain_bones[chain_id]
    if len(bones) <= 1:
        return 0.0

    n = len(bones)
    total_weight = 0.0
    weighted_sum = 0.0
    for j, bone in enumerate(bones):
        w = weights.get(bone, 0.0)
        if w > 0:
            weighted_sum += w * (j / (n - 1))
            total_weight += w

    if total_weight == 0:
        return None
    return weighted_sum / total_weight


def deform_metrics(gen_csv_rows: list[list], ref_csv_rows: list[list], match_tolerance: float = 0.002,
                   gen_scale: float = 1.0, ref_scale: float = 1.0) -> dict:
    """Compute deformation metrics between generated and reference skinned vertex CSVs.

    Matches vertices by bind position nearest neighbor within tolerance,
    computes RMS and max of position differences across all frames.
    gen_scale and ref_scale are applied to the bind positions and deformed positions respectively."""
    if not gen_csv_rows or not ref_csv_rows:
        return {"rms_error": 0.0, "max_error": 0.0, "matched_vertices": 0}

    ref_bind: dict[int, tuple] = {}
    for row in ref_csv_rows:
        frame, vertex = int(row[0]), int(row[1])
        if vertex not in ref_bind:
            ref_bind[vertex] = (float(row[2]) * ref_scale, float(row[3]) * ref_scale, float(row[4]) * ref_scale)

    gen_bind: dict[int, tuple] = {}
    for row in gen_csv_rows:
        frame, vertex = int(row[0]), int(row[1])
        if vertex not in gen_bind:
            gen_bind[vertex] = (float(row[2]) * gen_scale, float(row[3]) * gen_scale, float(row[4]) * gen_scale)

    ref_bind_keys = list(ref_bind.keys())
    ref_positions = np.array(list(ref_bind.values()), dtype=np.float64)
    gen_bind_keys = list(gen_bind.keys())
    gen_positions = np.array(list(gen_bind.values()), dtype=np.float64)

    best_indices, best_dists = nearest_indices(gen_positions, ref_positions)

    matches: dict[int, int] = {}
    for i, dist in enumerate(best_dists):
        if dist <= match_tolerance:
            matches[gen_bind_keys[i]] = ref_bind_keys[best_indices[i]]

    if not matches:
        return {"rms_error": 0.0, "max_error": 0.0, "matched_vertices": 0}

    gen_positions: dict[int, dict[int, tuple]] = {}
    for row in gen_csv_rows:
        frame, vertex = int(row[0]), int(row[1])
        gen_positions.setdefault(frame, {})[vertex] = (float(row[5]) * gen_scale, float(row[6]) * gen_scale, float(row[7]) * gen_scale)

    ref_positions_map: dict[int, dict[int, tuple]] = {}
    for row in ref_csv_rows:
        frame, vertex = int(row[0]), int(row[1])
        ref_positions_map.setdefault(frame, {})[vertex] = (float(row[5]) * ref_scale, float(row[6]) * ref_scale, float(row[7]) * ref_scale)

    squared_errors = []
    max_error = 0.0
    for frame in gen_positions:
        if frame not in ref_positions_map:
            continue
        for gen_vertex, ref_vertex in matches.items():
            if gen_vertex in gen_positions[frame] and ref_vertex in ref_positions_map[frame]:
                g_pos = gen_positions[frame][gen_vertex]
                r_pos = ref_positions_map[frame][ref_vertex]
                error = _dist(g_pos, r_pos)
                squared_errors.append(error * error)
                if error > max_error:
                    max_error = error

    rms_error = math.sqrt(sum(squared_errors) / len(squared_errors)) if squared_errors else 0.0

    return {
        "rms_error": round(rms_error, 6),
        "max_error": round(max_error, 6),
        "matched_vertices": len(matches),
    }


def joint_metrics(gen_chains: list[list[tuple]], ref_chains: list[list[tuple]], miss_radius: float = 0.03) -> dict:
    """Compute joint-level metrics between generated and reference chains.

    Returns a dict with:
      - ref_chain_count, gen_chain_count, chain_count_diff
      - rms_distance: RMS of nearest-neighbor distances from each generated joint to reference joints
      - max_distance: maximum nearest-neighbor distance
      - miss_rate: fraction of reference joints without a generated joint within miss_radius
      - chain_correspondence: per generated chain, the best-matching reference chain by nearest-neighbor vote
    """
    all_ref_joints = []
    for i, chain in enumerate(ref_chains):
        for j in chain:
            all_ref_joints.append((i, j))

    all_gen_joints = []
    for i, chain in enumerate(gen_chains):
        for j in chain:
            all_gen_joints.append((i, j))

    ref_chain_count = len(ref_chains)
    gen_chain_count = len(gen_chains)
    chain_count_diff = gen_chain_count - ref_chain_count

    if not all_gen_joints or not all_ref_joints:
        return {
            "ref_chain_count": ref_chain_count,
            "gen_chain_count": gen_chain_count,
            "chain_count_diff": chain_count_diff,
            "rms_distance": 0.0,
            "max_distance": 0.0,
            "miss_rate": 1.0 if all_ref_joints else 0.0,
            "chain_correspondence": [],
        }

    distances = []
    for gen_chain_i, gen_joint in all_gen_joints:
        best_dist = min(_dist(gen_joint, rj) for _, rj in all_ref_joints)
        distances.append(best_dist)

    rms_distance = math.sqrt(sum(d * d for d in distances) / len(distances))
    max_distance = max(distances)

    missed = 0
    for ref_chain_i, ref_joint in all_ref_joints:
        best_dist = min(_dist(ref_joint, gj) for _, gj in all_gen_joints)
        if best_dist > miss_radius:
            missed += 1
    miss_rate = missed / len(all_ref_joints)

    chain_correspondence = []
    for gen_chain_i, gen_chain in enumerate(gen_chains):
        votes: dict[int, int] = {}
        for gj in gen_chain:
            best_ref_chain_i = -1
            best_dist = float("inf")
            for ref_chain_i, ref_chain in enumerate(ref_chains):
                for rj in ref_chain:
                    d = _dist(gj, rj)
                    if d < best_dist:
                        best_dist = d
                        best_ref_chain_i = ref_chain_i
            votes[best_ref_chain_i] = votes.get(best_ref_chain_i, 0) + 1
        if votes:
            best_match = max(votes, key=votes.get)
            chain_correspondence.append({
                "gen_chain": gen_chain_i,
                "matched_ref_chain": best_match,
                "vote_count": votes[best_match],
            })

    return {
        "ref_chain_count": ref_chain_count,
        "gen_chain_count": gen_chain_count,
        "chain_count_diff": chain_count_diff,
        "rms_distance": round(rms_distance, 6),
        "max_distance": round(max_distance, 6),
        "miss_rate": round(miss_rate, 6),
        "chain_correspondence": chain_correspondence,
    }


def generated_mesh_names(gen_weights: list[dict]) -> set[str]:
    """Mesh names the generation wrote weights to; the engine labels a part as <mesh>:<material>."""
    return {entry["mesh"].split(":", 1)[0] for entry in gen_weights}


def generated_root_bone(gen_bones: list[dict]) -> str | None:
    """The humanoid bone the generated chains hang from: the one parent that is not a generated bone."""
    generated = {b["name"] for b in gen_bones}
    roots = {b["parent"] for b in gen_bones if b.get("parent") not in generated}
    return next(iter(roots)) if len(roots) == 1 else None


def humanoid_ancestor(bone_name: str, bone_by_name: dict[str, dict]) -> str | None:
    """Walk up the deleted bones to the first parent that is not a deleted bone."""
    current = bone_name
    while current in bone_by_name:
        current = bone_by_name[current].get("parent")
    return current


def reference_for_meshes(ref_data: dict, mesh_names: set[str], root_bone: str | None = None) -> tuple[list[dict], list[dict]]:
    """Restrict the reference to the hand-rigged bones that weight the given meshes and hang from root_bone.

    Returns (deleted_bones, vertex_weights). Chains whose bones never touch those meshes, or that hang
    from another humanoid bone (breast bones weighting a dress), belong to another garment and would only
    count as misses. Without a mesh overlap the whole reference is used."""
    all_bones = ref_data.get("deleted_bones", [])
    all_weights = ref_data.get("vertex_weights", [])
    mesh_weights = [entry for entry in all_weights if entry["mesh"] in mesh_names]
    if not mesh_weights:
        return all_bones, all_weights

    bone_by_name = {b["name"]: b for b in all_bones}
    touched = {entry["bone"] for entry in mesh_weights}
    under_root = {name for name in touched if humanoid_ancestor(name, bone_by_name) == root_bone}
    if under_root:
        touched = under_root
    kept = set()
    for name in touched:
        current = name
        while current in bone_by_name and current not in kept:
            kept.add(current)
            current = bone_by_name[current].get("parent")
    bones = [b for b in all_bones if b["name"] in kept]
    weights = [entry for entry in mesh_weights if entry["bone"] in kept]
    return bones, weights


def evaluate(ref_data: dict, gen_data: dict, reference_scale: float = 1.0, generated_scale: float = 1.0,
             miss_radius: float = 0.03, match_tolerance: float = 0.002,
             deform_generated: str | None = None, deform_reference: str | None = None) -> dict:
    """Evaluate chain generation accuracy against reference data."""
    if ref_data.get("space") != REFERENCE_SPACE:
        raise ValueError(
            f"reference JSON must declare space={REFERENCE_SPACE!r} (written by strip_secondary_bones.py), "
            f"got {ref_data.get('space')!r}"
        )
    chain_gen = gen_data.get("chain_gen", {})
    gen_bones = chain_gen.get("bones", [])
    gen_weights = chain_gen.get("vertex_weights", [])

    deleted_bones, ref_weights = reference_for_meshes(
        ref_data, generated_mesh_names(gen_weights), generated_root_bone(gen_bones)
    )

    ref_chains = reference_chains(deleted_bones)
    ref_head_lookup = {b["name"]: scale_position(b["head"], reference_scale) for b in deleted_bones}
    ref_chains_yup = chain_heads(ref_chains, ref_head_lookup)

    gen_bones_scaled = [{"name": b["name"], "parent": b.get("parent", ""), "head": scale_position(b["head"], generated_scale)} for b in gen_bones]
    gen_chains = generated_chains(gen_bones_scaled)
    gen_head_lookup = {b["name"]: tuple(b["head"]) for b in gen_bones_scaled}
    gen_chains_yup = chain_heads(gen_chains, gen_head_lookup)

    joint_m = joint_metrics(gen_chains_yup, ref_chains_yup, miss_radius)

    transformed_ref_weights = [
        {**entry, "position": list(scale_position(entry["position"], reference_scale))}
        for entry in ref_weights
    ]
    transformed_gen_weights = [
        {**entry, "position": list(scale_position(entry["position"], generated_scale))}
        for entry in gen_weights
    ]
    weight_m = weight_metrics(transformed_gen_weights, transformed_ref_weights, joint_m["chain_correspondence"],
                              gen_chains, ref_chains, match_tolerance)

    deform_m = None
    if deform_generated is not None and deform_reference is not None:
        gen_rows = [line.strip().split(",") for line in Path(deform_generated).read_text().strip().split("\n") if line.strip()]
        ref_rows = [line.strip().split(",") for line in Path(deform_reference).read_text().strip().split("\n") if line.strip()]
        deform_m = deform_metrics(gen_rows, ref_rows, match_tolerance,
                                  gen_scale=generated_scale, ref_scale=reference_scale)

    output = {"joint": joint_m, "weight": weight_m}
    if deform_m is not None:
        output["deform"] = deform_m
    return output


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Measure chain bone generation accuracy against reference."
    )
    parser.add_argument("--reference", required=True, help="Path to reference JSON (S1 strip_secondary_bones.py output)")
    parser.add_argument("--generated", required=True, help="Path to generated JSON (engine --batch-anim-dump output)")
    parser.add_argument("--out", required=True, help="Output directory")
    parser.add_argument("--reference-scale", type=float, default=1.0, help="Scale factor for reference positions (default 1.0)")
    parser.add_argument("--generated-scale", type=float, default=1.0, help="Scale factor for generated positions (default 1.0)")
    parser.add_argument("--miss-radius", type=float, default=0.03, help="Radius in meters for miss detection (default 0.03)")
    parser.add_argument("--match-tolerance", type=float, default=0.002, help="Tolerance for vertex matching in weight/deform metrics (default 0.002)")
    parser.add_argument("--deform-generated", help="Path to generated skinned vertices CSV (for deformation metrics)")
    parser.add_argument("--deform-reference", help="Path to reference skinned vertices CSV (for deformation metrics)")
    args = parser.parse_args()

    ref_data = json.loads(Path(args.reference).read_text())
    gen_data = json.loads(Path(args.generated).read_text())

    output = evaluate(ref_data, gen_data, args.reference_scale, args.generated_scale,
                      args.miss_radius, args.match_tolerance, args.deform_generated, args.deform_reference)

    joint_m = output["joint"]
    weight_m = output["weight"]
    deform_m = output.get("deform")

    os.makedirs(args.out, exist_ok=True)
    out_path = os.path.join(args.out, "chain_gen_eval.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False, sort_keys=True)

    md_path = os.path.join(args.out, "chain_gen_eval.md")
    with open(md_path, "w", encoding="utf-8") as f:
        f.write("# Chain Generation Evaluation\n\n")
        f.write("## Joint Metrics\n\n")
        f.write("| Metric | Value |\n")
        f.write("|--------|-------|\n")
        for k, v in sorted(joint_m.items()):
            if k != "chain_correspondence":
                f.write(f"| {k} | {v} |\n")
        f.write("\n## Weight Metrics\n\n")
        f.write("| Metric | Value |\n")
        f.write("|--------|-------|\n")
        for k, v in sorted(weight_m.items()):
            f.write(f"| {k} | {v} |\n")
        if deform_m is not None:
            f.write("\n## Deformation Metrics\n\n")
            f.write("| Metric | Value |\n")
            f.write("|--------|-------|\n")
            for k, v in sorted(deform_m.items()):
                f.write(f"| {k} | {v} |\n")

    print(json.dumps(output))


if __name__ == "__main__":
    main()
