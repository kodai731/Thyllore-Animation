"""Export the water resolve shader into Blender-compatible GLSL."""

import argparse
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..")))

from blender_addon.common.slang_export import export_shader, rewrite_sampler_uv, write_shader  # noqa: E402

RESOLVE_SHADER = "water/resolveFragment.slang"
OUTPUT_NAMES = ["outColor"]
SCENE_COLOR_SAMPLER = "sceneColorSampler"


def remap_scene_color_to_capture_rect(lines: list[str]) -> list[str]:
    """Blender captures only the water's screen rect, so full-screen uvs are remapped through sceneColorRect."""
    text = rewrite_sampler_uv(
        "\n".join(lines),
        SCENE_COLOR_SAMPLER,
        lambda uv: f"(({uv}) - sceneColorRect.xy) * sceneColorRect.zw",
    )
    leftover = [line for line in text.split("\n") if SCENE_COLOR_SAMPLER in line and "sceneColorRect" not in line]
    if leftover:
        raise SystemExit(f"{SCENE_COLOR_SAMPLER} reads that the rect remap does not cover: {leftover}")
    return text.split("\n")


def main() -> None:
    parser = argparse.ArgumentParser(description="Export GLSL shaders for Blender addon")
    parser.add_argument("--repo-root", default=".", help="Repository root directory")
    parser.add_argument("--out", required=True, help="Output directory for generated files")
    args = parser.parse_args()

    repo_root = os.path.abspath(args.repo_root)
    lines, bindings = export_shader(RESOLVE_SHADER, "fragment", [], OUTPUT_NAMES, repo_root)
    lines = remap_scene_color_to_capture_rect(lines)
    write_shader(os.path.abspath(args.out), "water_torus", lines, bindings)


if __name__ == "__main__":
    main()
