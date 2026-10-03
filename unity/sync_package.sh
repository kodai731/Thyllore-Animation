#!/usr/bin/env bash
set -euo pipefail

# Copies unity/com.thyllore.avatar into <work dir>/package and registers it in
# <work dir>/project/Packages/manifest.json as a local (file:) package. The copy
# keeps Unity's generated .meta files out of the repo. package.json is the single
# source of the package name.

UNITY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK_DIR="${1:?usage: sync_package.sh <work dir with project/>}"
PACKAGE_NAME="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["name"])' "$UNITY_DIR/com.thyllore.avatar/package.json")"
PACKAGE_COPY="$WORK_DIR/package/$PACKAGE_NAME"

rm -rf "$PACKAGE_COPY"
mkdir -p "$PACKAGE_COPY"
cp -r "$UNITY_DIR/com.thyllore.avatar/." "$PACKAGE_COPY/"

python3 - "$WORK_DIR/project/Packages/manifest.json" "$PACKAGE_NAME" <<'PYEOF'
import json, sys
path, name = sys.argv[1], sys.argv[2]
manifest = json.load(open(path))
manifest["dependencies"][name] = f"file:../../package/{name}"
manifest["dependencies"]["com.unity.test-framework"] = "1.1.33"
json.dump(manifest, open(path, "w"), indent=2)
PYEOF
echo "$PACKAGE_NAME"
