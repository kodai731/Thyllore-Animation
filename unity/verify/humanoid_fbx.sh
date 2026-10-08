#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK_DIR="${UNITY_HUMANOID_FBX_DIR:-$REPO_ROOT/target/unity_humanoid_fbx}"
PROJECT_DIR="$WORK_DIR/project"
IMAGE_TAG="thyllore-unity-gui"
DOCKER_DIR="$REPO_ROOT/unity/docker"
LICENSE_DIR="${UNITY_LICENSE_DIR:-$HOME/.config/unity3d/Unity/licenses}"

SKIP_ENGINE=0
if [[ "${1:-}" == "--skip-engine" ]]; then
    SKIP_ENGINE=1
fi

POSE_JSON="$WORK_DIR/humanoid_pose.json"

run_in_container() {
    docker run --rm \
        --user "$(id -u):$(id -g)" \
        -e HOME=/work/home \
        -v "$WORK_DIR:/work" \
        -v "$LICENSE_DIR:/work/home/.config/unity3d/Unity/licenses:ro" \
        -v /etc/machine-id:/etc/machine-id:ro \
        -w /work/project \
        "$@"
}

ensure_project() {
    if [[ -f "$PROJECT_DIR/ProjectSettings/ProjectVersion.txt" ]]; then
        return
    fi
    echo "creating Unity project"
    run_in_container "$IMAGE_TAG" \
        unity-editor -batchmode -nographics -quit \
            -createProject /work/project -logFile /work/create.log
}

if [[ "$SKIP_ENGINE" -eq 0 ]]; then
    echo "running template_smoke.py (engine part)"
    (cd "$REPO_ROOT" && uv run python3 tools/template_smoke.py --dood)
fi

ENGINE_A_JSON="$REPO_ROOT/target/template_smoke/a.json"
ENGINE_FBX="$REPO_ROOT/target/template_smoke/exported.fbx"

if [[ ! -f "$ENGINE_A_JSON" ]]; then
    echo "engine dump not found: $ENGINE_A_JSON (run without --skip-engine)" >&2
    exit 1
fi
if [[ ! -f "$ENGINE_FBX" ]]; then
    echo "exported FBX not found: $ENGINE_FBX (run without --skip-engine)" >&2
    exit 1
fi

echo "building Unity project"
docker build -t "$IMAGE_TAG" "$DOCKER_DIR"

mkdir -p "$PROJECT_DIR" "$WORK_DIR/home/.config/unity3d/Unity" "$WORK_DIR/home/.local/share"
ensure_project

rm -rf "$PROJECT_DIR/Assets/Editor" "$PROJECT_DIR/Assets/Check" \
       "$PROJECT_DIR/Assets/Editor.meta" "$PROJECT_DIR/Assets/Check.meta"
mkdir -p "$PROJECT_DIR/Assets/Editor" "$PROJECT_DIR/Assets/Check"
cp "$REPO_ROOT/unity/verify/HumanoidFbxCheck.cs" "$PROJECT_DIR/Assets/Editor/"
cp "$REPO_ROOT/unity/com.thyllore.avatar/Editor/HumanoidClipImport.cs" "$PROJECT_DIR/Assets/Editor/"
cp "$ENGINE_FBX" "$PROJECT_DIR/Assets/Check/"

echo "running Unity HumanoidFbxCheck (batch)"
set +e
run_in_container \
    -e THYLLORE_FBX_ASSET="Assets/Check/exported.fbx" \
    -e THYLLORE_POSE_TIMES="0,1" \
    -e THYLLORE_POSE_OUT="/work/humanoid_pose.json" \
    "$IMAGE_TAG" \
    unity-editor -batchmode -nographics -projectPath /work/project \
        -executeMethod Thyllore.Avatar.HumanoidFbxCheck.Run -logFile /work/check.log
status=$?
set -e

grep -E "HUMANOIDCHECK|error CS" "$WORK_DIR/check.log" || true
echo "log: $WORK_DIR/check.log"

if [[ $status -ne 0 ]]; then
    echo "Unity exited with code $status" >&2
    exit $status
fi

if [[ ! -f "$POSE_JSON" ]]; then
    echo "Unity did not write pose JSON: $POSE_JSON" >&2
    exit 1
fi

echo "comparing engine dump vs Unity pose"
(cd "$REPO_ROOT" && uv run python3 tools/unity_bone_compare.py "$ENGINE_A_JSON" "$POSE_JSON")
