#!/usr/bin/env bash
# Tags a release if this commit changes a published package since the last release, and prints
# the tag. Prints nothing if there's nothing to release.
#
# It uses a version set by hand if that hasn't been released yet, and otherwise the next patch
# version. That version goes in a commit on top of this one, which only the tag points at, so
# main never needs a commit from CI.
set -euo pipefail

cd "$(dirname "$0")/.."

# What ends up in a package. Tests and lockfiles aren't.
package_paths=(
  crates/appsync-scalars
  crates/appsync-scalars-wasm
  bindings
  scripts/build-wasm.sh
  ':(exclude)crates/appsync-scalars/tests'
  ':(exclude)bindings/python/tests'
  ':(exclude)bindings/python/uv.lock'
  ':(exclude)bindings/js/test'
  ':(exclude)bindings/go/*_test.go'
)

git fetch --quiet --tags origin

# The last release is the one built from the commit closest to this one. Dates can't say, because
# a tag's commit can be made long after the merge it released.
fewest_commits_since=""
for tag in $(git tag --list 'v[0-9]*'); do
  base="$(git merge-base "$tag" HEAD)"
  commits_since="$(git rev-list --count "$base..HEAD")"
  if [ -z "$fewest_commits_since" ] || [ "$commits_since" -lt "$fewest_commits_since" ]; then
    fewest_commits_since="$commits_since"
    last_release="$tag"
    released_from="$base"
  fi
done

if git diff --quiet "$released_from" HEAD -- "${package_paths[@]}"; then
  echo "No published package changed since $last_release" >&2
  exit 0
fi

current="$(sed -n 's/^version = "\(.*\)"$/\1/p' crates/appsync-scalars/Cargo.toml)"
if [ -n "$(git tag --list "v$current")" ]; then
  latest_stable="$(git tag --list 'v[0-9]*' | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1)"
  IFS=. read -r major minor patch <<< "${latest_stable#v}"
  version="$major.$minor.$((patch + 1))"
else
  version="$current"
fi

if [ "$version" != "$current" ]; then
  scripts/set-version.sh "$version" > /dev/null
  git add crates/appsync-scalars/Cargo.toml bindings/python/pyproject.toml bindings/js/package.json \
    Cargo.lock bindings/python/uv.lock
  git -c user.name='github-actions[bot]' \
    -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
    commit --quiet -m "Release $version"
fi

git tag "v$version"
git push --quiet origin "refs/tags/v$version"
echo "v$version"
