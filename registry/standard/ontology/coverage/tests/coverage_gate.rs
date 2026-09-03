//! Integration tests for the ontology coverage gate (masterplan L0.5).
//!
//! The first two tests run against the REAL sources (context, schemas,
//! oh.ttl, oh.md) — they are the CI gate itself. The fixture tests prove
//! the gate actually fires: a fixture Turtle file with one declaration
//! removed must fail coverage, and a doctored oh.md must trip the
//! doc-drift check.

use std::path::{Path, PathBuf};

use oh_coverage::{Paths, doc, load, model, report, sources, ttl};

/// `registry/standard/`, resolved from this crate's location
/// (`registry/standard/ontology/coverage`).
fn standard_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("standard root exists")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn full_coverage_passes_on_the_real_sources() {
    let loaded = load(&Paths::under(&standard_root())).expect("sources load");
    let coverage = report::check(&loaded.model, &loaded.requirements);
    assert!(
        coverage.failures.is_empty(),
        "coverage gate must be green on the committed sources:\n{}",
        report::render(&coverage)
    );
    assert!(
        coverage.warnings.is_empty(),
        "no declared term may be orphaned on the committed sources:\n{}",
        report::render(&coverage)
    );
    // Sanity floor: the emitted set must stay substantial — an extraction
    // regression that silently emits nothing would otherwise pass.
    assert!(
        loaded.requirements.terms.len() >= 40,
        "expected at least 40 emitted terms, got {}",
        loaded.requirements.terms.len()
    );
    assert!(
        loaded.requirements.values.len() >= 15,
        "expected at least 15 emitted values, got {}",
        loaded.requirements.values.len()
    );
}

#[test]
fn committed_doc_matches_regeneration() {
    let paths = Paths::under(&standard_root());
    let loaded = load(&paths).expect("sources load");
    let committed = std::fs::read_to_string(&paths.ontology_doc).expect("oh.md exists");
    assert_eq!(
        committed,
        doc::generate(&loaded.model),
        "oh.md drifted from oh.ttl — regenerate with --write-doc"
    );
}

#[test]
fn gate_fires_when_a_declaration_is_removed() {
    // Fixture: oh.ttl with the whole oh:probeTarget declaration removed.
    let paths = Paths::under(&standard_root());
    let loaded = load(&paths).expect("sources load");
    let doctored =
        std::fs::read_to_string(fixture("oh-missing-probe-target.ttl")).expect("fixture exists");
    let doctored_model =
        model::build(&ttl::parse(&doctored).expect("fixture parses")).expect("fixture models");
    let coverage = report::check(&doctored_model, &loaded.requirements);
    assert!(
        coverage
            .failures
            .iter()
            .any(|f| f.contains("oh:probeTarget")),
        "removing oh:probeTarget must fail the gate; failures: {:?}",
        coverage.failures
    );
}

#[test]
fn doc_drift_gate_fires_on_a_doctored_doc() {
    let paths = Paths::under(&standard_root());
    let loaded = load(&paths).expect("sources load");
    let generated = doc::generate(&loaded.model);
    let doctored = std::fs::read_to_string(fixture("oh-doctored.md")).expect("fixture exists");
    assert_ne!(
        generated, doctored,
        "the doctored doc fixture must differ from the regeneration"
    );
}

#[test]
fn orphaned_declaration_is_listed_as_a_warning_only() {
    // A declared class with no link whatsoever into the emitted core.
    let src = "@prefix oh: <https://ld.openhelvetia.swiss/schema/> .\n\
               @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
               <https://ld.openhelvetia.swiss/schema/> a owl:Ontology .\n\
               oh:Manifest a owl:Class .\n\
               oh:Widget a owl:Class .\n";
    let m = model::build(&ttl::parse(src).expect("parse")).expect("model");
    let mut reqs = sources::Requirements::default();
    reqs.add_term("https://ld.openhelvetia.swiss/schema/Manifest", "test");
    let coverage = report::check(&m, &reqs);
    assert!(coverage.ok(), "orphans must not fail the gate");
    assert!(
        coverage.warnings.iter().any(|w| w.contains("oh:Widget")),
        "the orphan must be listed: {:?}",
        coverage.warnings
    );
}
