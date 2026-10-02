#!/usr/bin/env bash
set -euo pipefail

# End-to-end check of the avatar workflow on a synthetic rig (nothing from purchased avatars):
#   1. Blender builds a cube with a humanoid armature and blend shapes (rig.py)
#   2. the engine loads it, edits and keys blend shapes, and exports the sidecar and .anim files
#   3. Unity imports the FBX, applies the sidecar (unity/com.thyllore.avatar) and samples the .anim;
#      BatchCheck.cs compares the sampled weights with what the engine evaluated

VERIFY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$VERIFY_DIR/../.." && pwd)"
WORK_DIR="${UNITY_VERIFY_DIR:-$REPO_ROOT/target/unity_verify}"
PROJECT_DIR="$WORK_DIR/project"
UNITY_EDITOR="${UNITY_EDITOR:-$HOME/Unity/Hub/Editor/2022.3.22f1/Editor/Unity}"
VRC_GET="${VRC_GET:-$WORK_DIR/vrc-get}"
VRC_GET_URL="https://github.com/vrc-get/vrc-get/releases/download/v1.9.2/x86_64-unknown-linux-musl-vrc-get"
BLENDER="${BLENDER:-$REPO_ROOT/blender/docker/run_background.sh}"

MODEL_NAME="synthetic_avatar"
MODEL_FBX="$WORK_DIR/$MODEL_NAME.fbx"
SIDECAR_JSON="$WORK_DIR/$MODEL_NAME.avatar.json"
UNITY_EXPORT_DIR="$WORK_DIR/${MODEL_NAME}_unity"
EXPRESSION_NAME="happy"
SPRING_BONE_PREFIX="Left_braid"
ENGINE_FRAME_COUNT=30

ENGINE_EDITS=(
    "10:timeline_time=0"
    "10:morph_weight=eye_blink:1.0"
    "10:morph_weight=vrc.v_aa:0.25"
    "10:key_morph_weights"

    "12:timeline_time=0.5"
    "12:morph_weight=eye_blink:0.0"
    "12:morph_weight=mouth_smile:0.8"
    "12:morph_weight=vrc.v_aa:1.0"
    "12:key_morph_weights"
    "12:capture_expression=$EXPRESSION_NAME"

    "14:timeline_time=1.0"
    "14:morph_weight=eye_blink:0.5"
    "14:morph_weight=mouth_smile:0.2"
    "14:key_morph_weights"

    "16:add_spring_chains=$SPRING_BONE_PREFIX"
    "16:export_avatar_sidecar"
    "16:export_expression_anims"
    "16:export_morph_track_anim"

    "18:dump_morph_track_samples"
)

require_tool() {
    if [[ ! -x "$1" ]] && ! command -v "$1" >/dev/null 2>&1; then
        echo "missing: $1" >&2
        exit 1
    fi
}

require_file() {
    if [[ ! -s "$1" ]]; then
        echo "$2 did not produce $1" >&2
        exit 1
    fi
}

ensure_vrc_get() {
    if [[ -x "$VRC_GET" ]]; then
        return
    fi
    echo "downloading vrc-get"
    curl -sSL -o "$VRC_GET" "$VRC_GET_URL"
    chmod +x "$VRC_GET"
}

ensure_project() {
    if [[ -f "$PROJECT_DIR/ProjectSettings/ProjectVersion.txt" ]]; then
        return
    fi
    echo "creating Unity project"
    "$UNITY_EDITOR" -batchmode -nographics -quit \
        -createProject "$PROJECT_DIR" -logFile "$WORK_DIR/create.log"
}

ensure_vrchat_sdk() {
    if [[ -d "$PROJECT_DIR/Packages/com.vrchat.avatars" ]]; then
        return
    fi
    echo "installing VRChat SDK"
    (cd "$PROJECT_DIR" && "$VRC_GET" install com.vrchat.avatars --yes)
}

sync_scripts() {
    bash "$REPO_ROOT/unity/sync_package.sh" "$WORK_DIR" >/dev/null
    local editor_dir="$PROJECT_DIR/Assets/Editor"
    rm -rf "$editor_dir"
    mkdir -p "$editor_dir"
    cp "$VERIFY_DIR/BatchCheck.cs" "$editor_dir/"
    rm -rf "$PROJECT_DIR/Assets/Avatar" "$PROJECT_DIR/Assets/Avatar.meta"
}

build_rig() {
    echo "generating synthetic rig"
    local rig_dir
    rig_dir="$(mktemp -d)"
    "$BLENDER" -b --python "$VERIFY_DIR/rig.py" -- "$rig_dir" >"$WORK_DIR/blender.log" 2>&1
    require_file "$rig_dir/$MODEL_NAME.fbx" "Blender (see $WORK_DIR/blender.log)"
    cp "$rig_dir/$MODEL_NAME.fbx" "$MODEL_FBX"
    rm -rf "$rig_dir"
}

edit_in_engine() {
    echo "editing blend shapes in the engine"
    local scene_file="$WORK_DIR/$MODEL_NAME.scene.ron"
    printf '(\n    version: 8,\n    model: (\n        path: "%s",\n    ),\n)\n' "$MODEL_FBX" >"$scene_file"
    rm -rf "$UNITY_EXPORT_DIR" "$SIDECAR_JSON"

    local edit_args=()
    local edit
    for edit in "${ENGINE_EDITS[@]}"; do
        edit_args+=(--batch-debug-action-at "$edit")
    done

    (cd "$REPO_ROOT" && cargo run -q --bin thyllore-animation -- \
        --batch-scene "$scene_file" \
        --batch-screenshot "$WORK_DIR/engine.png" \
        --batch-frames "$ENGINE_FRAME_COUNT" \
        "${edit_args[@]}") >"$WORK_DIR/engine.log" 2>&1

    require_file "$SIDECAR_JSON" "the engine (see $WORK_DIR/engine.log)"
    require_file "$UNITY_EXPORT_DIR/$EXPRESSION_NAME.anim" "the engine (see $WORK_DIR/engine.log)"
    MORPH_SAMPLES_JSON="$(find "$UNITY_EXPORT_DIR" -name '*.samples.json' | head -n 1)"
    require_file "${MORPH_SAMPLES_JSON:-$UNITY_EXPORT_DIR/<clip>.samples.json}" \
        "the engine (see $WORK_DIR/engine.log)"
}

run_batch_check() {
    echo "running Unity batch check"
    set +e
    THYLLORE_FBX="$MODEL_FBX" \
    THYLLORE_SIDECAR="$SIDECAR_JSON" \
    THYLLORE_EXPRESSION_ANIM="$UNITY_EXPORT_DIR/$EXPRESSION_NAME.anim" \
    THYLLORE_MORPH_SAMPLES="$MORPH_SAMPLES_JSON" \
    "$UNITY_EDITOR" -batchmode -nographics -projectPath "$PROJECT_DIR" \
        -executeMethod Thyllore.Avatar.BatchCheck.Run -logFile "$WORK_DIR/batch.log"
    local status=$?
    set -e

    grep -E "error CS" "$WORK_DIR/batch.log" | grep -v "com.vrchat.base" || true
    grep -E "BATCHCHECK" "$WORK_DIR/batch.log" | sed 's/^.*BATCHCHECK/BATCHCHECK/'
    echo "log: $WORK_DIR/batch.log"
    return $status
}

require_tool "$UNITY_EDITOR"
require_tool "$BLENDER"
mkdir -p "$WORK_DIR"
ensure_vrc_get
ensure_project
ensure_vrchat_sdk
sync_scripts
build_rig
edit_in_engine
run_batch_check
