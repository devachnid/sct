#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# FHIR conformance regression gate for CI (and local use).
#
# Builds the committed synthetic RF2 fixture (tests/fixtures/rf2/) into a SQLite
# database, starts `sct serve` over it, and runs the conformance suite with a
# minimal fixture set matched to that 22-concept fixture
# (benchmarks/fixtures/conformance-ci/). Exits non-zero on ANY conformance
# failure, so it can gate a build.
#
# Needs: a built `sct` with the serve feature (default), plus curl, jq, sqlite3.
# Override the binary with SCT_BIN, the port with CONF_PORT.

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

SCT="${SCT_BIN:-target/debug/sct}"
PORT="${CONF_PORT:-8199}"
FIXTURES="benchmarks/fixtures/conformance-ci"

[[ -x "$SCT" ]] || { echo "error: sct binary not found at '$SCT' (build it, or set SCT_BIN)"; exit 1; }

tmp=$(mktemp -d)
serve_pid=""
cleanup() { [[ -n "$serve_pid" ]] && kill "$serve_pid" 2>/dev/null || true; rm -rf "$tmp"; }
trap cleanup EXIT

echo "building fixture artefacts from tests/fixtures/rf2 ..."
"$SCT" ndjson --rf2 tests/fixtures/rf2 --output "$tmp/synth.ndjson" >/dev/null
"$SCT" sqlite --input "$tmp/synth.ndjson" --output "$tmp/synth.db" --transitive-closure >/dev/null

echo "starting sct serve on :$PORT ..."
"$SCT" serve --db "$tmp/synth.db" --port "$PORT" --fhir-base /fhir >"$tmp/serve.log" 2>&1 &
serve_pid=$!

for i in $(seq 1 30); do
  if curl -fsS "http://localhost:$PORT/fhir/metadata" >/dev/null 2>&1; then break; fi
  if [[ $i -eq 30 ]]; then echo "error: sct serve did not become healthy"; cat "$tmp/serve.log"; exit 1; fi
  sleep 1
done

echo "running conformance suite ..."
benchmarks/conformance.sh \
  --server "http://localhost:$PORT/fhir" \
  --fixtures "$FIXTURES" \
  --operations metadata,lookup,validate-code,expand,subsumes,valueset-validate,errors
