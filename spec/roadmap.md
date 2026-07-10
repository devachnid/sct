# SNOMED Local-First Tooling - Roadmap

Outstanding work and next steps. Completed work is removed; see git log for history.

---

### TODO

In no particular order. (Distribution items - code signing, the Docker Hub image, and
registry submissions - are tracked in detail under [Distribution](#distribution) below,
not here.)

* [ ] **Progress + ETA for long-running builds.** `embed`, `ndjson`, and `sqlite` are
      long-running processes with a known start and a known total (e.g. `embed`'s
      "81216/836761 concepts embedded"), so each can show progress and an estimated
      time remaining, even if the estimate is rough. Real per-phase timings are now on
      hand from this session's build-pipeline profiling (`benchmarks/profile.sh`,
      RF2→NDJSON / NDJSON→SQLite+FTS / TCT breakdown) to calibrate the estimates against.
* [~] revise the benchmarks and automate them, so that we end up with a nice-looking, comprehensive benchmarking comparison which includes comparing `sct` with local or remote Terminology servers, as well as comparing within `sct` the different search backends (`lexical` vs `fst`) and the impact of different index configurations (e.g. with or without labels). FHIR conformance fixtures and a terminology-server runner now exist in `benchmarks/conformance.sh`, and conformance runs as a **CI regression gate** (`benchmarks/conformance-ci.sh` builds the committed synthetic RF2 fixture, starts `sct serve`, and asserts against a minimal fixture set on every push). `bench.sh` can now compare either sct's native SQLite (`--sct-sqlite`) or sct serve (`--sct-fhir`) against a comparator FHIR server (`--vs`). Remaining: broader fixture coverage, comparator server compose profiles, and published reports. Concurrent load testing is now its own item (below).
* [~] **Concurrent load-testing harness for `sct serve`.** **Shipped:** `benchmarks/load.sh` drives `sct serve` under sustained concurrency via `oha` (keep-alive, JSON), ramping the client count (1, 2, 4, 8, 16, 32, 64, 128) across `$lookup` / `$validate-code` / `$subsumes` / `$expand` and reporting **throughput (req/s), tail latency (p50/p95/p99/p99.9), and error rate** per level plus the saturation peak; `--write-report` and optional `--stat-container` (server memory under load) included. See `benchmarks/README.md`. A single-server test with no comparator, so results are publishable. **Remaining:** run it on a real box (client separate from the server so they don't fight for cores), publish latency-vs-concurrency / throughput-vs-concurrency curves, and chase the scaling question a first co-located run already hinted at - throughput peaked at low concurrency and then declined, which is exactly the predicted place to look: `sct serve` is Tokio multi-thread with every DB op on `spawn_blocking` over concurrent read-only SQLite connections, but it opens a **fresh `open_db_readonly` connection per request (no pool)**, so the harness should confirm whether that connection-per-request is the bottleneck and whether a pool is worth adding. This is where sct's lean, GC-free process may shine against a JVM server's GC pauses - or expose a bottleneck to fix.
* [~] **Benchmark reporting policy.** Public `sct` benchmark numbers are **`sct`-solo** - throughput, latency, and its very small (~26 MB) resident footprint, none of which need a comparator - plus the **fully-owned `sct serve` vs local Snowstorm Lite** run (same machine, loopback). Same-hardware comparisons against a reference *commercial* terminology server have been run for internal validation but are **kept out of the repo** (`.private/`) out of respect for that server's licence terms; do not add commercial-server comparison numbers to any committed doc, README, or roadmap. Methodology that applies to every benchmark: run the client **co-located with the server(s) over loopback** (no proxy / TLS / wide-area hop) so the figures reflect server compute, not the client's distance from the box; benchmark scripts take server URLs as arguments and **never hardcode a host or credentials** into committed files.
* [ ] **Externally-verified FHIR conformance.** Our `benchmarks/conformance.sh` is home-grown - HL7-*aligned* (it exercises the real FHIR R4 terminology operations and has been cross-checked against Ontoserver and Snowstorm) but *not* an official/certified artefact, and the docs say so. To make a stronger, third-party claim, validate `sct serve` with the **HL7 FHIR Validator** (point it at `sct serve` as the terminology backend and validate real resources / Implementation Guides with SNOMED CT bindings). Add it to CI against the committed RF2 fixture so regressions are caught. The heavier, fuller path - a **FHIR `TestScript`** suite run in **Touchstone** (AEGIS) - is a later complement, not a prerequisite. Deferred; we'll do the FHIR Validator step sometime. See [`docs/fhir-conformance-benchmarks.md`](../docs/fhir-conformance-benchmarks.md).
* [~] mermaid diagrams for the architecture and data flow, to visually explain how the different components fit together and how data moves through the system. **Shipped:** the RF2 → NDJSON → artefacts data-flow diagram now renders as Mermaid in the README (GitHub) and the walkthrough overview, and Mermaid is wired into the docs site (`pymdownx.superfences` custom fence). Remaining: a dedicated FST / search-internals diagram (FST is easier to explain visually), and worked diagrams that use real SNOMED examples to make the concepts concrete. The other ASCII blocks in the docs (file-tree layouts, `sct paths` output, the TUI wireframe, `sct diagram` tree output) are intentionally left as literal text - Mermaid would represent them worse.
* [ ] SNOMED primer - understanding SNOMED basics (concepts, descriptions, relationships, refsets, ECL, etc.) is a barrier to entry for new users. A concise primer that explains these core concepts in plain language, with examples, is needed. It may need to take different approaches for technical vs clinical audiences. It should be in a section of the docs.
* [ ] **`sct semantic` result quality.** Verified against the UK Monolith with `nomic-embed-text` (see [`docs/commands/semantic.md`](../docs/commands/semantic.md#known-limitations)): the current one-document-per-concept embedding scheme (PT + FSN + all synonyms comma-joined + hierarchy path) has two real, reproducible failure modes - synonym dilution (a concept whose PT is technical/Latin but whose colloquial synonym is one-of-several in a list can rank behind unrelated concepts that merely repeat the query phrase; `"heart attack"` puts *Myocardial infarction* at rank 11) and category drift (query lands in the right clinical neighbourhood but the wrong hierarchy branch, e.g. procedure instead of disorder). Separately, `nomic-embed-text` has no medical fine-tuning, so idiomatic colloquial phrases with no shared vocabulary root fail outright (`"sticky blood"`, `"sugar sickness"`, `"baby turning blue"` all miss their intended concept entirely). Candidate directions, not yet chosen: (1) per-synonym embeddings with max-pooling across a concept's aliases instead of one blended document - the architecturally "correct" fix for the dilution case, at the cost of N× more vectors/storage; (2) a hybrid ranker blending the FTS5 lexical score with the semantic score, so an exact substring match can't be out-ranked by a diluted vector; (3) a clinically fine-tuned embedding model (SapBERT etc., ONNX-backed) - see the model-selection survey in [`spec/commands/embed.md`](commands/embed.md), though note that file describes a much larger ONNX/benchmarking feature set than what is currently implemented (Ollama-only). Any of these needs before/after evaluation against a fixed query set before merging, the same discipline used for the perf work elsewhere in this repo.

## In progress / near-term

### Distribution

Shipped: multi-platform release binaries (including Windows x86_64 and Linux aarch64), SHA-256 checksums, `.deb` / `.rpm` packages, unsigned macOS `.dmg` images, standalone Windows `.exe`, `install.sh` / `install.ps1`, cargo-binstall, crates.io, the shared `pacharanero/tap` Homebrew tap, a Scoop bucket, and a Docker Compose stack (`sct` + a Caddy reverse proxy for automatic HTTPS, optional basic auth, CORS - see [`spec/deployment.md`](deployment.md)) published as a multi-arch image to Docker Hub (`pacharanero/sct`) on every release. Release artefacts and package-manager manifests are auto-bumped by the release workflow. See the docs installation tabs and [`docs/deploy/`](../docs/deploy/index.md) (a no-clone route using the published image, and a build-from-source route). Outstanding:

- [ ] Publish the same image to GHCR too (Docker Hub only today) - cheap addition,
      reuses `GITHUB_TOKEN`, no new secret needed.
- [ ] macOS code signing + notarization (requires Apple Developer ID, $99/yr) so users
      don't have to `chmod +x` and bypass Gatekeeper
- [ ] Windows Authenticode signing (requires cert from CA) so SmartScreen doesn't block.
      Guide: <https://ngrok.com/blog/so-you-want-to-sign-for-windows>
- [ ] Submit to `homebrew-core` once project hits 30+ stars and has stable release cadence
      (would enable `brew install sct` without the tap)
- [ ] Submit to `winget` after Windows signing is in place
- [ ] Nix flake

### Quality

- [ ] Extend the end-to-end test (`tests/end_to_end.rs`, over the committed synthetic
      `tests/fixtures/rf2/` snapshot) to also assert through the `sct mcp` server tools. The
      RF2 → NDJSON → SQLite → query path (lexical / lookup / ECL / refset / TCT) is now covered;
      the MCP tool handlers are not yet exercised end-to-end.
- [ ] Smoke test for `sct embed`: embed a handful of concepts, query for "heart attack", assert
      myocardial infarction concepts appear in top results
- [ ] **End-to-end CLI tests** with `assert_cmd` - run `sct` as a binary against tiny fixtures
      under `tests/fixtures/` and assert on exit codes, output files, and stdout. Would cover
      contract-level regressions (argument parsing, file naming, `sct trud check` exit-2
      semantics, `sct codelist validate` exit codes) that inline unit tests cannot.
- [ ] **Network-layer tests for `sct trud`** using `wiremock` to stand up a fake TRUD API.
      The current 41 trud tests are all pure helpers - `fetch_releases`, `probe_edition`,
      `run_download`, and the SHA-256 mismatch / re-download paths are entirely untested.
- [ ] **De-flake trud tests' environment variables** - the `HOME` / `SCT_DATA_HOME` tests use
      `unsafe { std::env::set_var(...) }` while `cargo test` runs in parallel. Currently
      passing but fragile; `temp-env` or `serial_test` would remove the global-state race.
- [ ] **Snapshot tests for formatted output** - `sct diff`, `sct trud list`, `sct info` all
      emit human-readable tables/summaries. `insta` would freeze the current shape and catch
      accidental format regressions without hand-written `contains` assertions.
- [ ] **Doctests on library public items** - now that `src/lib.rs` exposes `build_records`,
      `Rf2Dataset::load`, the rf2 parsers, etc. as genuine library surface, `///` examples on
      these items double as living documentation and get tested by `cargo test` for free.
- [ ] **Coverage measurement** - run `cargo-tarpaulin` (or similar) in CI to surface blind
      spots. `src/commands/mcp.rs` is ~1,800 lines; worth knowing which tool handlers are
      lightly covered.

---

## Features

### `sct codelist` - clinical code list management

Core shipped: `new`, `add` (including `--ecl` and stdin `-`), `remove`, `validate`, `stats`, `diff`, and `export` to csv / opencodelists-csv / markdown (with `--include-maps` crosswalks) / **fhir-json** (a FHIR R4 ValueSet, via the same shared builder `sct serve` uses, so exported and served forms are identical; `--url` sets the canonical base). See [`docs/commands/codelist.md`](../docs/commands/codelist.md). Outstanding:

- [ ] `sct codelist export <file> --format rf2` - the remaining export format. Blocked on a
      decision, not effort: a valid RF2 Simple Reference Set needs a real SNOMED CT namespace
      (a `refsetId`, `moduleId`, and member-row UUIDs) that a codelist does not carry, so it
      cannot be emitted correctly without that input. `--format fhir-json` shipped and covers
      the portable-standards-export need in the meantime; `rf2` today returns a clear
      not-yet-implemented message explaining the namespace requirement.
- [ ] **Multi-terminology codelists (format v2)** - future extension now that the
      terminology workspace can contain SNOMED CT, CTV3, Read v2, ICD-10, and OPCS-4.
      Would allow first-class non-SNOMED source codes in a codelist, e.g. historical
      Read v2 codes with no modern SNOMED equivalent, instead of treating SNOMED as
      the canonical pivot for every list. The `--include-maps` export is the interim
      solution for SNOMED-canonical lists; v2 is for genuinely cross-terminology source
      artefacts.
- [ ] `sct codelist search <file> <query>` - interactive FTS5 search → include/exclude
      (CLI surface exists and is documented in `--help`, but the handler is currently a
      stub - `bail!("... is not yet implemented")`. Low-hanging: the plumbing is there.)
- [ ] `sct codelist import --from <source>` - OCL, CSV, RF2, FHIR import
      (same: `--from opencodelists/csv/rf2/fhir-json` is a real, validated flag, but the
      handler stubs out. Low-hanging alongside `search` above.)

### Interactive "search as you type"

- [ ] **Live incremental search mode** - an interactive terminal mode where results
      update on every keystroke. A genuinely compelling CLI demo (terminology lookup
      feels instant), and the basis for an embeddable search component later.
  - **Pluggable backends.** The same UI over a selectable backend: `--backend fts5`
        ([`sct lexical`](../docs/commands/lexical.md) / SQLite), `--backend fst`
        ([`sct fst`](commands/fst.md) - its sub-millisecond prefix/fuzzy lookup is *ideal*
        for per-keystroke latency), later `semantic`, or a blended ranker. Backends share
        one trait (query string → ranked hits) so adding one is cheap.
  - **Parameters.** `--limit` (live results shown), `--min-chars` (threshold before
        results appear), plus natural extensions: debounce interval, fuzzy distance,
        hierarchy/semantic-tag filter, and which display fields to show.
  - **Embeddable, not just a CLI.** A later cut should expose the same engine as a
        component another program can drive - e.g. a webapp talking to it via a backend
        process. A **line-oriented stdio protocol** (query line in → JSON-lines results
        out, cancellable on the next query) is the obvious shape, and could reuse the
        MCP server's stdio framing. That keeps one search core behind both the
        interactive TUI and a programmatic transport.
  - **Implementation still open.** Terminal UI via the existing `--features tui`
        (ratatui/crossterm) or a lighter readline-style redraw; whether the interactive
        mode and the stdio component are one binary mode with two front-ends or separate.
        Decide once a backend trait and the result shape are pinned down.

### Concept history & inactivation storytelling

Today inactive concepts are dropped by default (`--include-inactive` retains them), and even when retained `sct` shows little of *why* or *what happened*. The goal: optionally surface retired concepts and tell the full story - when a concept was added, when it was inactivated, what (if anything) replaced it, how long it was in service, and **why** it was inactivated. That last one is the interesting, currently-missing piece.

Most of the data is already in RF2, and much of it `sct` already ingests:

- **Replaced by / same as / possibly equivalent** - the Historical **Association** reference set, already parsed into `concept_history` under `--refsets all` and used by [`sct map --forward-history`](../docs/commands/map.md). This is the "what replaced it" edge, with its association type (REPLACED BY / SAME AS / POSSIBLY EQUIVALENT TO / MOVED TO / WAS A). Ready to surface now.
- **Why inactivated** - the **Concept inactivation indicator**, a **`der2_cRefset_AttributeValue*` refset** (values: Ambiguous, Duplicate, Erroneous, Limited, Outdated, Moved elsewhere, Classification-derived, etc.). This refset is **not currently parsed** - it's the "Attribute value refsets" line already listed as remaining under [`sct ndjson --refsets all`](#future--larger-scope). Parsing it is the single prerequisite for the "why", and the highest-value part of this feature.
- **When inactivated** - the concept row's `effectiveTime` at the point `active` flips to 0. Available from the Snapshot.
- **When added / years in service** - needs the **Full** RF2 (not the Snapshot) to know the true birth `effectiveTime`; this is the same dependency as the "Point-in-time + through-time reporting" item below. Snapshot-only can approximate from the earliest release `sct` has ingested, but the honest full answer needs Full RF2.

Displaying retired concepts is feasible because inactive concepts keep active descriptions (FSNs), so there is always a human-readable label to show. Proposed surface, splitting into a tractable near-term slice and a larger one:

- **Near-term (Snapshot + AttributeValue parsing, no Full RF2):** parse the inactivation-indicator refset, then for an inactive concept make `sct lookup`, the MCP `snomed_concept` tool, and FHIR `CodeSystem/$lookup` show *inactive status + inactivation date + inactivation reason + association target(s) with their preferred terms*. A dedicated `sct history <id>` (or `sct lookup --history`) tells the story in one view.
- **Larger (Full RF2):** true added-date, years-in-service, and a through-time timeline - folds into the Full-RF2 point-in-time work below.

Pairs naturally with the "Semantic drift summariser" LLM item, which already imagines narrating inactivations, and with codeagogo's inactive-concept warning (see the AEHRC integrations below).

---

## Future / larger scope

> **Cross-terminology mapping + DMWB replacement.** The RF2-native terminology/mapping
> core is now shipped: CTV3 maps, SNOMED CT -> ICD-10 / OPCS-4 ExtendedMap rows,
> Association-refset history forwarding, `sct map` (the unified cross-terminology
> command; `transcode` and `crosswalk` remain as aliases),
> codelist `--include-maps`, FHIR `ConceptMap/$translate`, and Read v2 import
> from the final TRUD item 9 flat-file release. `sct trud download
> --multi-terminology` now builds a SNOMED/CTV3/Read v2/ICD-10/OPCS-4 workspace
> in one command. The Access `.mdb` path remains intentionally documented as
> historical analysis only: `jetdb` cannot decode DMWB's Binary `SCUI` column,
> and item 9 is the cleaner authoritative source. See
> [`spec/cross-terminology-mapping.md`](cross-terminology-mapping.md) and the
> DMWB walkthrough in the docs site.

### Performance internals (potential, not committed)

Both of these are **candidates, not decisions** - each is grounded in a real
profile, but neither has cleared a "worth the surface area / risk" bar yet.

- [ ] **Potential: DB-wide INTEGER SCTID columns.** `concept_ancestors` already
      uses INTEGER id columns (shipped with the `u64` transitive-closure work:
      ~17% faster TCT, ~35% smaller table). Extending that to every SCTID column
      (`concepts.id`, `concept_isa`, `concept_relationships`, `refset_members`,
      `concept_history` source/target) would roughly halve the bytes per id
      across the whole database and speed the text-sort-heavy index builds, while
      dropping the query-time `CAST(... AS INTEGER)` shims. The catch is why it is
      only *potential*: ~15 cross-table JOINs compare those columns to
      `concepts.id`, so they must all flip together or SQLite's INTEGER-vs-TEXT
      affinity silently returns zero rows; the FTS5 rowid semantics need care
      (an `INTEGER PRIMARY KEY` aliases `rowid`); the non-numeric code columns
      (CTV3 / ICD-10 / OPCS-4 crossmaps) must stay TEXT; and it needs a
      schema-version bump + rebuild. A real DB-size win, but a careful day's work
      whose payoff over the already-shipped `concept_ancestors` piece is unproven.

- [ ] **Potential: optional one-pass RF2 → SQLite build.** The build writes a
      ~1.4 GB NDJSON (RF2 → NDJSON) and then reads it straight back (NDJSON →
      SQLite); that intermediate write + read is the single largest build cost
      (I/O-bound, not CPU). A fused streaming RF2 → SQLite path would delete it.
      **This does not change sct's file-first design:** NDJSON stays the default
      and the canonical, inspectable, distributable artifact - the fused path
      would be a purely *additive, opt-in* shortcut (e.g. `sct build --direct`)
      for users who only need the database and don't want the intermediate file.
      Deferred; the file-first pipeline is the priority.

- [ ] **History MCP surface** - RF2 Association history is parsed and loaded into
      `concept_history`, and `sct map --forward-history` uses it. Still missing:
      expose the same forwarding through an MCP `snomed_resolve` tool.
- [~] **`sct serve`** - HTTP FHIR R4 terminology server backed by SQLite. Drop-in replacement
      for Ontoserver, Snowstorm, and the NHS FHIR Terminology Server. Full spec in
      [`spec/commands/serve.md`](commands/serve.md); user docs in
      [`docs/commands/serve.md`](../docs/commands/serve.md).

  **Phase 1 - Core operations** ✅ **shipped** (feature-gated `serve`): `/metadata`,
  `CodeSystem/$lookup` / `$validate-code` / `$subsumes`, `ValueSet/$expand` with text filter
  **and full ECL** (via the ECL engine - well beyond the spec's original "simple ECL" scope),
  `--fhir-base` prefix, `OperationOutcome` errors. Remaining: FHIR batch Bundle.

  **Near-term priorities (from the June 2026 Ontoserver gap review).** These four cover the
  bulk of what real FHIR terminology clients actually call, without taking on multi-terminology
  content, multi-version routing, or NCTS syndication (the genuinely large Ontoserver gaps,
  deferred deliberately). They map onto the phases below but are the prioritised cut:

  1. ✅ **shipped** - **Serve `.codelist` files as stored/named FHIR ValueSets.**
     `sct serve --codelists <dir>` (default `./codelists`) scans `*.codelist` at startup,
     resolves composition, and exposes each as a ValueSet: `GET /ValueSet` (searchset Bundle),
     `GET /ValueSet/{id}` (full resource with `compose`), `GET /ValueSet/{id}/$expand`, and
     `$expand?url=<canonical>`. Canonical URL is `{server-base}/ValueSet/{id}`; expansion display
     is reconciled against the live DB (stored term as fallback). Security is "public by
     placement". Drafts are served (status reflected). Remaining nice-to-haves: optional `url:`
     front-matter override, `--exclude-draft`.
  2. ✅ **shipped** - **`ValueSet/$validate-code`** for a stored `.codelist` (set membership +
     live display) and for an implicit ECL value set (`?url=...fhir_vs=ecl/...`, via the ECL
     engine). Complements `CodeSystem/$validate-code`.
  3. ✅ **shipped** - **`ConceptMap/$translate`** over the crossmap engine. Supports
     SNOMED CT, CTV3, ICD-10, OPCS-4, and Read v2 when the backing DB contains those
     maps. ICD-10 / OPCS-4 come from `--refsets all`; Read v2 comes from TRUD item 9
     via `sct read2 import` or `sct trud download --multi-terminology`.
  4. **`$expand` parameter completeness + FHIR batch Bundles** - `activeOnly`, `displayLanguage`,
     specific `designation` / `property` filters, version params (`system-version` /
     `valueSetVersion`); `POST /` transaction/batch Bundle handler; `CodeSystem` resource read;
     `TerminologyCapabilities` (`/metadata?mode=terminology`).

  **Phase 2 - remaining hierarchy/ValueSet bits** (stored `.codelist` ValueSets and
  `ValueSet/$validate-code` per the priorities above; `CodeSystem` resource read; pagination
  polish - `$expand` ECL/`<<`/`<!`/`>>`/`>!`/boolean already done in Phase 1)

  **Phase 3 - Refsets + ConceptMap** ✅ **mostly shipped**: `^` ECL member-of operator
  works over Simple refsets; `ConceptMap/$translate` works over CTV3, ICD-10, OPCS-4
  and any loaded Read v2 rows; Association refsets load concept history under
  `--refsets all`. Remaining refset families: Complex refsets and AttributeValue refsets.

  **Phase 4 - R5 + hardening** (FHIR R5 CapabilityStatement; published Docker image /
  systemd unit; parameter completeness; batch Bundle support)
- [~] **`sct ndjson --refsets all`** - RF2-native DMWB-relevant map/history ingestion is
      shipped: ExtendedMap rows load into `crossmaps`, Association rows load into
      `concept_history` via a history sidecar, and default `simple` mode still omits the
      heavy data. Remaining derivative-2 refset shapes:
      - **Complex refsets** (`der2_Refset_Complex*Snapshot*.txt`) - adds attribute payload columns
        beyond simple membership; needs a wider row type and a strategy for surfacing those
        attributes to downstream consumers
      - **Attribute value refsets** (`der2_cRefset_AttributeValue*Snapshot*.txt`) - concept-to-value
        annotations used by some UK national refsets
      - **Additional ExtendedMap systems** beyond the known ICD-10 / OPCS-4 refset ids, if a
        future RF2 release adds new map targets that should be classified by
        `rf2::extended_map_system`.

      Each refset family gets its own table or column extension; `refset_members` (concept-only,
      already shipped) stays as-is.

- [ ] **MCP crossmaps** - CLI/codelist/FHIR crossmap support is shipped. Extend the MCP
      `snomed_map` tool beyond CTV3/Read v2 so it can expose ICD-10 / OPCS-4 `crossmaps`
      and history-forwarding results too.
- [ ] **First-class ICD-10 / ICD-11 support** - current `sct` support for ICD-10 is
      map-centric: UK/International SNOMED CT -> ICD-10 ExtendedMap rows are
      already imported into `crossmaps` with `sct ndjson --refsets all`, and
      `sct map`, codelist `--include-maps`, and FHIR
      `ConceptMap/$translate` can use them. What is missing is ICD itself as a
      searchable/servable code system: code titles, hierarchy, includes/excludes,
      synonyms/index terms, validation, lookup, expansion, and version metadata.

      Initial research (June 2026):
      - ICD-10 access is tractable. WHO exposes ICD-10 2019 through the ICD API
        and browser; the ICD API supported-release list includes ICD-10 releases
        2008, 2010, 2016, and 2019. NHS England's Classifications Browser exposes
        ICD-10 5th Edition 2026 and related standards, with site content under
        OGL unless excepted. For UK users, ICD-10 5th Edition is the practical
        target because it is what NHS morbidity coding uses.
      - ICD-11 access is also tractable, but different. WHO publishes ICD-11 MMS
        spreadsheets from the browser, exposes ICD-11 through the OAuth-protected
        ICD API, and provides local ICD API deployments via Docker, Windows
        service, and Linux systemd. ICD-11 content is licensed CC BY-ND 3.0 IGO;
        WHO clarifies that incorporating the classification in software is not
        an adaptation if code, title, and URI are preserved, but mapping/crosswalk
        production requires separate written agreement from WHO.
      - Crossmaps differ sharply by generation. SNOMED -> ICD-10 maps already
        arrive in SNOMED RF2 ExtendedMap refsets and are mostly an import/display
        problem. WHO provides ICD-10 -> ICD-11 mapping tables in the ICD-11 MMS
        browser. A public, production SNOMED CT -> ICD-11 map is not currently an
        assumed input; treat it as unavailable until a licensable source is found.
      - Sources checked: WHO ICD API supported classifications
        <https://icd.who.int/icdapi/docs2/SupportedClassifications/>, WHO ICD API
        authentication/local deployment docs, WHO ICD-11 MMS 2026 browser
        <https://icd.who.int/browse/2026-01/mms/en>, WHO ICD-11 licence PDF
        <https://icd.who.int/en/docs/ICD11-license.pdf>, NHS England
        Classifications Browser <https://classbrowser.nhs.uk/> and licence page
        <https://classbrowser.nhs.uk/license.html>.

      Proposed shape:
      - Add a generic `code_systems` / `codes` / `code_relationships` model for
        non-SNOMED classifications rather than forcing ICD rows into `concepts`.
        Preserve source URI/version/license/provenance per code system.
      - Start with an `sct icd import` command for local files/API exports:
        ICD-10 tabular data first, then ICD-11 MMS spreadsheet/API export.
        Do not redistribute WHO/NHS source content in `sct` releases.
      - Extend `sct lookup`, lexical search, codelist validation/export, and
        `sct serve` so `CodeSystem/$lookup` and `$validate-code` work for ICD-10
        and ICD-11 code systems as well as SNOMED CT.
      - Keep crossmaps in the existing general `crossmaps` table. Add ICD-10 ->
        ICD-11 maps only if the WHO mapping-table licence permits local import;
        do not generate or ship new ICD crosswalks without explicit licence review.
- [ ] **IPS Free Set bundling** - investigate bundling the pre-processed NDJSON artefact of the
      SNOMED International IPS Free Set (freely available from MLDS without affiliate membership)
      to make `sct lexical`, `sct mcp`, and `sct serve` work out-of-the-box for IPS tooling
      without any RF2 download step. *Requires licence verification before distribution.*

---

## Ideas harvested from prior-art Python tooling

A mature hobbyist Python toolchain (`cheethame2017/sct` on Bitbucket, Apache-2.0,
by Ed Cheetham) implements a deep set of SNOMED inspection/analysis features built
up over years. It is CLI-menu-driven, Linux/Android-oriented, and depends on Neo4j
for state-valid ancestry. We are harvesting *ideas* here, not code - any of these
would be a clean-room Rust reimplementation in `sct`'s local-first, file-based,
single-binary idiom. Ordered roughly by how well they fit `sct` and how distinctive
they'd be. Source: <https://bitbucket.org/cheethame2017/sct/src/development/>.

- [~] **Concept-definition diagrams (`sct diagram <id>`)** - render a concept's logical
      definition + ancestry as Graphviz dot / SVG: the focus concept's defining
      relationships, a proximal-supertype view, and descendant "treemaps". This is the
      single most compelling, most demoable gap - `sct` already holds every relationship
      row needed; the work is a dot/SVG emitter plus layout options. Complements the
      planned Observable/D3 viewer (that's interactive-browser; this is static,
      file-based, pipeline-friendly - drop a concept SVG straight into docs or a PR).
      **Slice 1 shipped:** `definition` / `ancestors` / `descendants` / `neighbourhood`
      views in `tree` / `dot` / `mermaid`, with SVG→PNG/JPG conversion recipes. Remaining:
      primitive/defined node styling (needs a `definition_status` schema column), DOT
      attribute-group clusters, and a built-in `svg` format via `layout-rs`. Spec + user
      docs: [`spec/commands/diagram.md`](diagram.md), [`docs/commands/diagram.md`](../docs/commands/diagram.md).

- [~] **Set → minimal ECL refactoring (`sct ecl compress`)** - given a raw set of SCTIDs
      (e.g. a `.codelist`), synthesise the *smallest* equivalent ECL expression: collapse
      members into `<<` subsumption clauses with include/exclude refinements, rather than
      enumerating every id. The inverse of `$expand`. Genuinely hard and genuinely
      valuable - turns a hand-curated concept list back into a maintainable, release-stable
      intensional definition. Pairs directly with the `.codelist` and ECL engine work
      already shipped. **Slice 1 shipped:** greedy include roots + clean exclusions + an
      exactness residual net (verified by re-expansion), `--codelist` / stdin input,
      `--intensional-only`, `--max-exclusions`, `--pretty`, `--stats`. Remaining:
      straddling-exclusion push-down for tighter expressions, `^refset` cover clauses,
      and `sct codelist export --format ecl` wiring. Spec + user docs:
      [`spec/commands/ecl-compress.md`](ecl-compress.md), [`docs/commands/ecl.md`](../docs/commands/ecl.md).

- [ ] **Proximal primitive supertypes (`sct pps <id>`)** - compute a fully-defined
      concept's proximal primitive parents (the classification/normal-form operation
      underpinning subsumption and post-coordination QA). Builds on the shipped TCT plus
      definition-status; a core informatics primitive no local tool offers cheaply.
      *Medium-high.*

- [ ] **Set algebra over ECL results** - name query results (`A`, `B`, `C`...) and combine
      them with AND / OR / NOT, feeding named sets back into further ECL as if they were
      refsets. A thin algebra layer over the existing ECL engine that makes interactive
      codelist construction far more powerful. Natural fit for the planned live/stdio
      search component and `sct codelist`. *Medium-high.*

- [ ] **Point-in-time + through-time reporting** - reconstruct the terminology at an
      arbitrary `effectiveTime` from the **Full** RF2 release (not just the Snapshot), and
      emit "through-time" matrices of how a concept's ancestors / descendants / refset
      membership changed across releases. Extends `sct diff` (two-release) and the shipped
      Association-history forwarding into true temporal browsing. Larger scope - needs Full
      RF2 ingestion - but squarely in `sct`'s diff/history lane. *Medium; larger.*

- [ ] **Refset / hierarchy comparison analytics** - compare two refsets' membership, and
      profile a refset's members by top-level hierarchy chapter (spot the one cardiology
      concept in an otherwise-respiratory set). Cheap, high-signal codelist-QA on top of
      shipped refset data; overlaps the "Explain this refset" LLM item but is deterministic
      and needs no model. *Medium.*

- [ ] **SCG / OWL axiom handling** - parse and pretty-print Semantic Compositional Grammar
      expressions and the OWL axiom refset (`sctid ⊑ ...`). Neither is currently surfaced;
      would let `sct` show a concept's actual DL definition, not just relationship rows.
      *Medium; specialist.*

- [ ] **Further crossmap targets from the harness set** - the Python repo carries
      experimental mappings to HPO (Human Phenotype Ontology), MedDRA, HGNC gene symbols,
      and NICIP (UK imaging codes). HPO and NICIP are the most tractable UK-relevant
      additions to the existing crossmap engine; MedDRA/HGNC are licence-gated. Fold into
      the `crossmaps` table alongside ICD-10 / OPCS-4 / CTV3 / Read v2. *Low-medium;
      licence review per source.*

- [ ] **MRCM constraint diagrams** - render Machine-Readable Concept Model domain/attribute
      /range constraints as diagrams, for content authors validating post-coordination.
      Niche but unserved by any local tool. *Low; specialist.*

## Exploration & data-science surfaces

With the RF2 → NDJSON → SQLite/Parquet/Arrow pipeline and MCP server in place, `sct`
is positioned to become the ontology backend for a much wider set of surfaces than a
single CLI. The items below sketch what a "richer than a SNOMED browser, flexible like
the CLI" middle ground could look like - roughly in priority order, with the first two
being the next concrete pieces of work.

### Next up (chosen targets)

- [ ] **DuckDB integration** - ship a `sct duckdb` subcommand (or a documented recipe)
      that exposes the SQLite DB as a set of DuckDB views plus helper macros. DuckDB is
      where the data-science ecosystem is converging (Python/R/JS bindings, zero-install,
      Parquet-native) and this is the single highest-leverage integration for analytical
      users.

      Concretely:
      - `ATTACH 'snomed.db' AS sct (TYPE SQLITE)` gets us raw access; the goal is a
        layer of views/macros that hide the RF2-ism of the schema.
      - Macros to implement: `sct_is_a(child, ancestor)`, `sct_descendants(id)`,
        `sct_ancestors(id)`, `sct_fsn(id)`, `sct_pt(id)`, `sct_in_refset(id, refset_id)`.
        These should be thin wrappers over the existing `concept_isa` / `refset_members`
        tables. Once `tct` (transitive closure) is built, `sct_descendants` becomes a
        single indexed lookup instead of a recursive CTE.
      - Also expose the Parquet artefact directly via `read_parquet()` - no attach
        needed, great for Colab/Kaggle where the user just downloads the `.parquet`.
      - Constraint: DuckDB's SQLite scanner is read-only and doesn't see FTS5 virtual
        tables; lexical search must either go through `sct` CLI, or we materialise a
        plain `concepts_text` table alongside FTS5 at `sqlite` build time.
      - Pointer: <https://duckdb.org/docs/extensions/sqlite>

- [ ] **Notebook entrypoints - marimo + Jupyter, layered over a `.sql` bootstrap**

      Rather than pick one notebook system, ship the real artefact as plain SQL and
      put veneers over it. DuckDB is the interop layer; every notebook system and every
      language binding can host it, so "polyglot notebook" is a red herring - the SQL
      itself is the same everywhere.

      Three pieces, cheapest first:

      1. **`examples/duckdb/bootstrap.sql`** - the lingua franca. Raw `ATTACH` + view
         + macro definitions (see DuckDB item above). Works from any notebook, any
         language binding, or a bare `duckdb` CLI. This is the real committed artefact;
         the notebooks below are thin wrappers.

      2. **`examples/marimo/snomed_explorer.py`** - the flagship interactive authoring
         surface. Reactive DAG is the core win: change a refset picker and the codelist
         preview re-renders automatically. Stored as plain `.py`, so it diffs cleanly,
         reviews cleanly, and `ruff`/`mypy` just work. `marimo run` also serves it as a
         standalone web app, which makes it *more* accessible to non-Python users than
         a Jupyter kernel they'd have to install.
         - Widgets: `mo.ui.text` for FTS search, `mo.ui.dropdown` over `snomed_refsets()`,
           `mo.ui.multiselect` for hierarchy top-levels, `altair`/`plotly` for a
           descendants sunburst.
         - Output: every session ends with a `.codelist` file written to disk - authoring
           tool, not just a viewer.
         - Constraint: marimo is young (v0.9 as of Apr 2026); verify current release
           before pinning, and check its SQL-cell story since raw DuckDB queries need
           to feel first-class.
         - Pointer: <https://marimo.io/>

      3. **`examples/jupyter/quickstart.ipynb`** - the polyglot on-ramp. A thin notebook
         showing `ATTACH 'snomed.db'` plus a handful of queries, with an "Open in Colab"
         button in the README. Jupyter's strengths (kernels for R/Julia/JS, Colab/Kaggle
         ubiquity, `.ipynb` rendering on GitHub) make it the right choice for the demo
         surface; its weaknesses (non-reactive, ugly diffs, hidden state) make it the
         wrong choice for the authoring tool. Splitting the roles lets both shine.

      **Why not Jupyter for the authoring tool:** `.ipynb` is JSON-with-embedded-outputs
      so diffs and PR review are painful; cells aren't reactive so the "filter upstream,
      codelist downstream" UX requires manual re-runs; hidden state from out-of-order
      execution is a real hazard when the output is a committed codelist.

      **Why not marimo for the demo:** Python-only cells; younger ecosystem; less
      familiar to the average clinical-data-science user arriving from Colab.

### Further exploration surfaces

- [ ] **JupyterLab magic + IPython reprs** - `%sct` magic wrapping the CLI, plus
      `_repr_html_` on a `ConceptId` type so `8517006` renders as a rich card (FSN,
      parents, children, refset memberships) inline. Pandas accessor: `series.sct.describe()`
      on any Series of SCTIDs. Lower-ceiling than marimo but meets users where they are.

- [ ] **NetworkX / igraph adapter** over the IS-A closure. Enables centrality,
      community detection, shortest-path queries ("semantic distance between asthma
      and COPD"). Probably just a helper function `sct.to_networkx(db_path, root=...)`
      that materialises a subgraph - the full 4.5M-edge graph is too large for
      interactive NetworkX use.

- [ ] **Kuzu graph export** - embedded, Cypher, no server. Better fit than Neo4j for
      a local-first tool. Export the full relationship graph (not just IS-A) so users
      can run `MATCH (d:Disorder)-[:FINDING_SITE]->(b:BodyStructure) WHERE ...`.
      Constraint: the current NDJSON schema captures relationships; need to confirm
      the Kuzu DDL generation story.

- [ ] **HuggingFace datasets card** publishing the embeddings + concept metadata as a
      reusable dataset. Instant reach into clinical-NLP / LLM-eval repos. Licence
      implication: only publishable for the IPS Free Set or user-ingested content, not
      the UK release.

- [ ] **LangChain / LlamaIndex retriever** - a `SnomedRetriever` class that RAG apps
      can import. Turns `sct` into infrastructure for clinical agents rather than a
      standalone tool. Thin wrapper over `sct semantic` + `sct lexical`.

- [ ] **DSPy recipe for concept normalisation** - free-text symptom → candidate SCTID
      with confidence, as a reusable DSPy signature. Good demo of the embeddings
      + FTS combo and a natural "ship a notebook" artefact.

- [ ] **UMAP / HDBSCAN embedding dashboard** - drop concepts on a 2D canvas, lasso-select,
      export the selection as a codelist. "SNOMED microscope." Likely implemented inside
      the marimo notebook rather than as a separate surface.

- [ ] **Observable / D3 hierarchical viewer** served by a local `sct serve --ui` - radial
      tree of descendants, zoomable, with refset overlay colouring. Complements the
      notebook story for users who want a GUI without installing Python.

### Editor & desktop integrations (AEHRC interop)

AEHRC (the CSIRO team behind Ontoserver) ships two open-source tools that are natural front-ends for `sct`'s local engine, precisely because they speak standard FHIR R4 terminology operations - exactly what `sct serve` already implements. Both reinforce the payoff of `sct serve` being genuinely FHIR-conformant: existing terminology tooling can adopt it as a fast, offline, local backend with zero code changes. Researched July 2026.

- [ ] **`aehrc/ecl-lsp` as an editor front-end for the ECL engine.** A Language Server Protocol implementation for SNOMED CT ECL, with plugins for VSCode, IntelliJ, Eclipse, Neovim, Sublime, and Emacs: real-time ECL diagnostics, completion, hover, formatting, eight refactoring actions, and an inline concept-count code lens. It resolves concepts and evaluates ECL through a **configurable FHIR terminology server** (`ValueSet/$expand`, `CodeSystem/$validate-code`, `$lookup`, under the `ecl.*` settings namespace). Since [`sct serve`](../docs/commands/serve.md) implements exactly those operations with a full ECL-aware `$expand`, it should be a drop-in **offline, local** backend - editor-integrated ECL authoring with no Ontoserver/Snowstorm and no network round-trip.
  - **Near-term:** verify and document pointing ecl-lsp at `sct serve` (a docs recipe plus a conformance check over the specific operations/parameters ecl-lsp actually calls).
  - **Larger:** a native `sct lsp` mode so no server process is needed at all - `sct` already owns the ECL engine, so the work is the LSP plumbing (stdio JSON-RPC, which the MCP server already frames) rather than any new terminology logic.
  - Source: <https://github.com/aehrc/ecl-lsp>
- [ ] **`aehrc/codeagogo` - system-wide code lookup backed by `sct`.** A macOS menu-bar utility for clinical-terminology lookup/search/annotation from any application via global hotkeys; it auto-detects SNOMED codes with Verhoeff check-digit validation and validates concepts against a terminology server in the background, warning on inactive or unknown concepts. Two angles: (1) point codeagogo at a local `sct serve` instead of a remote Ontoserver, for offline system-wide lookup over the exact release the user licenses; (2) harvest the ideas into `sct` itself - Verhoeff check-digit auto-detection in `sct lookup`, and the inactive-concept warning, which dovetails directly with the concept-history/inactivation feature above. Source: <https://github.com/aehrc/codeagogo>

### Clinical-data interoperability

- [ ] **MIMIC / eICU crosswalk notebook** - joins the public MIT ICU datasets to SCT via
      existing ICD-10 / dm+d maps, produces prevalence heatmaps by hierarchy top-level.
      Strong Colab/Kaggle demo because the ICU data is already on those platforms.
      Constraint: MIMIC requires PhysioNet credentialled access; the notebook should work
      against the demo subset for unauthenticated users.
      Pointer: <https://physionet.org/content/mimiciv-demo/>

- [ ] **SNOMED CT AI Benchmark - entity-linking normalisation backend** - SNOMED
      International, with DrivenData and Veratai, has turned its 2023-24 *Entity Linking
      Challenge* into a **continuous AI Benchmark** that scores how well models code
      free-text clinical notes to SNOMED CT (podcast "Measuring AI against SNOMED CT",
      introduced June 2026). The task decomposes into two stages:
      1. **Clinical entity recognition (span detection)** - find the spans of text that
         name a clinical concept. This is genuinely an ML/NER problem (the baseline used a
         fine-tuned DeBERTa BIO token classifier) and is **out of scope for `sct`** - we
         have no custom model and don't intend to ship one.
      2. **Entity linking / normalisation** - map each detected span to a specific SCTID.
         This is candidate generation + disambiguation against the terminology, and it is
         **exactly what `sct` already is**: a fast, local, zero-server lookup engine over
         every SNOMED description. The winning "KIRI" team used a *dictionary-based*
         method; `sct lexical` (FTS5), `sct fst` (sub-ms prefix/fuzzy), and `sct semantic`
         (embeddings) together are a dictionary-based normaliser with a fuzzy and a
         semantic tier on top.

      Why this is worth doing:
      - **A drop-in normalisation library/service for anyone building an entity-linking
        pipeline.** They bring the NER (or an LLM span extractor); `sct` answers
        "span text → ranked SCTID candidates" in microseconds, offline, over the exact
        release they license. Natural fit for the planned live/stdio search component and
        the `SnomedRetriever` (LangChain/LlamaIndex) and DSPy normalisation items above.
      - **A published dictionary-only baseline on the benchmark.** Run `sct` as the linker
        stage against gold spans and report the score - a concrete, reproducible
        "how far does pure fast lexical/fuzzy/semantic matching get you" number, and a
        compelling demo of the search backends.
      - **Data + metric are already in our orbit.** Ground truth is MIMIC-IV-Note
        discharge summaries on PhysioNet (the same corpus as the MIMIC crosswalk item
        below); scoring is macro-averaged **character-level IoU**. Both are tractable to
        wire into `benchmarks/`. Constraint: MIMIC needs PhysioNet credentialled access, and
        the annotated challenge set has its own DUA - do not redistribute either in `sct`.

      Pointers: podcast/announcement
      <https://forums.snomed.org/t/podcast-measuring-ai-against-snomed-ct-introducing-the-snomed-ct-ai-benchmark/1427>;
      benchmark write-up <https://drivendata.co/blog/snomed-ct-entity-linking-benchmark>;
      winners <https://drivendata.co/blog/snomed-ct-entity-linking-challenge-winners>;
      dataset <https://physionet.org/content/snomed-ct-entity-challenge/>;
      winning code <https://github.com/drivendataorg/snomed-ct-entity-linking>;
      JAMIA paper <https://doi.org/10.1093/jamia/ocaf104>.

- [ ] **OMOP CDM bridge** - bidirectional mapping between OMOP `concept_id` /
      vocabulary IDs and SCTIDs. Would land `sct` inside the OHDSI workflow. Needs
      ingestion of the OMOP vocabulary CSVs (Athena download) and a `concept_maps`
      extension. Pointer: <https://athena.ohdsi.org/>

- [ ] **FHIR ValueSet / ConceptMap round-trip** - emit a `.codelist` as a FHIR
      `ValueSet` resource and re-ingest the result of a terminology server `$expand`.
      Makes `sct codelist` interoperable with Ontoserver, Snowstorm, and the NHS FHIR
      Terminology Server. Natural pairing with the in-progress `sct serve` work -
      same data model, opposite direction.

### LLM-assisted authoring

- [ ] **"Explain this refset" agent** - an MCP client + small model prompt that writes
      plain-English rationale for every member of a refset, flagging outliers ("this
      concept sits under X, unlike the other 27 members which sit under Y - intentional?").
      Run nightly on curated refsets; commit diffs as a form of continuous review.
      Uses: detecting refset drift, onboarding new curators, sanity-checking imports.

- [ ] **Semantic drift summariser** - layer on top of `sct diff`. Raw row counts become
      an LLM-summarised narrative: "Release added 412 concepts, mostly under
      Pharmaceutical/biologic product (COVID-19 boosters); 37 concepts inactivated in
      Clinical finding, of which 31 replaced by more specific children." Much higher
      signal-to-noise than the current diff output.

### The too-wild one

- [ ] **`sct mud` - SNOMED as a text adventure.** Rooms are concepts, exits are
      relationships. `> go finding-site` walks from *Myocardial infarction* into *Heart
      structure*; `> look` shows FSN, synonyms, and sibling concepts as "other travellers
      here"; `> inventory` is your in-progress codelist. An optional LLM dungeon-master
      narrates the clinical picture as you wander. Absurd on the surface, but it's the
      first interface that would make a medical student *play* with the ontology, and
      the traversal patterns discovered during play are genuinely useful codelist seeds.
      Ship as a subcommand; minimal dependencies; pure terminal UX.
