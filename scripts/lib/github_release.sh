#!/usr/bin/env bash
# Sourced by the setup scripts: downloads one asset of a GitHub release.
# The browser download URL (github.com/.../releases/download/...) intermittently answers 504 or drops
# the connection; the release API serves the same asset reliably, so it is tried first when gh has a token.

download_github_release_asset() {
    local repo="$1" tag="$2" asset="$3" dir="$4"
    if [[ -n "${GH_TOKEN:-${GITHUB_TOKEN:-}}" ]] && command -v gh >/dev/null 2>&1; then
        echo "downloading $asset from the $repo $tag release through gh"
        if GH_TOKEN="${GH_TOKEN:-$GITHUB_TOKEN}" gh release download "$tag" --repo "$repo" --pattern "$asset" --dir "$dir"; then
            return 0
        fi
        echo "gh release download failed, falling back to curl" >&2
    fi
    local url="https://github.com/$repo/releases/download/$tag/$asset"
    echo "downloading $url"
    curl -fsSL --retry 5 --retry-delay 15 --retry-all-errors -o "$dir/$asset" "$url"
}
