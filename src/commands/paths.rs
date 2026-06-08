// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `sct paths` - Print the directories and files sct uses by default.
//!
//! Diagnostic surface for the path-resolution chain defined in
//! `crate::paths`. Running `sct paths` answers the question "where does sct
//! look for things?" and exactly which rule won on this machine.

use anyhow::Result;
use clap::Parser;

use crate::paths::{self, DATA_SUBDIR, RELEASES_SUBDIR};

#[derive(Parser, Debug)]
pub struct Args {}

pub fn run(_args: Args) -> Result<()> {
    let data_home = paths::data_home();
    let config_home = paths::config_home();
    let config_path = paths::config_path();

    print_row(
        "data home:",
        &paths::display_path(&data_home),
        data_home_hint(),
    );
    print_row(
        "config home:",
        &paths::display_path(&config_home),
        config_home_hint(),
    );
    print_row(
        "config file:",
        &paths::display_path(&config_path),
        if config_path.exists() {
            "exists"
        } else {
            "(none)"
        }
        .into(),
    );
    println!();

    // Database
    match paths::resolve_db(None) {
        Ok(r) => print_row("database:", &paths::display_path(&r.path), r.source.label()),
        Err(_) => print_row("database:", "─", "not found".into()),
    }

    // Embeddings
    match paths::resolve_embeddings(None) {
        Ok(r) => print_row(
            "embeddings:",
            &paths::display_path(&r.path),
            r.source.label(),
        ),
        Err(_) => print_row("embeddings:", "─", "not found".into()),
    }
    println!();

    // TRUD-managed write dirs
    let releases = data_home.join(RELEASES_SUBDIR);
    let data = data_home.join(DATA_SUBDIR);
    print_row(
        "trud releases:",
        &paths::display_path(&releases),
        dir_summary(&releases),
    );
    print_row(
        "trud data:",
        &paths::display_path(&data),
        dir_summary(&data),
    );

    Ok(())
}

fn print_row(label: &str, path: &str, hint: String) {
    println!("{label:<16} {path:<60} {hint}");
}

fn data_home_hint() -> String {
    if std::env::var("SCT_DATA_HOME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .is_some()
    {
        "$SCT_DATA_HOME".into()
    } else if std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .is_some()
    {
        "$XDG_DATA_HOME/sct".into()
    } else {
        "default".into()
    }
}

fn config_home_hint() -> String {
    if std::env::var("SCT_CONFIG_HOME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .is_some()
    {
        "$SCT_CONFIG_HOME".into()
    } else if std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .is_some()
    {
        "$XDG_CONFIG_HOME/sct".into()
    } else {
        "default".into()
    }
}

fn dir_summary(p: &std::path::Path) -> String {
    if !p.exists() {
        return "missing".into();
    }
    match std::fs::read_dir(p) {
        Ok(rd) => {
            let n = rd.filter_map(|e| e.ok()).count();
            format!("{n} file{}", if n == 1 { "" } else { "s" })
        }
        Err(_) => "unreadable".into(),
    }
}
