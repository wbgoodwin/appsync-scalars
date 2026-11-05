#!/usr/bin/env bash
# Installs a package the way a user would, in an empty project, then checks it against the shared
# cases and the recorded AppSync results its API can express.
#
# Usage: scripts/acceptance.sh <python|npm|rust|go> [tag]
#
# With a tag like v1.2.3, the package comes from its registry. Without one, it's built from this
# checkout. Python uses the wheel in dist/, and npm needs scripts/build-wasm.sh to have run.
set -euo pipefail

cd "$(dirname "$0")/.."

language="$1"
tag="${2:-}"
version="${tag#v}"
root="$PWD"
project="$(mktemp -d)"

# A new version can take a few minutes to reach a registry, so only those installs are retried.
retry_install() {
  local attempt
  for attempt in 1 2 3 4 5 6 7 8 9 10; do
    if "$@"; then
      return 0
    fi
    if [ -z "$tag" ] || [ "$attempt" = 10 ]; then
      return 1
    fi
    echo "Not available yet, so trying again in 30 seconds"
    sleep 30
  done
}

check_python() {
  uv run --quiet --no-project --isolated "$@" python conformance/acceptance/check.py conformance
}

case "$language" in
  python)
    if [ -n "$tag" ]; then
      # Python writes a release candidate as 1.2.3rc1 rather than 1.2.3-rc.1.
      python_version="$(printf '%s' "$version" | sed -E 's/-rc\.([0-9]+)$/rc\1/')"
      retry_install check_python --refresh-package appsync-scalars --with "appsync-scalars==$python_version"
    else
      check_python --with "./$(echo dist/*.whl)"
    fi
    ;;
  npm)
    cp conformance/acceptance/check.mjs "$project/"
    if [ -n "$tag" ]; then
      source="appsync-scalars@$version"
    else
      (cd bindings/js && npm pack --silent --ignore-scripts --pack-destination "$project" > /dev/null)
      source="$(echo "$project"/appsync-scalars-*.tgz)"
    fi
    cd "$project"
    npm init -y > /dev/null
    retry_install npm install --silent --no-audit --no-fund --prefer-online "$source"
    node check.mjs "$root/conformance"
    ;;
  rust)
    cp -R conformance/acceptance/rust/. "$project/"
    if [ -n "$tag" ]; then
      dependency=("appsync-scalars@=$version")
    else
      cargo package --quiet --no-verify --allow-dirty -p appsync-scalars
      tar xzf "$(ls -t target/package/appsync-scalars-*.crate | head -1)" -C "$project"
      dependency=(--path "$(echo "$project"/appsync-scalars-*/)")
    fi
    cd "$project"
    retry_install cargo add --quiet "${dependency[@]}"
    cargo run --quiet -- "$root/conformance"
    ;;
  go)
    cp conformance/acceptance/go/* "$project/"
    module=github.com/wbgoodwin/appsync-scalars/bindings/go
    cd "$project"
    if [ -n "$tag" ]; then
      retry_install go get "$module@$tag"
    else
      go mod edit -require="$module@v0.0.0" -replace="$module=$root/bindings/go"
      go mod tidy
    fi
    go run . "$root/conformance"
    ;;
  *)
    echo "unknown language $language" >&2
    exit 2
    ;;
esac
