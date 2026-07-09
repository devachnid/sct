# sct semantic

Semantic similarity search over a SNOMED CT Arrow IPC embeddings file.

Embeds your query text via Ollama and performs cosine similarity against all concept embeddings in the `.arrow` file produced by [`sct embed`](embed.md). Returns the concepts whose meaning is closest to your query text, including some concepts that don't share any keywords with it.

!!! warning "Experimental - read this before trusting a result"
    `sct semantic` runs a small, general-purpose text-embedding model (`nomic-embed-text` by default) - not a language model, and not one trained on medical text. It is genuinely useful as an **adjunct** to keyword search, surfacing paraphrase-level matches that [`sct lexical`](lexical.md) structurally cannot find. It is **not** reliable enough to trust unreviewed, and it has no real clinical or world knowledge: it does well when a query shares *some* vocabulary root with a concept's own text, and it can fail outright on idiomatic phrases that don't ("sugar sickness" for diabetes, "sticky blood" for hypercoagulable state - see [Known limitations](#known-limitations) below for real, verified examples).

    **If you need deterministic, exact results, use [`sct lexical`](lexical.md).** If you need genuine meaning-level bridging - recognising that "sugar sickness" means diabetes - hand the terminology to an LLM instead: over [`sct mcp`](mcp.md) it can combine `snomed_search` and `snomed_semantic_search` judiciously and bring real-world knowledge that this embedding model doesn't have, while still grounding its answer in real SNOMED concepts.

---

## Usage

```
sct semantic <QUERY> [--embeddings <FILE>] [--model <MODEL>] [--ollama-url <URL>] [--limit <N>]
```

## Options

| Flag | Default | Description |
|---|---|---|
| `<QUERY>` | *(required)* | Natural-language search query. |
| `--embeddings <FILE>` | discovered (see [Path resolution](../path-resolution.md)) | Arrow IPC file produced by `sct embed`. |
| `--model <MODEL>` | `nomic-embed-text` | Ollama model - must match the model used when building the embeddings. |
| `--ollama-url <URL>` | `http://localhost:11434` | Ollama base URL. |
| `--limit <N>` | `10` | Maximum number of results. |
| `--ids` | off | Emit only matching SCTIDs (newline-delimited) for piping into other commands. |

---

## Prerequisites

Ollama must be running with the same model that was used to build the embeddings:

```bash
ollama serve
ollama pull nomic-embed-text  # if not already pulled
```

---

## Examples

```bash
# Paraphrase queries that share no SNOMED vocabulary at all - sct lexical
# returns nothing for any of these; sct semantic finds real candidates
sct semantic "can't stop peeing"          # → urinary urgency/incontinence concepts
sct semantic "chest pain climbing stairs" # → chest pain disorder concepts

# Return more results
sct semantic "difficulty breathing" --limit 20

# Pipe matching SCTIDs into a code list (review the file before trusting it -
# see the warning above)
sct semantic "urinary urgency" --ids --limit 30 | sct codelist add urgency.codelist -

# Use embeddings built with a different model
sct semantic "fracture" \
  --embeddings snomed-embeddings-small.arrow \
  --model mxbai-embed-large

# Use embeddings on a remote host
sct semantic "epilepsy" --ollama-url http://192.168.1.100:11434
```

---

## Output

```
$ sct semantic "can't stop peeing"

0.6815 | 87557004 | Urge incontinence of urine
0.6762 | 249296002 | Sudden stoppage of urine flow
0.6757 | 299271000000100 | Urge to pass urine again shortly after finishing voiding
0.6710 | 249289004 | Must urinate repeatedly to empty urinary bladder
0.6562 | 5972002 | Delay when starting to pass urine
```

(Real output, UK Monolith 42.3.0, `nomic-embed-text`. The columns are `{score} | {id} | {preferred_term}`.)

The first column is the **cosine similarity** between the query vector and the concept embedding - a value between 0 and 1, where 1 means identical direction in vector space and 0 means completely unrelated.

**There is no reliable score threshold that separates a good match from noise.** With `nomic-embed-text`, real scores across a wide range of queries cluster in roughly 0.60-0.80 whether the top result is exactly right or completely wrong - a score of 0.66 might be a solid clinical match, or it might be latching onto an unrelated word (see [Known limitations](#known-limitations)). Judge the returned *concept*, not the number. What the number *is* reliable for is relative ranking within one query's results - rank 1 is the model's best guess, and results usually degrade in relevance as you go down the list, even if the score gap between them is small.

---

## How it works

1. Your query text is sent to Ollama, which returns a 768-dimensional float32 vector.
2. The `.arrow` file is scanned; cosine similarity is computed between the query vector and each concept's embedding.
3. The top-N concepts by score are printed.

The query is embedded using the same text template as `sct embed`, so the query vector lives in the same embedding space as the concept vectors. The search is entirely local - no network call beyond the Ollama process running on your machine.

---

## Comparison with `sct lexical`

| | `sct lexical` | `sct semantic` |
|---|---|---|
| Basis | Keyword matching (FTS5) | Meaning / vector similarity |
| Input | SQLite `.db` | Arrow `.arrow` + Ollama |
| Speed | Instant | A few seconds (Ollama round-trip + scanning the full `.arrow` file - observed 2-7 s on an 831k-concept UK Monolith) |
| Finds synonyms | Only if indexed | Yes |
| Finds related concepts without shared words | No | Yes |
| Works offline | Yes | Requires local Ollama |
| Deterministic / reliable | Yes | No - review results before trusting them |

Use `sct lexical` when you know the SNOMED term, or need a dependable, repeatable result. Use `sct semantic` for exploring plain-language descriptions, but check what it returns.

---

## Known limitations

Real, verified failures from testing against the UK Monolith with `nomic-embed-text` - kept here so expectations stay calibrated as the model or embedding scheme change.

**Idiomatic phrases the model has no medical grounding for.** A colloquial phrase that doesn't share a vocabulary root with its clinical concept can fail outright, with no signal in the score that anything went wrong:

```
$ sct semantic "sticky blood" --limit 3
0.6286 | 276403006 | Neonatal cord sticky
0.6222 | 104051000119105 | Exposure to body fluid due to accidental needle stick injury
0.6195 | 77262006 | Heel stick
```

The intended target - hypercoagulable state - never appears. The model has latched onto "sticky" and "stick" as tokens, not the medical idiom.

**Synonym dilution.** A concept whose *preferred term* is technical/Latin, but whose colloquial synonym is one of several in a list, can rank behind concepts that merely repeat the query phrase. `sct semantic "heart attack"` puts *Myocardial infarction* (`22298006`) at rank 11 (score 0.6998) - one place outside the default `--limit 10` - behind several concepts that just happen to contain the word "heart" (`Fear of having a heart attack`, `Contusion to heart`).

**Category drift.** A query can land in the right clinical neighbourhood but the wrong part of SNOMED's hierarchy. `sct semantic "water on the lungs"` top-scores *Measurement of extravascular lung water* (a procedure) rather than the disorder a clinician means by that phrase (pulmonary oedema).

None of this is a bug in `sct` - the scores are exactly what the embedding model produces for the given text, reproducibly. It reflects `nomic-embed-text` being a general-purpose model with no medical fine-tuning. A future improvement path (per-synonym embeddings with max-pooling, or a clinically-trained model) is tracked in [`spec/roadmap.md`](https://github.com/pacharanero/sct/blob/main/spec/roadmap.md); until then, treat every result as a candidate to review, not an answer.

---

## See also

- [`sct lexical`](lexical.md) - keyword search (faster, no Ollama required)
- [`sct embed`](embed.md) - build the embeddings file
- [`sct mcp`](mcp.md) - the same search exposed as `snomed_semantic_search` for AI clients