import argparse
import json
import re


def unity_curve_attribute(name):
    finger = re.match(r"^(Left|Right)\s+(Thumb|Index|Middle|Ring|Little)(\s+\d+)?\s+(\w+.*)$", name)
    if finger:
        side, digit, num, rest = finger.groups()
        hand = "LeftHand" if side == "Left" else "RightHand"
        num_part = num.strip() if num else ""
        if num_part:
            return f"{hand}.{digit}.{num_part} {rest}"
        return f"{hand}.{digit}.{rest}"
    return name


def format_float(v):
    return f"{v:.6f}"


def build_slope_table(probes):
    probe_data = {}
    for p in probes:
        bone = p["bone"]
        axis = p["axis"]
        angle = p["angle"]
        key = (bone, axis, "positive" if angle > 0 else "negative")
        if key not in probe_data:
            probe_data[key] = {}
        for d in p["deltas"]:
            probe_data[key][d["index"]] = d["delta"]

    slopes_by_muscle = {}

    bone_axis_pairs = set()
    for (bone, axis, _) in probe_data:
        bone_axis_pairs.add((bone, axis))

    for (bone, axis) in sorted(bone_axis_pairs):
        pos_key = (bone, axis, "positive")
        neg_key = (bone, axis, "negative")
        pos_deltas = probe_data.get(pos_key, {})
        neg_deltas = probe_data.get(neg_key, {})

        all_indices = set(pos_deltas.keys()) | set(neg_deltas.keys())

        for idx in sorted(all_indices):
            pos_delta = pos_deltas.get(idx, 0.0)
            neg_delta = neg_deltas.get(idx, 0.0)

            if abs(pos_delta) < 0.01 and abs(neg_delta) < 0.01:
                continue

            positive = round(pos_delta / 20.0, 6)
            negative = round(neg_delta / (-20.0), 6)

            slopes_by_muscle.setdefault(idx, []).append({
                "role": bone,
                "axis": axis,
                "positive": positive,
                "negative": negative,
            })

    return slopes_by_muscle


def write_toml(path, muscles, rest_muscles, slopes_by_muscle):
    lines = []

    for m in muscles:
        idx = m["index"]
        name = m["name"]
        role = m["bone"]
        dof = m["dof"]
        mn = m["min"]
        mx = m["max"]
        rest = round(rest_muscles[idx], 6)

        lines.append("[[muscle]]")
        lines.append(f"index = {idx}")
        lines.append(f"name = \"{name}\"")
        lines.append(f"attribute = \"{unity_curve_attribute(name)}\"")
        lines.append(f"role = \"{role}\"")
        lines.append(f"dof = {dof}")
        lines.append(f"min = {format_float(mn)}")
        lines.append(f"max = {format_float(mx)}")
        lines.append(f"rest = {format_float(rest)}")

        slopes = slopes_by_muscle.get(idx, [])
        if not slopes:
            lines.append("slopes = []")
        else:
            for s in slopes:
                lines.append("[[muscle.slopes]]")
                lines.append(f"role = \"{s['role']}\"")
                lines.append(f"axis = \"{s['axis']}\"")
                lines.append(f"positive = {format_float(s['positive'])}")
                lines.append(f"negative = {format_float(s['negative'])}")

        lines.append("")

    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input_json", help="Path to Unity muscles.json")
    parser.add_argument("output_toml", help="Path to output TOML")
    args = parser.parse_args()

    with open(args.input_json, "r", encoding="utf-8") as f:
        data = json.load(f)

    muscles = data["muscles"]
    rest_muscles = data["rest_muscles"]
    probes = data["probes"]

    slopes_by_muscle = build_slope_table(probes)
    write_toml(args.output_toml, muscles, rest_muscles, slopes_by_muscle)


if __name__ == "__main__":
    main()
