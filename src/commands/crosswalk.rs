// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `sct crosswalk` - show all cross-terminology equivalents of a single code at
//! once (the text equivalent of the NHS Data Migration Workbench's
//! tri-terminology BROWSE view). Built on the same maps as `sct transcode`.

use anyhow::{bail, Context, Result};
use clap::Parser;
use rusqlite::Connection;
use std::io::Write;
use std::path::PathBuf;

use crate::commands::transcode::transcode_one;

const SYSTEMS: [&str; 5] = ["snomed", "read2", "ctv3", "icd10", "opcs4"];

/// All cross-terminology equivalents of one code: its SNOMED pivot + preferred
/// term, and the equivalent code(s) in every other terminology.
pub struct Crosswalk {
    pub snomed: String,
    pub display: String,
    /// `(system, codes)` for every terminology other than the source.
    pub equivalents: Vec<(&'static str, Vec<String>)>,
}

/// Resolve every cross-terminology equivalent of `code` (in terminology `from`).
/// The pure core of `sct crosswalk` (no I/O), exposed for tests and reuse.
pub fn equivalents(conn: &Connection, from: &str, code: &str) -> Result<Crosswalk> {
    if !SYSTEMS.contains(&from) {
        bail!(
            "unknown terminology {from:?}; expected one of {}",
            SYSTEMS.join(", ")
        );
    }
    let pivots = transcode_one(conn, from, code, "snomed", false)?;
    let (snomed, display) = pivots
        .first()
        .map(|m| (m.snomed.clone(), m.display.clone().unwrap_or_default()))
        .unwrap_or_default();

    let mut equivalents = Vec::new();
    for to in SYSTEMS {
        if to == from {
            continue;
        }
        let mut codes: Vec<String> = transcode_one(conn, from, code, to, false)?
            .into_iter()
            .map(|m| m.target)
            .collect();
        codes.sort();
        codes.dedup();
        equivalents.push((to, codes));
    }
    Ok(Crosswalk {
        snomed,
        display,
        equivalents,
    })
}

#[derive(Parser, Debug)]
pub struct Args {
    /// The code to crosswalk.
    pub code: String,

    /// Terminology of `<code>`: snomed (default) | read2 | ctv3 | icd10 | opcs4.
    #[arg(long, default_value = "snomed")]
    pub from: String,

    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,

    /// SNOMED CT SQLite database. Discovered via the usual path-resolution chain
    /// when omitted (see `docs/path-resolution.md`).
    #[arg(long)]
    pub db: Option<PathBuf>,
}

pub fn run(args: Args) -> Result<()> {
    let from = args.from.to_lowercase();
    let db = crate::paths::resolve_db(args.db.as_deref())?.path;
    let conn = crate::commands::open_db_readonly(&db, None)
        .with_context(|| format!("opening database {}", db.display()))?;

    let cw = equivalents(&conn, &from, &args.code)?;

    if args.json {
        let map: std::collections::BTreeMap<&str, &Vec<String>> =
            cw.equivalents.iter().map(|(s, c)| (*s, c)).collect();
        println!(
            "{}",
            serde_json::json!({
                "code": args.code,
                "from": from,
                "snomed": cw.snomed,
                "display": cw.display,
                "equivalents": map,
            })
        );
    } else {
        let mut out = std::io::stdout().lock();
        if from == "snomed" {
            writeln!(out, "{}  {}", args.code, cw.display)?;
        } else if cw.snomed.is_empty() {
            writeln!(out, "{} ({from})  ->  (no SNOMED CT match)", args.code)?;
        } else {
            writeln!(
                out,
                "{} ({from})  ->  SNOMED {}  {}",
                args.code, cw.snomed, cw.display
            )?;
        }
        for (sys, codes) in &cw.equivalents {
            let val = if codes.is_empty() {
                "(none)".to_string()
            } else {
                codes.join(", ")
            };
            writeln!(out, "  {:<7} {val}", format!("{sys}:"))?;
        }
    }
    Ok(())
}
