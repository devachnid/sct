// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later

//! In-process query-latency benchmarks for the FST index.
//!
//! These measure the sub-microsecond-to-millisecond lookups that the shell
//! benchmarks under `bench/` cannot reach (process-spawn overhead dwarfs them).
//!
//! By default the benchmark builds a small synthetic index so it runs anywhere
//! with no external data. To benchmark against a real release, point it at a
//! prebuilt artefact:
//!
//! ```sh
//! SCT_FST_BENCH_INDEX=./snomed.fst cargo bench --bench fst_bench
//! ```
//!
//! The synthetic index is too small for the numbers to mean much on their own;
//! its purpose is to keep the benchmark runnable and to catch latency
//! regressions in CI-less local runs. The real comparison against SQLite FTS5
//! (size + latency) is produced from `snomed.ndjson` / `snomed.db` per
//! `spec/commands/fst.md` §6.

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::path::PathBuf;

use sct_rs::index::{self, Index};

/// A handful of synthetic concepts with shared phrase structure, so prefix and
/// word search have something to chew on.
fn synthetic_ndjson() -> String {
    let bodies = ["femur", "tibia", "humerus", "radius", "ulna", "fibula"];
    let sides = ["left", "right"];
    let mut lines = Vec::new();
    let mut id = 100_000_000u64;
    for body in bodies {
        for side in sides {
            for part in ["shaft of", "neck of", "head of"] {
                id += 1;
                let term = format!("Fracture of {part} {side} {body}");
                lines.push(format!(
                    r#"{{"id":"{id}","fsn":"{term} (disorder)","preferred_term":"{term}","synonyms":["{body} fracture"],"hierarchy":"Clinical finding","hierarchy_path":[],"parents":[],"children_count":0,"active":true,"module":"x","effective_time":"","attributes":{{}},"schema_version":3}}"#
                ));
            }
        }
    }
    lines.join("\n")
}

fn load_index() -> (Index, Option<PathBuf>) {
    if let Ok(path) = std::env::var("SCT_FST_BENCH_INDEX") {
        let path = PathBuf::from(path);
        let idx = Index::open(&path).expect("opening SCT_FST_BENCH_INDEX");
        return (idx, Some(path));
    }
    // Build a synthetic index into a tempfile and open it.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("synthetic.fst");
    {
        let mut out = std::fs::File::create(&path).unwrap();
        index::build(std::io::Cursor::new(synthetic_ndjson()), &mut out).unwrap();
    }
    let idx = Index::open(&path).expect("opening synthetic index");
    // Leak the tempdir so the file outlives the benchmark run.
    std::mem::forget(dir);
    (idx, None)
}

fn bench_queries(c: &mut Criterion) {
    let (idx, real) = load_index();
    // Pick query terms that exist in either the real or synthetic corpus.
    let (exact, prefix, fuzzy, words): (&str, &str, &str, &[&str]) = if real.is_some() {
        (
            "myocardial infarction",
            "myocard",
            "myocaridal infarction",
            &["fracture", "femur"],
        )
    } else {
        (
            "fracture of shaft of left femur",
            "fracture of shaft",
            "fracture of shaft of left femer",
            &["fracture", "femur"],
        )
    };

    c.bench_function("lookup_exact", |b| {
        b.iter(|| black_box(idx.lookup_exact(black_box(exact))))
    });
    c.bench_function("lookup_prefix", |b| {
        b.iter(|| black_box(idx.lookup_prefix(black_box(prefix), 10).unwrap()))
    });
    c.bench_function("lookup_fuzzy_d1", |b| {
        b.iter(|| black_box(idx.lookup_fuzzy(black_box(fuzzy), 1, 10).unwrap()))
    });
    c.bench_function("lookup_fuzzy_d2", |b| {
        b.iter(|| black_box(idx.lookup_fuzzy(black_box(fuzzy), 2, 10).unwrap()))
    });
    c.bench_function("lookup_words", |b| {
        b.iter(|| black_box(idx.lookup_words(black_box(words), 10)))
    });
}

criterion_group!(benches, bench_queries);
criterion_main!(benches);
