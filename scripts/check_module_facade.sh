#!/usr/bin/env bash
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

test_output="$(cargo test -q -p thyllore-grpc-client --no-default-features --test module_facade 2>&1)"
test_status=$?

if [[ $test_status -eq 0 ]]; then
    exit 0
fi

violations="$(sed -n '/^---- .* stdout ----$/,/^failures:$/p' <<<"$test_output" | grep -vE '^(failures:|note: )')"

echo "module_facade failed (.claude/rules/module-structure.md):" >&2
if [[ -n "$violations" ]]; then
    echo "$violations" >&2
else
    tail -n 40 <<<"$test_output" >&2
fi
exit 1
