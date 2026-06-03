//! Evaluate an ECL [`Expr`] against a SNOMED CT SQLite database, returning the
//! set of matching concept SCTIDs. See `specs/ecl.md` §6.
//!
//! Set algebra runs in Rust over `BTreeSet<String>`; hierarchy and refset
//! membership are pulled from SQLite via recursive CTEs. This is correct and
//! adequate for codelist-scale queries; whole-AST SQL compilation for very
//! large result sets is a documented future optimisation.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::BTreeSet;

use crate::ecl::ast::{BoolOp, Expr, Op, Refinement};

/// A set of concept SCTIDs.
pub type IdSet = BTreeSet<String>;

/// Evaluate an ECL expression against `conn`.
pub fn evaluate(conn: &Connection, expr: &Expr) -> Result<IdSet> {
    eval_expr(conn, expr)
}

fn eval_expr(conn: &Connection, expr: &Expr) -> Result<IdSet> {
    match expr {
        Expr::Wildcard => all_concepts(conn),
        Expr::Concept(id) => Ok(std::iter::once(id.clone()).collect()),
        Expr::Op(op, inner) => {
            let base = eval_expr(conn, inner)?;
            eval_op(conn, *op, &base)
        }
        Expr::Bool(op, a, b) => {
            let sa = eval_expr(conn, a)?;
            let sb = eval_expr(conn, b)?;
            Ok(match op {
                BoolOp::And => sa.intersection(&sb).cloned().collect(),
                BoolOp::Or => sa.union(&sb).cloned().collect(),
                BoolOp::Minus => sa.difference(&sb).cloned().collect(),
            })
        }
        Expr::Refined(focus, refinement) => {
            let f = eval_expr(conn, focus)?;
            eval_refinement(conn, &f, refinement)
        }
    }
}

fn all_concepts(conn: &Connection) -> Result<IdSet> {
    let mut stmt = conn
        .prepare_cached("SELECT id FROM concepts WHERE active = 1")
        .context("preparing wildcard query")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut set = IdSet::new();
    for r in rows {
        set.insert(r?);
    }
    Ok(set)
}

fn eval_op(conn: &Connection, op: Op, base: &IdSet) -> Result<IdSet> {
    let mut out = IdSet::new();
    match op {
        Op::DescendantOf | Op::DescendantOrSelfOf => {
            for id in base {
                collect_transitive(conn, id, true, &mut out)?;
            }
            if op == Op::DescendantOrSelfOf {
                out.extend(base.iter().cloned());
            }
        }
        Op::AncestorOf | Op::AncestorOrSelfOf => {
            for id in base {
                collect_transitive(conn, id, false, &mut out)?;
            }
            if op == Op::AncestorOrSelfOf {
                out.extend(base.iter().cloned());
            }
        }
        Op::ChildOf => {
            for id in base {
                collect_one_hop(conn, id, true, &mut out)?;
            }
        }
        Op::ParentOf => {
            for id in base {
                collect_one_hop(conn, id, false, &mut out)?;
            }
        }
        Op::MemberOf => {
            for id in base {
                collect_members(conn, id, &mut out)?;
            }
        }
    }
    Ok(out)
}

/// Descendants (`down = true`) or ancestors (`down = false`) of `id` via a
/// recursive CTE over `concept_isa`. `UNION` dedups, so the DAG terminates.
fn collect_transitive(conn: &Connection, id: &str, down: bool, out: &mut IdSet) -> Result<()> {
    let sql = if down {
        "WITH RECURSIVE d(id) AS (
            SELECT child_id FROM concept_isa WHERE parent_id = ?1
            UNION
            SELECT ci.child_id FROM concept_isa ci JOIN d ON ci.parent_id = d.id
         ) SELECT id FROM d"
    } else {
        "WITH RECURSIVE a(id) AS (
            SELECT parent_id FROM concept_isa WHERE child_id = ?1
            UNION
            SELECT ci.parent_id FROM concept_isa ci JOIN a ON ci.child_id = a.id
         ) SELECT id FROM a"
    };
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map([id], |r| r.get::<_, String>(0))?;
    for r in rows {
        out.insert(r?);
    }
    Ok(())
}

fn collect_one_hop(conn: &Connection, id: &str, down: bool, out: &mut IdSet) -> Result<()> {
    let sql = if down {
        "SELECT child_id FROM concept_isa WHERE parent_id = ?1"
    } else {
        "SELECT parent_id FROM concept_isa WHERE child_id = ?1"
    };
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map([id], |r| r.get::<_, String>(0))?;
    for r in rows {
        out.insert(r?);
    }
    Ok(())
}

fn collect_members(conn: &Connection, refset_id: &str, out: &mut IdSet) -> Result<()> {
    let mut stmt = conn.prepare_cached(
        "SELECT referenced_component_id FROM refset_members WHERE refset_id = ?1",
    )?;
    let rows = stmt.query_map([refset_id], |r| r.get::<_, String>(0))?;
    for r in rows {
        out.insert(r?);
    }
    Ok(())
}

fn eval_refinement(conn: &Connection, focus: &IdSet, r: &Refinement) -> Result<IdSet> {
    match r {
        Refinement::And(a, b) => {
            let sa = eval_refinement(conn, focus, a)?;
            let sb = eval_refinement(conn, focus, b)?;
            Ok(sa.intersection(&sb).cloned().collect())
        }
        Refinement::Or(a, b) => {
            let sa = eval_refinement(conn, focus, a)?;
            let sb = eval_refinement(conn, focus, b)?;
            Ok(sa.union(&sb).cloned().collect())
        }
        // v1: a group is a flat conjunction (group cardinality deferred).
        Refinement::Group(inner) => eval_refinement(conn, focus, inner),
        Refinement::Attr {
            attr,
            negate,
            value,
        } => eval_attr(conn, focus, attr, *negate, value),
    }
}

/// Whether the `concept_relationships` table exists. Databases built before
/// schema v4 lack it, so attribute refinement needs a rebuild.
fn has_relationships_table(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name='concept_relationships'",
        [],
        |_| Ok(()),
    )
    .is_ok()
}

fn eval_attr(
    conn: &Connection,
    focus: &IdSet,
    attr: &Expr,
    negate: bool,
    value: &Expr,
) -> Result<IdSet> {
    if !has_relationships_table(conn) {
        anyhow::bail!(
            "ECL attribute refinement needs the 'concept_relationships' table, \
             which this database predates. Rebuild it with a current sct: \
             `sct ndjson` then `sct sqlite`."
        );
    }
    // `None` means wildcard (any type / any value).
    let type_filter: Option<IdSet> = match attr {
        Expr::Wildcard => None,
        _ => Some(eval_expr(conn, attr)?),
    };
    let value_filter: Option<IdSet> = match value {
        Expr::Wildcard => None,
        _ => Some(eval_expr(conn, value)?),
    };

    let mut matched = IdSet::new();
    match &type_filter {
        Some(types) => {
            let mut stmt = conn.prepare_cached(
                "SELECT source_id, destination_id FROM concept_relationships WHERE type_id = ?1",
            )?;
            for t in types {
                let rows = stmt.query_map([t], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })?;
                for row in rows {
                    let (source, dest) = row?;
                    consider(source, &dest, value_filter.as_ref(), negate, &mut matched);
                }
            }
        }
        None => {
            // Wildcard attribute type — full scan. Acceptable at codelist scale;
            // see specs/ecl.md §6 on the eventual SQL-compilation path.
            let mut stmt =
                conn.prepare_cached("SELECT source_id, destination_id FROM concept_relationships")?;
            let rows =
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            for row in rows {
                let (source, dest) = row?;
                consider(source, &dest, value_filter.as_ref(), negate, &mut matched);
            }
        }
    }

    Ok(focus.intersection(&matched).cloned().collect())
}

/// Record `source` as a match if its relationship `dest` satisfies the value
/// constraint. `None` value filter means "any destination" (`= *`).
fn consider(
    source: String,
    dest: &str,
    value_filter: Option<&IdSet>,
    negate: bool,
    matched: &mut IdSet,
) {
    let in_value = match value_filter {
        None => true,
        Some(vs) => vs.contains(dest),
    };
    // `=` matches when in_value; `!=` matches when not.
    if in_value != negate {
        matched.insert(source);
    }
}
