#!/usr/bin/env bash
# Stops a release unless the commit is on main, every package has the tag's version, and
# Cargo.lock is up to date.
# Usage: scripts/check-release.sh v1.2.3 or v1.2.3-rc.1
set -euo pipefail

cd "$(dirname "$0")/.."

tag="$1"
version="${tag#v}"
status=0

if ! printf '%s' "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-rc\.[0-9]+)?$'; then
  echo "$tag isn't a release tag: it needs the form v1.2.3 or v1.2.3-rc.1" >&2
  exit 1
fi

# A tag publishes whatever commit it points at, so that has to be code that reached main. An
# automatic release adds one commit on top, which may only set versions. The branch goes by its
# full name, because a tag called origin/main would otherwise take its place.
git fetch --quiet origin main
main=refs/remotes/origin/main
if ! git merge-base --is-ancestor HEAD "$main"; then
  version_files=(crates/appsync-scalars/Cargo.toml bindings/python/pyproject.toml
    bindings/js/package.json Cargo.lock bindings/python/uv.lock)
  other_files="$(git diff --name-only HEAD^ HEAD -- . "${version_files[@]/#/:(exclude)}")"
  # Every changed line in those files has to be a version line, so the commit can't slip in a
  # package script or a dependency under the name of a version change. uv.lock also records the
  # lockfile format's revision, which changes with the uv version that wrote it.
  other_lines="$(git diff -U0 HEAD^ HEAD -- "${version_files[@]}" \
    | grep -E '^[-+]' | grep -Ev '^(\+\+\+|---) ' \
    | grep -Ev '^[-+] *((version = |"version": )"[^"]*",?|revision = [0-9]+)$' || true)"
  if [ -n "$other_files" ] || [ -n "$other_lines" ] || ! git merge-base --is-ancestor HEAD^ "$main"; then
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
