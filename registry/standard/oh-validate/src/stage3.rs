//! Stage 3 of the validation chain (E16 Ziff. 5c): SHACL validation
//! of the manifest's RDF graph via rudof, against the vendored shapes
//! (`shapes/manifest-v1.ttl`).
//!
//! Pipeline: the document is expanded through the SAME offline
//! machinery as stage 2 (VendoredLoader — foreign IRIs refused by
//! construction), converted to N-Triples by us, and handed to rudof
//! as in-memory data. rudof never parses JSON-LD itself and therefore
//! never resolves anything from the network.
//!
//! E16 mandate: rudof's own test coverage is unpublished, so every
//! constraint the shapes rely on is proven to FIRE by our own
//! conformance suite (`tests/stage3.rs`) — a constraint without a
//! failing negative case is treated as not enforced.

use std::path::Path;

use anyhow::{Context as _, Result};
use json_ld::rdf_types::{self, Id, Term};
use json_ld::syntax::Parse;
use json_ld::{Flatten, JsonLdProcessor, RdfQuads, RemoteDocument};
use rudof_lib::formats::{DataFormat, InputSpec, ResultShaclValidationFormat, ShaclFormat};
use rudof_lib::{Rudof, RudofConfig};

/// Concrete node identifier for the unit vocabulary used here.
type NodeId = Id<json_ld::iref::IriBuf, rdf_types::BlankIdBuf>;

use crate::stage2::VendoredLoader;

/// Runs the stage-3 gate: returns `None` when the graph conforms,
/// otherwise the full RDF validation report (for humans and for the
/// conformance suite's constraint assertions).
pub fn check(raw: &str, loader: &VendoredLoader, shapes: &Path) -> Result<Option<String>> {
    let ntriples = to_ntriples(raw, loader)?;
    validate(&ntriples, shapes)
}

/// Expands the document offline and serializes its graph as
/// N-Triples. Public for the conformance suite.
pub fn to_ntriples(raw: &str, loader: &VendoredLoader) -> Result<String> {
    to_ntriples_prefixed(raw, loader, "")
}

/// Like [`to_ntriples`], but blank-node labels get `prefix` inserted
/// after `_:`. Stage 4 merges many documents into ONE graph — without
/// per-document prefixes their generator-issued labels (`_:b0`, …)
/// would collide and silently merge distinct nodes.
pub fn to_ntriples_prefixed(raw: &str, loader: &VendoredLoader, prefix: &str) -> Result<String> {
    let mut body: serde_json::Value =
        serde_json::from_str(raw).context("stage 3: parsing document")?;
    if let Some(obj) = body.as_object_mut() {
        obj.remove("$schema");
    }

    let (ld_value, _) = json_ld::syntax::Value::parse_str(
        &serde_json::to_string(&body).context("stage 3: serializing body")?,
    )
    .map_err(|e| anyhow::anyhow!("stage 3: json-syntax parse: {e}"))?;

    let input = RemoteDocument::new(
        None::<json_ld::iref::IriBuf>,
        Some("application/ld+json".parse().expect("valid mime")),
        ld_value,
    );

    let expanded = futures::executor::block_on(async { input.expand(loader).await })
        .map_err(|e| anyhow::anyhow!("stage 3: expansion failed: {e}"))?;

    // JSON-LD → RDF runs over the node map: nested node objects only
    // become triples after flattening (the ExpandedDocument iterator
    // emits top-level nodes only — verified live against the corpus).
    // `ordered = true` keeps the output deterministic.
    let mut generator = rdf_types::generator::Blank::new();
    let flattened = expanded
        .flatten(&mut generator, true)
        .map_err(|e| anyhow::anyhow!("stage 3: flattening failed: {e}"))?;

    let mut out = String::new();
    for quad in flattened.rdf_quads(&mut generator, None) {
        let rdf_types::Quad(s, p, o, _g) = quad;
        out.push_str(&format_id(&s, prefix));
        out.push(' ');
        out.push_str(&format_id(&p, prefix));
        out.push(' ');
        match &o {
            Term::Id(id) => out.push_str(&format_id(id, prefix)),
            Term::Literal(lit) => out.push_str(&format_literal(lit)),
        }
        out.push_str(" .\n");
    }
    if std::env::var_os("OH_STAGE3_DEBUG").is_some() {
        eprintln!("--- N-Triples ---\n{out}");
    }
    Ok(out)
}

fn format_id(id: &NodeId, prefix: &str) -> String {
    match id {
        Id::Iri(i) => format!("<{i}>"),
        Id::Blank(b) => {
            let label = b.as_str().strip_prefix("_:").unwrap_or(b.as_str());
            format!("_:{prefix}{label}")
        }
    }
}

fn format_literal(lit: &rdf_types::Literal) -> String {
    let escaped = escape_nt(lit.value.as_str());
    match &lit.type_ {
        rdf_types::LiteralType::Any(dt) => {
            // Plain xsd:string stays bare per N-Triples canon.
            if dt.as_str() == "http://www.w3.org/2001/XMLSchema#string" {
                format!("\"{escaped}\"")
            } else {
                format!("\"{escaped}\"^^<{dt}>")
            }
        }
        rdf_types::LiteralType::LangString(tag) => {
            format!("\"{escaped}\"@{tag}")
        }
    }
}

fn escape_nt(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '\\' => vec!['\\', '\\'],
            '"' => vec!['\\', '"'],
            '\n' => vec!['\\', 'n'],
            '\r' => vec!['\\', 'r'],
            '\t' => vec!['\\', 't'],
            other => vec![other],
        })
        .collect()
}

/// Feeds N-Triples data plus the vendored shapes to rudof and reads
/// the validation report back as Turtle. `None` = conforms.
pub fn validate(ntriples: &str, shapes: &Path) -> Result<Option<String>> {
    let mut rudof = Rudof::new(RudofConfig::default());

    rudof
        .load_shacl_shapes()
        .with_shacl_schema(&InputSpec::path(shapes))
        .with_shacl_schema_format(&ShaclFormat::Turtle)
        .execute()
        .map_err(|e| anyhow::anyhow!("stage 3: loading shapes: {e}"))?;

    let data = [InputSpec::str(ntriples)];
    rudof
        .load_data()
        .with_data(&data)
        .with_data_format(&DataFormat::NTriples)
        .execute()
        .map_err(|e| anyhow::anyhow!("stage 3: loading data: {e}"))?;

    rudof
        .validate_shacl()
        .execute()
        .map_err(|e| anyhow::anyhow!("stage 3: validation run: {e}"))?;

    let mut buf: Vec<u8> = Vec::new();
    rudof
        .serialize_shacl_validation_results(&mut buf)
        .with_result_shacl_validation_format(&ResultShaclValidationFormat::Turtle)
        .execute()
        .map_err(|e| anyhow::anyhow!("stage 3: serializing report: {e}"))?;
    let report = String::from_utf8(buf).context("stage 3: report is not UTF-8")?;

    if report_conforms(&report) {
        Ok(None)
    } else {
        if std::env::var_os("OH_STAGE3_DEBUG").is_some() {
            eprintln!("--- SHACL report ---\n{report}");
        }
        Ok(Some(report))
    }
}

/// A report conforms iff it carries `sh:conforms true` and no
/// validation results. Parsed defensively on the Turtle text.
fn report_conforms(report: &str) -> bool {
    let has_false = report.contains("false");
    let has_result = report.contains("ValidationResult") || report.contains("resultPath");
    !(has_false || has_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nt_escaping_covers_the_specials() {
        assert_eq!(escape_nt("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
    }

    #[test]
    fn conforms_detection_is_defensive() {
        assert!(report_conforms(
            "[] a sh:ValidationReport ; sh:conforms true ."
        ));
        assert!(!report_conforms("[] sh:conforms false ."));
        assert!(!report_conforms(
            "… a sh:ValidationResult ; sh:resultPath dct:title ."
        ));
    }
}
