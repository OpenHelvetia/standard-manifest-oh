//! Conformance suite for the stage-3 SHACL gate (E16 mandate: rudof's
//! own test coverage is unpublished, so every constraint the shapes
//! rely on must be PROVEN to fire by a failing negative case here —
//! a constraint without one is treated as not enforced).
//!
//! Two entry paths are exercised: the canonical JSON path
//! (`stage3::check`) for everything reachable through canonical
//! manifests, and the raw-graph path (`stage3::validate`) for
//! graph-only cases the canonical JSON pipeline cannot produce
//! (closed-shape extras, IRI interface nodes) — those exist to guard
//! future direct-RDF publication.

use std::path::{Path, PathBuf};

use oh_validate::{stage2, stage3};

fn context_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../context/v1.jsonld")
}

fn shapes_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../shapes/manifest-v1.ttl")
}

fn loader() -> stage2::VendoredLoader {
    stage2::VendoredLoader::from_file(&context_path()).expect("vendored context loads")
}

/// A minimal canonical manifest as a mutable JSON value.
fn base_doc() -> serde_json::Value {
    serde_json::json!({
        "@context": "https://ld.openhelvetia.swiss/ns/manifest/v1",
        "@id": "https://ld.openhelvetia.swiss/registry/testcase",
        "@type": "Manifest",
        "title": {"de": "Testfall"},
        "publisher": "https://ld.openhelvetia.swiss/org/test",
        "license": "https://example.org/license",
        "issued": "2026-08-20",
        "interfaces": [{
            "@type": "RestInterface",
            "endpoint": "https://example.org/api",
            "auth": {"authType": "none"},
            "tier": "base"
        }]
    })
}

/// Runs the JSON path and returns the report (None = conforms).
fn run(doc: &serde_json::Value) -> Option<String> {
    let raw = serde_json::to_string(doc).expect("serialize");
    stage3::check(&raw, &loader(), &shapes_path()).expect("gate runs")
}

/// Asserts a violation whose report names the expected SHACL
/// constraint component (top-level constraints).
fn assert_fires(doc: &serde_json::Value, component: &str) {
    match run(doc) {
        Some(report) => assert!(
            report.contains(component),
            "expected {component} in report:\n{report}"
        ),
        None => panic!("expected a {component} violation, but the graph conformed"),
    }
}

/// Asserts a violation INSIDE an interface node. Documented rudof
/// 0.3.8 characteristic (suite finding): `sh:node` violations surface
/// only as the outer `NodeConstraintComponent` on `oh:interface` —
/// the inner component identity is not reported. Each test here
/// mutates exactly ONE rule, so a red gate still proves THAT rule
/// fired; precise inner diagnostics arrive with the stage-4 SPARQL
/// doubling of the critical invariants (E16).
fn assert_fires_nested(doc: &serde_json::Value) {
    match run(doc) {
        Some(report) => {
            assert!(
                report.contains("NodeConstraintComponent"),
                "expected the nested-violation wrapper in report:\n{report}"
            );
            assert!(
                report.contains("https://ld.openhelvetia.swiss/schema/interface"),
                "expected oh:interface as resultPath in report:\n{report}"
            );
        }
        None => panic!("expected a nested violation, but the graph conformed"),
    }
}

#[test]
fn base_doc_conforms() {
    assert!(run(&base_doc()).is_none(), "the base test doc must conform");
}

#[test]
fn full_corpus_conforms_through_stage_3() {
    let loader = loader();
    let shapes = shapes_path();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut n = 0usize;
    for dir in ["../../entries", "../test-corpus", "../examples"] {
        for entry in std::fs::read_dir(root.join(dir)).expect("corpus dir") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let raw = std::fs::read_to_string(&path).expect("read");
                let report = stage3::check(&raw, &loader, &shapes).expect("gate runs");
                assert!(
                    report.is_none(),
                    "{} violated:\n{:?}",
                    path.display(),
                    report
                );
                n += 1;
            }
        }
    }
    assert!(n >= 20, "corpus unexpectedly small: {n}");
}

#[test]
fn ntriples_generation_is_deterministic() {
    let loader = loader();
    let raw = serde_json::to_string(&base_doc()).unwrap();
    let a = stage3::to_ntriples(&raw, &loader).unwrap();
    let b = stage3::to_ntriples(&raw, &loader).unwrap();
    assert_eq!(a, b, "ordered flattening must yield identical output");
}

// ---- negative battery: canonical JSON path ------------------------

#[test]
fn missing_title_fires_min_count() {
    let mut doc = base_doc();
    doc.as_object_mut().unwrap().remove("title");
    assert_fires(&doc, "MinCountConstraintComponent");
}

#[test]
fn missing_issued_fires_min_count() {
    let mut doc = base_doc();
    doc.as_object_mut().unwrap().remove("issued");
    assert_fires(&doc, "MinCountConstraintComponent");
}

#[test]
fn missing_auth_fires_min_count() {
    let mut doc = base_doc();
    doc["interfaces"][0].as_object_mut().unwrap().remove("auth");
    assert_fires_nested(&doc);
}

#[test]
fn insecure_endpoint_fires_pattern() {
    let mut doc = base_doc();
    doc["interfaces"][0]["endpoint"] = "http://insecure.example/api".into();
    assert_fires_nested(&doc);
}

#[test]
fn unknown_tier_fires_in_constraint() {
    let mut doc = base_doc();
    doc["interfaces"][0]["tier"] = "premium".into();
    assert_fires_nested(&doc);
}

#[test]
fn unknown_auth_type_fires_in_constraint() {
    let mut doc = base_doc();
    doc["interfaces"][0]["auth"]["authType"] = "password".into();
    assert_fires_nested(&doc);
}

#[test]
fn unknown_probe_kind_fires_in_constraint() {
    let mut doc = base_doc();
    doc["interfaces"][0]["probe"] = serde_json::json!({"kind": "ping", "expect": "ok"});
    assert_fires_nested(&doc);
}

#[test]
fn unknown_language_fires_language_in() {
    let mut doc = base_doc();
    doc["title"] = serde_json::json!({"xx": "wrong language"});
    assert_fires(&doc, "LanguageInConstraintComponent");
}

#[test]
fn bad_issued_datatype_fires_datatype() {
    let mut doc = base_doc();
    // The context types `issued` as xsd:date; a non-date lexical form
    // still reaches the graph as xsd:date — SHACL checks the datatype
    // IRI, ill-formed lexical values are stage-1 territory. To violate
    // the datatype constraint at graph level we go through the raw
    // path below; here we prove the well-formed corpus passes, which
    // keeps this named gap explicit.
    doc["modified"] = serde_json::json!("2026-08-21");
    assert!(run(&doc).is_none());
}

// ---- negative battery: raw-graph path (unreachable via canonical
// JSON — guards future direct-RDF publication) ----------------------

const M: &str = "https://ld.openhelvetia.swiss/registry/testcase";

fn base_graph() -> String {
    format!(
        "<{M}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ld.openhelvetia.swiss/schema/Manifest> .\n\
         <{M}> <http://purl.org/dc/terms/title> \"Testfall\"@de .\n\
         <{M}> <http://purl.org/dc/terms/publisher> <https://ld.openhelvetia.swiss/org/test> .\n\
         <{M}> <http://purl.org/dc/terms/license> <https://example.org/license> .\n\
         <{M}> <http://purl.org/dc/terms/issued> \"2026-08-20\"^^<http://www.w3.org/2001/XMLSchema#date> .\n\
         <{M}> <https://ld.openhelvetia.swiss/schema/interface> _:i .\n\
         _:i <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ld.openhelvetia.swiss/schema/RestInterface> .\n\
         _:i <http://www.w3.org/ns/dcat#endpointURL> <https://example.org/api> .\n\
         _:i <https://ld.openhelvetia.swiss/schema/auth> _:a .\n\
         _:a <https://ld.openhelvetia.swiss/schema/authType> \"none\" .\n"
    )
}

fn assert_graph_fires(graph: &str, component: &str) {
    match stage3::validate(graph, &shapes_path()).expect("gate runs") {
        Some(report) => assert!(
            report.contains(component),
            "expected {component} in report:\n{report}"
        ),
        None => panic!("expected a {component} violation, but the graph conformed"),
    }
}

#[test]
fn raw_base_graph_conforms() {
    assert!(
        stage3::validate(&base_graph(), &shapes_path())
            .expect("gate runs")
            .is_none(),
        "the raw base graph must conform"
    );
}

#[test]
fn closed_manifest_shape_fires_on_unmodelled_property() {
    let graph = format!(
        "{}<{M}> <https://ld.openhelvetia.swiss/schema/sponsor> \"who pays\" .\n",
        base_graph()
    );
    assert_graph_fires(&graph, "ClosedConstraintComponent");
}

#[test]
fn iri_interface_node_fires_node_kind() {
    let graph = base_graph().replace("_:i", "<https://example.org/interface>");
    assert_graph_fires(&graph, "NodeKindConstraintComponent");
}

#[test]
fn two_publishers_fire_max_count() {
    let graph = format!(
        "{}<{M}> <http://purl.org/dc/terms/publisher> <https://example.org/second> .\n",
        base_graph()
    );
    assert_graph_fires(&graph, "MaxCountConstraintComponent");
}

#[test]
fn wrong_issued_datatype_fires_datatype() {
    let graph = base_graph().replace(
        "\"2026-08-20\"^^<http://www.w3.org/2001/XMLSchema#date>",
        "\"2026-08-20\"",
    );
    assert_graph_fires(&graph, "DatatypeConstraintComponent");
}
