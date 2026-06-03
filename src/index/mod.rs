//! FST-backed lexical index for SNOMED CT.
//!
//! A single-file, mmap-able terminology index built from `sct`'s canonical
//! NDJSON, offering exact / prefix / fuzzy / word-intersection lookup. See
//! `specs/fst.md` for the design, rationale, and the benchmark this is meant to
//! settle (FST vs the existing SQLite FTS5 path).
//!
//! - [`build`] — NDJSON → `snomed.fst`
//! - [`query::Index`] — open and query an artefact
//! - [`format`] — the on-disk container layout and value packing
//! - [`normalise`] — term normalisation (lossless w.r.t. accents/punctuation)

pub mod build;
pub mod format;
pub mod normalise;
pub mod query;

pub use build::{build, BuildStats};
pub use query::{Hit, Index};
