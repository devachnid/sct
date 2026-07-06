#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
# SPDX-License-Identifier: AGPL-3.0-or-later

# operations/bulk.sh - resolve all fixture concepts in one request/query
#
# Local:  Single SELECT ... WHERE id IN (...) across all fixture IDs
# Remote: FHIR batch Bundle POST (falls back to sequential $lookup calls
#         if the server does not support batch).
#
# This operation most clearly illustrates the per-round-trip cost of HTTP.

run_bulk() {
  # Load IDs from fixtures file (strip comments and labels)
  local -a ids=()
  while IFS='|' read -r id _rest; do
    [[ "$id" =~ ^[[:space:]]*# ]] && continue
    [[ -z "$id" ]] && continue
    ids+=( "$id" )
  done < "${BENCH_DIR}/fixtures/concepts.txt"

  local n="${#ids[@]}"
  printf '  → bulk lookup (%d concepts) ...\n' "$n" >&2

  local lms lsd
  local_time_bulk "${ids[@]}" >/dev/null; lms=$TIMING_MEDIAN
  lsd=$TIMING_STDDEV

  local rms="-" rsd="-" notes=""
  if [[ -n "$BENCH_SERVER" ]]; then
    # Call directly (not in a subshell) so FHIR_BULK_MODE propagates back.
    FHIR_BULK_MODE="sequential"
    if fhir_time_bulk "${ids[@]}" >/dev/null 2>&1; then
      rms=$TIMING_MEDIAN
      rsd=$TIMING_STDDEV
      if [[ "$FHIR_BULK_MODE" == "sequential" ]]; then
        notes="server does not support FHIR batch; ${n} sequential \$lookup calls issued"
      else
        notes="FHIR batch bundle (${n} entries)"
      fi
    else
      rms="-"; rsd="-"; notes="fhir call failed"
    fi
  fi

  append_result "bulk" "bulk lookup (${n} concepts)" \
    "$lms" "$lsd" "$rms" "$rsd" "$notes"
}
