#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
# SPDX-License-Identifier: AGPL-3.0-or-later

# bench.sh - sct benchmark suite entry point
#
# Compares sct (local SQLite) against a FHIR R4 terminology server across
# six operations and renders a fair, like-for-like timing report.
#
# Usage:
#   benchmarks/bench.sh [OPTIONS]
#
# Options:
#   The sct side is one of (default: --sct-sqlite ./snomed.db):
#     --sct-sqlite PATH   sct's native SQLite path (alias: --db)
#     --sct-fhir URL      sct serve, over FHIR  e.g. http://localhost:8081/fhir
#   The comparator is always a FHIR server:
#     --vs URL            comparator FHIR base URL (alias: --server)
#                         e.g. https://terminology.myserver.org/fhir
#
#   So: --sct-sqlite <db> --vs <snowstorm>   → sct native vs a FHIR server
#       --sct-fhir <url>  --vs <snowstorm>   → sct serve vs a FHIR server (like-for-like)
#   --runs N            timed iterations per operation (default: 5)
#   --warmup N          warmup iterations before timing (default: 1)
#   --operations LIST   comma-separated subset: lookup,search,children,
#                       ancestors,subsumption,bulk  (default: all)
#   --format FORMAT     table (default) | json | csv | chart
#   --no-remote         skip FHIR calls entirely
#   --timeout SECS      per-request curl timeout (default: 30)
#   --output FILE       write report to FILE in addition to stdout
#   --write-benchmarks  write a timestamped report to benchmarks/reports/
#   --help              show this message

set -uo pipefail

BENCH_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ── defaults ──────────────────────────────────────────────────────────────────
# The "sct side" is either its native SQLite path (--sct-sqlite <db>, default) or
# sct serve over FHIR (--sct-fhir <url>). The comparator is always a FHIR server
# (--vs / --server). SCT_FHIR non-empty selects FHIR mode for the sct side.
BENCH_DB="./snomed.db"
SCT_FHIR=""
BENCH_SERVER=""
# Bound so `set -u` never trips when there is no local DB to introspect
# (--sct-fhir mode); local_snomed_info overwrites these in SQLite mode.
SNOMED_VERSION=""
SNOMED_CONCEPT_COUNT=""
BENCH_RUNS=5
BENCH_WARMUP=1
BENCH_TIMEOUT=30
BENCH_FORMAT="table"
BENCH_OPERATIONS="lookup,search,children,ancestors,subsumption,bulk"
BENCH_WRITE_BENCHMARKS=false
BENCH_OUTPUT_FILE=""

# ── argument parsing ──────────────────────────────────────────────────────────
_die() { printf 'error: %s\n' "$*" >&2; exit 1; }

_show_help() {
  sed -n '/^# Usage:/,/^[^#]/{ /^#/{ s/^# \?//; p } }' "${BASH_SOURCE[0]}"
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --server|--vs)       BENCH_SERVER="$2"; shift 2 ;;
    --db|--sct-sqlite)   BENCH_DB="$2"; shift 2 ;;
    --sct-fhir)          SCT_FHIR="$2"; shift 2 ;;
    --runs)              BENCH_RUNS="$2"; shift 2 ;;
    --warmup)            BENCH_WARMUP="$2"; shift 2 ;;
    --operations)        BENCH_OPERATIONS="$2"; shift 2 ;;
    --format)            BENCH_FORMAT="$2"; shift 2 ;;
    --no-remote)         BENCH_SERVER=""; shift ;;
    --timeout)           BENCH_TIMEOUT="$2"; shift 2 ;;
    --output)            BENCH_OUTPUT_FILE="$2"; shift 2 ;;
    --write-benchmarks)  BENCH_WRITE_BENCHMARKS=true; shift ;;
    --help|-h)           _show_help ;;
    *)                   _die "unknown option: $1" ;;
  esac
done

# ── dependency check ──────────────────────────────────────────────────────────
_check_deps() {
  local missing=()
  for cmd in sqlite3 jq awk; do
    command -v "$cmd" >/dev/null 2>&1 || missing+=("$cmd")
  done
  if [[ -n "$BENCH_SERVER" ]]; then
    command -v curl >/dev/null 2>&1 || missing+=("curl")
  fi
  if (( ${#missing[@]} > 0 )); then
    _die "missing required tools: ${missing[*]}"
  fi
  if ! command -v hyperfine >/dev/null 2>&1; then
    printf 'note: hyperfine not found - using bash manual timing (less accurate).\n' >&2
    printf '      install: cargo install hyperfine\n\n' >&2
  fi
}

# ── validate DB ───────────────────────────────────────────────────────────────
_check_db() {
  [[ -f "$BENCH_DB" ]] || _die "database not found: $BENCH_DB (run sct sqlite first)"
  sqlite3 "$BENCH_DB" "SELECT COUNT(*) FROM concepts LIMIT 1" >/dev/null 2>&1 \
    || _die "cannot query database: $BENCH_DB"
}

# ── source library files ──────────────────────────────────────────────────────
# shellcheck source=lib/timing.sh
source "${BENCH_DIR}/lib/timing.sh"
# shellcheck source=lib/local.sh
source "${BENCH_DIR}/lib/local.sh"
# shellcheck source=lib/fhir.sh
source "${BENCH_DIR}/lib/fhir.sh"
# shellcheck source=lib/report.sh
source "${BENCH_DIR}/lib/report.sh"

# ── sct side: native SQLite, or sct serve over FHIR ───────────────────────────
if [[ -n "$SCT_FHIR" ]]; then
  BENCH_SCT_LABEL="sct (fhir)"
  SCT_FHIR="${SCT_FHIR%/}"
  # Point the sct-side timers at sct's own FHIR endpoint instead of SQLite. Each
  # temporarily swaps BENCH_SERVER (which fhir.sh reads) to the sct URL, so the
  # sct side runs the identical FHIR operations as the comparator - a fair
  # server-to-server comparison with no operation code duplicated.
  local_time_lookup()    { local _s=$BENCH_SERVER; BENCH_SERVER=$SCT_FHIR; fhir_time_lookup "$@"; BENCH_SERVER=$_s; }
  local_time_search()    { local _s=$BENCH_SERVER; BENCH_SERVER=$SCT_FHIR; fhir_time_search "$@"; BENCH_SERVER=$_s; }
  local_time_children()  { local _s=$BENCH_SERVER; BENCH_SERVER=$SCT_FHIR; fhir_time_children "$@"; BENCH_SERVER=$_s; }
  local_time_subsumes()  { local _s=$BENCH_SERVER; BENCH_SERVER=$SCT_FHIR; fhir_time_subsumes "$@"; BENCH_SERVER=$_s; }
  local_time_ancestors() { local _s=$BENCH_SERVER; BENCH_SERVER=$SCT_FHIR; fhir_time_ancestors_iterative "$@"; BENCH_SERVER=$_s; }
  local_time_bulk()      { local _s=$BENCH_SERVER; BENCH_SERVER=$SCT_FHIR; fhir_time_bulk "$@"; BENCH_SERVER=$_s; }
  local_concept_depth()  { echo "?"; }
  local_snomed_info()    { :; }   # no local DB to introspect in FHIR mode
else
  BENCH_SCT_LABEL="sct (sqlite)"
fi

# ── shared result accumulator ─────────────────────────────────────────────────
# Columns (tab-separated): op | label | local_ms | local_sd | remote_ms | remote_sd | notes
BENCH_TMPDIR=$(mktemp -d /tmp/bench_XXXXXX)
BENCH_RESULTS_TSV="${BENCH_TMPDIR}/results.tsv"
BENCH_DATE=$(date +%Y-%m-%d)
BENCH_DATETIME=$(date +%Y-%m-%dT%H%M%S)

# Called by each operations/*.sh to write one result row.
append_result() {
  local op="$1" label="$2" lms="$3" lsd="$4" rms="$5" rsd="$6" notes="${7:-}"
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$op" "$label" "$lms" "$lsd" "$rms" "$rsd" "$notes" \
    >> "$BENCH_RESULTS_TSV"
}

# ── main ──────────────────────────────────────────────────────────────────────
_check_deps
[[ -n "$SCT_FHIR" ]] || _check_db

printf 'sct benchmark - %s\n' "$BENCH_DATE" >&2
if [[ -n "$SCT_FHIR" ]]; then
  printf 'sct side: %s (FHIR)\n' "$SCT_FHIR" >&2
  # Verify the sct FHIR endpoint is up (check_fhir_server reads BENCH_SERVER).
  _s=$BENCH_SERVER; BENCH_SERVER=$SCT_FHIR
  check_fhir_server >/dev/null 2>&1 || _die "sct FHIR endpoint unreachable: $SCT_FHIR"
  BENCH_SERVER=$_s
else
  printf 'sct side: %s (SQLite)\n' "$(realpath "$BENCH_DB" 2>/dev/null || printf '%s' "$BENCH_DB")" >&2
  # Resolve DB to absolute path for consistent display in report.
  BENCH_DB="$(realpath "$BENCH_DB" 2>/dev/null || printf '%s' "$BENCH_DB")"
  # Collect SNOMED metadata from the DB.
  local_snomed_info
  printf 'snomed version: %s (%s active concepts)\n' \
    "${SNOMED_VERSION:-?}" "${SNOMED_CONCEPT_COUNT:-?}" >&2
fi

# Check comparator server connectivity.
FHIR_PING_MS=0
if [[ -n "$BENCH_SERVER" ]]; then
  printf 'checking remote: %s ...\n' "$BENCH_SERVER" >&2
  if check_fhir_server; then
    printf 'remote ok (ping: %s ms)\n' "$FHIR_PING_MS" >&2
  else
    printf 'warning: remote server unreachable - running local-only benchmark.\n' >&2
    BENCH_SERVER=""
  fi
fi

printf 'runs: %s (warmup: %s) | timing: %s\n\n' \
  "$BENCH_RUNS" "$BENCH_WARMUP" "$(timing_tool_name)" >&2

# Run requested operations.
IFS=',' read -ra OPS <<< "$BENCH_OPERATIONS"
for op in "${OPS[@]}"; do
  op="${op// /}"  # trim whitespace
  opfile="${BENCH_DIR}/operations/${op}.sh"
  if [[ ! -f "$opfile" ]]; then
    printf 'warning: unknown operation "%s" - skipped.\n' "$op" >&2
    continue
  fi
  # shellcheck source=/dev/null
  source "$opfile"
  "run_${op}"
done

# Render report.
case "$BENCH_FORMAT" in
  table) render_table "$BENCH_RESULTS_TSV" ;;
  json)  render_json  "$BENCH_RESULTS_TSV" ;;
  csv)   render_csv   "$BENCH_RESULTS_TSV" ;;
  chart) render_chart "$BENCH_RESULTS_TSV" ;;
  *)     _die "unknown format: $BENCH_FORMAT (use table, json, csv, or chart)" ;;
esac

# Write to --output FILE if requested.
if [[ -n "$BENCH_OUTPUT_FILE" ]]; then
  case "$BENCH_FORMAT" in
    table) render_table "$BENCH_RESULTS_TSV" > "$BENCH_OUTPUT_FILE" ;;
    json)  render_json  "$BENCH_RESULTS_TSV" > "$BENCH_OUTPUT_FILE" ;;
    csv)   render_csv   "$BENCH_RESULTS_TSV" > "$BENCH_OUTPUT_FILE" ;;
    chart) render_chart "$BENCH_RESULTS_TSV" > "$BENCH_OUTPUT_FILE" ;;
  esac
  printf '\nwrote report to %s\n' "$BENCH_OUTPUT_FILE" >&2
fi

# Write benchmarks file if requested.
# Filename: benchmarks/reports/YYYY-MM-DDTHHMMSS-<server-slug>.md
# (or benchmarks/reports/YYYY-MM-DDTHHMMSS-local.md when no server is configured)
if $BENCH_WRITE_BENCHMARKS; then
  mkdir -p "${BENCH_DIR}/reports"
  if [[ -n "$BENCH_SERVER" ]]; then
    # Derive a filesystem-safe slug from the server URL:
    # strip scheme, replace non-alphanumeric runs with hyphens, trim trailing dash
    _server_slug=$(printf '%s' "$BENCH_SERVER" \
      | sed 's|^[a-z]*://||' \
      | tr -cs 'a-zA-Z0-9' '-' \
      | sed 's/-$//')
  else
    _server_slug="local"
  fi
  _bench_outfile="${BENCH_DIR}/reports/${BENCH_DATETIME}-${_server_slug}.md"
  render_markdown "$BENCH_RESULTS_TSV" "$_bench_outfile"
fi

# Clean up temp files.
rm -rf "$BENCH_TMPDIR"
