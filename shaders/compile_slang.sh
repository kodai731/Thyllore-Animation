#!/usr/bin/env bash
# Compiles the Slang wind stages to SPIR-V with the same output names as the GLSL build
# (assets/shaders/wind/*.spv) so they can be swapped in for a bit-identity capture.
#   shaders/compile_slang.sh <out_dir>
set -euo pipefail

SLANG_ROOT="${SLANG_ROOT:-$HOME/.local/slang}"
SLANGC="$SLANG_ROOT/bin/slangc"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT_DIR="${1:?usage: compile_slang.sh <out_dir>}"
mkdir -p "$OUT_DIR/wind"

compile_stage() {
    local source="$1" output="$2"
    "$SLANGC" "$HERE/$source" -I "$HERE" \
        ${SLANG_FLAGS:-} -DWIND_SHADOW_VOLUME -target spirv -entry main -o "$OUT_DIR/$output"
}

compile_stage wind/resolveFragment.slang wind/resolveFrag.spv
compile_stage wind/shadowBakeCompute.slang wind/shadowBakeComp.spv
compile_stage wind/upsampleFragment.slang wind/upsampleFrag.spv
