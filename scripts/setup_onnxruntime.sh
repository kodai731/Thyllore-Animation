#!/usr/bin/env bash
# Installs the Linux x64 ONNX Runtime from the Microsoft.ML.OnnxRuntime NuGet package into vendor/onnxruntime
# (the path .cargo/config.toml points ORT_DYLIB_PATH at) for CI and fresh machines.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ORT_VERSION="${ORT_VERSION:-1.23.2}"
ORT_DIR="${ORT_DIR:-$REPO_ROOT/vendor/onnxruntime}"
INSTALL_DIR="$ORT_DIR/onnxruntime-linux-x64-${ORT_VERSION}"
ORT_DYLIB="$INSTALL_DIR/lib/libonnxruntime.so"

PACKAGE="microsoft.ml.onnxruntime"
PACKAGE_URL="https://api.nuget.org/v3-flatcontainer/${PACKAGE}/${ORT_VERSION}/${PACKAGE}.${ORT_VERSION}.nupkg"

if [[ -f "$ORT_DYLIB" ]]; then
    echo "ONNX Runtime $ORT_VERSION already at $ORT_DYLIB"
else
    TMP="$(mktemp -d)"
    trap 'rm -rf "$TMP"' EXIT

    echo "downloading $PACKAGE_URL"
    curl -fsSL --retry 5 --retry-delay 15 --retry-all-errors -o "$TMP/onnxruntime.nupkg" "$PACKAGE_URL"

    mkdir -p "$INSTALL_DIR/lib"
    unzip -q -o -j "$TMP/onnxruntime.nupkg" 'runtimes/linux-x64/native/*' -d "$INSTALL_DIR/lib"
    unzip -q -o -j "$TMP/onnxruntime.nupkg" LICENSE ThirdPartyNotices.txt -d "$INSTALL_DIR"

    echo "installed ONNX Runtime $ORT_VERSION to $INSTALL_DIR"
fi
