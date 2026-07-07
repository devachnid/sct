# `sct static` - Static-File Terminology Server

A spec for `sct static`: a build-time command that materialises the SQLite artefact
into a tree of static JSON files that any HTTP server (or `file://` consumer) can
serve as a read-only FHIR-shaped terminology service. Zero runtime, zero
infrastructure, perfect cacheability, deployable to GitHub Pages, S3, Cloudflare R2,
a USB stick, or a `/state` directory inside another repo.

---

## Overview

```bash
sct static --db snomed.db --out ./terminology-site/ \
    [--valuesets valuesets.toml] \
    [--codelists ./codelists/] \
    [--shape fhir|sct|both] \
    [--search lunr|pagefind|none] \
    [--base-url https://terminology.example.org/]
```

Walks the SQLite database and emits a directory tree of static JSON files keyed
by SCTID, refset id, and value-set id. The output is the precomputed
materialisation of every read-only operation `sct serve` supports, plus a small
client-side search index. A user uploads the directory and immediately has a
working terminology service.

---

## Why this exists

`sct serve` is the dynamic case: long-running process, SQLite file, sub-millisecond
queries, runtime ECL evaluation. It's the right answer when you need ad-hoc
queries or a true server.

`sct static` is the static case: no process, no database at runtime, no ECL
engine in the request path. Everything is pre-expanded at build time. This is
the right answer when:

- The set of useful queries is known ahead of time (the common case in
  clinical informatics - a fixed set of value sets, refsets, and lookups
  drives 99% of traffic)
- The deploy target is a CDN, a static site host, or a non-server environment
  (browser-only apps, GitEHR's `/state` directory, an offline tablet)
- Operational simplicity matters more than ad-hoc query flexibility
- The audience is broad enough that "spin up a Java server" is too high a bar

It also democratises the idea of a terminology server - anyone with a SNOMED
license and a static host can publish one.

---

## Relationship to existing commands

This is the most important question to answer up-front, because three commands
now produce per-concept files. They are kept separate because their consumers
are different.

| Command | Output | Primary consumer | Shape |
|---|---|---|---|
| `sct markdown` | Markdown per concept | Humans, LLMs, RAG, `grep` | Prose, hierarchy as text |
| `sct static` | JSON per concept + indexes | Programmatic clients, FHIR consumers | FHIR `Parameters` / bespoke JSON |
| `sct serve` | HTTP responses (live) | Same as `sct static`, but live | Same JSON, runtime-generated |

`sct static` and `sct serve` produce **byte-identical responses** for the
operations they both support. The static command is essentially `sct serve`
walked over every reachable input at build time. They share the FHIR
response-builder library (see `library-rs.md`).

`sct markdown` stays separate. Different audience (people, not machines),
different format (prose, not structured), different use case (browse and
search, not query and validate).

---

## Prerequisites

### 1. ECL parser as a shared library module (blocker)

Both `sct serve` Phase 2 and `sct static` need a real ECL parser. Today the
SQLite layer hand-rolls support for `<<`, `<!`, `>>`, `>!`, and basic boolean
operators by string-matching the URL. This is fine for a handful of patterns
but cannot handle `^` member-of (now unblocked by Simple refset loading), let
alone refinements.

**Proposed:** a new `crate::ecl` module that:

- Parses ECL expressions into an AST (subset of the official grammar - focus
  on the operators clinical refsets actually use)
- Compiles each AST node into the SQL needed against `concepts`,
  `concept_isa`, and `refset_members`
- Returns a `Vec<SCTID>` (for static expansion) or a SQL `SELECT` (for
  `sct serve` runtime use)

Initial coverage: the same operator set in the `serve.md` table, plus `^`
(now possible since refset_members is loaded). Refinements
(`* : 246075003 = 372687004`) deferred to a later phase - the static command
should warn-and-skip ECL it can't compile rather than failing the build.

**Why this pays off twice:** `sct serve` Phase 2 needs it, `sct static` needs
it, and `sct codelist` could grow an `ecl: <<73211009` field that auto-expands
on validate. One parser, three consumers.

### 2. Transitive closure (already on the roadmap as `sct tct`)

Subsumption checks need ancestor closures. If `tct` is built and persisted in
the SQLite DB, `sct static` reads it directly; otherwise it computes on the
fly and warns about build time.

### 3. None for the JSON I/O itself

Pure file writes; serde already in the dep tree.

---

## Output layout

```
terminology-site/
  index.json                          # site metadata, version, capability summary
  CodeSystem/
    snomed-ct.json                    # CodeSystem resource describing the loaded edition
  concepts/
    {sctid}.json                      # $lookup response, all properties
    {sctid}/
      ancestors.json                  # transitive ancestor list, for $subsumes
      children.json                   # direct children (already in the lookup, hoisted for caching)
  refsets/
    {refset_id}.json                  # refset metadata
    {refset_id}/members.json          # full member list (paginated for large refsets)
  valuesets/
    {valueset_id}.json                # ValueSet resource (definition)
    {valueset_id}/expansion.json      # pre-expanded ValueSet
  maps/
    {map_name}/
      {sctid}.json                    # ConceptMap/$translate response per source code
  search/
    index.json                        # FTS index manifest
    shard-{n}.json                    # sharded inverted index
```

Sharding rules:

- Concepts: ~830k files in the UK Monolith. Filesystems handle this fine.
  No sub-sharding by SCTID prefix unless the deploy target needs it (some
  object stores prefer fewer keys per "directory"). A `--shard-concepts`
  flag covers that case.
- Refset members: paginated at 1000 entries per file
  (`members-001.json`, etc.) with a `next` link. Stays under the 1MB
  threshold most CDNs prefer.
- Search index: sharded by token frequency band; manifest tells the client
  which shards to load lazily.

---

## Output shapes

`--shape fhir` (default): every JSON file is a valid FHIR R4 `Parameters` or
`ValueSet` or `CodeSystem` resource - byte-identical to what `sct serve` would
return. URL conventions match the FHIR spec where possible:

| FHIR endpoint | Static path |
|---|---|
| `GET /CodeSystem/$lookup?code=22298006` | `concepts/22298006.json` |
| `GET /CodeSystem/$subsumes?codeA=X&codeB=Y` | client reads `concepts/X/ancestors.json`, checks for Y |
| `GET /ValueSet/$expand?url=…vs1` | `valuesets/vs1/expansion.json` |
| `GET /ConceptMap/$translate?code=X&system=…icd10` | `maps/icd10/X.json` |

A trivial JS shim (or 20-line Cloudflare Worker) translates between FHIR
endpoint URLs and the static paths, so existing FHIR clients work unchanged.
This is documented as `clients/fhir-shim.js`.

`--shape sct`: bespoke JSON, smaller and simpler. Useful for
non-FHIR consumers (GitEHR UI, browser tools). The keys mirror the NDJSON
schema directly.

`--shape both`: emit both, doubling output size. Reasonable for a
fully public deployment.

---

## What value sets get pre-expanded

This is the meat of the command. The static export is only useful if it
contains the value sets people actually query. Three sources, in order of
priority:

1. **Codelists in the repo** - every `.codelist` file in `--codelists` is
   compiled to a value set. The codelist's `id` becomes the value-set id.
   This is the primary mechanism: codelists are the curated, versioned
   "common ECL" the user mentioned.

2. **`valuesets.toml`** - a config file listing additional ECL expressions
   to expand:

   ```toml
   [[valueset]]
   id = "diabetes-disorders"
   url = "http://example.org/fhir/ValueSet/diabetes"
   ecl = "<<73211009"

   [[valueset]]
   id = "ips-conditions"
   ecl = "^816080008"   # IPS Conditions reference set
   ```

3. **Implicit SNOMED value sets for every loaded refset** - `^{refset_id}`
   for each refset in `refset_members`. Cheap, useful, and what the
   FHIR `http://snomed.info/sct?fhir_vs=refset/X` URL expects.

Anything not in this list is not queryable. The static export is intentionally
not a general ECL engine; if you need that, run `sct serve`.

---

## Search index

Three options, picked at build time:

- `--search lunr`: ship a Lunr.js-compatible JSON index. Mature, ~5MB
  compressed for 100k entries, slower above 200k.
- `--search pagefind`: ship a Pagefind index (sharded, lazy-loaded, designed
  for static sites). Better fit for the full International Edition (~400k
  concepts) because the client only loads the shards relevant to the query.
- `--search none`: skip the index. The client can still walk
  `concepts/*.json` if it wants, but no built-in text search.

Default: `pagefind` if the concept count is over 100k, `lunr` otherwise.

The index is over `preferred_term` + `synonyms` only. FSN is excluded by
default (too noisy for typeahead) but available via `--search-include-fsn`.

---

## Composition with codelists

The natural workflow becomes:

1. Author or import codelists into a repo (e.g. `sct-codelists/`)
2. Run `sct static --codelists ./codelists --db snomed.db --out ./site`
3. `git push` the site directory to GitHub Pages

Result: a static FHIR-shaped terminology endpoint where every codelist in
the repo is a queryable, expanded `ValueSet`. Updating a codelist + rebuilding
+ pushing is the entire deploy pipeline. No server, no Snowstorm, no
Ontoserver license.

For the GPS+codelists strategy described in `docs/gps-ips.md`, this is the
delivery mechanism. GPS gives you the namespace, codelists give you the
groupings, `sct static` gives you the queryable endpoint.

---

## Phasing

**Phase 1 - Core lookup**

- ECL parser module (initial subset: simple operators + `^`)
- `sct static` skeleton: emit `concepts/{sctid}.json`, `refsets/`, `index.json`
- FHIR shape only
- No search index (or `--search none`)

Sufficient for: lookup, refset membership, basic subsumption.

**Phase 2 - Value sets and search**

- Pre-expand codelists and `valuesets.toml`
- Emit `valuesets/{id}/expansion.json`
- `--search pagefind` and `--search lunr`
- Bespoke `--shape sct` output

Sufficient for: codelist-driven static deployments, GitEHR integration.

**Phase 3 - Maps and FHIR shim**

- Emit `maps/` for every loaded `ConceptMap` (depends on Concept Maps roadmap item)
- Ship `clients/fhir-shim.js` that maps `/CodeSystem/$lookup?code=X` to
  `concepts/X.json`
- Document Cloudflare Worker example (~20 lines)

**Phase 4 - Hardening**

- Sharding flags for object-store deploys
- Build-time provenance: every JSON file carries the same `_provenance`
  block as other sct artefacts
- Optional `tar.gz` packaging for offline deploy

---

## Open questions

1. **Output size on the full International Edition.** ~830k concept files,
   each with parents/children/synonyms - back-of-envelope, ~1-2GB raw,
   ~300-500MB gzipped. Acceptable for a static deploy but not for a GitHub
   Pages repo (1GB soft limit). May need a "lite" mode that emits only what
   the codelists reference.

2. **ND-licensed data and static deploys.** The GPS is CC BY-ND but format
   shifting is permitted (see `docs/gps-ips.md`). Pre-expanding the GPS into
   `concepts/{sctid}.json` is a format shift; pre-computing ancestors over
   GPS data is not (the GPS has no ancestry, so this is moot for GPS-only
   builds). Full-edition static deploys remain bound by the Affiliate
   License - same as `sct serve`.

3. **FHIR or bespoke first.** FHIR shape is the bigger market but the JSON
   is verbose. Bespoke JSON is friendlier for the GitEHR / browser case.
   Probably ship FHIR first (Phase 1) since the shim story is clean,
   add bespoke in Phase 2.

4. **Should the FHIR shim live in this repo?** A 20-line Worker is not much
   to host but it's also not much to copy-paste. Probably ship it as a
   reference implementation under `examples/` rather than treating it as a
   first-class artefact.
