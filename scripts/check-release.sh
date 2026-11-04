#!/usr/bin/env bash
# Stops a release unless every package has the tag's version and the Go module has its tag.
# Usage: scripts/check-release.sh v1.2.3
set -euo pipefail

cd "$(dirname "$0")/.."

tag="$1"
version="${tag#v}"
status=0

check() {
  if [ "$2" != "$version" ]; then
    echo "$1 has version $2, but the tag is $tag" >&2
    status=1
  fi
}

check crates/appsync-scalars/Cargo.toml "$(sed -n 's/^version = "\(.*\)"$/\1/p' crates/appsync-scalars/Cargo.toml)"
check bindings/python/pyproject.toml "$(sed -n 's/^version = "\(.*\)"$/\1/p' bindings/python/pyproject.toml)"
check bindings/js/package.json "$(sed -n 's/^  "version": "\(.*\)",$/\1/p' bindings/js/package.json)"

# Go finds a module in a subdirectory by a tag with the directory as a prefix.
if [ -z "$(git ls-remote origin "refs/tags/bindings/go/$tag")" ]; then
  echo "tag bindings/go/$tag is missing, so go get would not find this version" >&2
  status=1
fi

exit "$status"
