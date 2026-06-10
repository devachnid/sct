// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `sct transcode` over the synthetic fixture built with `--refsets all`
//! (so ICD-10/OPCS-4 crossmaps + concept history are present).

use rusqlite::Connection;
use sct_rs::commands::ndjson::{self, RefsetMode};
use sct_rs::commands::sqlite;
use sct_rs::commands::transcode::transcode_one;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/rf2/SnomedCT_SyntheticTest_PRODUCTION_20260101T120000Z")
}

fn build() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let ndjson = dir.path().join("syn.ndjson");
    let db = dir.path().join("syn.db");
    ndjson::run(ndjson::Args {
        rf2_dirs: vec![fixture_dir()],
        locale: "en-GB".into(),
        output: Some(ndjson.clone()),
        include_inactive: false,
        refsets: RefsetMode::All,
    })
    .unwrap();
    sqlite::run(sqlite::Args {
        input: ndjson,
        output: db.clone(),
        transitive_closure: false,
        include_self: false,
    })
    .unwrap();
    (dir, Connection::open(&db).unwrap())
}

fn targets(c: &Connection, from: &str, code: &str, to: &str, fwd: bool) -> Vec<String> {
    let mut v: Vec<String> = transcode_one(c, from, code, to, fwd)
        .unwrap()
        .into_iter()
        .map(|m| m.target)
        .collect();
    v.sort();
    v
}

#[test]
fn snomed_to_icd10_and_reverse() {
    let (_d, c) = build();
    assert_eq!(targets(&c, "snomed", "22298006", "icd10", false), ["I219"]);
    // Reverse: ICD-10 -> SNOMED.
    assert_eq!(targets(&c, "icd10", "I219", "snomed", false), ["22298006"]);
}

#[test]
fn snomed_to_opcs4() {
    let (_d, c) = build();
    assert_eq!(targets(&c, "snomed", "80146002", "opcs4", false), ["H011"]);
}

#[test]
fn ctv3_to_icd10_two_hop() {
    let (_d, c) = build();
    // CTV3 X200 -> SNOMED 22298006 -> ICD-10 I219.
    assert_eq!(targets(&c, "ctv3", "X200", "icd10", false), ["I219"]);
}

#[test]
fn history_forwarding_of_inactive_pivot() {
    let (_d, c) = build();
    // 9468002 is inactive; without forwarding it maps to nothing useful.
    assert!(targets(&c, "snomed", "9468002", "snomed", false) == ["9468002"]);
    // With forwarding it resolves to its same_as / replaced_by targets.
    assert_eq!(
        targets(&c, "snomed", "9468002", "snomed", true),
        ["195967001", "22298006"]
    );
}

#[test]
fn unmapped_code_yields_nothing() {
    let (_d, c) = build();
    assert!(targets(&c, "icd10", "Z999", "snomed", false).is_empty());
}
