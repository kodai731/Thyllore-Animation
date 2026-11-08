#!/usr/bin/env bash
# Installs the Linux x64 ONNX Runtime release into vendor/onnxruntime (the path .cargo/config.toml
# points ORT_DYLIB_PATH at) for CI and fresh machines.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ORT_VERSION="${ORT_VERSION:-1.23.2}"
ORT_DIR="${ORT_DIR:-$REPO_ROOT/vendor/onnxruntime}"
ASSET="onnxruntime-linux-x64-${ORT_VERSION}.tgz"
ORT_DYLIB="$ORT_DIR/onnxruntime-linux-x64-${ORT_VERSION}/lib/libonnxruntime.so"

# The browser download URL intermittently drops the connection or answers 504; the release API
# serves the same asset reliably, so it is tried first when gh has a token.
download_asset() {
    local dir="$1"
    if [[ -n "${GH_TOKEN:-${GITHUB_TOKEN:-}}" ]] && command -v gh >/dev/null 2>&1; then
        echo "downloading $ASSET from the microsoft/onnxruntime v${ORT_VERSION} release through gh"
        if GH_TOKEN="${GH_TOKEN:-$GITHUB_TOKEN}" gh release download "v${ORT_VERSION}" --repo microsoft/onnxruntime --pattern "$ASSET" --dir "$dir"; then
            return 0
        fi
        echo "gh release download failed, falling back to curl" >&2
    fi
    local url="https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/${ASSET}"
    echo "downloading $url"
    curl -fsSL --retry 5 --retry-delay 15 --retry-all-errors -o "$dir/$ASSET" "$url"
}

if [[ -f "$ORT_DYLIB" ]]; then
    echo "ONNX Runtime $ORT_VERSION already at $ORT_DYLIB"
else
    TMP="$(mktemp -d)"
    download_asset "$TMP"
    mkdir -p "$ORT_DIR"
    tar -xzf "$TMP/$ASSET" -C "$ORT_DIR"
    rm -rf "$TMP"
    echo "installed ONNX Runtime $ORT_VERSION to $ORT_DIR"
fi
