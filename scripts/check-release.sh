#!/usr/bin/env bash
# Stops a release unless every package has the tag's version and Cargo.lock is up to date.
# Usage: scripts/check-release.sh v1.2.3 or v1.2.3-rc.1
set -euo pipefail

cd "$(dirname "$0")/.."

tag="$1"
version="${tag#v}"
status=0

check() {
  if [ "$2" != "$3" ]; then
    echo "$1 has version $2, but the tag $tag needs $3" >&2
    status=1
  fi
}

# Python writes a release candidate as 1.2.3rc1 rather than 1.2.3-rc.1.
python_version="$(printf '%s' "$version" | sed -E 's/-rc\.([0-9]+)$/rc\1/')"

check crates/appsync-scalars/Cargo.toml "$(sed -n 's/^version = "\(.*\)"$/\1/p' crates/appsync-scalars/Cargo.toml)" "$version"
check bindings/python/pyproject.toml "$(sed -n 's/^version = "\(.*\)"$/\1/p' bindings/python/pyproject.toml)" "$python_version"
check bindings/js/package.json "$(sed -n 's/^  "version": "\(.*\)",$/\1/p' bindings/js/package.json)" "$version"

# Publishing to crates.io uses --locked, and would otherwise fail after other registries succeed.
if ! cargo metadata --locked --format-version 1 > /dev/null; then
  echo "Cargo.lock is out of date, so run cargo check and commit it" >&2
  status=1
fi

exit "$status"
