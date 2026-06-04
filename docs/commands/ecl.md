# sct ecl

Evaluate a SNOMED CT [Expression Constraint Language](https://confluence.ihtsdotools.org/display/DOCECL) (ECL) expression against the database and emit the matching concept SCTIDs.

**When to use:** you want the *set* of concepts a query selects - to pipe into another command, build a code list, feed a script, or paste into a SQL `IN (…)`. ECL `<<73211009` means "Diabetes mellitus and all its subtypes".

`sct ecl` is the reusable engine behind [`sct codelist add --ecl`](codelist.md). Because it writes plain SCTIDs to stdout, it composes with everything else (see the [composability principle](../../specs/spec.md)).

---

## Usage

```
sct ecl expand <EXPRESSION> [--db <FILE>] [--json]
```

## Options

| Argument / Flag | Default | Description |
|---|---|---|
| `<EXPRESSION>` | *(required)* | The ECL expression, e.g. `"<<73211009"`. Pass `-` to read the expression from stdin. |
| `--db <FILE>` | discovered (see [Path resolution](../path-resolution.md)) | SQLite database produced by `sct sqlite`. |
| `--json` | off | Emit a JSON array of SCTID strings instead of newline-delimited ids. |

stdout is the result set (one SCTID per line, or a JSON array). The human-readable match count is written to **stderr**, so it never pollutes a pipe.

---

## Examples

```bash
# Just the ids
sct ecl expand "<<73211009"

# Pipe into a code list (see `sct codelist add … -`)
sct ecl expand "<<73211009" | sct codelist add diabetes.codelist -

# Pipe into a lookup, or jq, or a file
sct ecl expand "<<73211009 MINUS <<46635009" > type2.txt
sct ecl expand "^447562003" --json | jq length

# Read the expression itself from stdin
echo "<<404684003 : 363698007 = <<39057004" | sct ecl expand -
```

---

## Supported ECL

| Construct | Example | Meaning |
|---|---|---|
| Self | `73211009` | the concept itself |
| Wildcard | `*` | any concept |
| Descendants | `<73211009` / `<<73211009` | descendants / descendants-or-self |
| Ancestors | `>73211009` / `>>73211009` | ancestors / ancestors-or-self |
| Children / parents | `<!73211009` / `>!73211009` | direct children / parents |
| Refset member | `^447562003` | members of the reference set |
| Boolean | `A AND B`, `A OR B`, `A MINUS B` | intersection / union / difference |
| Refinement | `<<404684003 : 363698007 = <<39057004` | attribute constraint (comma-conjoined, `{ }` groups, `!=`) |

Optional `|term|` annotations are accepted and ignored. **Attribute refinement** (the `:` operator) needs a database built with schema v4+ (which adds the `concept_relationships` table); hierarchy and refset queries work on any database. Not yet supported (clear error, never silent mis-evaluation): cardinality `[min..max]`, reverse `R` and dotted `.` attributes. See [`specs/ecl.md`](https://github.com/pacharanero/sct/blob/main/specs/ecl.md).
