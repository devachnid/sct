# SNOMED Local-First Tooling - Roadmap

Outstanding work and next steps. Completed work is removed; see git log for history.

---

## In progress / near-term

### Distribution

Shipped: multi-platform release binaries (including Windows x86_64 and Linux aarch64), SHA-256 checksums, `install.sh` / `install.ps1`, cargo-binstall, a Homebrew tap, and a Scoop bucket - all auto-bumped by the release workflow. See the README install section. Outstanding:

- [ ] macOS code signing + notarization (requires Apple Developer ID, $99/yr) so users
      don't have to `chmod +x` and bypass Gatekeeper
- [ ] Windows Authenticode signing (requires cert from CA) so SmartScreen doesn't block
- [ ] `.deb` / `.rpm` via `cargo-deb` / `cargo-generate-rpm`, attached to GitHub Releases
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

Core shipped: `new`, `add` (including `--ecl` and stdin `-`), `remove`, `validate`, `stats`, `diff`, and `export` to csv / opencodelists-csv / markdown (with `--include-maps` crosswalks). See [`docs/commands/codelist.md`](commands/codelist.md). Outstanding:

- [ ] `sct codelist export <file> --format fhir-json/rf2` - remaining export formats
- [ ] **Multi-terminology codelists (format v2)** - future extension once Read v2 /
      ICD-10 / OPCS-4 maps are fully ingested. Would allow `terminology: [SNOMED CT,
      CTV3]` with first-class non-SNOMED codes (for historical Read v2 codes that
      have no modern SNOMED equivalent). The `--include-maps` export above is the
      interim solution for SNOMED-canonical lists; v2 is for genuinely cross-terminology
      source artefacts.
- [ ] `sct codelist search <file> <query>` - interactive FTS5 search → include/exclude
- [ ] `sct codelist import --from <source>` - OCL, CSV, RF2, FHIR import
- [ ] **Composable codelists** - let a `.codelist` include/reference other `.codelist`
      files, so lists can be built from reusable building blocks (e.g. a "diabetes"
      list that pulls in "type-1-diabetes" and "type-2-diabetes" sub-lists). This gives
      a flat-file, version-controllable, *transparent* way to compose terminology sets -
      doing for codelists what refsets and ECL do, but legibly in plain text rather than
      opaquely. Open design questions: include syntax (a front-matter `includes:` list,
      or an `@include <path-or-url>` body directive?); local-path vs URL/OCL references;
      how to resolve and flatten transitively (and detect cycles); whether the resolved
      members are materialised inline on `add`/`export` or kept as live references and
      expanded on demand; how `diff`/`stats`/`validate` report composed vs direct members;
      and interaction with ECL (an included list is just another concept source, so the
      two should compose). Pairs naturally with the ECL work - both are ways to *specify
      intent* rather than enumerate concepts; ECL is terse and powerful, composition is
      transparent and reviewable.

### Interactive "search as you type"

- [ ] **Live incremental search mode** - an interactive terminal mode where results
      update on every keystroke. A genuinely compelling CLI demo (terminology lookup
      feels instant), and the basis for an embeddable search component later.
  - **Pluggable backends.** The same UI over a selectable backend: `--backend fts5`
        ([`sct lexical`](commands/lexical.md) / SQLite), `--backend fst`
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
- [ ] **History files** - parse RF2 history substitution tables to map inactivated concept IDs
      forward to their replacements; expose via `snomed_resolve` MCP tool
- [ ] **`sct serve`** - HTTP FHIR R4 terminology server backed by SQLite. Drop-in replacement
      for Ontoserver, Snowstorm, and the NHS FHIR Terminology Server. Full spec in
      [`specs/commands/serve.md`](commands/serve.md).

  **Phase 1 - Core operations** (`$lookup`, `$validate-code`, `$subsumes`, `$expand` with
  text filter, CapabilityStatement, OperationOutcome errors, FHIR batch Bundle)

  **Phase 2 - ECL hierarchy** (`ValueSet/$expand` with `<<`, `<!`, `>>`, `>!`, boolean
  operators; pagination; `ValueSet/$validate-code`; `CodeSystem` resource read;
  `--fhir-base` path prefix for Ontoserver-compatible URLs)

  **Phase 3 - Refsets + ConceptMap** (`^` ECL member-of operator now unblocked - Simple
  refsets load into the `refset_members` table via `sct ndjson --refsets simple` + `sct sqlite`;
  `ConceptMap/$translate` for CTV3, Read v2, ICD-10, OPCS-4; complex/map/association refsets
  still to come via `--refsets all`)

  **Phase 4 - R5 + hardening** (FHIR R5 CapabilityStatement; named ValueSet registry;
  Docker image / systemd unit; full ECL attribute filter support - stretch goal)
- [ ] **`sct ndjson --refsets all`** - extend RF2 ingestion beyond Simple refsets to cover the
      remaining derivative-2 refset shapes. The CLI flag and `RefsetMode::All` enum variant
      already exist (added with the Simple refset work) and currently bail with "not yet
      implemented". Concretely needs:
      - **Complex refsets** (`der2_Refset_Complex*Snapshot*.txt`) - adds attribute payload columns
        beyond simple membership; needs a wider row type and a strategy for surfacing those
        attributes to downstream consumers
      - **Association refsets** (`der2_cRefset_Association*Snapshot*.txt`) - `SAME_AS`,
        `REPLACED_BY`, `MAY_BE_A`, etc. Foundation for the `History files` item below
      - **Attribute value refsets** (`der2_cRefset_AttributeValue*Snapshot*.txt`) - concept-to-value
        annotations used by some UK national refsets
      - **Extended map refsets** (`der2_iissssRefset_ExtendedMap*Snapshot*.txt`) - structured
        SNOMED→ICD-10 / OPCS-4 / LOINC map data; needs a new `concept_maps_rf2` table (designed
        in `specs/commands/serve.md`) to capture map_group, map_priority, map_rule, map_advice,
        correlation. This is the prerequisite for full `ConceptMap/$translate` in `sct serve`
        beyond the CTV3/Read v2 maps already supported.

      Each refset family gets its own table or column extension; `refset_members` (concept-only,
      already shipped) stays as-is.

- [ ] **Concept maps** - cross-map support: load SNOMED→ICD-10/OPCS-4 map files from RF2 and
      expose via `snomed_map` MCP tool
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
