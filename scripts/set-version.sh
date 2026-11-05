#!/usr/bin/env bash
# Sets the version of every published package.
# Usage: scripts/set-version.sh 1.2.3 or 1.2.3-rc.1
set -euo pipefail

cd "$(dirname "$0")/.."

version="$1"
# Python writes a release candidate as 1.2.3rc1 rather than 1.2.3-rc.1.
python_version="$(printf '%s' "$version" | sed -E 's/-rc\.([0-9]+)$/rc\1/')"

sed -i.bak -E "s/^version = \".*\"$/version = \"$version\"/" crates/appsync-scalars/Cargo.toml
sed -i.bak -E "s/^version = \".*\"$/version = \"$python_version\"/" bindings/python/pyproject.toml
sed -i.bak -E "s/^  \"version\": \".*\",$/  \"version\": \"$version\",/" bindings/js/package.json
rm crates/appsync-scalars/Cargo.toml.bak bindings/python/pyproject.toml.bak bindings/js/package.json.bak

cargo metadata --format-version 1 > /dev/null
(cd bindings/python && uv lock --quiet)
