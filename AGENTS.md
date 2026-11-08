# AGENTS.md

Guidance for AI agents (and humans) working in this repository.

## Overview

yaml-fm-lint lints the YAML front matter of markdown files. Version 2.0 is a
Rust rewrite of the Node.js tool. **1.x lives on the `v1` branch**; `main`
holds the 2.0 workspace, which is still being built. The CLI is currently a
stub: `--version` works and everything else exits with code 2.

The owner does not write Rust. Keep the code plain and easy to read.

## Layout

```
yaml-fm-lint/
├─ AGENTS.md, CLAUDE.md (@AGENTS.md), README.md, LICENSE
├─ Cargo.toml              # workspace; edition 2024; resolver 3; shared [workspace.lints]
├─ rust-toolchain.toml     # pinned stable; MSRV (1.88) is checked separately
├─ clippy.toml, .cargo/config.toml (the `cargo xtask` alias)
├─ crates/
│  ├─ core/   -> `yaml-fm-lint-core` (lib; no I/O of any kind)
│  │   ├─ src/  # extract, parse, tree, data, document, line_index, config, diagnostic,
│  │   │        # disable (comments), lint (rule runner), fix + edit (verified fixes),
│  │   │        # linter (public API), invariants (feature `invariants`, for fuzzing),
│  │   │        # rule_id, rules/<rule_id>.rs (one module per rule)
│  │   └─ tests/  # snapshots.rs, fix_props.rs, limits.rs
│  ├─ cli/    -> `yaml-fm-lint` (bin)
│  └─ wasm/   -> `yaml-fm-lint-wasm` (planned)
├─ xtask/                  # `cargo xtask <command>`; commands: parity-record, bench-corpus
├─ tests/fixtures/1x/      # markdown fixtures carried over from 1.x
├─ tests/fixtures/edge/    # edge-case fixtures (CRLF, BOM, unicode, ...); see its README
├─ tests/parity/expected-1x/  # recorded 1.7.1 results, one JSON per fixture
├─ deny.toml, renovate.json   # cargo-deny policy; Renovate config
├─ .github/workflows/ci.yml, fuzz-nightly.yml, .gitattributes, .gitignore
├─ CHANGELOG.md, SECURITY.md  # (planned)
├─ npm/                    # (planned) launcher and platform packages
├─ editors/vscode/         # (planned)
├─ schemas/, docs/         # (planned) generated
├─ fuzz/                   # cargo-fuzz targets (lint, fix); own workspace and Cargo.lock
└─ bench/                  # baseline.sh (hyperfine), RESULTS.md, results/*.json
```

## Commands

Run these from the repository root:

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
cargo +1.88 check -p yaml-fm-lint-core -p yaml-fm-lint --locked   # MSRV
cargo deny check                                                   # advisories, bans, licences, sources
```

All of them must pass before you commit. `Cargo.lock` is committed.

Other xtask commands (not part of the pre-commit checks):

```sh
cargo xtask parity-record   # run ONCE; needs node + npm; Linux/macOS only; results are committed
cargo xtask bench-corpus --files 1000 --seed 1 --out target/corpus-demo   # deterministic demo data in a throwaway folder
```

Do not point `bench-corpus` at `bench/corpus/10k` or `bench/corpus/100k`:
`bench/baseline.sh` owns those folders (seed 42) and refuses to run on any
other seed or file count. `bench/corpus/` is git-ignored.

Workflow checks (Docker; run after editing anything under `.github/`):

```sh
docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 -color=false
docker run --rm -v "$PWD:/repo" -w /repo ghcr.io/zizmorcore/zizmor:1.32.0 --offline .github/workflows/
npx --yes --package renovate@44.216.0 renovate-config-validator renovate.json
```

`cargo xtask parity-record` installs 1.7.1 into `target/parity-1x` and records
what it reports for every fixture. It is run once and the JSON in
`tests/parity/expected-1x/` is committed; re-run it only when fixtures change.
Fixtures and parity records are `-text` in `.gitattributes`, so never
normalise their line endings.

## Adding a rule

1. Add a variant to `RuleId` in `crates/core/src/rule_id.rs` (with its name, default
   level and flags) and a row to its tests.
2. Create `crates/core/src/rules/<rule_id>.rs` with the module-doc sections `# Why`,
   `# Examples` (bad and good) and, if it has a fix, `# Fix`; declare the module in
   `rules/mod.rs`.
3. Write the case table and run it: it must fail first.
4. Implement `check` and register it in `RULES` in `rules/mod.rs`.
5. If it has a fix, declare its `DataEffect` honestly: the fix engine re-parses the
   result and refuses any fix whose real effect on the data differs from the declared one.
6. Review the snapshot changes (`cargo insta review`), reading each one.
7. Run the standard check (the commands in "Commands" above).

## Fuzzing and coverage

```sh
cargo install cargo-llvm-cov@0.9.1 cargo-fuzz@0.13.2 --locked   # once
rustup component add llvm-tools-preview                        # once
cargo llvm-cov -p yaml-fm-lint-core --fail-under-lines 80      # coverage gate (CI job `coverage`)
rustup toolchain install nightly-2026-11-03 --profile minimal  # once; fuzzing needs nightly
cd fuzz && mkdir -p corpus/lint && cargo +nightly-2026-11-03 fuzz run lint corpus/lint ../tests/fixtures/1x ../tests/fixtures/edge -- -max_total_time=60
```

Targets: `lint` (every range is in bounds and on character boundaries) and
`fix` (`invariants::check_fix` holds). A crash is written to `fuzz/artifacts/<target>/`:
turn it into a regression test in the owning module, then fix the code.
`libfuzzer-sys` compiles C++, so the machine needs a C++ compiler. The `fuzz`
job in `ci.yml` runs each target for 60 seconds; `fuzz-nightly.yml` runs them for 15 minutes every night.

The nightly date (`nightly-2026-11-03`) is pinned in `ci.yml`, `fuzz-nightly.yml`
and here. Renovate cannot track it, so bump it by hand in all three places (and
`cargo +<new date> fuzz build` to check) when a nightly job breaks, or about once
a quarter. `fuzz/Cargo.lock` is committed; `fuzz/` is excluded from the root workspace.

## Snapshot tests

`crates/core/tests/snapshots.rs` snapshots what `lint` and `fix` do to every
fixture in `tests/fixtures/{1x,edge}` (default config); the `.snap` files live in
`crates/core/tests/snapshots/` and are `-text` in `.gitattributes`.

```sh
cargo install cargo-insta@1.49.0 --locked   # once
cargo insta test -p yaml-fm-lint-core       # run and collect changed snapshots
cargo insta review                          # accept or reject them one by one
```

Never accept a snapshot you haven't read: a changed snapshot is either a
deliberate behaviour change or a bug, and only reading it tells you which.
Fix the code, not the snapshot, when it shows a bug.
Some snapshots end lines with significant trailing spaces (block-scalar fixtures), so
never let an editor trim whitespace in `crates/core/tests/snapshots/*.snap`.

## Code rules

- No panics: `unwrap`, `expect`, `panic!`, `todo!` and indexing/slicing are
  denied by clippy outside tests. Return errors or use `get`.
- No `unsafe`. It is forbidden workspace-wide.
- Plain Rust: no custom macros, no type-state patterns, generics only where a
  dependency requires them.
- `yaml-fm-lint-core` does no filesystem, process, network or environment
  access, ever. Callers pass in text and receive results.
- `thiserror` in the core, `anyhow` in the CLI and `xtask`. User-facing
  messages never mention Rust internals.
- `core` has `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]`.
- `xtask` is a developer tool that never ships, so it does not inherit the
  workspace lints.

## Security rules (never break these)

- No executable config, ever. Config is JSON, parsed as data only.
- No network crates (`reqwest`, `hyper`, `ureq`, `curl`, `native-tls`,
  `openssl`); `cargo-deny` will ban them. Crates come from crates.io only.
- Reject schema `$ref` to other documents.
- Bound YAML input (size and nesting limits); build trees iteratively or with
  a depth limit.
- Use only the linear-time `regex` engine for schema `pattern`.
- `--fix` must re-parse and verify after each pass, never touch the body,
  write atomically and never write through symlinks.
- Supply chain: pin GitHub Actions to full commit SHAs, top-level
  `permissions: contents: read`, no `pull_request_target`,
  `persist-credentials: false` on checkout, no install scripts (npm installs
  use `--ignore-scripts`), allowed licences only (MIT,
  Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0, Zlib).

## Dependency updates (Renovate)

`renovate.json` auto-merges only dev-only minor/patch updates and GitHub
Actions minor, patch and digest updates (actions are SHA-pinned with a
`# vX.Y.Z` comment, so Renovate reports a bump such as v7.0.1 to v7.0.2 as
minor or patch, not as a digest change); runtime dependencies and every major
update need a manual merge. Renovate must never change the workspace `rust-version` (the
MSRV, 1.88). The MSRV is a deliberate policy: it is raised by hand, only when
a dependency needs a newer compiler. Renovate's cargo manager (44.216.0) does
not read `rust-version` at all, so no rule is needed today; if a future
Renovate version starts reporting it, add a `packageRule` disabling that
dependency type. `rust-toolchain.toml` (the pinned stable toolchain) is bumped
by Renovate's separate `rust-toolchain` manager, which is intended.

## Release rules

Agents never tag, publish, push, open pull requests, or change GitHub, npm or
Marketplace settings (including the `release` environment) without the
owner's explicit go-ahead at that moment.

## Ledgers

Non-trivial work keeps a ledger under `.llms/<topic>/` (a `status.md` with
goal, decisions, done, next, blockers). `.llms/` is git-ignored: never commit
it and never store secrets in it.

## Benchmarks

`bash bench/baseline.sh` (from the repo root) records the 1.7.1 and 1.8.0
baselines: it installs both versions, generates the data, runs a checked run
against the generator's manifest, then times four scenarios into
`bench/results/*.json`. Pass scenario names (`one-file`, `10k`, `100k`,
`fix-10k`) to run only some. Results and machine details live in
`bench/RESULTS.md`; README performance claims cite only that file.
