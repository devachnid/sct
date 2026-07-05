# sct transcode

> **Renamed.** `sct transcode` is now an **alias** of [`sct map`](map.md). The command still works unchanged; the documentation moved to keep one page for cross-terminology mapping.

`sct transcode --from <SYS> --to <SYS>` maps a stream of codes from one terminology to one other, pivoting through SNOMED CT. That is exactly `sct map --from <SYS> --to <SYS>` today - same flags, same behaviour:

```bash
# These are equivalent
cut -f1 gp_extract.tsv | sct transcode --from read2 --to snomed
cut -f1 gp_extract.tsv | sct map       --from read2 --to snomed
```

See [`sct map`](map.md) for the full, current reference (including the new `-f, --format text|tsv|csv|json` option; `--json` is still accepted as a deprecated alias for `--format json`).
