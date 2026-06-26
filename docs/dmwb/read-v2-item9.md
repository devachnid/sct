# Read v2 via TRUD Item 9

TRUD item 9, **NHS Data Migration**, is the route to the Read v2 maps that are
not present in current UK RF2 releases and cannot currently be recovered from
the DMWB Access pack with `jetdb`.

The release to use is:

| Field | Value |
|---|---|
| TRUD item | `9` |
| Archive | `nhs_datamigration_29.0.0_20200401000001.zip` |
| Product status | Final April 2020 production release, deprecated with support |
| Source terminologies | final 5-Byte Read v2 `21.0.0`, final CTV3 `25.0.0` |
| SNOMED CT target | SNOMED CT `29.0.0` / `29.0.1` |

NHS Digital's bundled release note says this final release replaces previous
Data Migration Pack releases and that no further scheduled releases will occur.
That makes it a static historical source rather than a monthly RF2 feed.

## Download

```bash
sct trud download --edition nhs_data_migration
```

Do not use `--pipeline`: this archive is not RF2. It is a zip of flat mapping
tables, documentation PDFs, primary care refsets, and legacy navigation subsets.
Downloading it does not currently modify `snomed.db`; the Read v2 importer is
not shipped yet.

You can also use the raw item number:

```bash
sct trud list --item 9
sct trud download --item 9
```

## What is in the archive

The files relevant to DMWB feature parity are under `Mapping Tables/Updated/`:

| File | Direction | Use |
|---|---|---|
| `Clinically Assured/rcsctmap2_uk_20200401000001.txt` | Read v2 + term code -> SNOMED CT | Primary source for Read v2 import. Carries SNOMED ConceptId, DescriptionId, `IS_ASSURED`, effective date, and map status. |
| `Not Clinically Assured/rcsctmap_uk_20200401000001.txt` | Read v2 + term code -> SNOMED CT | Older/simple table. Useful for comparison, but lacks DescriptionId and assurance metadata. |
| `Not Clinically Assured/rctermsctmap_uk_20200401000001.txt` | Read v2 + term text -> SNOMED CT | Fallback when the source record has text but no term code. |
| `Not Clinically Assured/rcmap_uk_20200401000001.txt` | Read v2 code only -> SNOMED CT | Fallback when the source record has no term code or text. Includes ambiguous `MapStatus=2` rows. |
| `Clinically Assured/rctctv3map_uk_20200401000001.txt` | Read v2 + term code -> CTV3 | Useful for legacy Read v2 -> CTV3 migration. |
| `Clinically Assured/ctv3sctmap2_uk_20200401000001.txt` | CTV3 + term id -> SNOMED CT | Flat-file CTV3 source, though current UK RF2 already gives `sct` CTV3 maps. |

The release also includes map-specific documentation PDFs. The Read v2 ->
SNOMED CT documentation defines the active-map rule and the meaning of
`IS_ASSURED`.

## Analysis Method

The archive is a stored zip, so it can be inspected without extraction:

```bash
unzip -l ~/.local/share/sct/releases/nhs_datamigration_29.0.0_20200401000001.zip
```

The primary Read v2 map header is:

```text
MapId	ReadCode	TermCode	ConceptId	DescriptionId	IS_ASSURED	EffectiveDate	MapStatus
```

Rows are tab-delimited and CR/LF terminated. The bundled documentation says:

| Column | Import meaning |
|---|---|
| `ReadCode` | Five-character Read v2 code. Must be treated case-sensitively. |
| `TermCode` | Usually two-character Read v2 term code. The ReadCode+TermCode pair identifies the source meaning. |
| `ConceptId` | Target SNOMED CT concept. |
| `DescriptionId` | Target SNOMED CT description to which assurance applies. |
| `IS_ASSURED` | `1` clinically assured, `0` not assured. Assurance applies to the ConceptId+DescriptionId pair, not just the concept. |
| `EffectiveDate` | Date the `MapStatus` value takes effect. |
| `MapStatus` | `1` active, `0` inactive. |

The active-map rule is the same one described in the release documentation:

1. Group rows by `MapId`.
2. Keep the row with the latest `EffectiveDate` at or before the target release date.
3. Use it only when `MapStatus > 0`.
4. Query distinct target concepts, because a small number of source pairs have duplicate active rows with different `MapId`s but the same target concept.

For the final April 2020 release, applying that method to
`rcsctmap2_uk_20200401000001.txt` gives:

| Metric | Count |
|---|---:|
| Latest `MapId` rows | 159,464 |
| Active latest rows | 102,057 |
| Active assured rows | 77,079 |
| Active unassured rows | 24,978 |
| Distinct active ReadCode+TermCode pairs | 102,057 |
| Distinct target SNOMED concepts | 68,414 |

The old/simple `rcsctmap_uk_20200401000001.txt` gives 102,152 active latest
rows and 102,066 distinct active ReadCode+TermCode pairs, but it lacks
DescriptionId and assurance metadata. That is why `sct` should prefer
`RcSctMap2`.

## Import Design, Not Yet A Command

This section describes the importer `sct` should grow next. It is not a current
walkthrough step.

The importer should load item 9 into the existing SNOMED SQLite database through
the general `crossmaps` table. It should not collapse Read v2 rows into only the
legacy `concept_maps(code, terminology, concept_id)` shape, because that loses
`DescriptionId`, `IS_ASSURED`, `MapId`, `EffectiveDate`, map status, and source
release provenance.

The production import should store Read v2 -> SNOMED rows with:

- `source_system = 'read2'`
- `source_code = <ReadCode><TermCode>`
- `source_term_code = <TermCode>`
- `target_system = 'snomed'`
- `target_code = ConceptId`
- `target_description_id = DescriptionId`
- `map_source = 'nhs_data_migration_item9'`
- `map_id = MapId`
- `effective_date = EffectiveDate`
- `active = MapStatus > 0`
- `map_status = MapStatus`
- `is_assured = IS_ASSURED`

Recommended source-code encoding for command-line input is the seven-character
Read v2 source key:

```text
<ReadCode><TermCode>
```

For example, `0111.` + `00` becomes `0111.00`, and `0....` + `11` becomes
`0....11`. This matches the way the release documentation writes examples such
as `9113.00`.

Fallback support can come later:

- `RcTermSctMap` when source records contain ReadCode plus term text but no
  TermCode.
- `RcMap` when source records contain only ReadCode. These rows are less precise
  and include ambiguous `MapStatus=2` rows, so they should be exposed explicitly
  rather than silently mixed into the ReadCode+TermCode map.
