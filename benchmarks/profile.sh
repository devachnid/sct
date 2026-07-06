#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# In-process CPU profiler for the sct core query paths.
#
# Builds sct with the `profiling` profile (release codegen + debug symbols, see
# Cargo.toml) and profiles ONE real query against a real database, so the report
# shows where wall-clock actually goes - not micro-benchmark guesses.
#
#   benchmarks/profile.sh 'ecl expand <<404684003' --db snomed.db
#   benchmarks/profile.sh 'lexical "myocardial infarction"' --db snomed.db
#
# It prefers a sampling flamegraph (perf + cargo-flamegraph -> clickable SVG) and
# falls back to valgrind/callgrind (deterministic instruction counts -> text
# report + a .callgrind file you can open in kcachegrind). Neither needs the
# other; callgrind works without perf permissions, which is why it is the
# default fallback.
#
# The complementary pieces:
#   - statistical micro-benchmarks:  cargo bench   (target/criterion/report/)
#   - this whole-binary profiler:    the hot-path map for a single real query
#
# Output goes to benchmarks/profiles/ (gitignored).

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

query="${1:-ecl expand <<404684003}"
shift || true
db="snomed.db"
# Allow `--db PATH` anywhere in the remaining args; everything else is ignored.
args=("$@")
for ((i = 0; i < ${#args[@]}; i++)); do
  [[ "${args[i]}" == "--db" ]] && db="${args[i + 1]:-$db}"
done
[[ -f "$db" ]] || { echo "error: database '$db' not found (pass --db PATH)"; exit 1; }

out="benchmarks/profiles"
mkdir -p "$out"
slug="$(printf '%s' "$query" | tr -cs 'A-Za-z0-9' '-' | sed 's/-*$//;s/^-*//')"

echo "building sct (profile: profiling) ..."
cargo build --profile profiling --bin sct >/dev/null 2>&1 || cargo build --profile profiling --bin sct
bin="target/profiling/sct"

# The query string is a full sct sub-invocation, e.g. `ecl expand <<404684003`.
# shellcheck disable=SC2206
cmd=($bin $query --db "$db")
echo "profiling: ${cmd[*]}"

paranoid="$(cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || echo 99)"
if command -v cargo-flamegraph >/dev/null 2>&1 && command -v perf >/dev/null 2>&1 \
   && [[ "$paranoid" -le 1 ]]; then
  svg="$out/${slug}.svg"
  echo "sampling profiler: perf + flamegraph -> $svg"
  CARGO_PROFILE=profiling flamegraph --profile profiling -o "$svg" -- "${cmd[@]:1}" >/dev/null
  echo "open $svg in a browser (click any frame to zoom)."
elif command -v valgrind >/dev/null 2>&1; then
  cg="$out/${slug}.callgrind"
  txt="$out/${slug}.txt"
  echo "deterministic profiler: valgrind/callgrind (perf unavailable or restricted)"
  echo "  (~30-50x slower than native; instruction counts, not wall-clock)"
  valgrind --tool=callgrind --callgrind-out-file="$cg" "${cmd[@]}" >/dev/null 2>"$out/${slug}.vglog"
  if command -v callgrind_annotate >/dev/null 2>&1; then
    callgrind_annotate --threshold=95 --auto=no "$cg" >"$txt" 2>/dev/null
    echo "=== top self-cost functions ==="
    sed -n '/Ir  *file:function/,/^$/p' "$txt" | head -25
    echo "full report: $txt   |   call graph: kcachegrind $cg"
  else
    echo "raw profile: $cg (install kcachegrind to visualise)"
  fi
else
  echo "error: need either (perf + cargo-flamegraph, paranoid<=1) or valgrind." >&2
  echo "  install: cargo install flamegraph  &&  sudo sysctl kernel.perf_event_paranoid=1" >&2
  echo "       or: your distro's valgrind package" >&2
  exit 1
fi
