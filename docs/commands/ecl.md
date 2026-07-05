# sct ecl

Evaluate a SNOMED CT [Expression Constraint Language](https://confluence.ihtsdotools.org/display/DOCECL) (ECL) expression against the database and emit the matching concept SCTIDs.

**When to use:** you want the *set* of concepts a query selects - to pipe into another command, build a code list, feed a script, or paste into a SQL `IN (…)`. ECL `<<73211009` means "Diabetes mellitus and all its subtypes".

`sct ecl` is the reusable engine behind [`sct codelist add --ecl`](codelist.md). Because it writes plain SCTIDs to stdout, it composes with everything else (see the [composability principle](https://github.com/pacharanero/sct/blob/main/specs/spec.md)).

---

## Usage

```
sct ecl expand <EXPRESSION> [--db <FILE>] [-f text|json|yaml]
```

## Options

| Argument / Flag | Default | Description |
|---|---|---|
| `<EXPRESSION>` | *(required)* | The ECL expression, e.g. `"<<73211009"`. Pass `-` to read the expression from stdin. |
| `--db <FILE>` | discovered (see [Path resolution](../path-resolution.md)) | SQLite database produced by `sct sqlite`. |
| `-f, --format <FMT>` | `text` | Output format: `text` (newline-delimited ids), `json` (array), or `yaml`. (`--json` is a deprecated alias for `--format json`.) |

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
sct ecl expand "^447562003" -f json | jq length

# Read the expression itself from stdin
echo "<<404684003 : 363698007 = <<39057004" | sct ecl expand -
```

---

## `sct ecl compress` - set → ECL

The inverse of `expand`: take an explicit set of SCTIDs (a hand-curated code list) and refactor it into a compact ECL expression that re-expands to *exactly* that set. Turns a brittle flat list into a self-documenting, release-stable intensional definition.

```
sct ecl compress [<IDS>...] [--codelist <FILE>] [--intensional-only]
                 [--max-exclusions <N>] [--pretty] [--stats] [--db <FILE>]
```

| Argument / Flag | Default | Description |
|---|---|---|
| `<IDS>...` | *(stdin)* | SCTIDs to compress. Pass `-` or no ids to read newline/whitespace-delimited ids from stdin. |
| `--codelist <FILE>` | - | Compress the effective members of a `.codelist` instead of ids. |
| `--intensional-only` | off | Emit only subsumption/exclusion clauses; do **not** add literal `OR`/`MINUS` residuals. Exits non-zero if the result is not exact. |
| `--max-exclusions <N>` | `32` | Cap the number of `MINUS <<x` clauses before the remainder falls to literal residuals. |
| `--pretty` | off | Break the expression across indented lines (text output only). |
| `--stats` | off | Print clause counts and intensional coverage to stderr. |
| `-f, --format <FMT>` | `text` | `text` prints the ECL expression; `json`/`yaml` emit a structured object (expression plus include/exclude/residual breakdown). |
| `--db <FILE>` | discovered | SQLite database. |

By default the result is **exact**: if the subsumption heuristic can't express the set cleanly, literal `OR id` / `MINUS id` residuals are appended so the expression provably reproduces the input (verified by re-expansion). A set with no hierarchical structure degrades gracefully to a list of `OR`ed ids - never to a wrong answer.

```bash
# A whole subtree collapses to one clause
sct ecl expand "<<73211009" | sct ecl compress -            # => <<73211009

# Subtree-minus-subtree
printf '73211009\n44054006\n' | sct ecl compress - --stats  # => <<73211009 MINUS <<46635009

# Round-trip: compress then expand returns the original set
sct ecl compress --codelist diabetes.codelist | sct ecl expand -
```

This is a greedy heuristic, not a proof of minimum size; `--stats` reports how much was expressed intensionally. See [`specs/commands/ecl-compress.md`](https://github.com/pacharanero/sct/blob/main/specs/commands/ecl-compress.md).

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
