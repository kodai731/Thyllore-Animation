#!/usr/bin/env bash
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [[ "$(jq -r '.stop_hook_active // false')" == "true" ]]; then
    exit 0
fi

if ! git -C "$REPO_ROOT" status --porcelain | grep -qE '(\.rs|Cargo\.toml)$'; then
    exit 0
fi

if ! bash "$REPO_ROOT/scripts/check_module_facade.sh"; then
    exit 2
fi
