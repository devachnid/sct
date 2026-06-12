# sct dmwb

Read NHS Data Migration Workbench (DMWB) `.mdb` (Microsoft Access) files via the pure-Rust [`jetdb`](https://crates.io/crates/jetdb) reader - no MS Access, no `mdbtools`. **Optional feature:** build with `--features dmwb`.

The goal is to ingest the maps DMWB carries that are *not* in standard SNOMED RF2 - chiefly the **Read v2** cross-maps (Read v2 was retired in 2020). The ICD-10 / OPCS-4 / CTV3 maps and concept history that DMWB also bundles are already available from the RF2 `sct` downloads via [`sct ndjson --refsets all`](ndjson.md), so prefer that route for those.

> **Validation status (June 2026).** `jetdb` 0.3 decodes DMWB's all-Text map tables cleanly (e.g. `SCTICDMAP`), but DMWB stores the **Read v2 code in a Binary `SCUI` column that jetdb 0.3 returns as empty**. So the Read v2 import - the one DMWB-unique datum - is **not yet viable** through this path. `sct dmwb` currently provides introspection only; see [`specs/cross-terminology-mapping.md`](https://github.com/pacharanero/sct/blob/main/specs/cross-terminology-mapping.md) §9.3 for the paths forward (TRUD item 9 flat files / upstream jetdb Binary support / an `mdbtools` pre-export).

## Subcommands

```bash
# List the tables in a DMWB .mdb
sct dmwb tables "DMWB NHS Data Migration Maps.mdb"

# Inspect a table's columns, types, and first rows
sct dmwb dump "DMWB NHS Data Migration Maps.mdb" SCTICDMAP --limit 5
```

`dump` reports each column's Access type and warns when a table has Binary columns that jetdb cannot decode.

## Licensing

NHS terminology and map data is Crown Copyright under the Open Government Licence, distributed via [TRUD](https://isd.digital.nhs.uk/trud) under your own subscription. `sct` reads your local files and never redistributes them.
