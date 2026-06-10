# Cross-terminology mapping, history, and the DMWB replacement

Status: **design spec** (June 2026). Drives a multi-phase build. Companion to
[`specs/roadmap.md`](roadmap.md) (the `sct serve`, `--refsets all`, history, and
concept-map items) and [`specs/commands/serve.md`](commands/serve.md).

## 1. Context and goal

The NHS **Data Migration Workbench (DMWB)** is an end-of-life Microsoft Access
application that maps across the NHS terminologies (Read v2, CTV3, SNOMED CT,
ICD-10, OPCS-4) with SNOMED CT as the reference terminology. A June 2026 gap
analysis (see the conversation that produced this spec) found DMWB is really two
products:

- **(A) a terminology cross-mapping + browsing + cluster tool** (the `BROWSE` and
  `QUERIES` screens) - the part most users touch; and
- **(B) an EPR patient-data migration + casemix analytics platform** (the `EPR
  DATA` screen) - a different product category.

**This spec covers replacing (A).** Part (B) - loading patient records, miscode
repair, casemix `INDUCE`/`GRAPH` analytics - is an explicit non-goal (see §10).

A proof-of-concept (June 2026) extracted DMWB's `.mdb` cross-maps with a pure
reader and loaded them against the real `snomed.db`: **102k Read v2→SNOMED + 118k
SNOMED→ICD-10 rows in 7.7 s**, fitting `sct`'s existing `concept_maps` model with
one new table, and ran the full `Read v2 → SNOMED → ICD-10` migration crosswalk
correctly (e.g. `Read 123.. → 160279000 "Family history of infectious disease" →
ICD-10 Z831`). The assurance flag carried through.

**Key discovery:** the SNOMED→ICD-10/OPCS-4 maps, the CTV3 maps, and the SNOMED
history/inactivation data are **already inside the UK Monolith RF2 that `sct`
already downloads** (as ExtendedMap, SimpleMap, and Association refsets) - so they
need only *parsing*, not a new data source. The **only** DMWB-unique data is the
**Read v2 maps** (Read v2 was retired in 2020; its maps are not in current RF2)
and the **Code Usage** frequency data.

## 2. Data sources and acquisition

### 2.1 Channel A - RF2-native (no DMWB needed)

The UK Monolith zip (`sct trud --edition uk_monolith`) already contains, as flat
RF2 text files `sct` can parse:

| File (Snapshot) | Yields | Today |
|---|---|---|
| `der2_iisssccRefset_ExtendedMap…` / `der2_iisssciRefset_ExtendedMap…` | SNOMED → **ICD-10** and **OPCS-4** maps (with map group / priority / rule / advice / correlation) | not parsed |
| `der2_iissscRefset_ComplexMap…` | complex maps (older shape) | not parsed |
| `der2_sRefset_SimpleMap…` | **CTV3** ↔ SNOMED | ✅ parsed |
| `der2_cRefset_Association…` | **historical associations** (REPLACED BY, SAME AS, POSSIBLY EQUIVALENT TO, MOVED TO, ALTERNATIVE) | not parsed |
| `der2_cRefset_AttributeValue…` | concept **inactivation reason** | not parsed |

This is the bulk of the value and needs no DMWB and no new acquisition - it is
the `--refsets all` work the roadmap already names.

### 2.2 Channel B - DMWB-unique data (via TRUD, like RF2)

Answering the design question directly: **yes, `sct trud` can acquire DMWB the
same way it acquires RF2.** DMWB is a TRUD item; the existing download/SHA-verify
machinery is item-agnostic.

- **TRUD item 98** - "SNOMED CT UK Data Migration Workbench": the Access pack,
  pre-loaded with SNOMED / CTV3 / Read / OPCS-4 / ICD-10 and the crossmaps. This
  is where the **Read v2 maps** (`RCTSCTMAP`, `RCTCTV3MAP`) and **Code Usage**
  live. Format: a zip of Jet/ACE `.mdb` files.
- **TRUD item 9** - "NHS Data Migration": the underlying many:one forward/backward
  crossmap products. **Open question (§11):** confirm whether item 9 ships the
  maps as *flat files*; if so it is the preferred source for Read v2 maps and
  avoids `.mdb` parsing entirely.

Add a built-in edition mapping (in `trud.rs` `builtin_editions()`):

```
dmwb              -> TRUD item 98   (Data Migration Workbench, .mdb pack)
nhs_data_migration -> TRUD item 9   (raw crossmap products; format TBC)
```

`sct trud download --edition dmwb` then reuses everything (releases dir, SHA-256,
`--skip-if-current`). A new `--pipeline` branch (see §6) runs the DMWB importer
instead of `ndjson`/`sqlite`, keyed off the edition's *kind* (terminology vs
mapping pack).

### 2.3 Reading `.mdb` from a single Rust binary

`sct` ships as one static binary with no external runtime deps; shelling out to
`mdbtools` would break that. The chosen approach is the pure-Rust
[`jetdb`](https://crates.io/crates/jetdb) crate (0.3.1, Jet3→ACE17, UTF-16LE),
which needs no ODBC/C libraries.

**Validation gate (blocking):** `jetdb` is young (low download count). Before
committing to it, prove it reads the *real* DMWB tables correctly - especially the
Jet4 UTF-16LE text columns where the POC's Python reader mis-split the Read v2
`SCUI` field (recoverable as the first 5 UTF-16 chars, but a sign the column
offsets are fiddly). If `jetdb` cannot read them cleanly, fall back to: (a) TRUD
item 9 flat files, or (b) a documented `mdbtools` pre-export to TSV that
`sct dmwb import` then ingests (keeps the core binary pure; `.mdb` parsing becomes
an optional power-user path).

### 2.4 Licensing

NHS terminology and map data is Crown Copyright under the **Open Government
Licence**, distributed via TRUD under the user's own TRUD subscription. `sct`
**must not ship or redistribute** any of it (maps, usage data, fixtures) - it
acquires it through the user's TRUD key, exactly as it does RF2 today. `sct`'s
code/parsers are AGPL-3.0-or-later; the data remains the user's, fetched locally.
(Pragmatic note: the NHS is not litigious over terminology-data IP absent NHS
brand-identity infringement, but the no-redistribution rule is cheap and correct.)

## 3. Data model (SQLite)

The POC split maps into two tables; the production model unifies them so all
directions and the ICD/OPCS map metadata fit one shape.

```sql
-- All cross-terminology maps, any direction.
CREATE TABLE crossmaps (
    source_system  TEXT NOT NULL,   -- 'snomed' | 'read2' | 'ctv3' | 'icd10' | 'opcs4'
    source_code    TEXT NOT NULL,
    target_system  TEXT NOT NULL,
    target_code    TEXT NOT NULL,
    map_group      INTEGER,         -- ExtendedMap grouping (alternatives)
    map_priority   INTEGER,         -- order within a group
    map_rule       TEXT,            -- ICD-10 map rule (age/sex/etc), nullable
    map_advice     TEXT,            -- human-readable advice, nullable
    correlation    TEXT,            -- 'exact'|'broad'|'narrow'|'inexact'|'unspecified'
    assured        INTEGER,         -- 1 if clinically assured (DMWB), else 0/NULL
    source_release TEXT,            -- provenance (effectiveTime / MAPVERSIONS)
    PRIMARY KEY (source_system, source_code, target_system, target_code, map_group)
);
CREATE INDEX idx_crossmaps_src ON crossmaps(source_system, source_code);
CREATE INDEX idx_crossmaps_tgt ON crossmaps(target_system, target_code);

-- Concept inactivation + forwarding (from RF2 Association + AttributeValue refsets,
-- or DMWB SCTHREL/SCTSUBST). Lets old records reference retired SCTIDs.
CREATE TABLE concept_history (
    concept_id          TEXT NOT NULL,  -- the inactivated concept
    association         TEXT NOT NULL,  -- 'replaced_by'|'same_as'|'possibly_equivalent_to'|'moved_to'|'alternative'|'was_a'
    target_id           TEXT,           -- replacement concept (NULL where none)
    inactivation_reason TEXT,           -- 'ambiguous'|'outdated'|'erroneous'|'moved_elsewhere'|'duplicate'|...
    PRIMARY KEY (concept_id, association, COALESCE(target_id, ''))
);
CREATE INDEX idx_history_concept ON concept_history(concept_id);

-- Real-world UK primary-care usage bands (DMWB Code Usage; item 98).
CREATE TABLE code_usage (
    code        TEXT NOT NULL,
    terminology TEXT NOT NULL,
    usage_band  INTEGER,             -- 1=top1000, 2=top5000, 3=top10000, 4=beyond, NULL=unknown
    PRIMARY KEY (code, terminology)
);

-- Provenance for each loaded map/dataset.
CREATE TABLE map_versions (
    dataset       TEXT PRIMARY KEY,  -- 'extended_map_icd10','rctsctmap',...
    release_date  TEXT,
    build_date    TEXT,
    source        TEXT               -- 'rf2' | 'dmwb_item98' | 'nhs_dm_item9'
);
```

**Backward compatibility:** keep the existing `concept_maps(code, terminology,
concept_id)` as a **VIEW** over `crossmaps` (the legacy→SNOMED slice), so today's
consumers (`sct codelist export --include-maps`, the CTV3 reverse lookup) keep
working unchanged:

```sql
CREATE VIEW concept_maps AS
  SELECT source_code AS code, source_system AS terminology, target_code AS concept_id
  FROM crossmaps WHERE target_system = 'snomed';
```

**Inactive concepts:** history forwarding needs to recognise retired SCTIDs.
`sct ndjson` currently drops inactive concepts. Add a slim `inactive_concepts(id
TEXT PRIMARY KEY, fsn TEXT, reason TEXT)` table (rather than carrying full inactive
rows in `concepts`) so we can forward and label them without bloating the active
graph. `--include-inactive` may additionally retain them in `concepts(active=0)`.

## 4. Ingestion - Channel A (RF2 refsets)

In `src/rf2.rs` / `src/commands/ndjson.rs` / `src/commands/sqlite.rs`:

1. **File detection** - extend `Rf2Files` to find `ExtendedMap`, `ComplexMap`,
   `Association`, and `AttributeValue` refset files (mirror the existing
   `SimpleMap` detection at `rf2.rs` ~L172).
2. **Parsers** - row structs + `parse_extended_map`, `parse_association`,
   `parse_attribute_value` (mirror `parse_simple_map` at `rf2.rs` ~L313). Map the
   ExtendedMap `mapTarget`/`mapGroup`/`mapPriority`/`mapRule`/`mapAdvice`/
   `correlationId` columns onto `crossmaps`; map the Association `refsetId`
   (REPLACED BY = 900000000000526001, SAME AS = 900000000000527005, etc.) onto
   `concept_history.association`.
3. **`RefsetMode::All`** - currently bails ("not yet implemented"); implement it
   to include these families. `RefsetMode::Simple` stays the default.
4. **NDJSON carriage** - emit maps/history/inactivation either as sidecar arrays
   on the concept record or as separate NDJSON streams (e.g.
   `snomed.crossmaps.ndjson`); `sct sqlite` loads them into the tables above. Keep
   the canonical concept NDJSON stable.
5. **Inactive retention** - populate `inactive_concepts` from the inactive RF2
   rows (the ones currently filtered out).

## 5. Ingestion - Channel B (`sct dmwb import`)

A new command (and library module) for the DMWB-unique data:

```
sct dmwb import <path-to-dmwb-dir-or-zip> [--db snomed.db]
```

- Reads, via `jetdb`: `RCTSCTMAP` (Read v2→SNOMED), `RCTCTV3MAP` (Read v2→CTV3),
  and the Code Usage table; loads them into `crossmaps` (`source_system='read2'`)
  and `code_usage`. Recovers the Read v2 code from the UTF-16LE `SCUI` field.
- Optionally loads `SCTHREL`/`SCTSUBST` into `concept_history` **only if** the RF2
  Association refset was not used (prefer RF2 as the canonical history source).
- Records provenance in `map_versions` (`source='dmwb_item98'`, from `MAPVERSIONS`).
- Idempotent: `INSERT OR REPLACE` keyed on the primary keys; safe to re-run on a
  new DMWB release.

## 6. Acquisition wiring (`sct trud`)

- Add `dmwb` (item 98) and `nhs_data_migration` (item 9) to `builtin_editions()`.
- `sct trud download --edition dmwb` works immediately (download + SHA verify).
- Extend the `--pipeline` branch in `run_download` to dispatch on edition kind:
  - terminology editions (`uk_monolith`, …) → `sct ndjson` → `sct sqlite` (as now);
  - mapping editions (`dmwb`) → `sct dmwb import` into the existing/target DB.
- End-state one-liner: `sct trud download --edition dmwb --pipeline --db snomed.db`
  downloads item 98 and loads the Read v2 maps + usage into your SNOMED DB.

## 7. Query surface (CLI)

### 7.1 `sct transcode` - DMWB's `TRANSCODE`, composable

```
sct transcode --from read2 --to snomed   < codes.txt
cat codes.csv | sct transcode --from read2 --to icd10 --forward-history --json
```

- Input: newline/CSV codes on stdin or `--input <file>`; the leading token per
  line is the code (so it composes with `sct ecl expand`, `grep`, etc.).
- Output: `input_code, target_code(s), correlation, assured[, display]`; newline
  or `--json`. Many:one and one:many handled (one output row per target).
- `--forward-history`: if a SNOMED target is inactive, forward via
  `concept_history` to its replacement (the migration core).
- Pure stdout for codes (warnings/counts to stderr), per the composability rule.

### 7.2 `sct crosswalk <code>` (and `sct lookup --crosswalk`)

Show every mapped equivalent of a code/concept across all loaded terminologies -
the text equivalent of DMWB's tri-terminology `BROWSE` triad.

### 7.3 `sct codelist` - cross-terminology

- **Export:** extend `--include-maps` to `read2,ctv3,icd10,opcs4` (already exists
  for CTV3-style maps; widen to the new systems).
- **Translate** (DMWB cluster translate): build a SNOMED `.codelist`, emit its
  Read v2 / CTV3 / ICD-10 equivalents - the roadmap's "multi-terminology codelists
  (format v2)". Pairs with composition: a cross-terminology list is just another
  member source.

### 7.4 History forwarding + usage display

- `sct lookup <inactive-sctid>` flags inactivation and prints the replacement.
- ECL/`$expand` gain an opt-in "forward inactive members" mode.
- Search/lookup output can show the usage band (DMWB's `#`/`=`/`-` symbols) when
  `code_usage` is loaded.

## 8. Server surface (`sct serve`)

- **`ConceptMap/$translate`** (FHIR R4) over `crossmaps`:
  `$translate?system=…&code=…&targetsystem=…[&reverse=true]`. This closes the
  roadmap's Phase-3 `$translate` item with real map data.
- **The DMWB Excel add-in can point at this.** Its README states it can target "a
  remote FHIR server"; a local `sct serve --features serve` with `$translate`
  gives analysts the familiar worksheet workflow on a fast, offline backend
  instead of the slow NHS remote - a zero-rewrite migration path for existing DMWB
  Excel users.
- `$lookup` reports inactivation + replacement from `concept_history`.
- (Later) `ConceptMap` resource read/search, mirroring the stored-ValueSet work.

## 9. Phasing

- **1a. RF2-native ICD-10 / OPCS-4 maps** ✅ **shipped** (NDJSON schema v5).
  `sct ndjson --refsets all` parses the ExtendedMap refsets into a per-concept
  `crossmaps` field; `sct sqlite` loads them into the `crossmaps` table
  (`source_system='snomed'` → `target_system='icd10'|'opcs4'`, with map group /
  priority / rule / advice and the source map refset). Refset→system
  classification lives in `rf2::extended_map_system` (seeded with the known UK +
  International refset SCTIDs). Default `--refsets simple` omits them (they are
  large). Tests: `tests/end_to_end.rs` (`extended_maps_load_into_crossmaps`,
  `simple_mode_omits_crossmaps`).
- **1b. RF2-native history + inactive forwarding** (in progress): the Association
  parser + `rf2::association_name` are in place and `Rf2Dataset::history` is
  populated under `--refsets all`; remaining work is the NDJSON→SQLite carriage
  (a sidecar history stream → a `concept_history` table) since history is keyed
  by inactive source concepts absent from the active stream.
2. **`sct transcode` + `sct crosswalk` + history forwarding** over the new tables.
3. **DMWB acquisition** (Channel B): `sct trud --edition dmwb` + `sct dmwb import`
   via `jetdb` → Read v2 maps + Code Usage. **Gated on the jetdb validation (§2.3).**
4. **`sct serve` `ConceptMap/$translate`** (+ DMWB Excel add-in compatibility).
5. **`sct codelist` cross-terminology translate** (multi-terminology v2).

## 10. Non-goals

- The **EPR / casemix patient-data layer** (loading patient records, data-quality
  repair, `INDUCE`/`GRAPH`/trends analytics). Different product; `sct`'s planned
  DuckDB + notebook surfaces are where that would live if ever pursued.
- The Access **GUI** / synchronised tri-terminology *graphical* browser. The CLI
  (`crosswalk`, `transcode`) and the existing `sct gui` suffice; a web tri-pane
  viewer is an optional far-future surface.
- **Redistributing** any NHS terminology/map/usage data.
- ICPC / ICNP maps - present in DMWB but low-priority; defer.

## 11. Open questions / validation gates

- **jetdb correctness** against the real (211 MB–705 MB Jet4) DMWB `.mdb`s,
  especially the Read v2 `SCUI` UTF-16 column. Blocking for Phase 3.
- **TRUD item numbers** - confirm 98 (DMWB) and 9 (NHS Data Migration) against live
  TRUD, and **whether item 9 ships flat-file maps** (would make Channel B pure-Rust
  with no `.mdb` at all - strongly preferred).
- **Map cardinality** - represent many:one and one:many cleanly in `crossmaps` and
  in `transcode` output (one row per target; expose `map_group`/`priority`).
- **ICD-10 ComplexMap rules** (age/sex/refinement `mapRule`) - honour, or store
  verbatim and leave application to the caller? Probably the latter initially.
- **Read v2 vs RCT vs CTV3** naming consistency in `source_system` values.
- **Inactive concepts** - slim `inactive_concepts` table vs full `active=0` rows in
  `concepts`. Default to the slim table.
- **History source of truth** - RF2 Association refset vs DMWB `SCTHREL`. Prefer
  RF2 (canonical, dated, no DMWB dependency); use DMWB only to backfill.

## 12. Why this matters

DMWB is a dying Access museum piece the NHS still depends on for terminology
migration. `sct` already exceeds its `BROWSE`/`QUERIES` core on speed, search,
ECL, codelists, FHIR serving, and portability; the missing piece is *map and
history data*, and the POC proved that data is either already in the RF2 `sct`
downloads or loadable from TRUD in seconds. This spec is the path from "great
SNOMED tool" to "credible open-source DMWB successor" - acquired and pipelined
through the same `sct trud` flow users already know.
