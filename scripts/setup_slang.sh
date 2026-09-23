#!/usr/bin/env bash
# Installs the slangc release pinned in crates/thyllore-shader-manifest/Cargo.toml into SLANG_ROOT
# (default ~/.local/slang) for CI and fresh machines; build.rs refuses any other version.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PINNED_VERSION="$(sed -n '/^\[package.metadata.slang\]/,/^\[/{s/^version = "\(.*\)"$/\1/p}' "$REPO_ROOT/crates/thyllore-shader-manifest/Cargo.toml")"
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

if [[ -x "$SLANG_ROOT/bin/slangc" ]] && "$SLANG_ROOT/bin/slangc" -v 2>&1 | grep -q "$SLANG_VERSION"; then
    echo "slangc $SLANG_VERSION already at $SLANG_ROOT"
else
    URL="https://github.com/shader-slang/slang/releases/download/v${SLANG_VERSION}/${ASSET}"
    TMP="$(mktemp -d)"
    echo "downloading $URL"
    curl -sSL -o "$TMP/$ASSET" "$URL"
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
