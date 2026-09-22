import argparse
import os
import re
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..")))

from blender_addon.common.slang_export import export_shader, write_shader  # noqa: E402

RESOLVE_SHADER = "wind/resolveFragment.slang"
BAKE_SHADER = "wind/shadowBakeCompute.slang"
UPSAMPLE_SHADER = "wind/upsampleFragment.slang"
SHADOW_GRID_SOURCE = "wind/include/shadow_volume.slang"
RESOLVE_DEFINES = ["WIND_SHADOW_VOLUME"]
SHADOW_GRID_AXES = ("RADIAL", "HEIGHT", "THETA", "SLOTS")


def shadow_volume_size(repo_root: str) -> list[int]:
    """Texture size [radial * slots, height, theta] read from the WindShadowGrid constants."""
    values: dict[str, int] = {}
    with open(os.path.join(repo_root, "shaders", SHADOW_GRID_SOURCE)) as f:
        for line in f:
            m = re.match(r"^\s*public\s+static\s+const\s+int\s+(\w+)\s*=\s*(\d+)\s*;", line)
            if m and m.group(1) in SHADOW_GRID_AXES:
                values[m.group(1)] = int(m.group(2))
    missing = set(SHADOW_GRID_AXES) - set(values)
    if missing:
        raise SystemExit(f"{SHADOW_GRID_SOURCE} lacks WindShadowGrid constants {sorted(missing)}")
    return [values["RADIAL"] * values["SLOTS"], values["HEIGHT"], values["THETA"]]


def main() -> None:
    parser = argparse.ArgumentParser(description="Export GLSL shaders for Blender addon")
    parser.add_argument("--repo-root", default=".", help="Repository root directory")
    parser.add_argument("--out", required=True, help="Output directory for generated files")
    args = parser.parse_args()

    repo_root = os.path.abspath(args.repo_root)
    out_dir = os.path.abspath(args.out)

    resolve_lines, resolve_bindings = export_shader(RESOLVE_SHADER, "fragment", RESOLVE_DEFINES, ["outColor"], repo_root)
    write_shader(out_dir, "wind_resolve", resolve_lines, resolve_bindings)

    bake_lines, bake_bindings = export_shader(BAKE_SHADER, "compute", RESOLVE_DEFINES, [], repo_root)
    bake_bindings["shadow_volume_size"] = shadow_volume_size(repo_root)
    write_shader(out_dir, "wind_shadow_bake", bake_lines, bake_bindings)

    upsample_lines, upsample_bindings = export_shader(UPSAMPLE_SHADER, "fragment", [], ["outColor"], repo_root)
    write_shader(out_dir, "wind_upsample", upsample_lines, upsample_bindings)


if __name__ == "__main__":
    main()
