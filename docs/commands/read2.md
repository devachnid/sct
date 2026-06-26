# sct read2

Import the final NHS Data Migration Read v2 maps from TRUD item 9 into an
existing `sct` SQLite database.

Most users should prefer the one-command workspace build:

```bash
sct trud download --multi-terminology
```

That downloads the UK Monolith, builds SQLite with inactive concepts and all RF2
maps/history, downloads TRUD item 9, and imports Read v2 into the same database.

Use `sct read2 import` when you already have both files locally.

## Usage

```bash
sct read2 import --archive nhs_datamigration_29.0.0_20200401000001.zip --db snomed.db
```

| Option | Description |
|---|---|
| `--archive <ZIP>` | TRUD item 9 NHS Data Migration archive. |
| `--db <FILE>` | SQLite database to update. Auto-discovered when omitted. |

## What It Imports

The importer reads the primary clinically assured Read v2 map:

```text
Mapping Tables/Updated/Clinically Assured/rcsctmap2_uk_20200401000001.txt
```

It applies the active-map rule from the NHS Data Migration documentation:

1. Group rows by `MapId`.
2. Keep the latest `EffectiveDate` row for each `MapId`.
3. Store it with `active = MapStatus > 0`.
4. Query commands use active rows by default.

Rows are stored in `crossmaps` as `read2 -> snomed` mappings. The importer
preserves:

- ReadCode + TermCode source key, e.g. `0111.00`
- target SNOMED concept
- target `DescriptionId`
- `IS_ASSURED`
- `MapId`
- `EffectiveDate`
- `MapStatus`
- item 9 source provenance

For compatibility with older code paths, active Read v2 rows are also written to
the legacy `concept_maps` reverse-lookup table.

## Examples

```bash
# Download item 9 only
sct trud download --edition nhs_data_migration

# Import it into an existing SNOMED database
sct read2 import \
  --archive ~/.local/share/sct/releases/nhs_datamigration_29.0.0_20200401000001.zip \
  --db snomed.db

# Use the imported maps
echo '0111.00' | sct transcode --from read2 --to snomed --db snomed.db
sct crosswalk 0111.00 --from read2 --db snomed.db
```

## Notes

The Read v2 source key is the five-character ReadCode plus the two-character
TermCode, with no separator:

```text
<ReadCode><TermCode>
```

For example, `0111.` + `00` becomes `0111.00`.
