#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
# SPDX-License-Identifier: AGPL-3.0-or-later

# operations/lookup.sh - single concept lookup by SCTID
#
# Representative fixture: 22298006 (Myocardial infarction)
# Local:  SELECT on concepts table (exact primary-key lookup)
# Remote: CodeSystem/$lookup?system=http://snomed.info/sct&code=...

run_lookup() {
  local code="22298006"
  printf '  → concept lookup (%s) ...\n' "$code" >&2

  local lms lsd
  local_time_lookup "$code" >/dev/null; lms=$TIMING_MEDIAN
  lsd=$TIMING_STDDEV

  local rms="-" rsd="-" notes=""
  if [[ -n "$BENCH_SERVER" ]]; then
    if fhir_time_lookup "$code" >/dev/null 2>&1; then
      rms=$TIMING_MEDIAN; rsd=$TIMING_STDDEV
    else
      rms="-"; rsd="-"; notes="fhir call failed"
    fi
  fi

  append_result "lookup" "concept lookup" "$lms" "$lsd" "$rms" "$rsd" "$notes"
}
