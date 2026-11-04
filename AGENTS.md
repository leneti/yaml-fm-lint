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
│  ├─ cli/    -> `yaml-fm-lint` (bin)
│  └─ wasm/   -> `yaml-fm-lint-wasm` (planned)
├─ xtask/                  # `cargo xtask <command>`; no commands yet
├─ tests/fixtures/1x/      # markdown fixtures carried over from 1.x
├─ tests/parity/           # (planned) recorded 1.x results
├─ deny.toml, renovate.json, CHANGELOG.md, SECURITY.md   # (planned)
├─ npm/                    # (planned) launcher and platform packages
├─ editors/vscode/         # (planned)
├─ schemas/, docs/         # (planned) generated
├─ fuzz/                   # (planned) cargo-fuzz targets
└─ bench/                  # (planned) hyperfine scripts, RESULTS.md
```

## Commands

Run these from the repository root:

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
cargo +1.88 check -p yaml-fm-lint-core -p yaml-fm-lint --locked   # MSRV
cargo deny check                                                   # advisories, bans, licences, sources
cargo xtask <command>                                              # none yet
```

Workflow checks (Docker; run after editing anything under `.github/`):

```sh
docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 -color=false
docker run --rm -v "$PWD:/repo" -w /repo ghcr.io/zizmorcore/zizmor:1.32.0 --offline .github/workflows/
npx --yes --package renovate@44.216.0 renovate-config-validator renovate.json
```

All of them must pass before you commit. `Cargo.lock` is committed.

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
  `persist-credentials: false` on checkout, allowed licences only (MIT,
  Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0, Zlib).

## Dependency updates (Renovate)

`renovate.json` auto-merges only dev-only minor/patch updates and GitHub
Actions digest bumps; runtime dependencies and every major update need a
manual merge. Renovate must never change the workspace `rust-version` (the
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
