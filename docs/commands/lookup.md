# sct lookup

Look up a single SNOMED CT concept by SCTID, or reverse-resolve a CTV3 (Read v3) code to its concept.

**When to use:** you have an identifier and want the full concept record (preferred term, FSN, hierarchy, parents, attributes, maps). For text search, use [`sct lexical`](lexical.md) or [`sct fst`](fst.md); for set queries, use [`sct codelist add --ecl`](codelist.md).

---

## Usage

```
sct lookup <CODE> [--db <FILE>] [--json]
```

## Options

| Argument / Flag | Default | Description |
|---|---|---|
| `<CODE>` | *(required)* | A numeric SCTID (e.g. `22298006`), or a CTV3 code (e.g. `XE0Uh`) for reverse lookup via the `concept_maps` table. |
| `--db <FILE>` | discovered (see [Path resolution](../path-resolution.md)) | SQLite database produced by `sct sqlite`. |
| `--json` | off | Emit the raw concept record as JSON instead of human-readable text. |
| `--ids` | off | Emit only the resolved SCTID(s), newline-delimited, for piping. With a CTV3 code, prints the mapped SNOMED concept id(s). |
| `--provenance` / `--no-provenance` | auto | Show or hide the release provenance footer (default: on for an interactive terminal). |

---

## Examples

```bash
# By SCTID
sct lookup 22298006

# Raw JSON (for scripting / piping to jq)
sct lookup 22298006 --json | jq '.preferred_term'

# Reverse lookup from a CTV3 code (requires a UK Monolith-derived database)
sct lookup XE0Uh

# Explicit database
sct lookup 73211009 --db /data/snomed.db
```

```bash
# Resolve a CTV3 code to its SNOMED SCTID for piping
sct lookup XE0Uh --ids
```

CTV3 reverse lookup requires a database built from a UK edition that includes the CTV3 simple map refset; on an International-only database those codes won't resolve.
