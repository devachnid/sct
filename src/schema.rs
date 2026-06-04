//! Canonical per-concept record type.
//!
//! This is the stable public interface between the NDJSON producer (`sct ndjson`)
//! and all downstream consumers (`sct sqlite`, `sct parquet`, `sct markdown`, `sct mcp`).
//!
//! The format is versioned with `schema_version` so consumers can detect
//! incompatible format changes at parse time.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Current NDJSON schema version. Increment when the record structure changes
/// in a backward-incompatible way.
///
/// v4: adds `relationships` (typed attribute triples, SCTID-keyed) to support
/// ECL attribute refinement. Additive — older records parse with an empty list.
pub const SCHEMA_VERSION: u32 = 4;

/// A lightweight reference to another concept (used in parents and attributes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptRef {
    pub id: String,
    pub fsn: String,
}

/// A typed attribute relationship, with all parts kept as SCTIDs (plus the
/// relationship group number). Unlike the display-oriented `attributes` map,
/// this preserves the attribute *type* SCTID and group, which ECL refinement
/// needs. See `specs/ecl.md` §4.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Attribute type SCTID, e.g. `363698007` (Finding site).
    pub type_id: String,
    /// Destination (value) concept SCTID.
    pub destination_id: String,
    /// RF2 relationship group number (0 = ungrouped).
    pub group: u32,
}

/// The per-concept JSON record written to the NDJSON artefact.
///
/// One record per line, sorted by `id` (ascending numeric SCTID).
#[derive(Debug, Serialize, Deserialize)]
pub struct ConceptRecord {
    pub id: String,
    pub fsn: String,
    pub preferred_term: String,
    pub synonyms: Vec<String>,
    pub hierarchy: String,
    pub hierarchy_path: Vec<String>,
    pub parents: Vec<ConceptRef>,
    pub children_count: usize,
    pub active: bool,
    pub module: String,
    pub effective_time: String,
    pub attributes: IndexMap<String, Vec<ConceptRef>>,
    /// CTV3 (Read v3) codes mapped to this concept (may be empty if no UK map loaded)
    #[serde(default)]
    pub ctv3_codes: Vec<String>,
    /// Read v2 codes mapped to this concept (may be empty if no UK map loaded)
    #[serde(default)]
    pub read2_codes: Vec<String>,
    /// SCTIDs of reference sets this concept belongs to. Populated when the
    /// NDJSON was built with `--refsets simple` (or higher). Each ID itself
    /// resolves to a concept in this dataset — look it up to get the refset's
    /// preferred term, module, and other metadata.
    #[serde(default)]
    pub refsets: Vec<String>,
    /// Typed attribute relationships (SCTID-keyed, with group). Populated from
    /// the same RF2 data that feeds `attributes`, but preserving the type SCTID
    /// and group for ECL. Empty on records from schema v3 and earlier.
    #[serde(default)]
    pub relationships: Vec<Relationship>,
    pub schema_version: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ConceptRecord {
        ConceptRecord {
            id: "22298006".into(),
            fsn: "Myocardial infarction (disorder)".into(),
            preferred_term: "Myocardial infarction".into(),
            synonyms: vec!["Heart attack".into()],
            hierarchy: "Clinical finding".into(),
            hierarchy_path: vec![],
            parents: vec![],
            children_count: 0,
            active: true,
            module: "x".into(),
            effective_time: "20260101".into(),
            attributes: IndexMap::new(),
            ctv3_codes: vec![],
            read2_codes: vec![],
            refsets: vec![],
            relationships: vec![Relationship {
                type_id: "363698007".into(),
                destination_id: "74281007".into(),
                group: 1,
            }],
            schema_version: SCHEMA_VERSION,
        }
    }

    #[test]
    fn v4_record_with_relationships_round_trips() {
        let rec = sample();
        let json = serde_json::to_string(&rec).unwrap();
        let back: ConceptRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.relationships.len(), 1);
        assert_eq!(back.relationships[0].type_id, "363698007");
        assert_eq!(back.relationships[0].destination_id, "74281007");
        assert_eq!(back.relationships[0].group, 1);
        assert_eq!(back.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn v3_json_without_relationships_still_parses() {
        // A pre-v4 record has no `relationships` key; serde default fills it empty.
        let v3 = r#"{"id":"73211009","fsn":"Diabetes mellitus (disorder)",
            "preferred_term":"Diabetes mellitus","synonyms":[],"hierarchy":"Clinical finding",
            "hierarchy_path":[],"parents":[],"children_count":0,"active":true,"module":"x",
            "effective_time":"","attributes":{},"schema_version":3}"#;
        let rec: ConceptRecord = serde_json::from_str(v3).unwrap();
        assert!(rec.relationships.is_empty());
        assert_eq!(rec.schema_version, 3);
    }
}
