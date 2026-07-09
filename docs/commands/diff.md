# sct diff `experimental!` :lucide-test-tube

Compare two SNOMED CT NDJSON artefacts and report what changed between releases.

**When to use:** you have two releases of the NDJSON artefact (e.g. March 2025 and March 2026) and want to know what concepts were added, retired, renamed, or moved.

---

## Usage

```
sct diff --old <NDJSON> --new <NDJSON> [-f text|ndjson|json|yaml] [--output <FILE>]
```

## Options

| Flag | Default | Description |
|---|---|---|
| `--old <FILE>` | *(required)* | The older NDJSON artefact (baseline). |
| `--new <FILE>` | *(required)* | The newer NDJSON artefact (comparison target). |
| `-f, --format <FMT>` | `text` | Output format: `text` (human-readable summary; alias `summary`), `ndjson` (one diff record per line, streaming), `json` (array of all records), or `yaml` (alias `yml`). |
| `-o, --output <FILE>` | stdout | Output file for the structured formats (`ndjson`/`json`/`yaml`). |

---

## Examples

```bash
# Human-readable summary to stdout
sct diff \
  --old snomed-uk-20250101.ndjson \
  --new snomed-uk-20260311.ndjson

# Machine-readable NDJSON diff for scripting
sct diff \
  --old snomed-uk-20250101.ndjson \
  --new snomed-uk-20260311.ndjson \
  --format ndjson \
  --output changes-2025-to-2026.ndjson

# Filter to just term changes
sct diff --old old.ndjson --new new.ndjson --format ndjson | \
  jq 'select(.change == "preferred_term_changed")'
```

---

## Change categories

| Category | Description |
|---|---|
| `added` | Concept present in `--new`, absent from `--old` |
| `inactivated` | Concept active in `--old`, absent or inactive in `--new` |
| `preferred_term_changed` | Preferred term changed between releases |
| `hierarchy_changed` | Concept moved to a different top-level hierarchy |

---

## NDJSON diff record format

One JSON object per changed concept:

```json
{"change": "added",                  "id": "...", "preferred_term": "...", "hierarchy": "..."}
{"change": "inactivated",            "id": "...", "preferred_term": "...", "hierarchy": "..."}
{"change": "preferred_term_changed", "id": "...", "old_preferred_term": "...", "new_preferred_term": "...", "hierarchy": "..."}
{"change": "hierarchy_changed",      "id": "...", "preferred_term": "...", "old_hierarchy": "...", "new_hierarchy": "..."}
```

---

## Typical workflow

```bash
# Build the new release
sct ndjson --rf2 SnomedCT_MonolithRF2_PRODUCTION_20260311T120000Z.zip \
           --output snomed-uk-20260311.ndjson

# Check what changed vs last release
sct diff \
  --old snomed-uk-20250901.ndjson \
  --new snomed-uk-20260311.ndjson

# Update codelists that reference changed concepts
sct diff --old snomed-uk-20250901.ndjson --new snomed-uk-20260311.ndjson \
  --format ndjson | jq 'select(.change == "inactivated") | .id' | \
  xargs -I{} sct codelist validate my-codelist.codelist --db snomed.db
```

---

## See also

- [`sct codelist diff`](codelist.md) - compare two `.codelist` files (different operation)
- [`sct info`](info.md) - inspect a single NDJSON artefact
