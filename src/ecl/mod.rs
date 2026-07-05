// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SNOMED CT Expression Constraint Language (ECL).
//!
//! A parser and evaluator for the supported ECL subset (`specs/ecl.md`). ECL is
//! the intermediate representation the query stack converges on: it backs
//! `sct codelist add --ecl`, the future `sct serve` `$expand`, and is the
//! compile target for SCT-QL.
//!
//! - [`parse`] - ECL text → [`ast::Expr`]
//! - [`evaluate`] - [`ast::Expr`] × SQLite → set of matching SCTIDs
//! - [`expand`] - convenience: ECL text × SQLite → sorted `Vec` of SCTIDs

pub mod ast;
pub mod compress;
pub mod eval;
pub mod lex;
pub mod parse;

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

pub use ast::Expr;
pub use eval::IdSet;
pub use parse::parse;

/// Parse and evaluate an ECL expression against the database, returning the
/// matching concept SCTIDs (ascending, deduplicated).
pub fn expand(conn: &Connection, ecl: &str) -> Result<Vec<String>> {
    let expr = parse(ecl).with_context(|| format!("parsing ECL {ecl:?}"))?;
    let set = eval::evaluate(conn, &expr).context("evaluating ECL")?;
    let mut ids: Vec<String> = set.into_iter().collect();
    // Numeric SCTID order rather than lexical.
    ids.sort_by(|a, b| match (a.parse::<u128>(), b.parse::<u128>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        _ => a.cmp(b),
    });
    Ok(ids)
}

/// Open a SNOMED CT SQLite database read-only and [`expand`] an ECL expression
/// against it. Convenience for callers that have a path rather than a live
/// connection (e.g. integration tests).
pub fn expand_path(db: &Path, ecl: &str) -> Result<Vec<String>> {
    let conn = crate::commands::open_db_readonly(db, None)
        .with_context(|| format!("opening {}", db.display()))?;
    expand(&conn, ecl)
}

/// Print a one-line stderr hint when the database lacks the transitive-closure
/// table (`concept_ancestors`). Without it, large `<<` / `>>` ECL evaluation
/// falls back to recursive CTEs and is much slower. Call from command entry
/// points that run ECL (`sct ecl`, `sct serve`, `sct codelist add --ecl`).
pub fn warn_if_no_tct(conn: &Connection) {
    let has_tct = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='concept_ancestors'",
            [],
            |_| Ok(()),
        )
        .is_ok();
    if !has_tct {
        eprintln!(
            "note: this database has no transitive-closure table, so large `<<` / `>>` ECL \
             queries fall back to slower recursive CTEs.\n  Build it once for a big speed-up: \
             `sct sqlite --transitive-closure` (or `sct tct --db <db>`)."
        );
    }
}
