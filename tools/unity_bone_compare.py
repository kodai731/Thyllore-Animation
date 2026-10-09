"""Compare engine bone debug dump against Unity pose output.

For each time and bone, computes the max absolute difference of the 3 world_position components
between the engine dump and Unity's output (with Unity x negated for left-hand to right-hand
coordinate conversion). Reports the ratio of that difference to Hips' y position (hips height)
for each bone. Both humanoid bones (with a non-null role) and extra bones (role=null) count toward
the pass/fail verdict.

    uv run python3 tools/unity_bone_compare.py <engine_dump.json> <unity_pose.json> [--tolerance 0.01]

Exit code 0 = ok (humanoid worst ratio <= tolerance), 1 = fail.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def compare_bone_positions(engine_dump: dict, unity_pose: dict, tolerance: float) -> dict:
    """Compare engine dump against Unity pose JSON.

    Returns a dict with keys: ok, humanoid_max_ratio, humanoid_worst_bone,
    missing_humanoid_bones, extra_max_ratio, extra_worst_bone, times.
    """
    engine_lookup: dict[float, dict[str, tuple]] = {}
    role_map: dict[str, str | None] = {}
    hips_y: float = 1.0

    for pose in engine_dump["poses"]:
        t = pose["time"]
        bone_dict: dict[str, tuple] = {}
        for b in pose["bones"]:
            name = b["name"]
            wp = tuple(b["world_position"])
            bone_dict[name] = wp
            if name not in role_map:
                role_map[name] = b.get("role")
            if b.get("role") == "Hips":
                hips_y = wp[1]

        engine_lookup[t] = bone_dict

    unity_lookup: dict[float, dict[str, list]] = {}
    for pose in unity_pose["poses"]:
        t = float(pose["time"])
        unity_lookup[t] = pose["bones"]

    times = sorted(engine_lookup.keys())

    humanoid_bone_names: set[str] = {name for name, role in role_map.items() if role is not None}

    humanoid_max_ratio = 0.0
    humanoid_worst_bone = ""
    extra_max_ratio = 0.0
    extra_worst_bone = ""
    missing_humanoid_bones: set[str] = set()

    for t in times:
        eng_bones = engine_lookup[t]
        uni_bones = unity_lookup.get(t, {})

        if not uni_bones and t not in unity_lookup:
            missing_humanoid_bones.update(humanoid_bone_names)
            continue

        for name, eng_wp in eng_bones.items():
            role = role_map.get(name)

            if name not in uni_bones:
                if role is not None:
                    missing_humanoid_bones.add(name)
                continue

            uni_wp = tuple(uni_bones[name])

            diff_x = abs(eng_wp[0] - (-uni_wp[0]))
            diff_y = abs(eng_wp[1] - uni_wp[1])
            diff_z = abs(eng_wp[2] - uni_wp[2])
            max_diff = max(diff_x, diff_y, diff_z)

            ratio = max_diff / abs(hips_y) if hips_y != 0 else max_diff

            if role is not None:
                if ratio > humanoid_max_ratio:
                    humanoid_max_ratio = ratio
                    humanoid_worst_bone = name
            else:
                if ratio > extra_max_ratio:
                    extra_max_ratio = ratio
                    extra_worst_bone = name

    ok = humanoid_max_ratio <= tolerance and extra_max_ratio <= tolerance and len(missing_humanoid_bones) == 0

    return {
        "ok": ok,
        "humanoid_max_ratio": round(humanoid_max_ratio, 6),
        "humanoid_worst_bone": humanoid_worst_bone,
        "missing_humanoid_bones": sorted(missing_humanoid_bones),
        "extra_max_ratio": round(extra_max_ratio, 6),
        "extra_worst_bone": extra_worst_bone,
        "times": times,
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compare engine bone debug dump against Unity pose output."
    )
    parser.add_argument("engine_dump", help="Path to engine debug dump JSON (--batch-anim-debug-dump output)")
    parser.add_argument("unity_pose", help="Path to Unity pose JSON")
    parser.add_argument("--tolerance", type=float, default=0.01,
                        help="Max allowed ratio for humanoid bones (default 0.01 = 1%%)")
    args = parser.parse_args()

    engine_dump = json.loads(Path(args.engine_dump).read_text())
    unity_pose = json.loads(Path(args.unity_pose).read_text())

    result = compare_bone_positions(engine_dump, unity_pose, args.tolerance)

    print(json.dumps(result))

    if not result["ok"]:
        sys.exit(1)


if __name__ == "__main__":
    main()
