# sct transcode

Map a stream of codes from one NHS terminology to another, pivoting through SNOMED CT. This is the `sct` equivalent of the NHS Data Migration Workbench `TRANSCODE` function - the workhorse of clinical data migration (forwarding legacy Read v2 / CTV3 GP records to SNOMED, or crosswalking SNOMED to ICD-10 / OPCS-4 for secondary-care reporting).

**Composable by design:** reads codes from stdin (or `--input`), writes TSV (or `--json`) to stdout, diagnostics to stderr - so it pipes straight into and out of `sct ecl expand`, `cut`, `grep`, `jq`, and `sct codelist`.

## Prerequisite

The easiest way to get all supported maps is:

```bash
sct trud download --multi-terminology
```

That builds a database containing CTV3, Read v2, ICD-10, OPCS-4, and concept
history. If building manually, the ICD-10 / OPCS-4 maps and concept history come
from a database built with [`sct ndjson --refsets all`](ndjson.md):

```bash
sct ndjson --rf2 release.zip --refsets all --output snomed.ndjson
sct sqlite --input snomed.ndjson --output snomed.db
```

CTV3 maps work on a UK-derived `snomed.db`; Read v2 rows come from
[`sct read2 import`](read2.md) over TRUD item 9. ICD-10 / OPCS-4 and
`--forward-history` need `--refsets all`. `sct transcode` fails with a clear
message if the database lacks the required maps.

## Usage

```bash
sct transcode --from <SYSTEM> --to <SYSTEM> [OPTIONS]
```

Systems: `snomed`, `read2`, `ctv3`, `icd10`, `opcs4`.

| Option | Description |
|---|---|
| `--from <SYSTEM>` | Source terminology of the input codes. |
| `--to <SYSTEM>` | Target terminology to map to. |
| `--input <FILE>` | Read codes from a file (leading token per line) instead of stdin. |
| `--forward-history` | Forward inactive SNOMED pivots to their replacement(s) via concept history. |
| `--json` | Emit JSON lines instead of TSV. |
| `--db <FILE>` | SNOMED CT database (auto-discovered when omitted). |

Output columns (TSV): `input_code`, `target_code`, `snomed_pivot`, `display`. One row per mapping (so many:one and one:many both expand to multiple rows).

## Examples

```bash
# SNOMED -> ICD-10 for a list of concepts
printf '22298006\n73211009\n' | sct transcode --from snomed --to icd10

# Legacy GP migration from CTV3 -> SNOMED, forwarding any retired targets
cat ctv3_codes.csv | sct transcode --from ctv3 --to snomed --forward-history

# Two-hop: CTV3 -> (SNOMED) -> ICD-10
echo 'X200E' | sct transcode --from ctv3 --to icd10

# Read v2 -> SNOMED, using the ReadCode+TermCode key
echo '0111.00' | sct transcode --from read2 --to snomed

# Compose with ECL: every descendant of Diabetes, as ICD-10
sct ecl expand '<<73211009' | sct transcode --from snomed --to icd10 --json

# Reverse lookup: which SNOMED concepts map to an ICD-10 code?
echo 'I219' | sct transcode --from icd10 --to snomed
```

## How it works

Every mapping pivots through SNOMED CT:

1. **Source → SNOMED** - `snomed` passes through; `ctv3`/`read2` resolve via `crossmaps` rows where the target is SNOMED CT; `icd10`/`opcs4` reverse-resolve via SNOMED CT → classification `crossmaps`. Older databases can still resolve CTV3/Read v2 through `concept_maps`.
2. **(optional) history forwarding** - if `--forward-history` and the pivot concept is inactive, it is forwarded to its `replaced_by` / `same_as` / `possibly_equivalent_to` target(s) from `concept_history`.
3. **SNOMED → target** - `snomed` passes through; all other systems resolve through `crossmaps`, falling back to `concept_maps` for legacy CTV3/Read v2 databases.

See [cross-terminology mapping](https://github.com/pacharanero/sct/blob/main/specs/cross-terminology-mapping.md) for the full design and the DMWB-replacement context.
