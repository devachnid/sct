# SNOMED Local-First Tooling - Roadmap

Outstanding work and next steps. Completed work is removed; see git log for history.

---


### TODO

In no particular order

* [ ] obtain Windows signing key https://ngrok.com/blog/so-you-want-to-sign-for-windows
* [~] revise the benchmarks and automate them, so that we end up with a nice-looking, comprehensive benchmarking comparison which includes comparing `sct` with local or remote Terminology servers, as well as comparing within `sct` the different search backends (`lexical` vs `fst`) and the impact of different index configurations (e.g. with or without labels). FHIR conformance fixtures and a terminology-server runner now exist in `bench/conformance.sh`; remaining work is broader fixture coverage, concurrency/percentile benchmarks, comparator server compose profiles, and published reports.
* [ ] Docker Hub image for the terminology server. `Dockerfile`, Compose, and docs are shipped;
      remaining work is publishing/versioning the image and documenting pull-based deployment.
* [ ] improve distribution - remaining work is signing/notarization and registry submissions;
      `.dmg`, `.exe`, `.deb`, `.rpm`, shared Homebrew tap, Scoop bucket, shell installers,
      crates.io, cargo-binstall, and Docker Compose are already shipped.
* [ ] mermaid diagrams for the architecture and data flow, to visually explain how the different components fit together and how data moves through the system. This would replace the ascii diagrams in the README and make it easier for users to understand the overall design and how the different pieces interact. FST can more easily be explained this way. We should use real SNOMED examples in the diagrams to make them more concrete and relatable.
* [ ] SNOMED primer - understanding SNOMED basics (concepts, descriptions, relationships, refsets, ECL, etc.) is a barrier to entry for new users. A concise primer that explains these core concepts in plain language, with examples, is needed. It may need to take different approaches for technical vs clinical audiences. It should be in a section of the docs.

## In progress / near-term

### Distribution

Shipped: multi-platform release binaries (including Windows x86_64 and Linux aarch64), SHA-256 checksums, `.deb` / `.rpm` packages, unsigned macOS `.dmg` images, standalone Windows `.exe`, `install.sh` / `install.ps1`, cargo-binstall, crates.io, the shared `pacharanero/tap` Homebrew tap, a Scoop bucket, and Docker Compose quickstart for the terminology server. Release artefacts and package-manager manifests are auto-bumped by the release workflow. See the docs installation tabs. Outstanding:

- [ ] Publish a Docker Hub / GHCR image for `sct serve`, with tags matching `sct` releases
- [ ] macOS code signing + notarization (requires Apple Developer ID, $99/yr) so users
      don't have to `chmod +x` and bypass Gatekeeper
- [ ] Windows Authenticode signing (requires cert from CA) so SmartScreen doesn't block
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

Core shipped: `new`, `add` (including `--ecl` and stdin `-`), `remove`, `validate`, `stats`, `diff`, and `export` to csv / opencodelists-csv / markdown (with `--include-maps` crosswalks). See [`docs/commands/codelist.md`](../docs/commands/codelist.md). Outstanding:

- [ ] `sct codelist export <file> --format fhir-json/rf2` - remaining export formats
- [ ] **Multi-terminology codelists (format v2)** - future extension now that the
      terminology workspace can contain SNOMED CT, CTV3, Read v2, ICD-10, and OPCS-4.
      Would allow first-class non-SNOMED source codes in a codelist, e.g. historical
      Read v2 codes with no modern SNOMED equivalent, instead of treating SNOMED as
      the canonical pivot for every list. The `--include-maps` export is the interim
      solution for SNOMED-canonical lists; v2 is for genuinely cross-terminology source
      artefacts.
- [ ] `sct codelist search <file> <query>` - interactive FTS5 search → include/exclude
- [ ] `sct codelist import --from <source>` - OCL, CSV, RF2, FHIR import
- [x] **Composable codelists** ✅ **shipped** - a `.codelist` composes others via an
      `includes:` front-matter list. References use a Docker-registry model: a bare id
      resolves to `<registry>/<id>.codelist` (registry defaults to `./codelists`,
      overridable via `--codelists` / `SCT_CODELISTS` / `[codelists] dir`), a path is
      relative to the including file, and an `http(s)://` URL is fetched and cached. Members
      are resolved live (own + included, recursively, parent exclusions win) by `stats`,
      `validate`, `export`, `diff`, and `sct serve`; `sct codelist include` edits the
      reference list and `sct codelist resolve` flattens to a standalone snapshot. Cycles
      and missing includes are detected. See [`docs/commands/codelist.md`](../docs/commands/codelist.md).
      Remaining: multi-terminology composition (format v2) and OCL references.

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

---

## Future / larger scope

> **Cross-terminology mapping + DMWB replacement.** The RF2-native terminology/mapping
> core is now shipped: CTV3 maps, SNOMED CT -> ICD-10 / OPCS-4 ExtendedMap rows,
> Association-refset history forwarding, `sct transcode`, `sct crosswalk`,
> codelist `--include-maps`, FHIR `ConceptMap/$translate`, and Read v2 import
> from the final TRUD item 9 flat-file release. `sct trud download
> --multi-terminology` now builds a SNOMED/CTV3/Read v2/ICD-10/OPCS-4 workspace
> in one command. The Access `.mdb` path remains intentionally documented as
> historical analysis only: `jetdb` cannot decode DMWB's Binary `SCUI` column,
> and item 9 is the cleaner authoritative source. See
> [`specs/cross-terminology-mapping.md`](cross-terminology-mapping.md) and the
> DMWB walkthrough in the docs site.

- [ ] **History MCP surface** - RF2 Association history is parsed and loaded into
      `concept_history`, and `sct transcode --forward-history` uses it. Still missing:
      expose the same forwarding through an MCP `snomed_resolve` tool.
- [~] **`sct serve`** - HTTP FHIR R4 terminology server backed by SQLite. Drop-in replacement
      for Ontoserver, Snowstorm, and the NHS FHIR Terminology Server. Full spec in
      [`specs/commands/serve.md`](commands/serve.md); user docs in
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
- [x] **Read v2 import** ✅ **shipped** - `sct read2 import` loads TRUD item 9
      (`nhs_datamigration_29.0.0_20200401000001.zip`) from
      `rcsctmap2_uk_20200401000001.txt`, selecting latest `EffectiveDate` per
      `MapId`, storing `MapStatus > 0` as the active flag, and preserving
      `DescriptionId`, `IS_ASSURED`, `MapId`, `EffectiveDate`, and source
      provenance in `crossmaps`. `sct trud download --multi-terminology` builds
      the full SNOMED/CTV3/Read v2/ICD-10/OPCS-4 workspace in one command.
- [ ] **First-class ICD-10 / ICD-11 support** - current `sct` support for ICD-10 is
      map-centric: UK/International SNOMED CT -> ICD-10 ExtendedMap rows are
      already imported into `crossmaps` with `sct ndjson --refsets all`, and
      `sct transcode`, `sct crosswalk`, codelist `--include-maps`, and FHIR
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

### Clinical-data interoperability

- [ ] **MIMIC / eICU crosswalk notebook** - joins the public MIT ICU datasets to SCT via
      existing ICD-10 / dm+d maps, produces prevalence heatmaps by hierarchy top-level.
      Strong Colab/Kaggle demo because the ICU data is already on those platforms.
      Constraint: MIMIC requires PhysioNet credentialled access; the notebook should work
      against the demo subset for unauthenticated users.
      Pointer: <https://physionet.org/content/mimiciv-demo/>

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
