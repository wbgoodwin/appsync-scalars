#!/usr/bin/env bash
# Stops a release unless the commit is on main, every package has the tag's version, and
# Cargo.lock is up to date.
# Usage: scripts/check-release.sh v1.2.3 or v1.2.3-rc.1
set -euo pipefail

cd "$(dirname "$0")/.."

tag="$1"
version="${tag#v}"
status=0

# A tag publishes whatever commit it points at, so that has to be code that reached main. An
# automatic release adds one commit on top, which may only set versions.
git fetch --quiet origin main
if ! git merge-base --is-ancestor HEAD origin/main; then
  other_changes="$(git diff --name-only HEAD^ HEAD -- . \
    ':(exclude)crates/appsync-scalars/Cargo.toml' ':(exclude)bindings/python/pyproject.toml' \
    ':(exclude)bindings/js/package.json' ':(exclude)Cargo.lock' ':(exclude)bindings/python/uv.lock')"
  if [ -n "$other_changes" ] || ! git merge-base --is-ancestor HEAD^ origin/main; then
    echo "$(git rev-parse --short HEAD) isn't on main, or is more than a version change on top of it" >&2
    status=1
  fi
fi

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
