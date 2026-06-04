# Open Queries

Lightweight tracker for open design questions. Address each and remove it (or fold it into the
relevant spec) once resolved.

*Last reviewed 2026-06-04. Resolved/obsolete items removed: Q2 (`sct codelist diff` vs `sct diff` —
both intentionally exist), Q4 (`sct refset` is now its own command, not a `codelist` alias; only
`valueset` is an alias), Q5 (release date is now stored in the NDJSON provenance header and the
SQLite `metadata` table rather than inferred from the filename).*

---

## Q6 — `sct ndjson` locale flag and multi-edition layering

**Context:** The `--locale` flag defaults to `en-GB`. When layering multiple `--rf2` sources
(e.g. International + UK Clinical), it is not specified how conflicts in preferred terms between
the base and extension language reference sets are resolved.

**Question:** Clarify and document the priority rule — does the last `--rf2` source win? Does the
locale filter apply across all supplied RF2 directories? Update
[`docs/commands/ndjson.md`](../docs/commands/ndjson.md) once confirmed.

---

## Q7 — `sct serve` and `sct codelist publish --to <url>`

**Context:** `sct codelist publish --to <url>` is intended to target "any `sct serve` endpoint
(future)", and [`sct serve`](commands/serve.md) is still future work (now unblocked by the ECL
engine).

**Question:** Until `sct serve` ships, `sct codelist publish` should target only OpenCodelists;
the `--to <url>` form is deferred. Confirm and keep the
[`docs/commands/codelist.md`](../docs/commands/codelist.md) publish section scoped accordingly.
