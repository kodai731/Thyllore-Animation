"""Rewrite version 7 scene files to version 8: effect components become nested by struct.

    python3 scripts/scene/migrate_v7_to_v8.py [--backup DIR] assets/scenes/*.scene.ron

The flat key of every effect parameter is mapped to its struct path (`noise_amplitude` ->
`noise.amplitude`); the mapping comes from the version 7 declaration tables baked into this
script. Everything outside the effect component blocks is copied byte for byte.
"""
from __future__ import annotations

import argparse
import re
import shutil
from pathlib import Path

FLAME_PATHS = {
    "color_base": "color.base", "color_tip": "color.tip",
    "temperature_base_k": "color.temperature_base_k", "temperature_tip_k": "color.temperature_tip_k",
    "use_blackbody": "color.use_blackbody", "occlusion_lum_ref": "color.occlusion_lum_ref",
    "noise_amplitude": "noise.amplitude", "noise_contrast": "noise.contrast",
    "noise_frequency": "noise.frequency", "noise_scroll_speed": "noise.scroll_speed",
    "noise_aniso_y": "noise.aniso_y", "noise_scale_mode": "noise.scale_mode",
    "noise_shaping_scale": "noise.shaping_scale", "erosion_noise_gain": "noise.erosion_gain",
    "warp_amp": "warp.amp", "warp_freq": "warp.freq", "rise_speed": "warp.rise_speed",
    "taper_power": "warp.taper_power", "warp_reach": "warp.reach",
    "edge_low": "edge.low", "edge_high": "edge.high", "white_boost": "edge.white_boost",
    "radius_tip_ratio": "edge.radius_tip_ratio", "edge_outer_sharpen": "edge.outer_sharpen",
    "wind_direction": "wind.direction", "bend_amount": "wind.bend_amount", "bend_power": "wind.bend_power",
    "envelope_peak": "envelope.peak", "envelope_base": "envelope.base", "envelope_tail": "envelope.tail",
    "contour_wiggle_amp": "contour.wiggle_amp", "aniso_axis_advect": "contour.aniso_axis_advect",
    "rte_bands": "contour.rte_bands", "sigma_dispersion": "contour.sigma_dispersion",
    "tip_carve_depth": "carve.tip.depth", "tip_carve_reach": "carve.tip.reach",
    "burnout_gain": "carve.burnout_gain",
    "swirl_gain": "swirl.gain", "swirl_speed": "swirl.speed",
    "meander_amp": "meander.amp", "meander_frequency": "meander.frequency",
    "mix_lo": "mix.lo", "mix_hi": "mix.hi", "mix_height_gain": "mix.height_gain",
    "mix_scale": "mix.scale", "mix_radial_gain": "mix.radial_gain",
    "density_exp": "thermal.density_exp", "temp_exp": "thermal.temp_exp", "wien_c_k": "thermal.wien_c_k",
    "twist_gain": "twist.gain", "twist_speed": "twist.speed",
    "branch_period": "branch.period", "branch_life": "branch.life", "branch_gain": "branch.gain",
    "branch_core_radius": "branch.core_radius", "branch_core_offset": "branch.core_offset",
    "branch_reach": "branch.reach", "branch_spread": "branch.spread",
    "branch_spawn_height": "branch.spawn_height", "branch_spawn_range": "branch.spawn_range",
    "branch_seed": "branch.seed",
}

LIGHTNING_PATHS = {
    **{k: f"shape.{k}" for k in ["source", "end_offset", "strikes_per_burst", "detail_levels", "tortuosity",
                                 "roughness", "core_radius", "tip_radius_ratio", "edge_fraction", "end_variance"]},
    **{f"branch_{k}": f"branch.{k}" for k in ["depth", "probability", "count", "zone_start", "zone_end", "angle",
                                              "length_ratio", "radius_ratio", "intensity_ratio"]},
    **{k: f"look.{k}" for k in ["core_intensity", "core_color", "rim_ratio", "rim_intensity", "rim_color",
                                "beam_radius", "beam_arc_count", "flash_gain", "flash_radius"]},
    **{k: f"timing.{k}" for k in ["growth_time", "burst_start", "burst_interval", "burst_jitter", "burst_count",
                                  "attack_time", "sustain_time", "release_time", "stroke_count", "stroke_interval",
                                  "stroke_decay", "flicker_amplitude", "flicker_period", "reseed_level",
                                  "reseed_period", "charge_ramp", "seed"]},
}

COMPONENT_PATHS = {"flame": FLAME_PATHS, "lightning": LIGHTNING_PATHS, "water_torus": {}, "wind_tornado": {}}
BLOCK_START = re.compile(r'^(\s*)"(flame|lightning|water_torus|wind_tornado)": \{\n', re.M)


def split_entries(body: str) -> list[tuple[str, str]]:
    """`"key": value` pairs of a flat RON map; values keep their original text."""
    entries = []
    depth = 0
    start = 0
    for i, char in enumerate(body):
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        elif char == "," and depth == 0:
            entries.append(body[start:i])
            start = i + 1
    tail = body[start:].strip()
    if tail:
        entries.append(tail)
    parsed = []
    for entry in entries:
        key, value = entry.strip().split(":", 1)
        parsed.append((key.strip().strip('"'), value.strip()))
    return parsed


def insert_nested(tree: dict, path: str, value: str) -> None:
    node = tree
    segments = path.split(".")
    for segment in segments[:-1]:
        node = node.setdefault(segment, {})
    node[segments[-1]] = value


def render_tree(tree: dict, indent: str, unit: str) -> str:
    lines = []
    for key, value in tree.items():
        if isinstance(value, dict):
            lines.append(f'{indent}"{key}": {{\n{render_tree(value, indent + unit, unit)}{indent}}},\n')
        else:
            lines.append(f'{indent}"{key}": {reindent(value, indent, unit)},\n')
    return "".join(lines)


def reindent(value: str, indent: str, unit: str) -> str:
    lines = value.split("\n")
    if len(lines) == 1:
        return value
    stripped = [line.strip() for line in lines]
    inner = "".join(f"{indent}{unit}{line}\n" for line in stripped[1:-1])
    return f"{stripped[0]}\n{inner}{indent}{stripped[-1]}"


def convert_source(value: str) -> str:
    if value == "Point":
        return '{ "kind": "Point" }'
    radius = re.search(r"radius:\s*([-0-9.e]+)", value)
    return f'{{ "kind": "Shell", "radius": {radius.group(1)} }}'


def migrate_block(kind: str, body: str, indent: str) -> str:
    unit = "    "
    paths = COMPONENT_PATHS[kind]
    tree: dict = {}
    for key, value in split_entries(body):
        if kind == "lightning" and key == "source":
            value = convert_source(value)
        insert_nested(tree, paths.get(key, key), value)
    return render_tree(tree, indent + unit, unit)


def migrate_text(text: str) -> str:
    out = []
    position = 0
    for match in BLOCK_START.finditer(text):
        if match.start() < position:
            continue
        indent, kind = match.group(1), match.group(2)
        close = text.index(f"\n{indent}}}", match.end())
        out.append(text[position:match.end()])
        out.append(migrate_block(kind, text[match.end():close], indent))
        position = close + 1
    out.append(text[position:])
    migrated = "".join(out)
    return migrated.replace("version: 7,", "version: 8,", 1)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--backup", type=Path, help="copy each original here before rewriting")
    parser.add_argument("scenes", nargs="+", type=Path)
    args = parser.parse_args()

    for scene in args.scenes:
        text = scene.read_text()
        if "version: 7," not in text:
            print(f"skip {scene}: not version 7")
            continue
        if args.backup:
            args.backup.mkdir(parents=True, exist_ok=True)
            shutil.copy2(scene, args.backup / scene.name)
        scene.write_text(migrate_text(text))
        print(f"migrated {scene}")


if __name__ == "__main__":
    main()
