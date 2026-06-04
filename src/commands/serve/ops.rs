//! FHIR terminology operations as pure functions over a `rusqlite::Connection`,
//! returning `serde_json::Value` FHIR resources (or [`FhirError`]). The HTTP
//! layer in `mod.rs` is a thin wrapper around these. See `specs/commands/serve.md`.

use rusqlite::Connection;
use serde_json::{json, Value};
use std::collections::HashSet;

use super::fhir::{
    designation, parameters, property_concept, value_set_expansion, FhirError, SNOMED_SYSTEM,
};

fn ex(e: rusqlite::Error) -> FhirError {
    FhirError::exception(e.to_string())
}

struct Concept {
    pt: String,
    fsn: String,
    synonyms: Vec<String>,
    active: bool,
    module: String,
    effective_time: String,
}

fn fetch_concept(conn: &Connection, code: &str) -> Result<Option<Concept>, FhirError> {
    let mut stmt = conn
        .prepare_cached(
            "SELECT preferred_term, fsn, synonyms, active, module, effective_time
             FROM concepts WHERE id = ?1",
        )
        .map_err(ex)?;
    let row = stmt.query_row([code], |r| {
        let synonyms_json: String = r.get(2)?;
        Ok(Concept {
            pt: r.get(0)?,
            fsn: r.get(1)?,
            synonyms: serde_json::from_str(&synonyms_json).unwrap_or_default(),
            active: r.get::<_, i64>(3)? != 0,
            module: r.get(4)?,
            effective_time: r.get(5)?,
        })
    });
    match row {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(ex(e)),
    }
}

/// SNOMED release version recorded in the DB provenance, for the `version`
/// parameter and CapabilityStatement.
pub fn release_version(conn: &Connection) -> Option<String> {
    crate::provenance::read_sqlite(conn)
        .ok()
        .flatten()
        .and_then(|p| {
            if !p.release_date.is_empty() {
                Some(p.release_date)
            } else if !p.release_id.is_empty() {
                Some(p.release_id)
            } else {
                None
            }
        })
}

/// Direct parents (`parent = true`) or children of a concept, active only.
fn direct(conn: &Connection, code: &str, parent: bool) -> Result<Vec<(String, String)>, FhirError> {
    let sql = if parent {
        "SELECT c.id, c.preferred_term FROM concept_isa ci JOIN concepts c ON c.id = ci.parent_id
         WHERE ci.child_id = ?1 AND c.active = 1 ORDER BY c.preferred_term"
    } else {
        "SELECT c.id, c.preferred_term FROM concept_isa ci JOIN concepts c ON c.id = ci.child_id
         WHERE ci.parent_id = ?1 AND c.active = 1 ORDER BY c.preferred_term"
    };
    let mut stmt = conn.prepare_cached(sql).map_err(ex)?;
    let rows = stmt
        .query_map([code], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(ex)?;
    rows.collect::<Result<_, _>>().map_err(ex)
}

/// All (transitive) ancestors of a concept, excluding itself.
fn ancestors(conn: &Connection, code: &str) -> Result<Vec<(String, String)>, FhirError> {
    let sql = "WITH RECURSIVE anc(id) AS (
                   SELECT ?1
                   UNION
                   SELECT ci.parent_id FROM concept_isa ci JOIN anc ON ci.child_id = anc.id
               )
               SELECT c.id, c.preferred_term FROM anc JOIN concepts c ON c.id = anc.id
               WHERE c.id != ?1 ORDER BY c.preferred_term";
    let mut stmt = conn.prepare_cached(sql).map_err(ex)?;
    let rows = stmt
        .query_map([code], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(ex)?;
    rows.collect::<Result<_, _>>().map_err(ex)
}

/// Is `descendant` subsumed by `ancestor` (i.e. is `ancestor` an ancestor-or-self)?
fn is_subsumed(conn: &Connection, descendant: &str, ancestor: &str) -> Result<bool, FhirError> {
    let sql = "WITH RECURSIVE anc(id) AS (
                   SELECT ?1
                   UNION
                   SELECT ci.parent_id FROM concept_isa ci JOIN anc ON ci.child_id = anc.id
               )
               SELECT EXISTS(SELECT 1 FROM anc WHERE id = ?2)";
    let exists: i64 = conn
        .query_row(sql, [descendant, ancestor], |r| r.get(0))
        .map_err(ex)?;
    Ok(exists != 0)
}

/// `CodeSystem/$lookup`.
pub fn lookup(conn: &Connection, code: &str, props: &[String]) -> Result<Value, FhirError> {
    let c = fetch_concept(conn, code)?
        .ok_or_else(|| FhirError::not_found(format!("Code '{code}' not found in SNOMED CT")))?;
    let want = |p: &str| props.iter().any(|x| x.eq_ignore_ascii_case(p));
    let none_requested = props.is_empty();

    let mut parameter = vec![
        json!({ "name": "name", "valueString": "SNOMED CT" }),
        json!({ "name": "display", "valueString": c.pt }),
    ];
    if let Some(v) = release_version(conn) {
        parameter.push(json!({ "name": "version", "valueString": v }));
    }
    if none_requested || want("designation") {
        parameter.push(designation(
            "900000000000003001",
            "Fully specified name",
            &c.fsn,
        ));
        for s in &c.synonyms {
            parameter.push(designation("900000000000013009", "Synonym", s));
        }
    }
    if want("parent") {
        for (id, pt) in direct(conn, code, true)? {
            parameter.push(property_concept("parent", &id, &pt));
        }
    }
    if want("child") {
        for (id, pt) in direct(conn, code, false)? {
            parameter.push(property_concept("child", &id, &pt));
        }
    }
    if want("ancestor") {
        for (id, pt) in ancestors(conn, code)? {
            parameter.push(property_concept("ancestor", &id, &pt));
        }
    }
    if want("inactive") {
        parameter.push(json!({ "name": "property", "part": [
            { "name": "code", "valueCode": "inactive" },
            { "name": "value", "valueBoolean": !c.active },
        ]}));
    }
    if want("moduleId") {
        parameter.push(json!({ "name": "property", "part": [
            { "name": "code", "valueCode": "moduleId" },
            { "name": "value", "valueCode": c.module },
        ]}));
    }
    if want("effectiveTime") {
        parameter.push(json!({ "name": "property", "part": [
            { "name": "code", "valueCode": "effectiveTime" },
            { "name": "value", "valueString": c.effective_time },
        ]}));
    }
    Ok(parameters(parameter))
}

/// `CodeSystem/$validate-code`. An unknown code is a valid `result=false`
/// response, not an error.
pub fn validate_code(
    conn: &Connection,
    code: &str,
    display: Option<&str>,
) -> Result<Value, FhirError> {
    match fetch_concept(conn, code)? {
        None => Ok(parameters(vec![
            json!({ "name": "result", "valueBoolean": false }),
            json!({ "name": "message", "valueString": format!("Code '{code}' not found in SNOMED CT") }),
        ])),
        Some(c) => {
            let mut params = vec![
                json!({ "name": "result", "valueBoolean": true }),
                json!({ "name": "display", "valueString": c.pt }),
            ];
            if let Some(d) = display {
                let matches = d == c.pt || d == c.fsn || c.synonyms.iter().any(|s| s == d);
                if !matches {
                    params.push(json!({ "name": "message",
                        "valueString": format!("Display '{d}' does not match any designation for {code}") }));
                }
            }
            if !c.active {
                params.push(json!({ "name": "message", "valueString": "Concept is inactive" }));
            }
            Ok(parameters(params))
        }
    }
}

/// `CodeSystem/$subsumes`.
pub fn subsumes(conn: &Connection, code_a: &str, code_b: &str) -> Result<Value, FhirError> {
    if fetch_concept(conn, code_a)?.is_none() {
        return Err(FhirError::not_found(format!("Code '{code_a}' not found")));
    }
    if fetch_concept(conn, code_b)?.is_none() {
        return Err(FhirError::not_found(format!("Code '{code_b}' not found")));
    }
    let outcome = if code_a == code_b {
        "equivalent"
    } else {
        let a_sub_b = is_subsumed(conn, code_a, code_b)?; // B is an ancestor of A
        let b_sub_a = is_subsumed(conn, code_b, code_a)?;
        match (a_sub_b, b_sub_a) {
            (true, true) => "equivalent",
            (true, false) => "subsumed-by",
            (false, true) => "subsumes",
            (false, false) => "not-subsumed",
        }
    };
    Ok(parameters(vec![
        json!({ "name": "outcome", "valueCode": outcome }),
    ]))
}

/// `ValueSet/$expand` over an optional ECL constraint and/or text filter.
pub fn expand(
    conn: &Connection,
    ecl: Option<&str>,
    filter: Option<&str>,
    count: usize,
    offset: usize,
    include_designations: bool,
) -> Result<Value, FhirError> {
    let count = count.min(1000);

    let matched: Vec<String> = match (ecl, filter) {
        // Entire implicit SNOMED ValueSet: paginate in SQL.
        (None, None) => {
            let total: i64 = conn
                .query_row("SELECT COUNT(*) FROM concepts WHERE active = 1", [], |r| {
                    r.get(0)
                })
                .map_err(ex)?;
            let mut stmt = conn
                .prepare("SELECT id FROM concepts WHERE active = 1 ORDER BY id LIMIT ?1 OFFSET ?2")
                .map_err(ex)?;
            let ids: Vec<String> = stmt
                .query_map([count as i64, offset as i64], |r| r.get(0))
                .map_err(ex)?
                .collect::<Result<_, _>>()
                .map_err(ex)?;
            let contains = build_contains(conn, &ids, include_designations)?;
            return Ok(value_set_expansion(total as usize, offset, count, contains));
        }
        (Some(e), None) => eval_ecl(conn, e)?,
        (None, Some(f)) => fts_ids(conn, f)?,
        (Some(e), Some(f)) => {
            let set: HashSet<String> = eval_ecl(conn, e)?.into_iter().collect();
            fts_ids(conn, f)?
                .into_iter()
                .filter(|id| set.contains(id))
                .collect()
        }
    };

    let total = matched.len();
    let start = offset.min(total);
    let end = (offset + count).min(total);
    let contains = build_contains(conn, &matched[start..end], include_designations)?;
    Ok(value_set_expansion(total, offset, count, contains))
}

fn eval_ecl(conn: &Connection, ecl: &str) -> Result<Vec<String>, FhirError> {
    crate::ecl::expand(conn, ecl).map_err(|e| FhirError::invalid(format!("ECL error: {e:#}")))
}

/// FTS5 ids ordered by relevance, capped. Plain text is wrapped as a phrase to
/// avoid FTS5 parse errors on bare special characters.
fn fts_ids(conn: &Connection, filter: &str) -> Result<Vec<String>, FhirError> {
    let q = sanitise_fts(filter);
    let mut stmt = conn
        .prepare_cached(
            "SELECT c.id FROM concepts_fts JOIN concepts c ON concepts_fts.rowid = c.rowid
             WHERE concepts_fts MATCH ?1 AND c.active = 1 ORDER BY rank LIMIT 5000",
        )
        .map_err(ex)?;
    let ids = stmt
        .query_map([q], |r| r.get::<_, String>(0))
        .map_err(ex)?
        .collect::<Result<_, _>>()
        .map_err(ex)?;
    Ok(ids)
}

fn sanitise_fts(q: &str) -> String {
    let has_ops = q.contains('"')
        || q.contains('*')
        || q.contains('^')
        || q.to_uppercase().contains(" AND ")
        || q.to_uppercase().contains(" OR ")
        || q.to_uppercase().contains(" NOT ");
    if has_ops {
        q.to_string()
    } else {
        format!("\"{}\"", q.replace('"', "\"\""))
    }
}

/// Build `expansion.contains` entries for a page of ids, preserving order and
/// skipping ids that aren't concepts (e.g. refset metadata).
fn build_contains(
    conn: &Connection,
    ids: &[String],
    include_designations: bool,
) -> Result<Vec<Value>, FhirError> {
    let mut stmt = conn
        .prepare_cached("SELECT preferred_term, fsn, synonyms FROM concepts WHERE id = ?1")
        .map_err(ex)?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let row = stmt.query_row([id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        });
        match row {
            Ok((pt, fsn, syn)) => {
                let mut entry = json!({ "system": SNOMED_SYSTEM, "code": id, "display": pt });
                if include_designations {
                    let synonyms: Vec<String> = serde_json::from_str(&syn).unwrap_or_default();
                    let mut des = vec![designation(
                        "900000000000003001",
                        "Fully specified name",
                        &fsn,
                    )];
                    for s in &synonyms {
                        des.push(designation("900000000000013009", "Synonym", s));
                    }
                    entry["designation"] = Value::Array(des);
                }
                out.push(entry);
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {}
            Err(e) => return Err(ex(e)),
        }
    }
    Ok(out)
}
