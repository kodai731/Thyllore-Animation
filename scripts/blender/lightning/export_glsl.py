import argparse
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..")))

from blender_addon.common.slang_export import export_shader, write_shader  # noqa: E402

RESOLVE_SHADER = "lightning/resolveFragment.slang"


def main() -> None:
    parser = argparse.ArgumentParser(description="Export GLSL shaders for Blender addon")
    parser.add_argument("--repo-root", default=".", help="Repository root directory")
    parser.add_argument("--out", required=True, help="Output directory for generated files")
    args = parser.parse_args()

    repo_root = os.path.abspath(args.repo_root)
    out_dir = os.path.abspath(args.out)

    resolve_lines, resolve_bindings = export_shader(RESOLVE_SHADER, "fragment", [], ["outColor"], repo_root)
    write_shader(out_dir, "lightning_resolve", resolve_lines, resolve_bindings)


if __name__ == "__main__":
    main()
