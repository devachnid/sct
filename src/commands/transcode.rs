// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `sct transcode` - map a stream of codes from one terminology to another,
//! pivoting through SNOMED CT. The CLI equivalent of the NHS Data Migration
//! Workbench `TRANSCODE` console function. Composable: reads codes from stdin
//! (or `--input`), writes TSV (or `--json`) to stdout, diagnostics to stderr.
//!
//! Supported systems: `snomed`, `read2`, `ctv3`, `icd10`, `opcs4`. The maps come
//! from `sct sqlite` built with `--refsets all` (ICD-10/OPCS-4 via `crossmaps`,
//! CTV3/Read v2 via `concept_maps`) - see `specs/cross-terminology-mapping.md`.

use anyhow::{bail, Context, Result};
use clap::Parser;
use rusqlite::{params, Connection, OptionalExtension};
use std::io::{BufRead, Write};
use std::path::PathBuf;

const SYSTEMS: [&str; 5] = ["snomed", "read2", "ctv3", "icd10", "opcs4"];

#[derive(Parser, Debug)]
pub struct Args {
    /// Source terminology of the input codes: snomed | read2 | ctv3 | icd10 | opcs4.
    #[arg(long)]
    pub from: String,

    /// Target terminology to map to: snomed | read2 | ctv3 | icd10 | opcs4.
    #[arg(long)]
    pub to: String,

    /// Read codes from this file (leading token per line) instead of stdin.
    #[arg(long)]
    pub input: Option<PathBuf>,

    /// Forward inactive SNOMED pivots to their replacement(s) via concept_history
    /// (needs a database built with `--refsets all`).
    #[arg(long)]
    pub forward_history: bool,

    /// Emit JSON lines instead of TSV.
    #[arg(long)]
    pub json: bool,

    /// SNOMED CT SQLite database. Discovered via the usual path-resolution chain
    /// when omitted (see `docs/path-resolution.md`).
    #[arg(long)]
    pub db: Option<PathBuf>,
}

pub fn run(args: Args) -> Result<()> {
    let from = args.from.to_lowercase();
    let to = args.to.to_lowercase();
    for s in [&from, &to] {
        if !SYSTEMS.contains(&s.as_str()) {
            bail!(
                "unknown terminology {s:?}; expected one of {}",
                SYSTEMS.join(", ")
            );
        }
    }

    let db = crate::paths::resolve_db(args.db.as_deref())?.path;
    let conn = crate::commands::open_db_readonly(&db, None)
        .with_context(|| format!("opening database {}", db.display()))?;

    // The crossmaps / concept_history tables only exist in databases built with
    // `--refsets all`. Fail fast with a clear message rather than a SQL error.
    let needs_crossmaps =
        ["icd10", "opcs4"].contains(&from.as_str()) || ["icd10", "opcs4"].contains(&to.as_str());
    if needs_crossmaps && !table_exists(&conn, "crossmaps") {
        bail!(
            "this database has no ICD-10/OPCS-4 maps. Rebuild with \
             `sct ndjson --rf2 <release> --refsets all` then `sct sqlite`."
        );
    }
    if args.forward_history && !table_exists(&conn, "concept_history") {
        bail!(
            "--forward-history needs concept history, absent from this database. \
             Rebuild with `sct ndjson --refsets all` then `sct sqlite`."
        );
    }

    let codes = read_codes(args.input.as_deref())?;
    let mut out = std::io::stdout().lock();
    let mut matched = 0usize;
    let mut unmatched = 0usize;

    for code in &codes {
        let rows = transcode_one(&conn, &from, code, &to, args.forward_history)?;
        if rows.is_empty() {
            unmatched += 1;
            continue;
        }
        matched += 1;
        for r in rows {
            if args.json {
                writeln!(
                    out,
                    "{}",
                    serde_json::json!({
                        "input": code, "target": r.target,
                        "snomed": r.snomed, "display": r.display,
                    })
                )?;
            } else {
                writeln!(
                    out,
                    "{}\t{}\t{}\t{}",
                    code,
                    r.target,
                    r.snomed,
                    r.display.unwrap_or_default()
                )?;
            }
        }
    }

    eprintln!(
        "transcode {from} -> {to}: {} input code(s), {matched} mapped, {unmatched} unmapped",
        codes.len()
    );
    Ok(())
}

/// One mapped output: the target code, the SNOMED pivot concept it went through,
/// and that concept's preferred term (when known).
pub struct Mapped {
    pub target: String,
    pub snomed: String,
    pub display: Option<String>,
}

/// Map a single `code` from terminology `from` to terminology `to`, pivoting
/// through SNOMED CT. The pure core of `sct transcode` (no I/O), exposed for
/// tests and library reuse.
pub fn transcode_one(
    conn: &Connection,
    from: &str,
    code: &str,
    to: &str,
    forward_history: bool,
) -> Result<Vec<Mapped>> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for pivot in to_snomed(conn, from, code)? {
        let forwarded = if forward_history {
            forward(conn, &pivot)?
        } else {
            vec![pivot]
        };
        for snomed in forwarded {
            let display = pt(conn, &snomed);
            for target in from_snomed(conn, &snomed, to)? {
                if seen.insert((snomed.clone(), target.clone())) {
                    out.push(Mapped {
                        target,
                        snomed: snomed.clone(),
                        display: display.clone(),
                    });
                }
            }
        }
    }
    Ok(out)
}

/// Resolve a code in `from` to its SNOMED concept id(s).
fn to_snomed(conn: &Connection, from: &str, code: &str) -> Result<Vec<String>> {
    match from {
        "snomed" => Ok(vec![code.to_string()]),
        "ctv3" | "read2" => collect(
            conn,
            "SELECT concept_id FROM concept_maps WHERE code = ?1 AND terminology = ?2",
            params![code, from],
        ),
        "icd10" | "opcs4" if table_exists(conn, "crossmaps") => collect(
            conn,
            "SELECT DISTINCT source_code FROM crossmaps WHERE target_system = ?1 AND target_code = ?2",
            params![from, code],
        ),
        "icd10" | "opcs4" => Ok(vec![]), // no crossmaps table -> no maps
        _ => bail!("unknown source terminology {from:?}"),
    }
}

/// Map a SNOMED concept id to its code(s) in the `to` terminology.
fn from_snomed(conn: &Connection, concept: &str, to: &str) -> Result<Vec<String>> {
    match to {
        "snomed" => Ok(vec![concept.to_string()]),
        "ctv3" | "read2" => collect(
            conn,
            "SELECT code FROM concept_maps WHERE concept_id = ?1 AND terminology = ?2",
            params![concept, to],
        ),
        "icd10" | "opcs4" if table_exists(conn, "crossmaps") => collect(
            conn,
            "SELECT DISTINCT target_code FROM crossmaps WHERE source_code = ?1 AND target_system = ?2",
            params![concept, to],
        ),
        "icd10" | "opcs4" => Ok(vec![]), // no crossmaps table -> no maps
        _ => bail!("unknown target terminology {to:?}"),
    }
}

/// Forward an inactive concept to its replacement(s). Active concepts (and those
/// with no recorded forwarding) pass through unchanged.
fn forward(conn: &Connection, concept: &str) -> Result<Vec<String>> {
    let active: Option<bool> = conn
        .query_row(
            "SELECT active FROM concepts WHERE id = ?1",
            [concept],
            |r| r.get::<_, i64>(0).map(|a| a != 0),
        )
        .optional()?;
    if active == Some(true) {
        return Ok(vec![concept.to_string()]);
    }
    let targets = collect(
        conn,
        "SELECT target_id FROM concept_history
         WHERE source_id = ?1 AND association IN ('replaced_by','same_as','possibly_equivalent_to')",
        params![concept],
    )?;
    if targets.is_empty() {
        Ok(vec![concept.to_string()])
    } else {
        Ok(targets)
    }
}

fn pt(conn: &Connection, id: &str) -> Option<String> {
    conn.query_row(
        "SELECT preferred_term FROM concepts WHERE id = ?1",
        [id],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name = ?1",
        [name],
        |_| Ok(()),
    )
    .optional()
    .ok()
    .flatten()
    .is_some()
}

fn collect(conn: &Connection, sql: &str, p: &[&dyn rusqlite::ToSql]) -> Result<Vec<String>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map(p, |r| r.get::<_, String>(0))?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Read codes from a file or stdin. The leading whitespace-delimited token of
/// each non-blank, non-`#` line is taken as the code (so `sct ecl expand`,
/// `cut`, `grep` output pipes straight in).
fn read_codes(input: Option<&std::path::Path>) -> Result<Vec<String>> {
    let reader: Box<dyn BufRead> = match input {
        Some(p) => Box::new(std::io::BufReader::new(
            std::fs::File::open(p).with_context(|| format!("opening {}", p.display()))?,
        )),
        None => Box::new(std::io::BufReader::new(std::io::stdin())),
    };
    let mut codes = Vec::new();
    for line in reader.lines() {
        let line = line.context("reading input")?;
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some(tok) = t.split_whitespace().next() {
            codes.push(tok.to_string());
        }
    }
    Ok(codes)
}
