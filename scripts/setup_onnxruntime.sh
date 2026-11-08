#!/usr/bin/env bash
# Installs the Linux x64 ONNX Runtime release into vendor/onnxruntime (the path .cargo/config.toml
# points ORT_DYLIB_PATH at) for CI and fresh machines.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ORT_VERSION="${ORT_VERSION:-1.23.2}"
ORT_DIR="${ORT_DIR:-$REPO_ROOT/vendor/onnxruntime}"
ASSET="onnxruntime-linux-x64-${ORT_VERSION}.tgz"
ORT_DYLIB="$ORT_DIR/onnxruntime-linux-x64-${ORT_VERSION}/lib/libonnxruntime.so"

source "$REPO_ROOT/scripts/lib/github_release.sh"

if [[ -f "$ORT_DYLIB" ]]; then
    echo "ONNX Runtime $ORT_VERSION already at $ORT_DYLIB"
else
    TMP="$(mktemp -d)"
    download_github_release_asset microsoft/onnxruntime "v${ORT_VERSION}" "$ASSET" "$TMP"
    mkdir -p "$ORT_DIR"
    tar -xzf "$TMP/$ASSET" -C "$ORT_DIR"
    rm -rf "$TMP"
    echo "installed ONNX Runtime $ORT_VERSION to $ORT_DIR"
fi
