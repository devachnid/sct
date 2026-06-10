# sct serve

Run a **FHIR R4 terminology server** over a SNOMED CT SQLite database - a lightweight, local, drop-in alternative to hosted services (Ontoserver, Snowstorm, the NHS FHIR Terminology Server) for development, testing, and small-scale production.

!!! note "Optional feature"
    `sct serve` is gated behind the `serve` Cargo feature (it pulls in an async HTTP stack). Build with `cargo install sct-rs --features serve`, or `cargo build --features serve`. It is also included in `--features full`.

**When to use:** a FHIR client (EHR, HL7 validator, SMART app, integration engine) needs `$lookup` / `$validate-code` / `$subsumes` / `$expand` and you want sub-millisecond, offline, single-binary terminology with no Elasticsearch, JVM, or Docker. The entire server is backed by one inspectable `snomed.db` file.

---

## Usage

```
sct serve [--db <FILE>] [--port <PORT>] [--host <HOST>] [--fhir-base <PATH>]
```

| Flag | Default | Description |
|---|---|---|
| `--db <FILE>` | discovered (see [Path resolution](../path-resolution.md)) | SQLite database produced by `sct sqlite`. |
| `--port <PORT>` | `8080` | TCP port to listen on. |
| `--host <HOST>` | `127.0.0.1` | Address to bind. Use `0.0.0.0` to accept remote connections. |
| `--fhir-base <PATH>` | `/` | Base path for all routes. Set to `/fhir` for Ontoserver-compatible URLs. |
| `--read-only` | on | The server never writes; the flag documents that intent. |

```bash
# Local dev server
sct serve --db snomed.db

# Ontoserver-compatible base path, reachable on the network
sct serve --db snomed.db --host 0.0.0.0 --port 8080 --fhir-base /fhir
```

Responses are `application/fhir+json`. An `Accept` header that requests XML exclusively gets a `406` (XML is not supported).

---

## Operations (Phase 1)

| Endpoint | What it does |
|---|---|
| `GET /metadata` | CapabilityStatement declaring the supported operations |
| `CodeSystem/$lookup` | Concept details: display, designations (FSN + synonyms), parents, children, ancestors, inactive, moduleId, effectiveTime |
| `CodeSystem/$validate-code` | Whether a code exists (and an optional `display` matches) |
| `CodeSystem/$subsumes` | Subsumption between two codes (`subsumes` / `subsumed-by` / `equivalent` / `not-subsumed`) |
| `ValueSet/$expand` | Expand by free-text `filter` (FTS5), **ECL**, or a stored `.codelist` (by canonical URL) |
| `ValueSet/$validate-code` | Whether a code is a member of a ValueSet (stored `.codelist` or implicit ECL) |
| `GET /ValueSet` | Searchset Bundle of the stored `.codelist` ValueSets |
| `GET /ValueSet/{id}` | The stored ValueSet resource (with `compose`) |
| `GET /ValueSet/{id}/$expand` | Expand a stored ValueSet by id |
| `ConceptMap/$translate` | Map a code across terminologies (SNOMED CT ↔ ICD-10 / OPCS-4 / CTV3 / Read v2) |

GET and POST are both accepted; parameters are read from the query string.

### `$expand` and ECL

`$expand` accepts the FHIR implicit SNOMED ValueSet URL. The text `filter` runs over FTS5; the `ecl/` form runs the **full [`sct` ECL engine](ecl.md)** - so `$expand` supports hierarchy (`<<`, `<!`, `>>`, `>!`), refset membership (`^`), boolean (`AND`/`OR`/`MINUS`), and attribute refinement (`:`), well beyond simple subtype expansion. ECL and `filter` combine (intersection).

```bash
# Subtypes of Diabetes mellitus (URL-encoded ECL "<<73211009")
curl 'http://localhost:8080/ValueSet/$expand?url=http://snomed.info/sct?fhir_vs=ecl/%3C%3C73211009'

# Free-text expansion
curl 'http://localhost:8080/ValueSet/$expand?url=http://snomed.info/sct?fhir_vs&filter=heart+attack&count=10'

# Attribute refinement (finding site)
curl 'http://localhost:8080/ValueSet/$expand?url=http://snomed.info/sct?fhir_vs=ecl/%3C%3C404684003%20:%20363698007%20=%20%3C%3C39057004'
```

`count` (default 100, max 1000) and `offset` paginate; the `expansion.total` reflects the full match set. `includeDesignations=true` adds FSN + synonyms to each entry.

### Stored ValueSets from `.codelist` files

Point `--codelists <dir>` (default `./codelists`) at a directory of [`.codelist`](codelist.md) files and the server exposes each as a named FHIR ValueSet. Composition is resolved at startup, so a list that `includes:` others is served as its full effective member set. **Security model: "public by placement"** - only files in the served directory are exposed; keep private lists elsewhere. Expansion reconciles each concept's display against the live database (falling back to the stored term for concepts absent from the loaded edition).

```bash
sct serve --db snomed.db --codelists ./codelists &

# List the served ValueSets
curl 'http://localhost:8080/ValueSet'

# Read one (full resource with compose.include.concept)
curl 'http://localhost:8080/ValueSet/diabetes'

# Expand it (by id, or by its canonical URL via $expand?url=...)
curl 'http://localhost:8080/ValueSet/diabetes/$expand?count=20'

# Validate membership
curl 'http://localhost:8080/ValueSet/$validate-code?url=http://localhost:8080/ValueSet/diabetes&code=46635009'
```

The canonical URL of a served list is `{server-base}/ValueSet/{id}`. `$validate-code` also works against an implicit ECL value set (`?url=http://snomed.info/sct?fhir_vs=ecl/...`).

### Cross-terminology translation (`ConceptMap/$translate`)

Map a code between SNOMED CT, ICD-10, OPCS-4, CTV3, and Read v2 using the same maps as [`sct transcode`](transcode.md). Needs a database built with [`sct ndjson --refsets all`](ndjson.md) for the ICD-10 / OPCS-4 maps.

```bash
# SNOMED CT -> ICD-10
curl 'http://localhost:8080/ConceptMap/$translate?system=http://snomed.info/sct&code=22298006&targetsystem=http://hl7.org/fhir/sid/icd-10'

# Bare names also accepted; reverse works too (ICD-10 -> SNOMED CT)
curl 'http://localhost:8080/ConceptMap/$translate?system=icd10&code=I219&targetsystem=snomed'
```

Returns a `Parameters` resource with `result` (boolean) and a `match` part per mapping. This is a drop-in target for the existing **DMWB Excel add-in**, which can point at a FHIR server - giving analysts the familiar worksheet workflow on a fast, offline backend.

### Examples

```bash
sct serve --db snomed.db &

curl 'http://localhost:8080/metadata'
curl 'http://localhost:8080/CodeSystem/$lookup?code=22298006&property=parent&property=designation'
curl 'http://localhost:8080/CodeSystem/$validate-code?code=22298006'
curl 'http://localhost:8080/CodeSystem/$subsumes?codeA=46635009&codeB=73211009'
```

Errors are FHIR `OperationOutcome` resources with the appropriate status (`404` unknown code, `400` invalid parameter, `406` XML requested, `500` server error).

---

## Scope and limitations

This is **Phase 1**. Known boundaries (see [`specs/commands/serve.md`](https://github.com/pacharanero/sct/blob/main/specs/commands/serve.md) for the full picture):

- **Single edition / single version** per process - the server serves whatever is in `--db`; a `version` parameter is accepted and logged but not used for routing.
- **Stored ValueSets** come from `.codelist` files (read-only, served from `--codelists`); there is no write/CRUD API for ValueSets, and no stored `ConceptMap` resources. `ConceptMap/$translate`, `$closure`, multi-version routing, and FHIR R5 are later phases.
- **`^` (refset) ECL** depends on refsets being loaded (`sct ndjson --refsets simple` + `sct sqlite`); **attribute refinement** depends on the schema-v4 `concept_relationships` table (rebuild with a current `sct`).
- **No auth / SMART on FHIR** - run it behind your own gateway if exposing it beyond localhost.
- **JSON only** - XML requests get a `406`.
