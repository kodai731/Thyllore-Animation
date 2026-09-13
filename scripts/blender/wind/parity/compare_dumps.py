"""Numerically diff an engine `dump_wind_debug` JSON against a Blender `Dump Wind Debug` JSON.

    python compare_dumps.py <engine.json> <blender.json> [tolerance]
Exit code 1 when any compared value differs by more than the tolerance."""
import json
import sys

import numpy as np

UBO_BLOCKS = ("model", "inverse_model", "shape", "wall", "optics", "albedo", "lighting", "streak", "streak2", "eddy", "eddy2", "puff_params", "puffs")
RUNTIME_KEYS = ("time", "time_scale", "time_offset", "position", "rotation")


def flat(value):
    return np.array(value, dtype=np.float64).reshape(-1)


def diff_line(label, ours, theirs):
    a, b = flat(ours), flat(theirs)
    if a.shape != b.shape:
        return label, float("inf"), f"shape {a.shape} vs {b.shape}"
    if a.size == 0:
        return label, 0.0, "empty"
    return label, float(np.max(np.abs(a - b))), f"engine={np.round(a[:4], 4).tolist()} blender={np.round(b[:4], 4).tolist()}"


def main():
    engine = json.load(open(sys.argv[1]))
    blender = json.load(open(sys.argv[2]))
    tolerance = float(sys.argv[3]) if len(sys.argv) > 3 else 1e-5

    rows = []
    engine_wind = engine["wind_instances"][0]
    blender_wind = blender["wind_instances"][0]
    for key in sorted(set(engine_wind["effect"]) | set(blender_wind["effect"])):
        if key not in engine_wind["effect"] or key not in blender_wind["effect"]:
            rows.append((f"effect.{key}", float("inf"), "missing on one side"))
            continue
        rows.append(diff_line(f"effect.{key}", engine_wind["effect"][key], blender_wind["effect"][key]))
    for block in UBO_BLOCKS:
        if block in engine_wind["ubo"] and block in blender_wind["ubo"]:
            rows.append(diff_line(f"ubo.{block}", engine_wind["ubo"][block], blender_wind["ubo"][block]))
    rows.append(diff_line("ubo.inv_view_proj", engine["projection"]["inv_view_proj_f64"], blender_wind["ubo"]["inv_view_proj"]))
    rows.append(diff_line("light.position", engine["light"]["position"], blender["light"]["position"]))
    rows.append(diff_line("camera.position", engine["camera"]["position"], blender["camera"]["position"]))
    rows.append(diff_line("projection.view", engine["projection"]["view"], blender["projection"]["view"]))
    rows.append(diff_line("projection.proj", engine["projection"]["proj"], blender["projection"]["proj"]))
    rows.append(diff_line("projection.screen_size", engine["projection"]["screen_size"], blender["projection"]["screen_size"]))

    failures = 0
    for label, diff, detail in rows:
        status = "ok " if diff <= tolerance else "NG "
        failures += diff > tolerance
        print(f"{status} {label:32s} max_abs_diff={diff:.3e}  {detail}")
    print(f"{len(rows) - failures}/{len(rows)} fields within {tolerance:g}")
    sys.exit(1 if failures else 0)


main()
