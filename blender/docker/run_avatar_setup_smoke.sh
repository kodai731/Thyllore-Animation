#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if ! ls blender_addon/wheels/*.whl >/dev/null 2>&1; then
    echo "blender_addon/wheels/ has no wheels — run scripts/collect_wheels.sh first"
    exit 1
fi

if ! docker image inspect thyllore-blender-xvfb:local >/dev/null 2>&1; then
    docker build -f blender/docker/Dockerfile.xvfb -t thyllore-blender-xvfb:local blender/docker
fi

mkdir -p "$REPO_ROOT/log/blender_avatar_setup_smoke"
LOG="$REPO_ROOT/log/blender_avatar_setup_smoke/smoke.log"

CANONICAL_ROOT="$(cd "$(readlink "$REPO_ROOT/assets/models")/../.." && pwd -P)"

docker run --rm -u "$(id -u):$(id -g)" -e HOME=/tmp \
    -v "$CANONICAL_ROOT:$CANONICAL_ROOT" \
    -v "$REPO_ROOT:$REPO_ROOT" \
    -w "$REPO_ROOT" \
    thyllore-blender-xvfb:local \
    blender --background --factory-startup -noaudio \
        --python-exit-code 1 \
        --python blender_addon/tests/smoke_avatar_setup.py \
    > "$LOG" 2>&1

grep -v '^\s*|' "$LOG" | grep -v gpu.debug | tail -20

grep -q "^AVATAR_SETUP_SMOKE ok" "$LOG"
