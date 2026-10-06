#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK_DIR="${UNITY_MUSCLE_DIR:-$REPO_ROOT/target/unity_muscle}"
PROJECT_DIR="$WORK_DIR/project"
HOME_DIR="$WORK_DIR/home"
LICENSE_DIR="${UNITY_LICENSE_DIR:-$HOME/.config/unity3d/Unity/licenses}"
IMAGE_TAG="thyllore-unity-gui"

if [[ ! -f "$LICENSE_DIR/UnityEntitlementLicense.xml" ]]; then
    echo "no Unity license in $LICENSE_DIR (sign in once with Unity Hub on this machine)" >&2
    exit 1
fi

docker build -t "$IMAGE_TAG" "$REPO_ROOT/unity/docker"

mkdir -p "$PROJECT_DIR" "$HOME_DIR/.config/unity3d/Unity" "$HOME_DIR/.local/share"

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

if [[ ! -f "$PROJECT_DIR/ProjectSettings/ProjectVersion.txt" ]]; then
    echo "creating Unity project"
    run_in_container "$IMAGE_TAG" \
        unity-editor -batchmode -nographics -quit -createProject /work/project
fi

if [[ ! -d "$PROJECT_DIR/Packages/com.unity.nuget.newtonsoft-json" ]]; then
    echo "adding Newtonsoft.Json package"
    python3 -c "
import json, pathlib
manifest = pathlib.Path('$PROJECT_DIR/Packages/manifest.json')
data = json.loads(manifest.read_text())
data['dependencies']['com.unity.nuget.newtonsoft-json'] = '3.2.1'
manifest.write_text(json.dumps(data, indent=4) + '\n')
"
fi

mkdir -p "$PROJECT_DIR/Assets/Editor"
cp "$REPO_ROOT/unity/muscle/Editor/MuscleProbe.cs" "$PROJECT_DIR/Assets/Editor/"

mkdir -p "$WORK_DIR/tmp"
STICK_TMPDIR="$(mktemp -d "$WORK_DIR/tmp/unity_muscle.XXXX")"
TEST_HUMANOID_SKELETON_JSON="$STICK_TMPDIR/skeleton.json" \
    cargo test -p thyllore-avatar-core --test test_humanoid_asset dump_test_humanoid_skeleton_json -- --ignored
BLENDER_CMD="${BLENDER:-bash "$REPO_ROOT/blender/docker/run_background.sh"}"
TMPDIR="$WORK_DIR/tmp" $BLENDER_CMD -b --python "$REPO_ROOT/unity/muscle/build_stick.py" -- "$STICK_TMPDIR/skeleton.json" "$STICK_TMPDIR"

mkdir -p "$PROJECT_DIR/Assets/Model"
cp "$STICK_TMPDIR/test_humanoid.fbx" "$PROJECT_DIR/Assets/Model/"

echo "running MuscleProbe"
run_in_container "$IMAGE_TAG" \
    unity-editor -batchmode -nographics -projectPath /work/project \
        -executeMethod Thyllore.Muscle.MuscleProbe.Run -quit -logFile /work/probe.log

if [[ ! -f "$PROJECT_DIR/muscles.json" ]]; then
    echo "muscles.json not found at $PROJECT_DIR/muscles.json — probe may have failed; last lines of log:" >&2
    tail -20 "$WORK_DIR/probe.log" >&2
    exit 1
fi

cp "$PROJECT_DIR/muscles.json" "$WORK_DIR/muscles.json"
echo "muscles.json written to $WORK_DIR/muscles.json"
