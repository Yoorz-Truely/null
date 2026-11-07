#!/bin/bash
# Run every fuzz target in fuzz/ for the given number of seconds each (default 60).
# Usage: tools/fuzz.sh [seconds]   (`just verify` runs it). Needs cargo-fuzz and a nightly
# toolchain. Each target's corpus grows in fuzz/corpus/<name> (ignored), seeded from the
# committed fuzz/seeds/<name>; crashes land in fuzz/artifacts/<name>. Inputs go up to 8 KB,
# the size of real files, past the largest seed.
set -euo pipefail
seconds=${1:-60}
if [ ! -d fuzz/fuzz_targets ]; then
  echo "fuzz: no fuzz targets yet"
  exit 0
fi
command -v cargo-fuzz >/dev/null || {
  echo "fuzz: cargo-fuzz is missing: cargo install cargo-fuzz, and rustup toolchain install nightly" >&2
  exit 1
}
for target in fuzz/fuzz_targets/*.rs; do
  name=$(basename "$target" .rs)
  echo "fuzz: $name for ${seconds}s"
  corpus="$PWD/fuzz/corpus/$name"
  mkdir -p "$corpus"
  seeds=()
  [ -d "fuzz/seeds/$name" ] && seeds=("$PWD/fuzz/seeds/$name")
  cargo +nightly fuzz run "$name" "$corpus" ${seeds[@]+"${seeds[@]}"} -- -max_total_time="$seconds" -max_len=8192
done
