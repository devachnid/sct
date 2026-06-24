# DMWB Walkthrough

Use `sct` as a local, scriptable replacement for the terminology parts of the
NHS Data Migration Workbench (DMWB): browsing mapped terms, transcoding batches
of codes, forwarding inactive SNOMED CT concepts, exporting mapped code lists,
and serving FHIR `ConceptMap/$translate`.

!!! note "Scope"
    This walkthrough covers DMWB's terminology migration workflow: `BROWSE`,
    `QUERIES`, `TRANSCODE`, cross-terminology codelists, and the Excel add-in
    path via FHIR. It does **not** cover DMWB's EPR patient-data loading,
    miscode repair, casemix analytics, or Access GUI.

!!! warning "Read v2 status"
    `sct` supports `read2` wherever the SQLite database contains Read v2 rows in
    `concept_maps`, but current UK RF2 releases do not contain the DMWB-unique
    Read v2 maps. The `sct dmwb` Access-file reader is currently introspection
    only because `jetdb` cannot decode DMWB's Binary `SCUI` Read v2 column. For
    now, the complete shipped path is CTV3, SNOMED CT, ICD-10, OPCS-4, and
    SNOMED CT history forwarding.

---

## Build a DMWB-ready database

Use the UK Monolith from NHS TRUD and ask `sct` to load all RF2 map and history
refsets:

```bash
sct trud download --edition uk_monolith \
                  --pipeline \
                  --include-inactive \
                  --refsets all
```

That command downloads the latest UK Monolith release, verifies it, builds the
NDJSON artefact, and builds the SQLite database under `~/.local/share/sct/data/`.

The two important flags are:

| Flag | Why it matters |
|---|---|
| `--refsets all` | Loads ExtendedMap refsets for SNOMED CT -> ICD-10 / OPCS-4 and Association refsets for concept history. |
| `--include-inactive` | Keeps inactive concepts in the database so old records can still be looked up and forwarded. |

If you already have the RF2 zip locally, run the same build by hand:

```bash
sct ndjson --rf2 uk_sct2mo_42.2.0_20260603000001Z.zip \
           --refsets all \
           --include-inactive \
           --output snomed.ndjson

sct sqlite --input snomed.ndjson --output snomed.db
```

Check that the database has the DMWB-replacement tables:

```bash
sqlite3 snomed.db "select count(*) from crossmaps"
sqlite3 snomed.db "select count(*) from concept_history"
sqlite3 snomed.db "select count(*) from concepts where active = 0"
```

---

## Browse one code across terminologies

DMWB's `BROWSE` screen shows equivalent codes around a SNOMED CT pivot. In `sct`,
use [`sct crosswalk`](../commands/crosswalk.md):

```bash
sct crosswalk 22298006 --db snomed.db
```

Typical output:

```text
22298006  Myocardial infarction
  read2:  (none)
  ctv3:   X200E
  icd10:  I219
  opcs4:  (none)
```

Start from another system when that is the code you have:

```bash
sct crosswalk X200E --from ctv3 --db snomed.db
sct crosswalk I219 --from icd10 --db snomed.db
```

Use JSON when another process needs the result:

```bash
sct crosswalk 22298006 --json --db snomed.db
```

---

## Transcode a batch of codes

DMWB's `TRANSCODE` workflow becomes [`sct transcode`](../commands/transcode.md):
one input code per line, one output row per mapping.

```bash
cat ctv3-codes.txt | sct transcode --from ctv3 --to snomed --db snomed.db
```

Crosswalk via SNOMED CT into ICD-10:

```bash
cat ctv3-codes.txt | sct transcode --from ctv3 --to icd10 --db snomed.db
```

Map SNOMED CT concepts to ICD-10:

```bash
printf '22298006\n73211009\n' \
  | sct transcode --from snomed --to icd10 --db snomed.db
```

Emit JSON lines for a downstream script:

```bash
cat ctv3-codes.txt \
  | sct transcode --from ctv3 --to icd10 --json --db snomed.db
```

---

## Forward inactive SNOMED CT concepts

Old clinical records often contain concepts that are inactive in the current
release. Build with `--refsets all --include-inactive`, then add
`--forward-history`:

```bash
cat old-sctids.txt \
  | sct transcode --from snomed --to snomed --forward-history --db snomed.db
```

The forwarding data comes from the RF2 Association refsets, so it is sourced from
the current SNOMED CT release rather than from DMWB's Access tables.

---

## Export a mapped code list

For a DMWB-style cluster translation, keep the canonical list in SNOMED CT and
append target terminology columns at export time:

```bash
sct codelist export codelists/mi.codelist \
  --format csv \
  --include-maps ctv3,icd10,opcs4 \
  --db snomed.db \
  --output mi-crosswalk.csv
```

This leaves the reviewed codelist stable and repeatable, while the exported CSV
can carry the downstream migration or reporting codes needed for a particular
job.

---

## Serve the Excel add-in path

DMWB's Excel add-in can target a FHIR terminology server. `sct serve` provides
FHIR R4 `ConceptMap/$translate` over the same local SQLite database:

```bash
sct serve --db snomed.db --host 127.0.0.1 --port 8080
```

Then call `$translate` directly:

```bash
curl 'http://localhost:8080/ConceptMap/$translate?system=http://snomed.info/sct&code=22298006&targetsystem=http://hl7.org/fhir/sid/icd-10'
```

Bare system names are accepted too:

```bash
curl 'http://localhost:8080/ConceptMap/$translate?system=ctv3&code=X200E&targetsystem=icd10'
```

See [`sct serve`](../commands/serve.md) for the full FHIR surface.

---

## Inspect a DMWB Access file

The optional [`sct dmwb`](../commands/dmwb.md) command can inspect DMWB `.mdb`
files without Microsoft Access or `mdbtools`:

```bash
sct dmwb tables "DMWB NHS Data Migration Maps.mdb"
sct dmwb dump "DMWB NHS Data Migration Maps.mdb" SCTICDMAP --limit 5
```

This is useful for validation and reverse engineering, but it is not the
recommended production ingestion path today. Prefer the RF2-native maps loaded
by `--refsets all`.

---

## Status matrix

| DMWB need | `sct` route | Status |
|---|---|---|
| Browse equivalent codes around a concept | `sct crosswalk` | Shipped |
| Batch terminology migration | `sct transcode` | Shipped |
| SNOMED CT -> ICD-10 / OPCS-4 maps | `sct ndjson --refsets all` + `sct sqlite` | Shipped |
| CTV3 <-> SNOMED CT maps | UK RF2 SimpleMap -> `concept_maps` | Shipped |
| Inactive concept forwarding | RF2 Association refsets + `--forward-history` | Shipped |
| Cross-terminology codelist exports | `sct codelist export --include-maps` | Shipped |
| FHIR translation for Excel / integration | `sct serve` `ConceptMap/$translate` | Shipped |
| DMWB Read v2 map import | planned `sct dmwb import` / TRUD item 9 path | Blocked |
| DMWB EPR DATA / casemix analytics | Out of scope | Not planned |

For the design history and remaining Read v2 options, see the
[cross-terminology mapping spec](https://github.com/pacharanero/sct/blob/main/specs/cross-terminology-mapping.md).
