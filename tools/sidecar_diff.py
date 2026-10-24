"""Diff two avatar sidecar JSON files and report mismatches.

Usage:
    python tools/sidecar_diff.py <engine.avatar.json> <addon.avatar.json>

Compares spring.chains, spring.colliders (fitted only), submesh_roles, and humanoid.
Prints one line per mismatch; exits 0 if identical, 1 otherwise."""
from __future__ import annotations

import json
import math
import sys


def diff_sidecars(engine: dict, addon: dict, collider_tolerance: float = 1e-3) -> list[str]:
    """Compare two avatar sidecar dicts and return a list of mismatch descriptions.

    Empty list means they match."""
    diffs: list[str] = []
    diffs.extend(_diff_chains(engine.get("spring", {}).get("chains", []),
                              addon.get("spring", {}).get("chains", [])))
    diffs.extend(_diff_colliders(engine.get("spring", {}).get("colliders", []),
                                 addon.get("spring", {}).get("colliders", []),
                                 collider_tolerance))
    diffs.extend(_diff_submesh_roles(engine.get("submesh_roles", []),
                                     addon.get("submesh_roles", [])))
    diffs.extend(_diff_humanoid(engine.get("humanoid", {}),
                                addon.get("humanoid", {})))
    return diffs


def _diff_chains(engine_chains: list[dict], addon_chains: list[dict]) -> list[str]:
    """Compare chains by name: bone order must match, 5 values compared with math.isclose."""
    engine_by_name = {c["name"]: c for c in engine_chains}
    addon_by_name = {c["name"]: c for c in addon_chains}

    all_names = sorted(set(engine_by_name) | set(addon_by_name))
    diffs: list[str] = []

    for name in all_names:
        ec = engine_by_name.get(name)
        ac = addon_by_name.get(name)
        if ec is None:
            diffs.append(f"spring.chains: chain {name!r} missing in engine")
            continue
        if ac is None:
            diffs.append(f"spring.chains: chain {name!r} missing in addon")
            continue

        ej = [j["bone"] for j in ec["joints"]]
        aj = [j["bone"] for j in ac["joints"]]
        if ej != aj:
            diffs.append(f"spring.chains[{name!r}]: bone order mismatch engine={ej} addon={aj}")
            continue

        for i, (ejt, ajt) in enumerate(zip(ec["joints"], ac["joints"])):
            v = _diff_joint_values(ejt, ajt)
            if v:
                diffs.append(f"spring.chains[{name!r}].joints[{i}]({ejt['bone']}): {v}")

    return diffs


def _diff_joint_values(engine: dict, addon: dict) -> str | None:
    """Compare 5 joint values with math.isclose; return description of first mismatch or None."""
    for key in ("stiffness", "drag_force", "gravity_power", "hit_radius"):
        ev = float(engine[key])
        av = float(addon[key])
        if not math.isclose(ev, av, rel_tol=1e-6, abs_tol=1e-6):
            return f"{key} engine={ev} addon={av}"

    eg = list(engine["gravity_dir"])
    ag = list(addon["gravity_dir"])
    for i in range(3):
        if not math.isclose(float(eg[i]), float(ag[i]), rel_tol=1e-6, abs_tol=1e-6):
            return f"gravity_dir[{i}] engine={eg[i]} addon={ag[i]}"

    return None


def _diff_colliders(engine_colliders: list[dict], addon_colliders: list[dict],
                    tolerance: float) -> list[str]:
    """Compare fitted colliders (origin == 'fitted') by bone set and per-bone radius.

    offset / tail are not compared — they use different local coordinate systems."""
    engine_fitted = [c for c in engine_colliders if c.get("origin") == "fitted"]
    addon_fitted = [c for c in addon_colliders if c.get("origin") == "fitted"]

    ebones = {c["bone"] for c in engine_fitted}
    abones = {c["bone"] for c in addon_fitted}

    diffs: list[str] = []
    if ebones != abones:
        missing_in_addon = ebones - abones
        missing_in_engine = abones - ebones
        parts = []
        if missing_in_addon:
            parts.append(f"missing in addon: {sorted(missing_in_addon)}")
        if missing_in_engine:
            parts.append(f"missing in engine: {sorted(missing_in_engine)}")
        diffs.append(f"spring.colliders (fitted): bone set mismatch — {'; '.join(parts)}")
        return diffs

    for bone in sorted(ebones):
        ec = next(c for c in engine_fitted if c["bone"] == bone)
        ac = next(c for c in addon_fitted if c["bone"] == bone)
        er = float(ec["radius"])
        ar = float(ac["radius"])
        if abs(er - ar) > tolerance:
            diffs.append(f"spring.colliders (fitted)[{bone}]: radius engine={er} addon={ar} "
                         f"(diff={abs(er - ar):.6f} > {tolerance})")

    return diffs


def _diff_submesh_roles(engine_roles: list[dict], addon_roles: list[dict]) -> list[str]:
    """Compare (material, role) sets — mesh name is not compared."""
    engine_set = {(r["material"], r["role"]) for r in engine_roles}
    addon_set = {(r["material"], r["role"]) for r in addon_roles}

    diffs: list[str] = []
    missing_in_addon = engine_set - addon_set
    missing_in_engine = addon_set - engine_set
    if missing_in_addon or missing_in_engine:
        parts = []
        if missing_in_addon:
            parts.append(f"missing in addon: {sorted(missing_in_addon)}")
        if missing_in_engine:
            parts.append(f"missing in engine: {sorted(missing_in_engine)}")
        diffs.append(f"submesh_roles: (material, role) set mismatch — {'; '.join(parts)}")
    return diffs


def _diff_humanoid(engine_humanoid: dict, addon_humanoid: dict) -> list[str]:
    """Compare humanoid overrides for exact match."""
    if engine_humanoid == addon_humanoid:
        return []

    diffs: list[str] = []
    all_roles = sorted(set(engine_humanoid) | set(addon_humanoid))
    for role in all_roles:
        ev = engine_humanoid.get(role)
        av = addon_humanoid.get(role)
        if ev != av:
            diffs.append(f"humanoid[{role!r}]: engine={ev!r} addon={av!r}")
    return diffs


def main() -> None:
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <engine.avatar.json> <addon.avatar.json>", file=sys.stderr)
        sys.exit(1)

    with open(sys.argv[1]) as f:
        engine = json.load(f)
    with open(sys.argv[2]) as f:
        addon = json.load(f)

    diffs = diff_sidecars(engine, addon)
    for d in diffs:
        print(d)

    if diffs:
        sys.exit(1)


if __name__ == "__main__":
    main()
