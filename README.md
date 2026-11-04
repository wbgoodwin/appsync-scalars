# appsync-scalars

Validates the AWS AppSync date and time scalars (`AWSDate`, `AWSTime`, `AWSDateTime` and `AWSTimestamp`) with one Rust implementation, available in Rust, Python, JavaScript and Go.

The rules come from results recorded from a live AppSync API, which differ from AWS's written descriptions. See [What AppSync accepts](#what-appsync-accepts).

## Usage

| Language | Install | Guide |
|---|---|---|
| Python | `pip install appsync-scalars` | [Python README](bindings/python/README.md), including Pydantic field types |
| JavaScript and TypeScript | `npm install appsync-scalars` | [JavaScript README](bindings/js/README.md) |
| Rust | `cargo add appsync-scalars` | [Rust README](crates/appsync-scalars/README.md) |
| Go | `go get github.com/wbgoodwin/appsync-scalars/bindings/go` | [Go README](bindings/go/README.md) |

## What AppSync accepts

AppSync's descriptions of these scalars are accurate for ordinary values. It also accepts and rejects some values they don't mention, and this library follows AppSync:

| Value | Example | AppSync's behavior |
|---|---|---|
| Lowercase `z` and `t` | `2021-01-11t12:30:24z` | Accepted |
| Empty fraction | `12:00:34.` | Accepted |
| Signed years of 5 to 10 digits | `+12021-01-11`, `-12021-01-11` | Accepted, up to 999,999,999 either way |
| `+` with a 4-digit year | `+2021-01-11` | Rejected |
| Negative year zero | `-0000-01-11` | Rejected |
| Hours-only offset | `+05` | Accepted by `AWSDateTime`, rejected by `AWSDate` and `AWSTime` |
| Offset beyond 18 hours | `+18:01` | Rejected by `AWSDateTime`, accepted by `AWSDate` and `AWSTime` up to `+23:59:59` |

`AWSTimestamp` follows its documented limits, -31557014167219200 to 31556889864403199.

AppSync gave the same verdict for a GraphQL variable and for resolver output on every recorded value. Inline literals differ in one way: AppSync wraps oversized integers, so `18446744073709551616` written in a query reaches the resolver as `0`. This library calls that value invalid, as AppSync does for variables and output.

## How it fits together

| Path | Purpose |
|---|---|
| [`crates/appsync-scalars`](crates/appsync-scalars/src/lib.rs) | The validator: Rust with no dependencies |
| [`crates/appsync-scalars-wasm`](crates/appsync-scalars-wasm/src/lib.rs) | The WebAssembly module, about 4 KB, shared by JavaScript and Go |
| [`bindings/python`](bindings/python/pyproject.toml) | Native extension built with PyO3 and maturin, plus the Pydantic types |
| [`bindings/js`](bindings/js/index.js) | npm package that embeds the module, so it has no install step or platform-specific builds |
| [`bindings/go`](bindings/go/appsyncscalars.go) | Go module that runs the embedded module through wazero, with no cgo |
| [`conformance/smoke.json`](conformance/smoke.json) | Test cases every language runs |
| [`conformance/`](conformance/README.md) | The live AppSync test suite |

Other languages with a WebAssembly runtime can load the same module. Its interface is described in [`crates/appsync-scalars-wasm/src/lib.rs`](crates/appsync-scalars-wasm/src/lib.rs).

## Development

You need `rustup`, which installs the pinned Rust version, plus `uv`, Node.js 20 or later, and Go 1.25 or later.

Run every language's tests, including installing each built package in an empty project:

```bash
scripts/test-all.sh
```

Check a published version the same way, with `python`, `npm`, `rust` or `go`:

```bash
scripts/acceptance.sh python v1.2.3
```

After changing the validator, rebuild the WebAssembly module and commit the Go copy, because `go get` only sees files in the repository:

```bash
scripts/build-wasm.sh
```

## Releasing

GitHub Actions runs the tests, and builds and tests a wheel on Linux, macOS and Windows, for every pull request and push to `main`.

To release, set the new version in `crates/appsync-scalars/Cargo.toml`, `bindings/python/pyproject.toml` and `bindings/js/package.json`, and run `cargo check` to update `Cargo.lock`. Merge that to `main`, then push a tag. The tag must match those versions, or the release stops before publishing anything.

```bash
git tag v1.2.3 && git push origin v1.2.3
```

The workflow repeats every check, publishes to crates.io, PyPI and npm, and adds the `bindings/go/v1.2.3` tag Go needs. It then installs each package from its registry and checks it against every recorded AppSync result. It skips anything already on a registry, so rerunning a partly failed release is safe.

Publishing stores no tokens. Each registry needs a trusted publisher for this repository, with the workflow `ci.yml` and the environment `release`. PyPI can set one up before the first release. crates.io and npm need the first version published by hand, and npm's publisher must allow `npm publish`.

### Release candidates

To try a release on the real registries first, use a version like `1.2.3-rc.1`, written as `1.2.3rc1` in `pyproject.toml`, and push the tag `v1.2.3-rc.1`. The same workflow runs, and normal installs keep getting the last stable version. Release candidates stay in each registry's version history, so save them for risky changes.

| Language | Install a release candidate |
|---|---|
| Python | `pip install appsync-scalars==1.2.3rc1` |
| JavaScript and TypeScript | `npm install appsync-scalars@next` |
| Rust | `cargo add appsync-scalars@=1.2.3-rc.1` |
| Go | `go get github.com/wbgoodwin/appsync-scalars/bindings/go@v1.2.3-rc.1` |
