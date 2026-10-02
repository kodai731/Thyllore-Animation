#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOCKER_DIR="$REPO_ROOT/unity/docker"
IMAGE_TAG="thyllore-unity-gui"
WORK_DIR="${UNITY_GUI_DIR:-$REPO_ROOT/target/unity_gui}"
PROJECT_DIR="$WORK_DIR/project"
HOME_DIR="$WORK_DIR/home"
LICENSE_DIR="$HOME/.config/unity3d/Unity/licenses"
MODEL=""
FRESH=0
SETUP_ONLY=0
SOFTWARE_GL=0

usage() {
    cat <<USAGE
Usage: $0 --model PATH.fbx [--fresh] [--setup-only] [--software-gl]

Opens the Unity Editor GUI (GameCI 2022.3.22f1 image, VRChat SDK via vrc-get)
on the host display with a scene that already contains the model, the
sidecar applied (Humanoid, visemes, blink, PhysBone) and an Animator whose
controller holds every .anim the engine exported next to the model
(<model>_unity/*.anim), so each expression preset can be scrubbed in the
Animation window. The project persists in target/unity_gui so later runs
only resync the model and the editor scripts.

  --model PATH.fbx   model edited in the engine; <stem>.* (sidecar JSON,
                     materials), <stem>_unity/ (.anim) and image files next
                     to it are copied into Assets/Avatar
  --fresh            delete the cached Unity project first (re-creates it and
                     reinstalls the VRChat SDK, several minutes)
  --setup-only       build the scene in batch mode and exit without a window
  --software-gl      Mesa llvmpipe instead of the NVIDIA GPU

The host Unity Personal license (~/.config/unity3d/Unity/licenses) and
/etc/machine-id are mounted read-only; the license is bound to the machine id.
USAGE
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --model) MODEL="$2"; shift 2 ;;
        --fresh) FRESH=1; shift ;;
        --setup-only) SETUP_ONLY=1; shift ;;
        --software-gl) SOFTWARE_GL=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown option: $1" >&2; usage; exit 2 ;;
    esac
done

if [[ -z "$MODEL" ]]; then
    echo "--model is required" >&2
    usage >&2
    exit 2
fi
if ! MODEL_ABS="$(realpath -e "$MODEL" 2>/dev/null)"; then
    echo "model not found: $MODEL" >&2
    exit 1
fi
if [[ ! -f "$LICENSE_DIR/UnityEntitlementLicense.xml" ]]; then
    echo "no Unity license in $LICENSE_DIR (sign in once with Unity Hub on this machine)" >&2
    exit 1
fi
if [[ "$SETUP_ONLY" -eq 0 && -z "${DISPLAY:-}" ]]; then
    echo "DISPLAY is not set - run from a graphical session" >&2
    exit 1
fi

MODEL_DIR="$(dirname "$MODEL_ABS")"
MODEL_FILE="$(basename "$MODEL_ABS")"
MODEL_STEM="${MODEL_FILE%.*}"
ASSET_DIR="Assets/Avatar"

docker build -t "$IMAGE_TAG" "$DOCKER_DIR"

if [[ "$FRESH" -eq 1 ]]; then
    rm -rf "$PROJECT_DIR"
fi
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

run_unity_batch() {
    local log_name="$1"
    shift
    run_in_container "$IMAGE_TAG" \
        unity-editor -nographics -logFile "/work/$log_name" "$@"
}

ensure_project() {
    if [[ -f "$PROJECT_DIR/ProjectSettings/ProjectVersion.txt" ]]; then
        return
    fi
    echo "creating Unity project"
    run_unity_batch create.log -quit -createProject /work/project
}

ensure_vrchat_sdk() {
    if [[ -d "$PROJECT_DIR/Packages/com.vrchat.avatars" ]]; then
        return
    fi
    echo "installing VRChat SDK"
    run_in_container "$IMAGE_TAG" vrc-get install com.vrchat.avatars --yes
}

sync_scripts() {
    bash "$REPO_ROOT/unity/sync_package.sh" "$WORK_DIR" >/dev/null
    local editor_dir="$PROJECT_DIR/Assets/Editor"
    rm -rf "$editor_dir"
    mkdir -p "$editor_dir"
    cp "$DOCKER_DIR/Editor/"*.cs "$editor_dir/"
}

sync_model() {
    echo "syncing model from $MODEL_DIR"
    rm -rf "$PROJECT_DIR/$ASSET_DIR" "$PROJECT_DIR/$ASSET_DIR.meta"
    mkdir -p "$PROJECT_DIR/$ASSET_DIR"
    cp "$MODEL_ABS" "$PROJECT_DIR/$ASSET_DIR/"
    find "$MODEL_DIR" -maxdepth 1 -type f \
        \( -name "$MODEL_STEM.*" -o -iname '*.png' -o -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.tga' -o -iname '*.bmp' -o -iname '*.tif' \) \
        -exec cp {} "$PROJECT_DIR/$ASSET_DIR/" \;
    if [[ -d "$MODEL_DIR/${MODEL_STEM}_unity" ]]; then
        cp -r "$MODEL_DIR/${MODEL_STEM}_unity" "$PROJECT_DIR/$ASSET_DIR/"
    fi
}

build_scene() {
    echo "building the avatar scene (batch)"
    run_in_container \
        -e THYLLORE_FBX_ASSET="$ASSET_DIR/$MODEL_FILE" \
        -e THYLLORE_SIDECAR_ASSET="$ASSET_DIR/$MODEL_STEM.avatar.json" \
        -e THYLLORE_ANIM_DIR_ASSET="$ASSET_DIR/${MODEL_STEM}_unity" \
        "$IMAGE_TAG" \
        unity-editor -nographics -logFile /work/setup.log \
            -projectPath /work/project -executeMethod Thyllore.Avatar.SceneSetup.Run
    grep -E "error CS|SCENESETUP" "$WORK_DIR/setup.log" | grep -v "com.vrchat.base" || true
}

launch_gui() {
    xhost +si:localuser:"$(id -un)" >/dev/null
    local gpu_flags=(--gpus all -e NVIDIA_DRIVER_CAPABILITIES=all)
    local gl_env=()
    if [[ -d /dev/dri ]]; then
        gpu_flags+=(--device /dev/dri)
    fi
    if [[ "$SOFTWARE_GL" -eq 1 ]]; then
        gpu_flags=()
        gl_env=(-e LIBGL_ALWAYS_SOFTWARE=1)
    fi

    echo "launching Unity GUI (log: $WORK_DIR/gui.log)"
    run_in_container \
        ${gpu_flags[@]+"${gpu_flags[@]}"} \
        ${gl_env[@]+"${gl_env[@]}"} \
        -e DISPLAY="$DISPLAY" \
        -v /tmp/.X11-unix:/tmp/.X11-unix:ro \
        "$IMAGE_TAG" \
        /opt/unity/Editor/Unity -logFile /work/gui.log \
            -projectPath /work/project -executeMethod Thyllore.Avatar.SceneSetup.Open
}

ensure_project
ensure_vrchat_sdk
sync_scripts
sync_model
build_scene
if [[ "$SETUP_ONLY" -eq 0 ]]; then
    launch_gui
fi
