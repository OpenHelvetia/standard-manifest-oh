//! Integration tests for the stage-2 round-trip gate (E16 Ziff. 5c).
//! Positive: the whole real corpus is in canonical form. Negative:
//! dropped terms are NAMED, the frame invariant refuses identified
//! interface nodes, and the offline loader refuses foreign IRIs.

use std::path::{Path, PathBuf};

fn context_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../context/v1.jsonld")
}

fn loader() -> oh_validate::stage2::VendoredLoader {
    oh_validate::stage2::VendoredLoader::from_file(&context_path()).expect("vendored context loads")
}

fn corpus_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in ["../../entries", "../test-corpus", "../examples"] {
        for entry in std::fs::read_dir(root.join(dir)).expect("corpus dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                files.push(path);
            }
        }
    }
    assert!(
        files.len() >= 20,
        "corpus unexpectedly small: {}",
        files.len()
    );
    files
}

#[test]
fn the_whole_corpus_is_in_canonical_form() {
    let loader = loader();
    for file in corpus_files() {
        let raw = std::fs::read_to_string(&file).expect("read corpus file");
        let findings = oh_validate::stage2::check(&raw, &loader).expect("gate runs");
        assert!(
            findings.is_empty(),
            "{} drifted: {:?}",
            file.display(),
            findings
        );
    }
}

#[test]
fn dropped_terms_are_named() {
    let loader = loader();
    // `sponsor` is not a context term and there is no `@vocab`: the
    // expansion silently drops it — the gate must say so BY NAME.
    let raw = r#"{
        "@context": "https://ld.openhelvetia.swiss/ns/manifest/v1",
        "@id": "https://ld.openhelvetia.swiss/registry/x",
        "@type": "Manifest",
        "title": {"de": "X"},
        "sponsor": "who pays",
        "issued": "2026-08-15"
    }"#;
    let findings = oh_validate::stage2::check(raw, &loader).expect("gate runs");
    assert_eq!(findings.len(), 1, "exactly one drift finding expected");
    let msg = findings[0].to_string();
    assert!(msg.contains("dropped terms"), "message: {msg}");
    assert!(
        msg.contains("sponsor"),
        "the dropped term must be named: {msg}"
    );
}

#[test]
fn identified_interface_nodes_are_refused() {
    let loader = loader();
    let raw = r#"{
        "@context": "https://ld.openhelvetia.swiss/ns/manifest/v1",
        "@id": "https://ld.openhelvetia.swiss/registry/x",
        "@type": "Manifest",
        "interfaces": [{"@id": "https://x/if", "@type": "RestInterface"}]
    }"#;
    let findings = oh_validate::stage2::check(raw, &loader).expect("gate runs");
    assert_eq!(findings.len(), 1);
    assert!(
        findings[0].to_string().contains("framing"),
        "must point at the framing gap: {}",
        findings[0]
    );
}

#[test]
fn foreign_context_iris_are_refused_offline() {
    let loader = loader();
    let raw = r#"{
        "@context": "https://evil.example/context",
        "@id": "https://ld.openhelvetia.swiss/registry/x",
        "@type": "Manifest"
    }"#;
    let err = oh_validate::stage2::check(raw, &loader).expect_err("must refuse");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("evil.example"),
        "the refused IRI must be visible: {msg}"
    );
}
