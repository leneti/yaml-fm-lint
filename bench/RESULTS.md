# Benchmark results

Baselines for yaml-fm-lint 1.7.1 and 1.8.0, recorded before any 2.0 core code exists. The README cites only this file for performance claims (spec section 13).

## Machine

| | |
|---|---|
| CPU | Intel(R) N150 |
| Cores | 4 (`nproc`) |
| RAM | 11.4 GiB (11,996,712 kB) |
| OS | Ubuntu 24.04.4 LTS |
| Kernel | 7.0.0-38-generic |
| Node | v22.20.0 |
| hyperfine | 2.0.0 |
| Load average before the run | 0.37, 1.73, 1.66 (1, 5 and 15 min; `uptime` at 2026-11-04T22:41:11Z) |

The 5- and 15-minute load averages reflect the build and smoke work that had just finished, not concurrent load during the run. The published numbers are from the idle run. An earlier run on a busy machine was discarded.

## Data

- Generator: `cargo xtask bench-corpus`, source `xtask/src/corpus.rs` at git blob `abc4eb987fe480baa60a95a7d4bd42779711a75e` (check with `git hash-object xtask/src/corpus.rs`), seed `42`.
- 10k set: 10,000 files, 1,032 with deliberate errors. 100k set: 100,000 files, 9,998 with deliberate errors.
- Before any timing, the script runs both versions once on the 10k set with `--oneline` and checks that the number of distinct paths flagged (1,032) equals the length of `error_files` in `manifest.json`. Both versions passed (1,032 = 1,032).
- A one-off manual check on the 100k set (not part of the script) also matched: both versions flagged 9,998 distinct paths, equal to the 9,998 `error_files`, out of 100,000 `.md` files.

## Results

All times are wall-clock, 10 runs after 3 warmups (`hyperfine --warmup 3 --runs 10 -i`). Commands run `node` directly, not through `npx`.

1.x only descends into sub-folders with `--recursive`. Without it, a directory run prints "No markdown files found" and lints nothing, so the 10k, 100k and fix-10k commands all pass `--recursive`.

### one-file

`node .../index.js bench/corpus/10k/s000/g0/post-000000.md` (1.7.1) and `node .../cli.js ...` (1.8.0). Dominated by Node startup, so it says little about linting speed.

| | 1.7.1 | 1.8.0 | 2.0 binary | 2.0 via npm launcher |
|---|---|---|---|---|
| mean ± σ (ms) | 65.5 ± 3.6 | 63.2 ± 2.2 | *pending (M5)* | *pending (M5)* |
| min (ms) | 61.7 | 59.1 | *pending (M5)* | *pending (M5)* |
| max (ms) | 73.3 | 66.7 | *pending (M5)* | *pending (M5)* |

### 10k

`<cmd> bench/corpus/10k --recursive`

| | 1.7.1 | 1.8.0 | 2.0 binary | 2.0 via npm launcher |
|---|---|---|---|---|
| mean ± σ (ms) | 759.6 ± 24.5 | 755.8 ± 18.3 | *pending (M5)* | *pending (M5)* |
| min (ms) | 731.4 | 734.6 | *pending (M5)* | *pending (M5)* |
| max (ms) | 812.7 | 785.0 | *pending (M5)* | *pending (M5)* |

### 100k

`<cmd> bench/corpus/100k --recursive`

| | 1.7.1 | 1.8.0 | 2.0 binary | 2.0 via npm launcher |
|---|---|---|---|---|
| mean ± σ (ms) | 5867.2 ± 74.3 | 5782.0 ± 46.7 | *pending (M5)* | *pending (M5)* |
| min (ms) | 5724.1 | 5730.3 | *pending (M5)* | *pending (M5)* |
| max (ms) | 5944.5 | 5871.2 | *pending (M5)* | *pending (M5)* |

### fix-10k

`<cmd> bench/corpus/fix --recursive --fix`, with `--prepare 'rm -rf bench/corpus/fix && cp -r bench/corpus/10k bench/corpus/fix'` so every run starts from unfixed files. Note that 1.x `--fix` rewrites every file, not only the broken ones, so this measures rewriting about 10,000 files.

| | 1.7.1 | 1.8.0 | 2.0 binary | 2.0 via npm launcher |
|---|---|---|---|---|
| mean ± σ (ms) | 1578.6 ± 30.6 | 1586.4 ± 40.0 | *pending (M5)* | *pending (M5)* |
| min (ms) | 1545.1 | 1542.8 | *pending (M5)* | *pending (M5)* |
| max (ms) | 1652.6 | 1680.0 | *pending (M5)* | *pending (M5)* |

## Reproduce

From the repo root, with `cargo`, `node`, `npm` and `hyperfine` on `PATH`, run `bash bench/baseline.sh`. It installs both versions into `bench/.tools/`, generates the data into `bench/corpus/` (skipped if present), runs the checked run, then times the scenarios and writes `bench/results/<scenario>.json`. Pass scenario names (`one-file`, `10k`, `100k`, `fix-10k`) to run only some. Keep the machine idle while it runs.

## Claims

The README's performance claims cite this file only. Any figure not in this file is not a claim.
