#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
DEFAULT_MODEL="$REPO_ROOT/assets/models/fuse-woman-1/edited/woman-1-fix-scale-only.fbx"
PLATFORM="linux_x86_64"
STAGE_DIR="$REPO_ROOT/build/docker_gui"
MODEL="$DEFAULT_MODEL"
REBUILD_ZIP=1
HEADLESS=0
GUI_ARGS=()
EXTRA_PYTHON_ARGS=()

usage() {
    cat <<USAGE
Usage: $0 [--skip-build] [--software-gl] [--headless] [--python PATH.py]... [model.fbx]

Builds the debug Avatar Setup addon ZIP of this checkout (wheel with debug-log,
Avatar Setup panels only: no curve copilot, skin weight transfer or ONNX model),
launches the pristine Blender Docker image on the NVIDIA GPU
(blender/docker/run_gui.sh) with the ZIP installed into a fresh profile, and
imports the test model (default: woman-1-fix-scale-only.fbx) and bakes
assets/templates/hips_sway.anim.ron onto it, so the avatar setup panels and the
swing preview can be used at once. The model's .avatar.json sidecar, when present, is
staged beside it under /scenes for Import Avatar Sidecar. Blender's console
output is mirrored to <repo>/log/log_blender_avatar_setup.log.

Screenshots: with THYLLORE_SCREENSHOT_DIR=/screenshots the whole window is saved
as /tmp/thyllore_screenshots/avatar_setup.png THYLLORE_SCREENSHOT_DELAY seconds
(default 3) after start; THYLLORE_SCREENSHOT_QUIT=1 closes Blender afterwards.
--headless runs Blender on Xvfb inside the container (no window on the host),
switches the sidebar to the avatar setup tab, takes that screenshot and quits;
the three variables keep any value set by the caller.

Options:
  --skip-build         Reuse dist/.last_built_zip
  --software-gl        Passed through to run_gui.sh (Mesa llvmpipe)
  --headless           Screenshot without a window (see above)
  --python PATH        Extra script inside the repo run after the model import
                       (e.g. to generate chains before the screenshot); repeatable
  -h, --help           Show this help
USAGE
}

build_addon_zip() {
    bash "$REPO_ROOT/scripts/collect_wheels.sh" --debug-log
    bash "$REPO_ROOT/scripts/build_blender_addon.sh" \
        --platform "$PLATFORM" --product avatar-setup --debug --build-mode A
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
