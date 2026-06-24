# sct dmwb

Read NHS Data Migration Workbench (DMWB) `.mdb` (Microsoft Access) files via the pure-Rust [`jetdb`](https://crates.io/crates/jetdb) reader - no MS Access, no `mdbtools`. **Optional feature:** build with `--features dmwb`.

The goal is to understand the maps DMWB carries that are *not* in standard SNOMED RF2 - chiefly the **Read v2** cross-maps (Read v2 was retired in 2020). The ICD-10 / OPCS-4 / CTV3 maps and concept history that DMWB also bundles are already available from the RF2 `sct` downloads via [`sct ndjson --refsets all`](ndjson.md), so prefer that route for those.

> **Validation status (June 2026).** `jetdb` 0.3 decodes DMWB's all-Text map tables cleanly (e.g. `SCTICDMAP`), but DMWB stores the **Read v2 code in a Binary `SCUI` column that jetdb 0.3 returns as empty**. So the Access-file Read v2 import is not viable through this path. TRUD item 9, **NHS Data Migration**, has now been confirmed as the preferred source for Read v2: it is the final April 2020 flat-file release. See [Read v2 via TRUD item 9](../dmwb/read-v2-item9.md).

## Subcommands

```bash
# List the tables in a DMWB .mdb
sct dmwb tables "DMWB NHS Data Migration Maps.mdb"

# Inspect a table's columns, types, and first rows
sct dmwb dump "DMWB NHS Data Migration Maps.mdb" SCTICDMAP --limit 5
```

`dump` reports each column's Access type and warns when a table has Binary columns that jetdb cannot decode.

## Read v2 maps

Use TRUD item 9 instead of the DMWB `.mdb` pack:

```bash
sct trud download --edition nhs_data_migration
```

The primary source file is
`Mapping Tables/Updated/Clinically Assured/rcsctmap2_uk_20200401000001.txt`.
It carries ReadCode, TermCode, target SNOMED ConceptId, target DescriptionId,
`IS_ASSURED`, effective date, and map status. The import methodology is documented
in [Read v2 via TRUD item 9](../dmwb/read-v2-item9.md).

## Licensing

NHS terminology and map data is Crown Copyright under the Open Government Licence, distributed via [TRUD](https://isd.digital.nhs.uk/trud) under your own subscription. `sct` reads your local files and never redistributes them.
