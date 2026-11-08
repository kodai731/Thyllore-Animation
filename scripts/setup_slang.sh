#!/usr/bin/env bash
# Installs the slangc release pinned in crates/thyllore-shader-manifest/Cargo.toml into SLANG_ROOT
# (default ~/.local/slang) for CI and fresh machines; build.rs refuses any other version.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PINNED_VERSION="$(awk -F'"' '/^\[package.metadata.slang\]/ { in_slang = 1; next } /^\[/ { in_slang = 0 } in_slang && /^version = / { print $2; exit }' "$REPO_ROOT/crates/thyllore-shader-manifest/Cargo.toml")"
if [[ -z "$PINNED_VERSION" ]]; then
    echo "no [package.metadata.slang] version in crates/thyllore-shader-manifest/Cargo.toml" >&2
    exit 2
fi
SLANG_VERSION="${SLANG_VERSION:-$PINNED_VERSION}"
SLANG_ROOT="${SLANG_ROOT:-$HOME/.local/slang}"

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)   ASSET="slang-${SLANG_VERSION}-linux-x86_64.tar.gz" ;;
    Linux-aarch64)  ASSET="slang-${SLANG_VERSION}-linux-aarch64.tar.gz" ;;
    Darwin-arm64)   ASSET="slang-${SLANG_VERSION}-macos-aarch64.zip" ;;
    Darwin-x86_64)  ASSET="slang-${SLANG_VERSION}-macos-x86_64.zip" ;;
    MINGW*|MSYS*|CYGWIN*) ASSET="slang-${SLANG_VERSION}-windows-x86_64.zip" ;;
    *) echo "unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 2 ;;
esac

# The browser download URL (github.com/.../releases/download/...) intermittently answers 504 with an
# HTML body; the release API serves the same asset reliably, so it is tried first when gh has a token.
download_asset() {
    local dir="$1"
    if [[ -n "${GH_TOKEN:-${GITHUB_TOKEN:-}}" ]] && command -v gh >/dev/null 2>&1; then
        echo "downloading $ASSET from the shader-slang/slang v${SLANG_VERSION} release through gh"
        if GH_TOKEN="${GH_TOKEN:-$GITHUB_TOKEN}" gh release download "v${SLANG_VERSION}" --repo shader-slang/slang --pattern "$ASSET" --dir "$dir"; then
            return 0
        fi
        echo "gh release download failed, falling back to curl" >&2
    fi
    local url="https://github.com/shader-slang/slang/releases/download/v${SLANG_VERSION}/${ASSET}"
    echo "downloading $url"
    curl -fsSL --retry 5 --retry-delay 15 --retry-all-errors -o "$dir/$ASSET" "$url"
}

if [[ -x "$SLANG_ROOT/bin/slangc" ]] && "$SLANG_ROOT/bin/slangc" -v 2>&1 | grep -q "$SLANG_VERSION"; then
    echo "slangc $SLANG_VERSION already at $SLANG_ROOT"
else
    TMP="$(mktemp -d)"
    download_asset "$TMP"
    rm -rf "$SLANG_ROOT"
    mkdir -p "$SLANG_ROOT"
    case "$ASSET" in
        *.tar.gz) tar -xzf "$TMP/$ASSET" -C "$SLANG_ROOT" ;;
        *.zip)    unzip -q "$TMP/$ASSET" -d "$SLANG_ROOT" ;;
    esac
    rm -rf "$TMP"
    echo "installed slangc $SLANG_VERSION to $SLANG_ROOT"
fi

if [[ -n "${GITHUB_ENV:-}" ]]; then
    echo "SLANG_ROOT=$SLANG_ROOT" >> "$GITHUB_ENV"
fi
