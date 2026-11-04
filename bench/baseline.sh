#!/usr/bin/env bash
# Baseline benchmarks for yaml-fm-lint 1.7.1 and 1.8.0.
# Usage (from the repo root): bash bench/baseline.sh [one-file] [10k] [100k] [fix-10k]
# No arguments runs all four scenarios.
set -euo pipefail

[ -f Cargo.toml ] && [ -d bench ] || { echo "run from the repo root" >&2; exit 1; }

SCENARIOS=("$@")
[ ${#SCENARIOS[@]} -gt 0 ] || SCENARIOS=(one-file 10k 100k fix-10k)

CMD_171="node bench/.tools/1.7.1/node_modules/yaml-fm-lint/index.js"
CMD_180="node bench/.tools/1.8.0/node_modules/yaml-fm-lint/cli.js"

# Install both versions without running install scripts.
for v in 1.7.1 1.8.0; do
  npm install --prefix "bench/.tools/$v" --ignore-scripts --no-audit --no-fund "yaml-fm-lint@$v"
done
[ -f bench/.tools/1.7.1/node_modules/yaml-fm-lint/index.js ] || { echo "missing 1.7.1 entry point" >&2; exit 1; }
[ -f bench/.tools/1.8.0/node_modules/yaml-fm-lint/cli.js ] || { echo "missing 1.8.0 entry point" >&2; exit 1; }

# Generate the data sets. An existing set is reused only if its manifest says
# it was made with seed 42 and the expected number of files.
manifest_field() { # manifest_field <manifest.json> <seed|files>
  if command -v jq >/dev/null; then
    jq -r ".$2" "$1"
  else
    node -e 'console.log(require(require("path").resolve(process.argv[1]))[process.argv[2]])' "$1" "$2"
  fi
}
for n in 10 100; do
  dir="bench/corpus/${n}k"
  if [ -f "$dir/manifest.json" ]; then
    seed=$(manifest_field "$dir/manifest.json" seed)
    files=$(manifest_field "$dir/manifest.json" files)
    if [ "$seed" != "42" ] || [ "$files" != "${n}000" ]; then
      echo "$dir was generated with seed $seed and $files files, but the baseline needs seed 42 and ${n}000 files." >&2
      echo "Delete $dir and run this script again." >&2
      exit 1
    fi
  else
    cargo xtask bench-corpus --files "${n}000" --seed 42 --out "$dir"
  fi
done

# Checked run: 1.x must flag exactly the files the generator marked as broken.
if command -v jq >/dev/null; then
  expected=$(jq '.error_files | length' bench/corpus/10k/manifest.json)
else
  expected=$(node -e 'console.log(require("./bench/corpus/10k/manifest.json").error_files.length)')
fi

check() {
  local name=$1 cmd=$2 out err code=0 found
  out=$(mktemp); err=$(mktemp)
  # shellcheck disable=SC2086
  $cmd bench/corpus/10k --recursive --oneline >"$out" 2>"$err" || code=$?
  if [ "$code" -gt 1 ]; then
    echo "checked run FAILED: $name exited with $code" >&2; head -20 "$err" >&2; exit 1
  fi
  if grep -Eq '^\s+at .*\(|Error: Cannot find|TypeError|ReferenceError' "$err" "$out"; then
    echo "checked run FAILED: $name printed a stack trace" >&2; head -20 "$err" "$out" >&2; exit 1
  fi
  # Strip ANSI colour, keep problem lines, drop ":line:col", keep the last token (the path).
  found=$(sed 's/\x1b\[[0-9;]*m//g' "$out" \
    | { grep -E '^YAML-(Error|Warning):' || true; } \
    | sed -E 's/:[0-9]+:[0-9]+[[:space:]]*$//' \
    | awk '{print $NF}' | sort -u | wc -l)
  echo "checked run: $name flagged $found distinct paths, manifest error_files = $expected (exit $code)"
  if [ "$found" -eq 0 ] || [ "$found" -ne "$expected" ]; then
    echo "checked run FAILED: $name flagged $found paths, manifest expects $expected" >&2
    exit 1
  fi
  rm -f "$out" "$err"
}
check 1.7.1 "$CMD_171"
check 1.8.0 "$CMD_180"

# Timing. 1.x only recurses into folders with --recursive; without it the
# directory runs would lint nothing.
mkdir -p bench/results
run() {
  local scenario=$1 target=$2; shift 2
  hyperfine --warmup 3 --runs 10 -i --export-json "bench/results/$scenario.json" \
    -n 1.7.1 -n 1.8.0 "$@" "$CMD_171 $target" "$CMD_180 $target"
}
for s in "${SCENARIOS[@]}"; do
  case "$s" in
    one-file) run one-file bench/corpus/10k/s000/g0/post-000000.md ;;
    10k)      run 10k "bench/corpus/10k --recursive" ;;
    100k)     run 100k "bench/corpus/100k --recursive" ;;
    fix-10k)  run fix-10k "bench/corpus/fix --recursive --fix" \
                --prepare 'rm -rf bench/corpus/fix && cp -r bench/corpus/10k bench/corpus/fix' ;;
    *) echo "unknown scenario: $s" >&2; exit 1 ;;
  esac
done
