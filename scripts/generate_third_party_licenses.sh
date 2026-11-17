set -euo pipefail

cargo about generate --workspace --all-features about.template.md -o THIRD_PARTY_CARGO_LICENSES.md
sed -i 's/\r$//' THIRD_PARTY_CARGO_LICENSES.md
