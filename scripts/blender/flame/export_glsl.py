"""Export the flame resolve shader into Blender-compatible GLSL."""

import argparse
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..")))

from blender_addon.common.slang_export import export_shader, write_shader  # noqa: E402

RESOLVE_SHADER = "flame/resolveFragment.slang"
OUTPUT_NAMES = ["outColor", "outHistory"]


def main() -> None:
    parser = argparse.ArgumentParser(description="Export GLSL shaders for Blender addon")
    parser.add_argument("--repo-root", default=".", help="Repository root directory")
    parser.add_argument("--out", required=True, help="Output directory for generated files")
    args = parser.parse_args()

    repo_root = os.path.abspath(args.repo_root)
    lines, bindings = export_shader(RESOLVE_SHADER, "fragment", [], OUTPUT_NAMES, repo_root)
    write_shader(os.path.abspath(args.out), "flame_resolve", lines, bindings)


if __name__ == "__main__":
    main()
