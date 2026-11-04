#!/usr/bin/env bash
# Installs the wheel in dist/ and runs the Python tests against it on the oldest and newest
# supported Python, because one wheel serves every version from 3.10 up.
set -euo pipefail

cd "$(dirname "$0")/.."

for python in 3.10 3.14; do
  rm -rf .wheel-venv
  uv venv --quiet --python "$python" .wheel-venv
  uv pip install --quiet --python .wheel-venv dist/*.whl pydantic pytest
  # Tests run from outside the source tree so they can only import the installed wheel.
  (cd bindings/python/tests && uv run --quiet --no-project --python ../../../.wheel-venv \
    python -m pytest -q -p no:cacheprovider --rootdir=. .)
done
rm -rf .wheel-venv
