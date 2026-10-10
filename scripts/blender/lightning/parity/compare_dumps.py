"""Numerically diff an engine `dump_lightning_debug` JSON against a Blender `Dump Lightning Debug` JSON.

    python compare_dumps.py <engine.json> <blender.json> [tolerance]
Exit code 1 when any compared value differs by more than the tolerance."""
import json
import sys

import numpy as np

UBO_BLOCKS = ("model", "inverse_model", "core", "rim", "shape", "inv_view_proj")


def flat(value):
    return np.array(value, dtype=np.float64).reshape(-1)


def segment_values(segments):
    return [[segment["a_r0"], segment["b_r1"], segment["misc"]] for segment in segments]


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
    engine_lightning = engine["lightning_instances"][0]
    blender_lightning = blender["lightning_instances"][0]
    for key in sorted(set(engine_lightning["effect"]) | set(blender_lightning["effect"])):
        if key not in engine_lightning["effect"] or key not in blender_lightning["effect"]:
            rows.append((f"effect.{key}", float("inf"), "missing on one side"))
            continue
        rows.append(diff_line(f"effect.{key}", engine_lightning["effect"][key], blender_lightning["effect"][key]))
    engine_ubo = engine_lightning.get("ubo", {})
    blender_ubo = blender_lightning.get("ubo", {})
    for block in UBO_BLOCKS:
        if block in engine_ubo and block in blender_ubo:
            rows.append(diff_line(f"ubo.{block}", engine_ubo[block], blender_ubo[block]))
    rows.append(diff_line("projection.view", engine["projection"]["view"], blender["projection"]["view"]))
    rows.append(diff_line("projection.proj", engine["projection"]["proj"], blender["projection"]["proj"]))
    rows.append(diff_line("projection.screen_size", engine["projection"]["screen_size"], blender["projection"]["screen_size"]))
    rows.append(diff_line("camera.position", engine["camera"]["position"], blender["camera"]["position"]))
    if "segments" in engine_lightning and "segments" in blender_lightning:
        rows.append(diff_line("segments", segment_values(engine_lightning["segments"]), segment_values(blender_lightning["segments"])))

    failures = 0
    for label, diff, detail in rows:
        status = "ok " if diff <= tolerance else "NG "
        failures += diff > tolerance
        print(f"{status} {label:32s} max_abs_diff={diff:.3e}  {detail}")
    print(f"{len(rows) - failures}/{len(rows)} fields within {tolerance:g}")
    sys.exit(1 if failures else 0)


main()
