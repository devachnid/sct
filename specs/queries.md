# Open Queries

Lightweight tracker for open design questions. Address each and remove it (or fold it into the
relevant spec) once resolved.

*Last reviewed 2026-06-04. Resolved/obsolete items removed: Q2 (`sct codelist diff` vs `sct diff` —
both intentionally exist), Q4 (`sct refset` is now its own command, not a `codelist` alias; only
`valueset` is an alias), Q5 (release date is now stored in the NDJSON provenance header and the
SQLite `metadata` table rather than inferred from the filename), Q6 (locale/layering — see below).*

> **Q6 resolved (2026-06-04, v0.5.2).** Investigated and fixed. **Findings:** (1) The standard
> SNOMED pattern is to consume a publisher-merged *Edition*, not to hand-layer snapshots; UK users
> should use the pre-merged **UK Monolith** from TRUD (NHS resolves base/extension conflicts), which
> `sct` already recognises. (2) Multiple `--rf2` sources layer last-supplied-wins (no Module
> Dependency Reference Set resolution) — adequate for the rare DIY case, but the Monolith is the
> recommended path. (3) **Bug found and fixed:** `--locale` was effectively ignored, because the
> language-reference-set id (the actual dialect selector — GB English `900000000000508004` vs US
> English `900000000000509007`, plus the UK realm refsets) was dropped at parse time and all
> language refsets were merged by description id. `--locale` now maps to an ordered list of language
> refset ids and selects the term Preferred in the highest-priority one (`en-GB` →
> *Appendicectomy*, `en-US` → *Appendectomy*). Documented in
> [`docs/commands/ndjson.md`](../docs/commands/ndjson.md).

---

## Q7 — `sct serve` and `sct codelist publish --to <url>`

**Context:** `sct codelist publish --to <url>` is intended to target "any `sct serve` endpoint
(future)", and [`sct serve`](commands/serve.md) is still future work (now unblocked by the ECL
engine).

**Question:** Until `sct serve` ships, `sct codelist publish` should target only OpenCodelists;
the `--to <url>` form is deferred. Confirm and keep the
[`docs/commands/codelist.md`](../docs/commands/codelist.md) publish section scoped accordingly.
