//! Conformance suite for the stage-4 whole-graph gate (E16 mandate:
//! every invariant must be PROVEN to fire by a failing negative case
//! — an invariant without one is treated as not enforced).
//!
//! Two entry paths: the corpus path (`stage4::check`) proving the
//! merge + fingerprint machinery on real documents, and the raw-graph
//! path (`stage4::run_invariants`) where each negative case violates
//! exactly ONE invariant and the violation message must NAME it —
//! this is the diagnostic precision the SPARQL doubling exists for
//! (rudof reports nested violations only as the outer wrapper).

use std::path::{Path, PathBuf};

use oh_validate::{stage2, stage3, stage4};

fn context_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../context/v1.jsonld")
}

fn loader() -> stage2::VendoredLoader {
    stage2::VendoredLoader::from_file(&context_path()).expect("vendored context loads")
}

/// All corpus documents (entries + examples), name-sorted.
fn corpus() -> Vec<(String, String)> {
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in ["../../entries", "../test-corpus", "../examples"] {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        for entry in std::fs::read_dir(dir).expect("corpus dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let raw = std::fs::read_to_string(&p).expect("read corpus file");
            (p.display().to_string(), raw)
        })
        .collect()
}

// --- corpus path -----------------------------------------------------

#[test]
fn corpus_passes_stage4() {
    let report = stage4::check(&corpus(), &loader()).expect("gate runs");
    assert!(
        report.violations.is_empty(),
        "corpus must satisfy all invariants: {:?}",
        report.violations
    );
    assert_eq!(report.fingerprint.len(), 64, "SHA-256 hex fingerprint");
    assert!(report.fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn fingerprint_is_deterministic_across_runs() {
    let docs = corpus();
    let a = stage4::check(&docs, &loader()).expect("run 1").fingerprint;
    let b = stage4::check(&docs, &loader()).expect("run 2").fingerprint;
    assert_eq!(a, b);
}

/// File enumeration order must not leak into the fingerprint: merge
/// order changes the per-document blank prefixes, RDFC-1.0 must
/// erase exactly that difference.
#[test]
fn fingerprint_is_merge_order_invariant() {
    let docs = corpus();
    let mut reversed = docs.clone();
    reversed.reverse();
    let a = stage4::check(&docs, &loader())
        .expect("forward")
        .fingerprint;
    let b = stage4::check(&reversed, &loader())
        .expect("reversed")
        .fingerprint;
    assert_eq!(a, b);
}

/// The collision hazard the prefixing exists for: the same document
/// serialized twice must contribute DISJOINT blank nodes — without
/// prefixes, generator labels (`_:b0`, …) would merge distinct nodes.
#[test]
fn per_document_prefixes_keep_blank_nodes_disjoint() {
    let (_, raw) = &corpus()[0];
    let single = stage3::to_ntriples_prefixed(raw, &loader(), "d0").expect("serialize");
    let merged = format!(
        "{single}{}",
        stage3::to_ntriples_prefixed(raw, &loader(), "d1").expect("serialize")
    );
    let count = |nt: &str| {
        let mut ds = oxigraph::model::Dataset::new();
        for q in oxigraph::io::RdfParser::from_format(oxigraph::io::RdfFormat::NTriples)
            .for_slice(nt.as_bytes())
        {
            ds.insert(&q.expect("parse"));
        }
        ds.len()
    };
    let single_n = count(&single);
    // Blank-node triples must double; only the shared IRI-subject
    // triples (the manifest's own statements) may deduplicate.
    assert!(
        count(&merged) > single_n,
        "merged graph must be strictly larger — blank nodes stayed disjoint"
    );
}

// --- raw-graph invariant battery -------------------------------------

const RDF_TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";

/// A minimal valid registry graph; each negative case below deviates
/// in exactly one invariant.
fn base_graph() -> String {
    let m = "<https://ld.openhelvetia.swiss/registry/t>";
    format!(
        "{m} {RDF_TYPE} <https://ld.openhelvetia.swiss/schema/Manifest> .\n\
         {m} <http://purl.org/dc/terms/title> \"Testfall\"@de .\n\
         {m} <https://ld.openhelvetia.swiss/schema/interface> _:if .\n\
         _:if {RDF_TYPE} <https://ld.openhelvetia.swiss/schema/RestInterface> .\n\
         _:if <http://www.w3.org/ns/dcat#endpointURL> <https://example.org/api> .\n\
         _:if <https://ld.openhelvetia.swiss/schema/auth> _:a .\n\
         _:a <https://ld.openhelvetia.swiss/schema/authType> \"none\" .\n"
    )
}

fn assert_invariant_fires(graph: &str, invariant: &str) {
    let violations = stage4::run_invariants(graph).expect("invariants run");
    assert!(
        violations.iter().any(|v| v.contains(invariant)),
        "expected invariant «{invariant}» to fire, got: {violations:?}"
    );
    assert_eq!(
        violations.len(),
        1,
        "exactly one invariant may fire per case: {violations:?}"
    );
}

#[test]
fn base_graph_is_clean() {
    let violations = stage4::run_invariants(&base_graph()).expect("invariants run");
    assert!(violations.is_empty(), "positive control: {violations:?}");
}

#[test]
fn fires_on_manifest_without_interface() {
    let graph = "<https://ld.openhelvetia.swiss/registry/t> \
         <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
         <https://ld.openhelvetia.swiss/schema/Manifest> .\n\
         <https://ld.openhelvetia.swiss/registry/t> \
         <http://purl.org/dc/terms/title> \"T\"@de .\n";
    assert_invariant_fires(graph, "manifest without interface");
}

#[test]
fn fires_on_manifest_without_title() {
    let graph = base_graph().replace(
        "<https://ld.openhelvetia.swiss/registry/t> <http://purl.org/dc/terms/title> \"Testfall\"@de .\n",
        "",
    );
    assert_invariant_fires(&graph, "manifest without title");
}

#[test]
fn fires_on_interface_without_auth() {
    let graph = base_graph().replace(
        "_:if <https://ld.openhelvetia.swiss/schema/auth> _:a .\n\
         _:a <https://ld.openhelvetia.swiss/schema/authType> \"none\" .\n",
        "",
    );
    assert_invariant_fires(&graph, "interface without auth declaration");
}

#[test]
fn fires_on_auth_without_auth_type() {
    let graph = base_graph().replace(
        "_:a <https://ld.openhelvetia.swiss/schema/authType> \"none\" .\n",
        "",
    );
    assert_invariant_fires(&graph, "auth without valid authType");
}

#[test]
fn fires_on_invalid_auth_type() {
    let graph = base_graph().replace("\"none\"", "\"password\"");
    assert_invariant_fires(&graph, "auth without valid authType");
}

#[test]
fn fires_on_iri_interface_node() {
    // The frame invariant: interface nodes must be blank nodes.
    let graph = base_graph().replace("_:if", "<https://example.org/if>");
    assert_invariant_fires(&graph, "interface node is an IRI");
}

#[test]
fn fires_on_undeclared_interface_type() {
    let graph = base_graph().replace(
        "<https://ld.openhelvetia.swiss/schema/RestInterface>",
        "<https://ld.openhelvetia.swiss/schema/SoapInterface>",
    );
    assert_invariant_fires(&graph, "interface type outside the declared classes");
}

#[test]
fn fires_on_untyped_interface() {
    let graph = base_graph().replace(
        "_:if <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
         <https://ld.openhelvetia.swiss/schema/RestInterface> .\n",
        "",
    );
    assert_invariant_fires(&graph, "interface type outside the declared classes");
}

#[test]
fn fires_on_non_https_endpoint() {
    let graph = base_graph().replace("<https://example.org/api>", "<http://example.org/api>");
    assert_invariant_fires(&graph, "endpoint is not https");
}

#[test]
fn fires_on_undeclared_tier() {
    let graph = format!(
        "{}_:if <https://ld.openhelvetia.swiss/schema/tier> \"premium\" .\n",
        base_graph()
    );
    assert_invariant_fires(&graph, "tier outside the declared set");
}

#[test]
fn fires_on_invalid_probe_kind() {
    let graph = format!(
        "{}_:if <https://ld.openhelvetia.swiss/schema/probe> _:p .\n\
         _:p <https://ld.openhelvetia.swiss/schema/probeKind> \"ping\" .\n\
         _:p <https://ld.openhelvetia.swiss/schema/probeExpect> \"ok\" .\n",
        base_graph()
    );
    assert_invariant_fires(&graph, "probe kind/expect outside the declared sets");
}

#[test]
fn accepts_the_stateless_era_probe_kind() {
    // The mirror of the case above, and the reason it is worth its own
    // test: an invariant that rejects everything is as wrong as one
    // that rejects nothing, and a new vocabulary value is exactly the
    // kind of change that gets added to a schema and forgotten in the
    // SPARQL. `mcp-discover` joined the closed set at the 2026-08-23
    // amendment (DIRECTORY.md §3).
    let graph = format!(
        "{}_:if <https://ld.openhelvetia.swiss/schema/probe> _:p .\n\
         _:p <https://ld.openhelvetia.swiss/schema/probeKind> \"mcp-discover\" .\n\
         _:p <https://ld.openhelvetia.swiss/schema/probeExpect> \"ok\" .\n",
        base_graph()
    );
    let violations = stage4::run_invariants(&graph).expect("invariants run");
    assert!(
        violations.is_empty(),
        "a declared probe kind must pass the closed-set invariant: {violations:?}"
    );
}

#[test]
fn fires_on_probe_without_expect() {
    let graph = format!(
        "{}_:if <https://ld.openhelvetia.swiss/schema/probe> _:p .\n\
         _:p <https://ld.openhelvetia.swiss/schema/probeKind> \"http-get\" .\n",
        base_graph()
    );
    assert_invariant_fires(&graph, "probe kind/expect outside the declared sets");
}
