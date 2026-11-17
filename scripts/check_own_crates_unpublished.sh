#!/usr/bin/env bash
set -euo pipefail

publishable_own_crates=$(
    cargo metadata --format-version 1 --all-features --locked \
        | jq -r '.workspace_root as $root
            | .packages[]
            | select(.source == null)
            | select(.manifest_path | startswith($root + "/vendor/") | not)
            | select(.publish != [])
            | .manifest_path | ltrimstr($root + "/")'
)

if [ -n "$publishable_own_crates" ]; then
    echo "These crates of this repository lack 'publish = false', so cargo-about lists them as third-party:" >&2
    echo "$publishable_own_crates" >&2
    exit 1
fi
