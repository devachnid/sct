//! `sct ecl expand` - evaluate a SNOMED CT ECL expression against the database
//! and emit the matching concept SCTIDs.
//!
//! Composable by design (see `specs/spec.md` - "Composability"): stdout is
//! newline-delimited SCTIDs, so it pipes straight into `sct codelist add <file> -`,
//! `sct lookup`, `jq`, or anything else. `--json` emits a JSON array instead.
//! The human-readable match count goes to stderr, keeping stdout clean for pipes.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    command: EclCommand,
}

#[derive(Subcommand, Debug)]
enum EclCommand {
    /// Evaluate an ECL expression and print the matching concept SCTIDs.
    Expand(ExpandArgs),
}

#[derive(Parser, Debug)]
struct ExpandArgs {
    /// ECL expression, e.g. `"<<73211009"`. Pass `-` to read it from stdin.
    expr: String,

    /// SNOMED CT SQLite database. See `docs/path-resolution.md` for the
    /// discovery order when this flag is omitted.
    #[arg(long)]
    db: Option<PathBuf>,

    /// Emit a JSON array of SCTID strings instead of newline-delimited ids.
    #[arg(long)]
    json: bool,
}

pub fn run(args: Args) -> Result<()> {
    match args.command {
        EclCommand::Expand(a) => expand(a),
    }
}

fn expand(args: ExpandArgs) -> Result<()> {
    let expr = if args.expr == "-" {
        let mut s = String::new();
        std::io::stdin()
            .read_to_string(&mut s)
            .context("reading ECL expression from stdin")?;
        s.trim().to_string()
    } else {
        args.expr
    };

    let db = crate::paths::resolve_db(args.db.as_deref())?.path;
    let conn = crate::commands::open_db_readonly(&db, None)?;
    let ids = crate::ecl::expand(&conn, &expr)?;

    eprintln!("{} concept(s) matched {expr:?}", ids.len());

    if args.json {
        println!("{}", serde_json::to_string(&ids)?);
    } else {
        let mut out = std::io::stdout().lock();
        for id in &ids {
            writeln!(out, "{id}")?;
        }
    }
    Ok(())
}
