# AGENTS.md

Repository workflow for `gsx/1`. [Architecture](docs/architecture.md) defines module boundaries; [contracts](docs/decisions/readme.md) define current behavior.

## Rules

- Format rules live in `gsx-core`; bindings translate types and errors only.
- Keep dependencies aligned with the [pipeline](docs/architecture.md#pipeline).
- Derive target rendering, declarations and capabilities from one rule table.
- Source ranges use UTF-16 units of the original input; output ranges follow the [wire contract](docs/decisions/bindings.md#wire-format).
- Tests cover library behavior and performance. The corpus is the acceptance test.
- Code comments are limited to required `SAFETY:` notes and `# Safety` sections; constraints belong in `docs/`.
- Repository text is English. Commit messages are one capitalized line.
- Write reference documentation in the present tense, without development history or duplicated contracts. Preserve constraints and decision rationale when condensing. Keep prose paragraphs on one line; format code examples for readability.

## Layout

| Path | Contents |
| --- | --- |
| `crates/`, `packages/`, `Package.swift` | [Core and bindings](docs/architecture.md) |
| `xtask/` | Builds, corpus generation, replay, release sets and benchmarks |
| `schema/` | Wire JSON Schema |
| `testdata/corpus/` | Generated acceptance results |
| `testdata/bench/` | Benchmark inputs and recorded measurements |
| `tools/` | Unicode generators and benchmark reports |
| `fuzz/` | Fuzz targets, seeds, dictionary and separate lockfile |

## Setup

Use rustup and the versions in [`rust-toolchain.toml`](rust-toolchain.toml), [`.nvmrc`](.nvmrc) and the npm manifest's [`devEngines`](packages/npm/package.json). Ensure Homebrew Rust does not shadow rustup. macOS package builds require Xcode; Wasm builds require Binaryen 133 (`wasm-opt`).

```bash
bindgen_version="$(cargo pkgid --locked -p wasm-bindgen | cut -d@ -f2)"
cargo install wasm-bindgen-cli --version "=$bindgen_version" --locked --root target/tools

rustup toolchain install nightly-2026-11-01 --profile minimal --component miri,rust-src
```

`xtask` finds wasm-bindgen in `target/tools/bin`; `WASM_BINDGEN` and `WASM_OPT` override executable paths. CI installation steps are in [ci.yml](.github/workflows/ci.yml).

## Checks

```bash
cargo fmt --all --check
CARGO_BUILD_WARNINGS=deny cargo clippy --workspace --all-targets --locked
CARGO_BUILD_WARNINGS=deny cargo clippy -p gsx-core -p gsx-wasm --locked --target wasm32-unknown-unknown
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo xtask header --check
cargo deny --locked check
cargo test --workspace --locked
cargo package -p gsx-core --locked
cargo +nightly-2026-11-01 miri test -p gsx-ffi --lib --locked
```

## Corpus

```bash
cargo xtask corpus
```

Regenerate expected results after a behavior change, review `git diff -- testdata/corpus`, and include the diff with the change. Never edit generated expectations by hand. Record new contract rules in `docs/decisions/`. Provider fixtures in `crates/gsx-core/tests/project/` are frozen; Unicode fixtures have dedicated [generators](docs/decisions/measure.md#unicode).

| File | Coverage |
| --- | --- |
| `syntax.jsonl` | Tokens and marker ranges |
| `document.jsonl` | Document trees |
| `diagnostics.jsonl` | Invalid input and diagnostic ranges |
| `editor.jsonl` | Incomplete input |
| `targets/*.jsonl` | Fields, measurements and changes |

`cargo test` compares corpus results exactly. Each case's `sources` identifies its syntax, publication or boundary requirement. Property checks cover panics, valid ranges, serializer idempotence, fitting within limits and determinism.

## Replay and performance

```bash
cargo xtask replay
cargo xtask replay npm
cargo xtask replay swift
cargo xtask replay python
```

Replay builds and installs packages as consumers, then compares raw parse, serialize and projection JSON with Rust byte for byte. Projection inputs run against every target with empty facts and a populated fact set covering placement, media, limits and threads. Generated size cases at and just over the `documentUnits` limit, including surrogate pairs and lone surrogates at the boundary, plus a lone surrogate well inside it, run parse, serialize and an `x-plain` projection, with empty facts and with a refused item ID. Generated limit cases set each limit fact, and all of them together, to 0, 1 and 4294967295 on every target with truncation, threads and story placement, so 32-bit Wasm arithmetic is compared with 64-bit Rust. Generated ceiling cases set each fact ceiling, at and one past its value, on every target and compare each binding's refusal message with Rust's. Media ID samples compare each binding's predicate answers and rule constants with Rust, and each consumer checks the media ID pattern against the predicate with its own regular expression engine. The full replay and Swift replay require macOS; npm and Python can run individually on Linux. `GSX_PYTHON` selects Python. npm replay enforces the 365000-byte `gzip -9` budget declared in [`xtask`](xtask/src/main.rs).

```bash
cargo xtask bench
cargo xtask bench --wasm
cargo bench -p gsx-core --bench instructions
mkdir -p fuzz/corpus/parse fuzz/corpus/project
cargo +nightly-2026-11-01 fuzz run parse fuzz/corpus/parse fuzz/seeds/parse -- -dict=fuzz/gsx.dict -max_len=4096
cargo +nightly-2026-11-01 fuzz run project fuzz/corpus/project fuzz/seeds/project -- -dict=fuzz/gsx.dict -max_total_time=60
```

Benchmarks write JSON and Markdown under `testdata/bench/baseline/`, identified by commit and runtime. Instruction benchmarks require Valgrind and a `gungraun-runner` matching `Cargo.lock`; CI compares them with the base commit and reports growth above 1% as a warning with the affected benchmarks in the job summary, without failing the check. Instruction counts provide the regression signal because shared-runner wall time is noisy. Fuzzing requires `cargo-fuzz`; CI runs both targets for 60 seconds with `fuzz/Cargo.lock` locked. New inputs go to the ignored `fuzz/corpus/`; the tracked seeds in `fuzz/seeds/` and the dictionary `fuzz/gsx.dict` start the fuzzers on `::time` formats, math, media figures and groups, thread facts, long paragraphs, blank line runs and nested containers. Each project seed is a fact header in the `arbitrary` layout of the target's `Input` followed by a parse seed; run `node fuzz/seeds.mjs` after changing `fuzz/seeds/parse/` or `Input`.

## Build

```bash
cargo xtask header
cargo xtask wasm
cargo xtask xcframework
cargo xtask wheel
cargo xtask dist
cargo xtask pin --check
```

`dist` defaults to `dist/v<workspace-version>` and requires an empty destination. It builds and replays the release set; `--wheels <directory>` adds Linux wheels built by CI. An optional destination follows `dist`. `pin [--check] [<release-set-directory>]` writes or checks the Swift binary URL and checksum, defaulting to that release directory.

Reproducible archives use fixed entry order, times and modes. Builds remap the workspace, Cargo home and Rust sources to `/gsx`, `/cargo` and `/rustc/<commit>` so the XCFramework checksum is independent of checkout paths. The Wasm profile uses `opt-level = "s"` for workspace code and `"z"` for dependencies.

## Release

The [Release workflow](.github/workflows/release.yml) verifies tag identity, runs gates, builds the release set, checks the Swift pin, verifies uploaded artifacts, publishes an immutable GitHub release and ships npm to GitHub Packages. Repository release immutability must be enabled.

1. Keep versions aligned in the workspace, npm and Python manifests and both Cargo lockfiles. The fuzz lockfile is updated with `cargo update -p gsx-core --manifest-path fuzz/Cargo.toml`.
2. Download the `release-set` artifact from the successful CI run for the release commit. Pin that set with `cargo xtask pin <release-set-directory>` and merge the pin to `main`.
3. Tag the release commit and push the annotated tag.
4. After the workflow succeeds, download the verified assets and publish to npmjs, PyPI and crates.io.

For the current release, replace `RUN_ID` with the successful CI run ID. Use empty directories for downloads. Publication commands require registry credentials.

```bash
gh run download RUN_ID --repo meritt/gsx.rs --name release-set --dir /tmp/gsx-set
cargo xtask pin /tmp/gsx-set
```

After merging the pin:

```bash
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

After the Release workflow succeeds:

```bash
gh release download vX.Y.Z --repo meritt/gsx.rs --pattern 'gsx-*' --dir /tmp/gsx-release

pnpm publish /tmp/gsx-release/gsx-vX.Y.Z-npm.tgz --access public
uv publish /tmp/gsx-release/gsx-X.Y.Z-py3-none-*.whl

git checkout vX.Y.Z
cargo publish -p gsx-core --locked
```

| Failure | Recovery |
| --- | --- |
| Identity, gates or build | Rerun transient failures. For a code correction before publication, delete the tag, fix `main` and retag. |
| Draft, readback or publish | Delete the unpublished release with `gh release delete vX.Y.Z --repo meritt/gsx.rs --yes`, then rerun. |
| Immutability verification | Rerun the failed job. |
| GitHub Packages upload | Publish the verified npm archive with `npm publish <archive> --registry https://npm.pkg.github.com` and a token with `write:packages`. |
| Incorrect published release | Publish a patch version. |
