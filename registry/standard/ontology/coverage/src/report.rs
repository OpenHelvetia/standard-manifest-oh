//! The coverage check and its printed report.
//!
//! Gate semantics (masterplan L0.5, E16 no. 5e):
//! - emitted-but-uncovered term or value  -> FAILURE (non-zero exit),
//! - extraction error (unmapped field)    -> FAILURE,
//! - declared-but-orphaned term           -> WARNING only (listed).
//!
//! A declared term that no source emits is NOT an orphan when it is linked
//! into the emitted core (transitively, over any oh:-to-oh: triple, or by
//! being used as a predicate): the URI-model classes and the named value
//! datatypes are declared support vocabulary, connected by ranges,
//! subclassing and rdfs:seeAlso. Only a term with no path into the emitted
//! core is orphaned.

use std::collections::{BTreeSet, VecDeque};

use crate::model::Model;
use crate::sources::Requirements;

/// Status of one term row in the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Emitted and covered (declared resp. referenced).
    Covered,
    /// Emitted but not covered — fails the gate.
    Missing,
    /// Declared, not emitted, but linked into the emitted core.
    Support,
    /// Declared, not emitted, not linked — warning.
    Orphan,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Status::Covered => "covered",
            Status::Missing => "MISSING",
            Status::Support => "support",
            Status::Orphan => "ORPHAN",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Row {
    pub term: String,
    pub status: Status,
    pub detail: String,
}

/// The full check result.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Emitted `oh:` terms (must be declared).
    pub declared_rows: Vec<Row>,
    /// Emitted foreign terms (must be referenced, never re-declared).
    pub referenced_rows: Vec<Row>,
    /// Emitted enumerated values (must be documented on the owning term).
    pub value_rows: Vec<Row>,
    /// Declared terms no source emits (support or orphan).
    pub extra_rows: Vec<Row>,
    pub failures: Vec<String>,
    pub warnings: Vec<String>,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Run the coverage check of `model` (from oh.ttl) against `reqs`.
pub fn check(model: &Model, reqs: &Requirements) -> Report {
    let mut report = Report {
        failures: reqs.errors.clone(),
        ..Report::default()
    };
    let declared = model.declared();

    for (iri, sources) in &reqs.terms {
        let detail = join_sources(sources);
        if Requirements::needs_declaration(iri) {
            let status = if declared.contains(iri.as_str()) {
                Status::Covered
            } else {
                report.failures.push(format!(
                    "emitted but not declared: {} ({detail})",
                    model.compact(iri)
                ));
                Status::Missing
            };
            report.declared_rows.push(Row {
                term: model.compact(iri),
                status,
                detail,
            });
        } else {
            let status = if model.referenced.contains(iri) {
                Status::Covered
            } else {
                report.failures.push(format!(
                    "emitted but not referenced (import missing): {} ({detail})",
                    model.compact(iri)
                ));
                Status::Missing
            };
            report.referenced_rows.push(Row {
                term: model.compact(iri),
                status,
                detail,
            });
        }
    }

    for ((owner, value), sources) in &reqs.values {
        let detail = join_sources(sources);
        let documented_on_owner = model
            .allowed_values
            .get(owner)
            .is_some_and(|vs| vs.contains(value));
        let documented_on_range = model.terms.get(owner).is_some_and(|term| {
            term.ranges.iter().any(|r| {
                model
                    .allowed_values
                    .get(r)
                    .is_some_and(|vs| vs.contains(value))
            })
        });
        let status = if documented_on_owner || documented_on_range {
            Status::Covered
        } else {
            report.failures.push(format!(
                "emitted value not documented: {} = '{value}' ({detail})",
                model.compact(owner)
            ));
            Status::Missing
        };
        report.value_rows.push(Row {
            term: format!("{} = '{value}'", model.compact(owner)),
            status,
            detail,
        });
    }

    // Orphan analysis: BFS over the undirected oh:-to-oh: adjacency,
    // seeded with the emitted-and-declared terms.
    let emitted: BTreeSet<&str> = reqs
        .terms
        .keys()
        .filter(|iri| Requirements::needs_declaration(iri))
        .map(String::as_str)
        .collect();
    let mut anchored: BTreeSet<&str> = emitted
        .iter()
        .copied()
        .filter(|iri| declared.contains(iri))
        .collect();
    let mut queue: VecDeque<&str> = anchored.iter().copied().collect();
    while let Some(term) = queue.pop_front() {
        if let Some(neighbours) = model.edges.get(term) {
            for n in neighbours {
                if declared.contains(n.as_str()) && anchored.insert(n) {
                    queue.push_back(n);
                }
            }
        }
    }
    for iri in &declared {
        if emitted.contains(iri) {
            continue;
        }
        let in_use_as_predicate = model.predicates_used.contains(*iri);
        if anchored.contains(iri) || in_use_as_predicate {
            report.extra_rows.push(Row {
                term: model.compact(iri),
                status: Status::Support,
                detail: if in_use_as_predicate {
                    "in use as a predicate in oh.ttl".to_string()
                } else {
                    "linked into the emitted core".to_string()
                },
            });
        } else {
            report.warnings.push(format!(
                "declared but orphaned (no link to any emitted term): {}",
                model.compact(iri)
            ));
            report.extra_rows.push(Row {
                term: model.compact(iri),
                status: Status::Orphan,
                detail: "no link to any emitted term".to_string(),
            });
        }
    }

    report
}

fn join_sources(sources: &BTreeSet<String>) -> String {
    sources.iter().cloned().collect::<Vec<_>>().join("; ")
}

/// Render the human-readable coverage table.
pub fn render(report: &Report) -> String {
    let mut out = String::new();
    section(
        &mut out,
        "Emitted oh: terms (must be declared)",
        &report.declared_rows,
    );
    section(
        &mut out,
        "Emitted foreign terms (must be referenced, never re-declared)",
        &report.referenced_rows,
    );
    section(&mut out, "Emitted enumerated values", &report.value_rows);
    section(
        &mut out,
        "Declared beyond the emitted set",
        &report.extra_rows,
    );

    let covered = |rows: &[Row]| rows.iter().filter(|r| r.status == Status::Covered).count();
    out.push_str(&format!(
        "Summary: {}/{} oh: terms, {}/{} foreign terms, {}/{} values covered; {} support terms; {} orphans; {} failures.\n",
        covered(&report.declared_rows),
        report.declared_rows.len(),
        covered(&report.referenced_rows),
        report.referenced_rows.len(),
        covered(&report.value_rows),
        report.value_rows.len(),
        report
            .extra_rows
            .iter()
            .filter(|r| r.status == Status::Support)
            .count(),
        report
            .extra_rows
            .iter()
            .filter(|r| r.status == Status::Orphan)
            .count(),
        report.failures.len(),
    ));

    if !report.warnings.is_empty() {
        out.push_str("\nWarnings:\n");
        for w in &report.warnings {
            out.push_str(&format!("  warning: {w}\n"));
        }
    }
    if !report.failures.is_empty() {
        out.push_str("\nFailures:\n");
        for f in &report.failures {
            out.push_str(&format!("  FAILURE: {f}\n"));
        }
    }
    out
}

fn section(out: &mut String, title: &str, rows: &[Row]) {
    out.push_str(&format!("== {title} ==\n"));
    let width = rows.iter().map(|r| r.term.len()).max().unwrap_or(0).max(4);
    for row in rows {
        out.push_str(&format!(
            "  {:width$}  {:8}  {}\n",
            row.term,
            row.status.label(),
            row.detail,
        ));
    }
    out.push('\n');
}
