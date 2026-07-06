#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
# SPDX-License-Identifier: AGPL-3.0-or-later

# operations/search.sh - free-text search (FTS5 vs ValueSet/$expand)
#
# Representative fixture: "heart attack"
# Local:  FTS5 MATCH query on concepts_fts
# Remote: ValueSet/$expand with filter parameter

run_search() {
  local term="heart attack"
  printf '  → text search ("%s") ...\n' "$term" >&2

  local lms lsd
  local_time_search "$term" 10 >/dev/null; lms=$TIMING_MEDIAN
  lsd=$TIMING_STDDEV

  local rms="-" rsd="-" notes=""
  if [[ -n "$BENCH_SERVER" ]]; then
    if fhir_time_search "$term" 10 >/dev/null 2>&1; then
      rms=$TIMING_MEDIAN; rsd=$TIMING_STDDEV
    else
      rms="-"; rsd="-"; notes="fhir call failed"
    fi
  fi

  append_result "search" "text search (top 10)" "$lms" "$lsd" "$rms" "$rsd" "$notes"
}
