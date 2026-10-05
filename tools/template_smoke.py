"""Smoke test: bake a role clip from template and verify round-trip via FBX export/import.

For each run this script:
  1. Writes target/template_smoke/scene.ron pointing at assets/models/test_humanoid/test_humanoid.fbx
  2. Runs the engine with --batch-anim-edit new_role_clip=oracle and key edits for several roles,
     --batch-anim-debug-dump a.json@0,1 and --batch-export-fbx exported.fbx and --batch-screenshot a.png
  3. Runs the engine again with scene_rt.ron pointing at exported.fbx and --batch-anim-debug-dump b.json@0,1
  4. Checks that pose t==1 matches expected_local_rotation (Ry(-y)*Rx(x)*Rz(-z)) within 1e-3
     and pose t==0 is identity quaternion
  5. Compares world_position between a.json and b.json dumps

    uv run python3 tools/template_smoke.py [--dood]

Exit code 0 = ran to completion (the JSON line on stdout holds ok/fail).
"""

from __future__ import annotations

import argparse
import json
import math
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from engine_harness import dood_wrap, engine_env, engine_path, repo_root

MODEL_PATH = "assets/models/test_humanoid/test_humanoid.fbx"
OUT_DIR = "target/template_smoke"

KEYS = {
    "LeftUpperArm": (10, 25, -40),
    "Head": (15, 30, 5),
    "Spine": (-20, 10, 0),
    "RightLowerLeg": (45, 0, 0),
}


def expected_local_rotation(x: float, y: float, z: float) -> tuple[float, float, float, float]:
    """Quaternion product Ry(-y) * Rx(x) * Rz(-z) in degrees."""
    xr = math.radians(x)
    yr = math.radians(y)
    zr = math.radians(z)

    cy = math.cos(yr / 2)
    sy = math.sin(yr / 2)
    ry = (cy, 0.0, -sy, 0.0)

    cx = math.cos(xr / 2)
    sx = math.sin(xr / 2)
    rx = (cx, sx, 0.0, 0.0)

    cz = math.cos(zr / 2)
    sz = math.sin(zr / 2)
    rz = (cz, 0.0, 0.0, -sz)

    return quat_mul(ry, quat_mul(rx, rz))


def quat_mul(a: tuple[float, float, float, float], b: tuple[float, float, float, float]) -> tuple[float, float, float, float]:
    aw, ax, ay, az = a
    bw, bx, by, bz = b
    return (
        aw * bw - ax * bx - ay * by - az * bz,
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
    )


def check_identity_rig_pose(dump: dict, keys: dict[str, tuple[int, int, int]]) -> list[dict]:
    """Check each pose's bones against expected quaternion.

    For t==1: compare local_rotation_quaternion to expected_local_rotation(x,y,z).
    For t==0: compare local_rotation_quaternion to identity [1,0,0,0].
    Sign flip is allowed (q and -q are the same rotation).
    Returns list of violations where max component difference exceeds 1e-3.
    """
    violations = []
    for pose in dump.get("poses", []):
        time = pose["time"]
        expected_t = 1.0 if time > 0.5 else 0.0
        for bone in pose["bones"]:
            role = bone.get("role")
            if role not in keys:
                continue
            x, y, z = keys[role]
            if expected_t == 1.0:
                expected_q = expected_local_rotation(x, y, z)
            else:
                expected_q = (1.0, 0.0, 0.0, 0.0)
            actual_q = tuple(bone["local_rotation_quaternion"])
            diff = max(abs(a - b) for a, b in zip(actual_q, expected_q))
            neg_diff = max(abs(a + b) for a, b in zip(actual_q, expected_q))
            if min(diff, neg_diff) > 1e-3:
                violations.append({
                    "time": time,
                    "role": role,
                    "expected": list(expected_q),
                    "actual": actual_q,
                    "max_diff": round(min(diff, neg_diff), 6),
                })
    return violations


def compare_dumps(a: dict, b: dict) -> float:
    """Max component difference of world_position for matching time+bone name."""
    max_diff = 0.0
    for pose_a in a.get("poses", []):
        time_a = pose_a["time"]
        for bone_a in pose_a["bones"]:
            name = bone_a["name"]
            for pose_b in b.get("poses", []):
                if pose_b["time"] != time_a:
                    continue
                for bone_b in pose_b["bones"]:
                    if bone_b["name"] != name:
                        continue
                    pa = bone_a["world_position"]
                    pb = bone_b["world_position"]
                    diff = max(abs(pa[i] - pb[i]) for i in range(3))
                    if diff > max_diff:
                        max_diff = diff
    return max_diff


def run_engine_first(out_dir: Path, scene_path: Path, dood: bool) -> subprocess.CompletedProcess[str]:
    """Run engine with role clip creation and key edits."""
    edit_args = ["--batch-anim-edit", "new_role_clip=oracle"]
    for role, (x, y, z) in KEYS.items():
        for axis, value in [("x", x), ("y", y), ("z", z)]:
            edit_args += ["--batch-anim-edit", f"key={role}.{axis}@0=0", "--batch-anim-edit", f"key={role}.{axis}@1={value}"]

    command = [
        str(engine_path()),
        "--batch-scene", str(scene_path),
        *edit_args,
        "--batch-anim-debug-dump", f"{out_dir}/a.json@0,1",
        "--batch-export-fbx", f"{out_dir}/exported.fbx",
        "--batch-screenshot", f"{out_dir}/a.png",
        "--batch-frames", "130",
    ]
    if dood:
        command = dood_wrap(command)

    print("[template_smoke] engine run 1", file=sys.stderr)
    return subprocess.run(
        command, capture_output=True, text=True, timeout=600,
        cwd=str(repo_root()), env=engine_env(),
    )


def run_engine_second(out_dir: Path, scene_path: Path, dood: bool) -> subprocess.CompletedProcess[str]:
    """Run engine with exported fbx and debug dump."""
    command = [
        str(engine_path()),
        "--batch-scene", str(scene_path),
        "--batch-anim-debug-dump", f"{out_dir}/b.json@0,1",
        "--batch-screenshot", f"{out_dir}/b.png",
        "--batch-frames", "130",
    ]
    if dood:
        command = dood_wrap(command)

    print("[template_smoke] engine run 2", file=sys.stderr)
    return subprocess.run(
        command, capture_output=True, text=True, timeout=600,
        cwd=str(repo_root()), env=engine_env(),
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="Smoke test: role clip template bake and FBX round-trip.")
    parser.add_argument("--dood", action="store_true", help="run the engine through the docker harness")
    args = parser.parse_args()

    root = repo_root()
    out_dir = root / OUT_DIR
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    scene_path = out_dir / "scene.ron"
    scene_content = f"""(
    version: 8,
    model: (
        path: "{root / MODEL_PATH}",
    ),
)
"""
    scene_path.write_text(scene_content)

    proc1 = run_engine_first(out_dir, scene_path, args.dood)
    last_line = proc1.stdout.strip().splitlines()[-1] if proc1.stdout.strip() else ""
    report = json.loads(last_line) if last_line.startswith("{") else {"ok": False, "error": "no JSON"}

    if not report.get("ok"):
        result = {"ok": False, "oracle_violations": [], "roundtrip_max_diff": -1.0, "error": (proc1.stderr + "\n" + proc1.stdout).splitlines()[-10:]}
        print(json.dumps(result))
        return

    a_path = out_dir / "a.json"
    if not a_path.is_file():
        result = {"ok": False, "oracle_violations": [], "roundtrip_max_diff": -1.0, "error": "a.json not written"}
        print(json.dumps(result))
        return

    scene_rt_path = out_dir / "scene_rt.ron"
    scene_rt_content = f"""(
    version: 8,
    model: (
        path: "{out_dir / 'exported.fbx'}",
    ),
)
"""
    scene_rt_path.write_text(scene_rt_content)

    proc2 = run_engine_second(out_dir, scene_rt_path, args.dood)
    last_line2 = proc2.stdout.strip().splitlines()[-1] if proc2.stdout.strip() else ""
    report2 = json.loads(last_line2) if last_line2.startswith("{") else {"ok": False, "error": "no JSON"}

    if not report2.get("ok"):
        result = {"ok": False, "oracle_violations": [], "roundtrip_max_diff": -1.0, "error": (proc2.stderr + "\n" + proc2.stdout).splitlines()[-10:]}
        print(json.dumps(result))
        return

    b_path = out_dir / "b.json"
    if not b_path.is_file():
        result = {"ok": False, "oracle_violations": [], "roundtrip_max_diff": -1.0, "error": "b.json not written"}
        print(json.dumps(result))
        return

    with open(a_path) as f:
        dump_a = json.load(f)
    with open(b_path) as f:
        dump_b = json.load(f)

    violations = check_identity_rig_pose(dump_a, KEYS)
    roundtrip_diff = compare_dumps(dump_a, dump_b)

    ok = len(violations) == 0 and roundtrip_diff < 1e-3
    result = {"ok": ok, "oracle_violations": violations, "roundtrip_max_diff": round(roundtrip_diff, 6)}
    print(json.dumps(result))


if __name__ == "__main__":
    main()
