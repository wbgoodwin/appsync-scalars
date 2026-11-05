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

GitHub Actions runs the tests, and builds and tests each package, for every pull request and push to `main`.

Merging a change to a published package releases it. Once the checks pass on `main`, the workflow picks the next patch version and tags it. It then repeats every check, publishes to crates.io, PyPI and npm, adds the `bindings/go/v1.2.3` tag Go needs, and installs each package from its registry to check it against every recorded AppSync result. Merges that only touch CI, tests or this README don't release anything.

For a minor or major version, set it in your pull request, and the merge releases that version instead:

```bash
scripts/set-version.sh 1.3.0
```

The version on `main` can lag behind the latest release, because each patch version goes in a commit that only its tag points at. The tags are the record of what was released.

Rulesets stop release tags being moved or deleted, because registries and Go's checksum database keep a version forever. If a release fails partway, rerun the failed jobs, which skip anything already on a registry. If its run never started, start it from the tag:

```bash
gh workflow run ci.yml --ref v1.2.3
```

Publishing stores no tokens. Each registry has a trusted publisher for this repository, with the workflow `ci.yml` and the environment `release`. `main` only takes pull requests that pass the `ci-passed` check.

### Release candidates

To try a release on the real registries first, set a version like `1.2.3-rc.1` in your pull request. The merge releases it through the same workflow, and normal installs keep getting the last stable version. Release candidates stay in each registry's version history, so save them for risky changes. When you're done testing, set the final version the same way, because other merges carry on from the last stable version.

```bash
scripts/set-version.sh 1.2.3-rc.1
```

| Language | Install a release candidate |
|---|---|
| Python | `pip install appsync-scalars==1.2.3rc1` |
| JavaScript and TypeScript | `npm install appsync-scalars@next` |
| Rust | `cargo add appsync-scalars@=1.2.3-rc.1` |
| Go | `go get github.com/wbgoodwin/appsync-scalars/bindings/go@v1.2.3-rc.1` |
