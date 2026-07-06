// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later

//! In-process latency benchmarks for the ECL parser (`sct::ecl::parse`).
//!
//! Parsing is the pure-Rust, DB-free half of the ECL stack (the other half,
//! `evaluate`, is SQLite-bound and is profiled at the whole-binary level by
//! `benchmarks/profile.sh` against a real `snomed.db`). These micro-benchmarks
//! isolate lex + parse so a regression in that path shows up here rather than
//! being masked by query I/O.
//!
//! Inputs span the shapes real callers submit: a bare focus concept, the
//! `<<` / `>>` hierarchy operators, set operators (`OR` / `MINUS`), and
//! attribute refinement with cardinality - roughly ascending in AST size.
//!
//! ```sh
//! cargo bench --bench ecl_bench
//! # HTML report + plots: target/criterion/report/index.html
//! ```

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

use sct_rs::ecl::parse;

/// (label, expression) pairs, ascending in parse complexity.
const CASES: &[(&str, &str)] = &[
    ("self", "73211009"),
    ("descendants", "<<73211009"),
    ("disjunction", "<<73211009 OR <<840539006"),
    ("minus", "<<73211009 MINUS <<46635009"),
    ("refinement", "<<404684003 : 363698007 = <<39057004"),
    (
        "nested_group",
        "(<<73211009 OR <<840539006) MINUS <<199223000",
    ),
    (
        "multi_attr",
        "<<404684003 : { 363698007 = <<39057004, 116676008 = <<415582006 }",
    ),
];

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecl_parse");
    for (label, expr) in CASES {
        // Fail fast if a sample stops parsing (keeps the corpus honest).
        parse(expr).unwrap_or_else(|e| panic!("sample {label:?} no longer parses: {e}"));
        group.bench_function(*label, |b| b.iter(|| black_box(parse(black_box(expr)))));
    }
    group.finish();
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
