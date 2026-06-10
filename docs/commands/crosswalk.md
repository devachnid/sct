# sct crosswalk

Show **all** cross-terminology equivalents of a single code at once - the text equivalent of the NHS Data Migration Workbench's tri-terminology BROWSE view. Where [`sct transcode`](transcode.md) maps a stream of codes from one terminology to one other, `sct crosswalk` takes one code and shows its equivalents in every terminology.

Built on the same maps as `sct transcode` (see [cross-terminology mapping](https://github.com/pacharanero/sct/blob/main/specs/cross-terminology-mapping.md)). ICD-10 / OPCS-4 columns need a database built with [`sct ndjson --refsets all`](ndjson.md); without it they show `(none)` (CTV3 / Read v2 still work).

## Usage

```bash
sct crosswalk <CODE> [--from <SYSTEM>] [--json] [--db <FILE>]
```

| Option | Default | Description |
|---|---|---|
| `<CODE>` | | The code to crosswalk. |
| `--from <SYSTEM>` | `snomed` | Terminology of `<CODE>`: `snomed`, `read2`, `ctv3`, `icd10`, `opcs4`. |
| `--json` | off | Emit JSON instead of human-readable text. |
| `--db <FILE>` | auto | SNOMED CT database. |

## Examples

```bash
# A SNOMED concept and all its equivalents
sct crosswalk 22298006
# 22298006  Myocardial infarction
#   read2:  G30..
#   ctv3:   X200E
#   icd10:  I219
#   opcs4:  (none)

# Start from a legacy code - resolves to SNOMED, then shows the rest
sct crosswalk X200E --from ctv3

# Machine-readable
sct crosswalk 22298006 --json
```
