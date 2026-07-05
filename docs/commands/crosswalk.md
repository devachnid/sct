# sct crosswalk

> **Renamed.** `sct crosswalk` is now an **alias** of [`sct map`](map.md). The command still works unchanged; the documentation moved to keep one page for cross-terminology mapping.

`sct crosswalk <CODE>` shows **all** cross-terminology equivalents of a single code at once. That is exactly `sct map <CODE>` today (with no `--to`):

```bash
# These are equivalent
sct crosswalk 22298006
sct map       22298006
```

See [`sct map`](map.md) for the full, current reference (including the new `-f, --format text|tsv|csv|json` option; `--json` is still accepted as a deprecated alias for `--format json`).
