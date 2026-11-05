#!/usr/bin/env bash
# Runs every language's tests against the same shared cases.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"

echo "== Rust"
(cd "$root" && cargo test --quiet)

echo "== WebAssembly build"
"$root/scripts/build-wasm.sh"

echo "== Python"
(cd "$root/bindings/python" && uv run --quiet --group dev pytest -q)
(cd "$root/bindings/python" && uv run --quiet --group dev python ../../conformance/acceptance/check.py ../../conformance)

echo "== JavaScript"
(cd "$root/bindings/js" && node --test)
(cd "$root/bindings/js" && npx --yes typescript@5.9.3 tsc --noEmit --strict --module nodenext --target es2022 test/types.ts)

echo "== Go"
(cd "$root/bindings/go" && CGO_ENABLED=0 go test ./... && go test -race ./...)

echo "== Built packages"
"$root/scripts/acceptance.sh" npm
"$root/scripts/acceptance.sh" rust
"$root/scripts/acceptance.sh" go
