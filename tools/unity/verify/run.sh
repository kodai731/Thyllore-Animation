#!/usr/bin/env bash
set -euo pipefail

# Headless check of tools/unity/Editor/*.cs against a synthetic rig:
# empty Unity project + VRChat SDK (vrc-get) + BatchCheck.cs via -executeMethod.
# Nothing from purchased avatars is used.

VERIFY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$VERIFY_DIR/../../.." && pwd)"
WORK_DIR="${UNITY_VERIFY_DIR:-$REPO_ROOT/target/unity_verify}"
PROJECT_DIR="$WORK_DIR/project"
UNITY_EDITOR="${UNITY_EDITOR:-$HOME/Unity/Hub/Editor/2022.3.22f1/Editor/Unity}"
VRC_GET="${VRC_GET:-$WORK_DIR/vrc-get}"
VRC_GET_URL="https://github.com/vrc-get/vrc-get/releases/download/v1.9.2/x86_64-unknown-linux-musl-vrc-get"
BLENDER="${BLENDER:-blender}"

require_tool() {
    if [[ ! -x "$1" ]] && ! command -v "$1" >/dev/null 2>&1; then
        echo "missing: $1" >&2
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
    python3 - "$PROJECT_DIR/Packages/manifest.json" <<'EOF'
import json, sys
path = sys.argv[1]
manifest = json.load(open(path))
manifest["dependencies"]["com.unity.test-framework"] = "1.1.33"
json.dump(manifest, open(path, "w"), indent=2)
EOF
}

sync_scripts() {
    local editor_dir="$PROJECT_DIR/Assets/Thyllore/Editor"
    rm -rf "$editor_dir"
    mkdir -p "$editor_dir"
    cp "$REPO_ROOT/tools/unity/Editor/"*.cs "$editor_dir/"
    cp "$VERIFY_DIR/BatchCheck.cs" "$editor_dir/"
    rm -rf "$PROJECT_DIR/Assets/Avatar" "$PROJECT_DIR/Assets/Avatar.meta"
}

build_fixtures() {
    echo "generating synthetic rig"
    "$BLENDER" -b --python "$VERIFY_DIR/rig.py" -- "$WORK_DIR" >"$WORK_DIR/blender.log" 2>&1
    echo "generating sidecar and .anim"
    (cd "$REPO_ROOT" && cargo run -q -p thyllore-unity-verify-fixtures -- "$WORK_DIR/rig.json" "$WORK_DIR")
}

run_batch_check() {
    echo "running Unity batch check"
    set +e
    THYLLORE_FBX="$WORK_DIR/synthetic_avatar.fbx" \
    THYLLORE_SIDECAR="$WORK_DIR/synthetic_avatar.avatar.json" \
    THYLLORE_ANIM="$WORK_DIR/happy.anim" \
    "$UNITY_EDITOR" -batchmode -nographics -projectPath "$PROJECT_DIR" \
        -executeMethod Thyllore.AvatarTools.BatchCheck.Run -logFile "$WORK_DIR/batch.log"
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
build_fixtures
run_batch_check
