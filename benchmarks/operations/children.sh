#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
# SPDX-License-Identifier: AGPL-3.0-or-later

# operations/children.sh - direct children of a concept
#
# Representative fixture: 73211009 (Diabetes mellitus - ~20 direct children)
# Local:  JOIN on concept_isa table
# Remote: ValueSet/$expand with ECL expression <!PARENT (direct children)

run_children() {
  local parent="73211009"
  printf '  → direct children (%s) ...\n' "$parent" >&2

  local lms lsd
  local_time_children "$parent" >/dev/null; lms=$TIMING_MEDIAN
  lsd=$TIMING_STDDEV

  local rms="-" rsd="-" notes=""
  if [[ -n "$BENCH_SERVER" ]]; then
    if fhir_time_children "$parent" >/dev/null 2>&1; then
      rms=$TIMING_MEDIAN; rsd=$TIMING_STDDEV
    else
      rms="-"; rsd="-"; notes="fhir call failed"
    fi
  fi

  append_result "children" "direct children" "$lms" "$lsd" "$rms" "$rsd" "$notes"
}
