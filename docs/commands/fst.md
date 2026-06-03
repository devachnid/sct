# sct fst

Build and query an **FST-backed lexical index** — a single, mmap-able `snomed.fst` file offering exact, prefix, fuzzy, and word-intersection search over SNOMED CT terms.

!!! warning "Experimental"
    `sct fst` is an additive, experimental feature. It does not replace or change any existing command — [`sct lexical`](lexical.md) (SQLite FTS5) remains the default keyword-search path. `sct fst` exists to evaluate a finite-state-transducer index as a lighter-weight, typo-tolerant alternative. See [`specs/fst.md`](https://github.com/pacharanero/sct/blob/main/specs/fst.md) for the design and benchmark results.

**When to use:** you want sub-millisecond prefix/autocomplete or **fuzzy (typo-tolerant)** matching that FTS5 can't do, or a lexical index you can mmap without opening the full database. For ranked BM25 keyword search, [`sct lexical`](lexical.md) is still the tool.

---

## Usage

```
sct fst build  --input <NDJSON> [--output <FST>]
sct fst search <QUERY> [--index <FST>] [--prefix | --fuzzy <N> | --words] [--limit <N>]
```

`build` consumes the canonical NDJSON produced by [`sct ndjson`](ndjson.md) — the same input as [`sct sqlite`](sqlite.md) and [`sct parquet`](parquet.md) — and inherits its active-only filtering and edition merge. The index is static: rebuild it once per SNOMED release.

---

## `sct fst build`

| Flag | Default | Description |
|---|---|---|
| `--input <FILE>` | *(required)* | NDJSON file produced by `sct ndjson`. Use `-` for stdin. |
| `--output <FILE>` | `snomed.fst` | Output index file. |

```bash
sct fst build --input snomed.ndjson --output snomed.fst
```

Build prints a short summary to stderr:

```
Built snomed.fst in 16.30s
  831132 concepts, 1949665 terms → 1252590 distinct keys, 177261 word tokens, 59 semantic tags
  160.4 MB on disk (168242528 bytes)
```

---

## `sct fst search`

| Argument / Flag | Default | Description |
|---|---|---|
| `<QUERY>` | *(required)* | The term or words to search for. |
| `--index <FILE>` | `snomed.fst` | Index file produced by `sct fst build`. |
| `--prefix` | off | Prefix (autocomplete) search. |
| `--fuzzy <N>` | off | Fuzzy search up to `N` edits (Levenshtein distance 1 or 2). |
| `--words` | off | Word-intersection: whitespace-split the query; return concepts whose terms contain **every** word. |
| `--limit <N>`, `-l` | `10` | Maximum number of results. |

`--prefix`, `--fuzzy`, and `--words` are mutually exclusive; with none of them the search is an exact (normalised) match.

```bash
# Exact term (case-insensitive)
sct fst search "myocardial infarction"

# Prefix / autocomplete
sct fst search myocard --prefix --limit 6

# Fuzzy — tolerates a single typo
sct fst search "diabetes mellitis" --fuzzy 1

# Word intersection — concepts whose terms contain both words
sct fst search "fracture femur" --words
```

---

## What gets indexed

For every concept in the NDJSON, the FSN, preferred term, and all synonyms become search keys. Keys are **normalised** for lookup, while the original-case preferred term is kept for display.

Normalisation (fixed, and stable across releases):

1. NFC Unicode normalisation
2. Unicode lowercase
3. Strip the trailing semantic tag from FSNs (e.g. `(disorder)`) — the tag is stored alongside the key, not in it
4. Collapse internal whitespace, trim

Normalisation is deliberately **lossless** with respect to accents and punctuation: `Ménière's disease` is indexed as `ménière's disease`, and the de-accented spelling will **not** match. This keeps clinically distinct terms distinct, at the cost of a larger index.

---

## Index file

`snomed.fst` is a single, self-contained, mmap-able file (no sidecar directory). It bundles two finite-state transducers (a term index and a word index), their posting lists, a display side-table, the semantic-tag table, and the release [provenance](../path-resolution.md). Opening it is a single constant-time mmap — the first query is the only one that touches disk pages.

---

## Comparison with `sct lexical`

| | `sct fst` | [`sct lexical`](lexical.md) |
|---|---|---|
| Backend | FST (mmap'd `snomed.fst`) | SQLite FTS5 (`snomed.db`) |
| Exact / prefix / word search | Yes | Yes |
| **Fuzzy (typo-tolerant)** | **Yes** (Levenshtein) | No |
| Ranked BM25 relevance | No | Yes |
| Query latency | ~1 µs–3.4 ms | ~0.5–1.2 ms (warm) |
| Start-up | single mmap | open SQLite DB |
| Status | experimental | stable, the default |

On a UK Monolith-scale edition (~831k concepts) the FST's lexical search structures are roughly the same size as the FTS5 inverted index (~104 MB vs ~103 MB), but query latency is one to two orders of magnitude lower and it adds fuzzy and prefix matching. The headline trade-off is **speed and typo-tolerance, not raw size**. Full numbers are in [`specs/fst.md` §10](https://github.com/pacharanero/sct/blob/main/specs/fst.md).

---

## Notes and current limitations

- **Fuzzy distance is measured over the whole key.** Edits accumulate across a phrase, so a two-typo query over a long FSN can exceed distance 2 and miss. Fuzzy is most effective on shorter terms / single words.
- **No ranking yet.** Results are ordered by a crude exact > prefix > fuzzy score, not BM25. Use [`sct lexical`](lexical.md) when relevance ordering matters.
- **Posting lists are uncompressed** in this first cut; a delta-varint encoding is the obvious next size optimisation.
- The index is licensed SNOMED CT content — like every other artefact, `*.fst` is git-ignored and never distributed here.
