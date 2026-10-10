#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
IMAGE_TAG="thyllore-blender-smoke"
BLENDER_VERSION="${THYLLORE_SMOKE_BLENDER_VERSION:-5.1.2}"
TEMP_DIR="$(realpath "${TMPDIR:-/tmp}")"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    cat <<EOF
Usage: $0 [blender args...]

Runs the pristine Docker Blender on the NVIDIA GPU without a window and
passes every argument to it, so it can stand in for a host Blender binary
(BlenderPath in .claude/local/paths.md):

  $0 --background --python script.py -- input.glb output.glb

The repo is mounted read-only and the temp directory ($TEMP_DIR)
writable, both at their host paths, so absolute paths under them mean the
same file inside the container. Files are written as the calling user.
EOF
    exit 0
fi

docker build -q --build-arg "BLENDER_VERSION=$BLENDER_VERSION" -t "$IMAGE_TAG" \
    "$REPO_ROOT/blender/docker" >/dev/null

GPU_FLAGS=(--gpus all -e NVIDIA_DRIVER_CAPABILITIES=all)
if [[ -d /dev/dri ]]; then
    GPU_FLAGS+=(--device /dev/dri)
fi

exec docker run --rm \
    "${GPU_FLAGS[@]}" \
    --user "$(id -u):$(id -g)" \
    -e HOME=/var/tmp/blender_home \
    -v "$REPO_ROOT:$REPO_ROOT:ro" \
    -v "$TEMP_DIR:$TEMP_DIR" \
    -w "$REPO_ROOT" \
    "$IMAGE_TAG" \
    bash -c 'mkdir -p "$HOME" && exec blender "$@"' blender "$@"
