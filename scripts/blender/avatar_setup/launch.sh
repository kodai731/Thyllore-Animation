#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
MAIN_CHECKOUT_ROOT="$(dirname "$(git -C "$REPO_ROOT" rev-parse --path-format=absolute --git-common-dir)")"
DEFAULT_MODEL="$REPO_ROOT/assets/models/fuse-woman-1/edited/woman-1-fix-scale-only.fbx"
CURVE_COPILOT_MODEL_FILENAME="curve_copilot_20260630_v2_k48opt.onnx"
PLATFORM="linux_x86_64"
STAGE_DIR="$REPO_ROOT/build/docker_gui"
MODEL="$DEFAULT_MODEL"
MODE="degrade"
REBUILD_ZIP=1
HEADLESS=0
GUI_ARGS=()
EXTRA_PYTHON_ARGS=()

usage() {
    cat <<USAGE
Usage: $0 [--mode degrade|full|private] [--skip-build] [--software-gl] [--headless]
          [--python PATH.py]... [model.fbx]

Builds the debug addon ZIP of this checkout (wheel with debug-log + ONNX model),
launches the pristine Blender Docker image on the NVIDIA GPU
(blender/docker/run_gui.sh) with the ZIP installed into a fresh profile, and
imports the test model (default: woman-1-fix-scale-only.fbx) so the avatar setup
panels can be used at once. The model's .avatar.json sidecar, when present, is
staged beside it under /scenes for Import Avatar Sidecar. Blender's console
output is mirrored to <repo>/log/log_blender_avatar_setup.log.

Screenshots: with THYLLORE_SCREENSHOT_DIR=/screenshots the whole window is saved
as /tmp/thyllore_screenshots/avatar_setup.png THYLLORE_SCREENSHOT_DELAY seconds
(default 3) after start; THYLLORE_SCREENSHOT_QUIT=1 closes Blender afterwards.
--headless runs Blender on Xvfb inside the container (no window on the host),
switches the sidebar to the avatar setup tab, takes that screenshot and quits;
the three variables keep any value set by the caller.

Options:
  --mode MODE          degrade=A, full=B, private=C (default: degrade)
  --skip-build         Reuse dist/.last_built_zip
  --software-gl        Passed through to run_gui.sh (Mesa llvmpipe)
  --headless           Screenshot without a window (see above)
  --python PATH        Extra script inside the repo run after the model import
                       (e.g. to generate chains before the screenshot); repeatable
  -h, --help           Show this help
USAGE
}

build_mode_for() {
    case "$1" in
        degrade) printf 'A' ;;
        full)    printf 'B' ;;
        private) printf 'C' ;;
        *) echo "invalid mode: $1 (expected degrade, full or private)" >&2; exit 2 ;;
    esac
}

source_feedback_env() {
    local env_file
    for env_file in "$REPO_ROOT/.config/curve_copilot_full.env" "$MAIN_CHECKOUT_ROOT/.config/curve_copilot_full.env"; do
        if [[ -r "$env_file" ]]; then
            set -a
            # shellcheck disable=SC1090
            source "$env_file"
            set +a
            return
        fi
    done
    echo "curve_copilot_full.env not found in .config/ of this checkout or the main checkout" >&2
    exit 2
}

build_addon_zip() {
    local onnx_model="${THYLLORE_CURVE_MODEL:-${THYLLORE_SHARED_DATA_DIR:-}/exports/$CURVE_COPILOT_MODEL_FILENAME}"
    if [[ ! -f "$onnx_model" ]]; then
        echo "curve_copilot model not found at $onnx_model (set THYLLORE_CURVE_MODEL or THYLLORE_SHARED_DATA_DIR)" >&2
        exit 1
    fi

    source_feedback_env
    bash "$REPO_ROOT/scripts/collect_wheels.sh" --debug-log
    bash "$REPO_ROOT/scripts/build_blender_addon.sh" \
        --platform "$PLATFORM" --debug --build-mode "$(build_mode_for "$MODE")" \
        --env test --include-onnx-model --onnx-source-path "$onnx_model"
}

stage_model() {
    local sidecar="${MODEL%.fbx}.avatar.json"
    STAGED_FILES=("$STAGE_DIR/$(basename "$MODEL")")
    mkdir -p "$STAGE_DIR"
    cp -f "$MODEL" "$STAGE_DIR/"
    if [[ -f "$sidecar" ]]; then
        cp -f "$sidecar" "$STAGE_DIR/"
        STAGED_FILES+=("$STAGE_DIR/$(basename "$sidecar")")
    fi
    trap 'rm -f "${STAGED_FILES[@]}"; rmdir "$STAGE_DIR" 2>/dev/null || true' EXIT
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --mode) MODE="$2"; shift 2 ;;
        --skip-build) REBUILD_ZIP=0; shift ;;
        --software-gl) GUI_ARGS+=(--software-gl); shift ;;
        --headless) HEADLESS=1; GUI_ARGS+=(--headless); shift ;;
        --python) EXTRA_PYTHON_ARGS+=(--python "$2"); shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) MODEL="$1"; shift ;;
    esac
done

if [[ ! -f "$MODEL" ]]; then
    echo "model not found: $MODEL" >&2
    exit 1
fi

if [[ "$REBUILD_ZIP" -eq 1 ]]; then
    build_addon_zip
fi
ZIP="$(cat "$REPO_ROOT/dist/.last_built_zip")"

stage_model
export THYLLORE_AVATAR_MODEL="/scenes/$(basename "$MODEL")"
if [[ "$HEADLESS" -eq 1 ]]; then
    export THYLLORE_SCREENSHOT_DIR="${THYLLORE_SCREENSHOT_DIR:-/screenshots}"
    export THYLLORE_SCREENSHOT_QUIT="${THYLLORE_SCREENSHOT_QUIT:-1}"
fi

LOG="$REPO_ROOT/log/log_blender_avatar_setup.log"
mkdir -p "$REPO_ROOT/log"
echo "[avatar_setup/launch] launching Blender (docker, NVIDIA) with $(basename "$ZIP") and $(basename "$MODEL")"
echo "[avatar_setup/launch] Blender stdout/stderr -> $LOG"
bash "$REPO_ROOT/blender/docker/run_gui.sh" \
    --zip "$ZIP" \
    --python "$REPO_ROOT/scripts/blender/avatar_setup/setup_scene.py" \
    ${EXTRA_PYTHON_ARGS[@]+"${EXTRA_PYTHON_ARGS[@]}"} \
    ${GUI_ARGS[@]+"${GUI_ARGS[@]}"} 2>&1 | tee "$LOG"
