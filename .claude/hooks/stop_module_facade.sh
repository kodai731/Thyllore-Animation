#!/usr/bin/env bash
set -uo pipefail

if ! command -v jq >/dev/null 2>&1; then
    echo "stop hook: jq is not on PATH, module_facade check skipped" >&2
    exit 0
fi

hook_input="$(cat)"

if [[ "$(jq -r '.stop_hook_active // false' <<<"$hook_input")" == "true" ]]; then
    exit 0
fi

session_directory="$(jq -r '.cwd // empty' <<<"$hook_input")"
REPO_ROOT="$(git -C "${session_directory:-$PWD}" rev-parse --show-toplevel 2>/dev/null)" || exit 0

if [[ ! -f "$REPO_ROOT/scripts/check_module_facade.sh" ]]; then
    exit 0
fi

if ! git -C "$REPO_ROOT" status --porcelain --untracked-files=all | grep -qE '(\.rs|Cargo\.toml)$'; then
    exit 0
fi

if ! bash "$REPO_ROOT/scripts/check_module_facade.sh"; then
    exit 2
fi
