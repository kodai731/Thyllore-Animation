#!/usr/bin/env bash
# Compiles the Slang wind stages to SPIR-V with the same output names as the GLSL build
# (assets/shaders/wind/*.spv) so they can be swapped in for a bit-identity capture.
#   shaders/slang/compile.sh <out_dir>
set -euo pipefail

SLANG_ROOT="${SLANG_ROOT:-$HOME/.local/slang}"
SLANGC="$SLANG_ROOT/bin/slangc"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT_DIR="${1:?usage: compile.sh <out_dir>}"
mkdir -p "$OUT_DIR/wind"

compile_stage() {
    local source="$1" entry="$2" output="$3"
    local raw="$OUT_DIR/$output.raw"
    "$SLANGC" "$HERE/$source" -I "$HERE/include" -I "$HERE/wind/include" \
        ${SLANG_FLAGS:-} -DWIND_SHADOW_VOLUME -target spirv -entry "$entry" -o "$raw"
    python3 "$HERE/tools/strip_layout_suffix.py" "$raw" "$OUT_DIR/$output"
    rm "$raw"
}

compile_stage wind/resolveFragment.slang fragMain wind/resolveFrag.spv
compile_stage wind/shadowBake.slang compMain wind/shadowBakeComp.spv
