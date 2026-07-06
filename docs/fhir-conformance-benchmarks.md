# FHIR Conformance And Benchmarks

`sct` has two separate checks for the terminology server:

1. **FHIR conformance checks**: does the server return valid FHIR R4 shapes and
   expected terminology semantics?
2. **Performance benchmarks**: once correctness passes, how fast is it compared
   with local SQLite and other FHIR terminology servers?

The distinction matters. A fast server that returns the wrong `$expand` result
is not useful, and a benchmark based on a handful of easy requests is too easy
to dismiss.

## HL7-Aligned, Not Official Certification

The conformance runner is aligned with the FHIR R4 terminology service contract:

- [`/metadata`](https://hl7.org/fhir/R4/http.html#capabilities)
- [`CodeSystem/$lookup`](https://hl7.org/fhir/R4/codesystem-operation-lookup.html)
- [`CodeSystem/$validate-code`](https://hl7.org/fhir/R4/codesystem-operation-validate-code.html)
- [`CodeSystem/$subsumes`](https://hl7.org/fhir/R4/codesystem-operation-subsumes.html)
- [`ValueSet/$expand`](https://hl7.org/fhir/R4/valueset-operation-expand.html)
- [`ValueSet/$validate-code`](https://hl7.org/fhir/R4/valueset-operation-validate-code.html)
- [`ConceptMap/$translate`](https://hl7.org/fhir/R4/conceptmap-operation-translate.html)

It is not an HL7 certification badge. For external validation, the closest
formal artefact is a FHIR [`TestScript`](https://hl7.org/fhir/R4/testscript.html)
suite, which can be run in tools such as Touchstone. The HL7 FHIR Validator is
also useful: point it at `sct serve` as its terminology server and validate real
FHIR resources or Implementation Guides with SNOMED CT bindings.

The local runner exists because benchmark evidence needs a stable, reproducible
workload that can run on developer machines, VPS deployments and CI.

## Run Conformance First

Start a terminology server:

```bash
sct serve --db snomed.db --host 127.0.0.1 --port 8080 --fhir-base /fhir
```

Then run:

```bash
benchmarks/conformance.sh --server http://localhost:8080/fhir
```

The runner checks:

| Area | What is asserted |
|---|---|
| CapabilityStatement | FHIR R4 version and advertised terminology operations |
| `$lookup` | `Parameters` shape, display text, designations and parent properties |
| `CodeSystem/$validate-code` | true/false outcomes, including display mismatch |
| `$expand` | ECL expansion, text filtering, result totals and expected members |
| `$subsumes` | `subsumes`, `subsumed-by`, `equivalent`, `not-subsumed` |
| `ValueSet/$validate-code` | membership against implicit SNOMED ECL ValueSets |
| `$translate` | SNOMED to ICD-10 and reverse mapping when advertised |
| Errors | FHIR `OperationOutcome` responses and expected HTTP status codes |

Write machine-readable output for CI or later reporting:

```bash
benchmarks/conformance.sh \
  --server http://localhost:8080/fhir \
  --output reports/sct-conformance.jsonl
```

If a target server does not advertise `ConceptMap/$translate`, the translate
checks are skipped by default. Use `--strict` when comparing only servers that
are expected to support the full `sct` surface.

## Then Benchmark

After conformance passes:

```bash
benchmarks/bench.sh \
  --db snomed.db \
  --server http://localhost:8080/fhir \
  --runs 20 \
  --warmup 5 \
  --write-benchmarks
```

The existing benchmark covers:

- concept lookup
- free-text search
- direct children
- ancestor traversal
- subsumption
- bulk lookup

The benchmark reports wall-clock medians and standard deviation. Local SQLite
timings include process startup. FHIR timings include HTTP overhead.

## Compare Against Snowstorm Or Ontoserver

To make a credible public claim:

1. Load the same SNOMED CT release into every server.
2. Run each server on the same hardware class.
3. Warm the filesystem, JVM, Elasticsearch/Lucene and SQLite caches.
4. Run `benchmarks/conformance.sh` first.
5. Only publish benchmark results for servers that pass the relevant
   conformance profile.
6. Record exact versions, heap settings, database size, disk type, CPU, RAM,
   operating system and release package.

Example:

```bash
# sct
benchmarks/conformance.sh --server http://localhost:8080/fhir
benchmarks/bench.sh --db snomed.db --server http://localhost:8080/fhir --runs 20 --warmup 5

# Snowstorm Lite
benchmarks/conformance.sh --server http://localhost:8081/fhir
benchmarks/bench.sh --db snomed.db --server http://localhost:8081/fhir --runs 20 --warmup 5

# Ontoserver or another FHIR terminology server
benchmarks/conformance.sh --server http://localhost:8082/fhir
benchmarks/bench.sh --db snomed.db --server http://localhost:8082/fhir --runs 20 --warmup 5
```

The conformance checks are deliberately fixture based so the same request
matrix can be used across implementations. The benchmark fixtures should be
expanded over time with more high-fanout hierarchies, deep concepts, inactive
concepts, refsets and cross-map workloads.

## Public Methodology

When publishing results, include:

- exact `sct` version and git commit
- SNOMED CT edition and release date
- whether refsets and crossmaps were loaded
- server base URL path, for example `/fhir`
- hardware and operating system
- Docker image tags or binary versions for comparator servers
- cache state: cold start, warm cache, or both
- number of runs, warmup runs and timeout
- full conformance pass/fail output
- raw benchmark output

The headline number should be scoped. For example, "`sct serve` is faster for
these read-only SNOMED CT terminology operations on this release and hardware"
is defensible. A general claim that one terminology server is universally
faster than another is not.
