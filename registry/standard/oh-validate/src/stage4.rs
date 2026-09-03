//! Stage 4 of the validation chain: the whole-graph gate. All
//! documents merge into ONE registry graph which is (a) fingerprinted
//! via RDFC-1.0 (SHA-256) — the deterministic build fingerprint of
//! E16 Ziff. 5d — and (b) checked against the critical invariants
//! doubled as SPARQL queries (E16 Ziff. 5c). The SPARQL doubling
//! deliberately restores what the rudof report cannot say (documented
//! stage-3 finding): each violation names its offending nodes
//! precisely.
//!
//! Blank-node hygiene: every document's labels are prefixed (`d3b0`)
//! before merging — without this, generator-issued labels would
//! collide across documents and silently merge distinct nodes.

use anyhow::{Context as _, Result};
use oxigraph::io::RdfFormat;
use oxigraph::model::Dataset;
use oxigraph::model::dataset::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm};
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use sha2::{Digest, Sha256};

use crate::stage2::VendoredLoader;
use crate::stage3;

/// Result of the whole-graph gate.
pub struct Stage4Report {
    /// RDFC-1.0 (SHA-256) fingerprint of the merged registry graph.
    pub fingerprint: String,
    /// Invariant violations, each naming its offenders. Empty = pass.
    pub violations: Vec<String>,
}

/// The critical invariants, doubled as SPARQL per E16. Each SELECT
/// returns the offending nodes; any row is a violation.
const INVARIANTS: &[(&str, &str)] = &[
    (
        "manifest without interface",
        "SELECT ?m WHERE { ?m a <https://ld.openhelvetia.swiss/schema/Manifest> . \
         FILTER NOT EXISTS { ?m <https://ld.openhelvetia.swiss/schema/interface> ?i } }",
    ),
    (
        "manifest without title",
        "SELECT ?m WHERE { ?m a <https://ld.openhelvetia.swiss/schema/Manifest> . \
         FILTER NOT EXISTS { ?m <http://purl.org/dc/terms/title> ?t } }",
    ),
    (
        "interface without auth declaration",
        "SELECT ?m ?if WHERE { ?m <https://ld.openhelvetia.swiss/schema/interface> ?if . \
         FILTER NOT EXISTS { ?if <https://ld.openhelvetia.swiss/schema/auth> ?a } }",
    ),
    (
        "auth without valid authType",
        "SELECT ?if ?t WHERE { ?if <https://ld.openhelvetia.swiss/schema/auth> ?a . \
         OPTIONAL { ?a <https://ld.openhelvetia.swiss/schema/authType> ?t } \
         FILTER ( !BOUND(?t) || ?t NOT IN (\"none\", \"apikey\", \"oauth2\", \"other\") ) }",
    ),
    (
        "interface node is an IRI (frame invariant)",
        "SELECT ?m ?if WHERE { ?m <https://ld.openhelvetia.swiss/schema/interface> ?if . \
         FILTER isIRI(?if) }",
    ),
    (
        "interface type outside the declared classes",
        "SELECT ?if ?t WHERE { ?m <https://ld.openhelvetia.swiss/schema/interface> ?if . \
         OPTIONAL { ?if a ?t } \
         FILTER ( !BOUND(?t) || ?t NOT IN ( \
           <https://ld.openhelvetia.swiss/schema/McpInterface>, \
           <https://ld.openhelvetia.swiss/schema/SparqlInterface>, \
           <https://ld.openhelvetia.swiss/schema/RestInterface>, \
           <https://ld.openhelvetia.swiss/schema/DownloadInterface> ) ) }",
    ),
    (
        "endpoint is not https",
        "SELECT ?if ?e WHERE { ?if <http://www.w3.org/ns/dcat#endpointURL> ?e . \
         FILTER ( !STRSTARTS(STR(?e), \"https://\") ) }",
    ),
    (
        "tier outside the declared set",
        "SELECT ?if ?t WHERE { ?if <https://ld.openhelvetia.swiss/schema/tier> ?t . \
         FILTER ( ?t NOT IN (\"base\", \"semantic\", \"generative\") ) }",
    ),
    (
        "probe kind/expect outside the declared sets",
        "SELECT ?if ?k ?e WHERE { ?if <https://ld.openhelvetia.swiss/schema/probe> ?p . \
         OPTIONAL { ?p <https://ld.openhelvetia.swiss/schema/probeKind> ?k } \
         OPTIONAL { ?p <https://ld.openhelvetia.swiss/schema/probeExpect> ?e } \
         FILTER ( !BOUND(?k) || !BOUND(?e) \
           || ?k NOT IN (\"sparql-ask\", \"http-get\", \"http-head\", \"mcp-initialize\", \"mcp-discover\") \
           || ?e NOT IN (\"ok\", \"boolean\", \"response\") ) }",
    ),
];

/// Builds the merged registry graph from all documents (blank nodes
/// prefixed per document) and runs fingerprint + invariants.
pub fn check(documents: &[(String, String)], loader: &VendoredLoader) -> Result<Stage4Report> {
    let mut combined = String::new();
    for (i, (_name, raw)) in documents.iter().enumerate() {
        let prefix = format!("d{i}");
        combined.push_str(&stage3::to_ntriples_prefixed(raw, loader, &prefix)?);
    }

    Ok(Stage4Report {
        fingerprint: fingerprint(&combined)?,
        violations: run_invariants(&combined)?,
    })
}

/// RDFC-1.0 canonicalization (SHA-256 flavour) of the merged graph,
/// hashed over the sorted canonical N-Quads. Deterministic across
/// builds AND across blank-node relabelings.
pub fn fingerprint(ntriples: &str) -> Result<String> {
    let mut dataset = Dataset::new();
    for quad in
        oxigraph::io::RdfParser::from_format(RdfFormat::NTriples).for_slice(ntriples.as_bytes())
    {
        let quad = quad.context("stage 4: parsing merged graph")?;
        dataset.insert(&quad);
    }
    dataset.canonicalize(CanonicalizationAlgorithm::Rdfc10 {
        hash_algorithm: CanonicalizationHashAlgorithm::Sha256,
    });

    let mut lines: Vec<String> = dataset.iter().map(|q| q.to_string()).collect();
    lines.sort();
    let serialized = lines.join("\n");

    let digest = Sha256::digest(serialized.as_bytes());
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// Runs every invariant; returns one message per violated invariant,
/// naming up to five offenders. Public for the conformance suite
/// (raw-graph battery, mirroring `stage3::validate`).
pub fn run_invariants(ntriples: &str) -> Result<Vec<String>> {
    let store = Store::new().context("stage 4: opening in-memory store")?;
    store
        .load_from_reader(RdfFormat::NTriples, ntriples.as_bytes())
        .context("stage 4: loading merged graph")?;

    let mut violations = Vec::new();
    for (name, query) in INVARIANTS {
        let results = SparqlEvaluator::new()
            .parse_query(*query)
            .with_context(|| format!("stage 4: parsing invariant «{name}»"))?
            .on_store(&store)
            .execute()
            .with_context(|| format!("stage 4: invariant query «{name}»"))?;
        if let QueryResults::Solutions(solutions) = results {
            let mut offenders = Vec::new();
            for solution in solutions {
                let solution = solution.context("stage 4: reading solution")?;
                let row: Vec<String> = solution
                    .iter()
                    .map(|(var, term)| format!("{var}={term}"))
                    .collect();
                offenders.push(row.join(" "));
            }
            if !offenders.is_empty() {
                let shown = offenders
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ");
                let more = offenders.len().saturating_sub(5);
                let suffix = if more > 0 {
                    format!(" (+{more} more)")
                } else {
                    String::new()
                };
                violations.push(format!("invariant «{name}»: {shown}{suffix}"));
            }
        }
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_blank_label_invariant() {
        let a = "_:x <https://p/> \"v\" .\n_:x <https://q/> _:y .\n";
        let b = "_:m <https://p/> \"v\" .\n_:m <https://q/> _:n .\n";
        assert_eq!(
            fingerprint(a).unwrap(),
            fingerprint(b).unwrap(),
            "RDFC must equate isomorphic graphs with different labels"
        );
    }

    #[test]
    fn fingerprint_changes_with_content() {
        let a = "_:x <https://p/> \"v\" .\n";
        let b = "_:x <https://p/> \"w\" .\n";
        assert_ne!(fingerprint(a).unwrap(), fingerprint(b).unwrap());
    }
}
