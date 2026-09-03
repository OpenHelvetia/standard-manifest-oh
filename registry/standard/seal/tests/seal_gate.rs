//! Conformance suite for the seal gate (L0.6). Same mandate as every
//! gate in this repo: each rule is PROVEN to fire by a failing
//! negative case — a rule without one is treated as not enforced.

use std::fs;
use std::path::{Path, PathBuf};

fn real_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Builds a minimal fake standard tree in the cargo tmpdir.
/// One draft artifact, one published artifact.
fn fixture(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean fixture");
    }
    fs::create_dir_all(root.join("context")).expect("mkdir");
    fs::create_dir_all(root.join("schema")).expect("mkdir");
    fs::write(root.join("context/v1.jsonld"), b"{\"@context\":{}}\n").expect("write");
    fs::write(root.join("schema/manifest.schema.json"), b"{}\n").expect("write");
    let register = serde_json::json!({
        "$comment": "test register",
        "locations": {
            "dirs": [
                {"path": ".", "extensions": ["json"]},
                {"path": "context", "extensions": ["jsonld"]},
                {"path": "schema", "extensions": ["json"]}
            ],
            "files": []
        },
        "artifacts": [
            {
                "path": "context/v1.jsonld",
                "iri": null,
                "status": "draft",
                "sha256": oh_seal::sha256_hex(b"{\"@context\":{}}\n"),
                "note": "draft artifact"
            },
            {
                "path": "schema/manifest.schema.json",
                "iri": null,
                "status": "published",
                "sha256": oh_seal::sha256_hex(b"{}\n"),
                "published_on": "2026-08-20",
                "note": "published artifact"
            }
        ]
    });
    fs::write(
        root.join("artifacts.json"),
        serde_json::to_string_pretty(&register).expect("serialize") + "\n",
    )
    .expect("write register");
    root
}

// --- the real tree ---------------------------------------------------

#[test]
fn real_register_is_green() {
    let findings = oh_seal::check(&real_root()).expect("gate runs");
    assert!(
        findings.is_empty(),
        "real tree must be sealed: {findings:?}"
    );
}

// --- rule battery -----------------------------------------------------

#[test]
fn fixture_is_green() {
    let root = fixture("green");
    let findings = oh_seal::check(&root).expect("gate runs");
    assert!(findings.is_empty(), "positive control: {findings:?}");
}

#[test]
fn fires_on_draft_drift_and_names_the_fix() {
    let root = fixture("draft-drift");
    fs::write(
        root.join("context/v1.jsonld"),
        b"{\"@context\":{\"x\":1}}\n",
    )
    .expect("mutate");
    let findings = oh_seal::check(&root).expect("gate runs");
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].contains("DRAFT DRIFT") && findings[0].contains("--update"));
}

#[test]
fn fires_on_published_modification_as_hard_stop() {
    let root = fixture("published-drift");
    fs::write(
        root.join("schema/manifest.schema.json"),
        b"{\"changed\":true}\n",
    )
    .expect("mutate");
    let findings = oh_seal::check(&root).expect("gate runs");
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].contains("PUBLISHED ARTIFACT MODIFIED"));
    assert!(findings[0].contains("NEW version"));
}

#[test]
fn fires_on_unregistered_artifact_file() {
    let root = fixture("closed-world");
    fs::write(root.join("context/v2.jsonld"), b"{}\n").expect("new file");
    let findings = oh_seal::check(&root).expect("gate runs");
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].contains("NOT in the register"));
}

#[test]
fn fires_on_registered_but_missing_file() {
    let root = fixture("missing");
    fs::remove_file(root.join("context/v1.jsonld")).expect("remove");
    let findings = oh_seal::check(&root).expect("gate runs");
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].contains("registered but missing"));
}

// --- state transitions ------------------------------------------------

#[test]
fn update_rerecords_draft_only() {
    let root = fixture("update");
    fs::write(root.join("context/v1.jsonld"), b"new\n").expect("mutate");
    oh_seal::update(&root, "context/v1.jsonld").expect("update runs");
    assert!(oh_seal::check(&root).expect("gate runs").is_empty());
    // Published artifacts refuse the same operation.
    fs::write(root.join("schema/manifest.schema.json"), b"new\n").expect("mutate");
    let err = oh_seal::update(&root, "schema/manifest.schema.json").unwrap_err();
    assert!(err.to_string().contains("byte-frozen"), "{err}");
}

#[test]
fn publish_seals_current_recorded_bytes_only() {
    let root = fixture("publish");
    // Stale hash refused.
    fs::write(root.join("context/v1.jsonld"), b"unrecorded\n").expect("mutate");
    let err = oh_seal::publish(&root, "context/v1.jsonld", "2026-08-20").unwrap_err();
    assert!(err.to_string().contains("stale"), "{err}");
    // Recorded bytes publish cleanly; second publish refused.
    oh_seal::update(&root, "context/v1.jsonld").expect("update");
    oh_seal::publish(&root, "context/v1.jsonld", "2026-08-20").expect("publish");
    let register = oh_seal::load_register(&root).expect("load");
    let row = &register.artifacts[0];
    assert_eq!(row.published_on.as_deref(), Some("2026-08-20"));
    let err = oh_seal::publish(&root, "context/v1.jsonld", "2026-08-21").unwrap_err();
    assert!(err.to_string().contains("already published"), "{err}");
    // Bad date format refused.
    let err = oh_seal::publish(&root, "schema/manifest.schema.json", "20.08.2026").unwrap_err();
    assert!(err.to_string().contains("YYYY-MM-DD"), "{err}");
}

#[test]
fn superseded_artifacts_are_frozen_history() {
    let root = fixture("superseded");
    // Reclassify the published row as superseded and prove the
    // same freeze semantics: modification fires, update refused.
    let mut register: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("artifacts.json")).expect("read"))
            .expect("parse");
    register["artifacts"][1]["status"] = serde_json::json!("superseded");
    register["artifacts"][1]
        .as_object_mut()
        .unwrap()
        .remove("published_on");
    std::fs::write(
        root.join("artifacts.json"),
        serde_json::to_string_pretty(&register).unwrap() + "\n",
    )
    .expect("write");

    assert!(
        oh_seal::check(&root).expect("gate runs").is_empty(),
        "superseded needs no published_on"
    );
    fs::write(root.join("schema/manifest.schema.json"), b"tampered\n").expect("mutate");
    let findings = oh_seal::check(&root).expect("gate runs");
    assert_eq!(findings.len(), 1);
    assert!(findings[0].contains("SUPERSEDED ARTIFACT MODIFIED"));
    let err = oh_seal::update(&root, "schema/manifest.schema.json").unwrap_err();
    assert!(err.to_string().contains("byte-frozen"), "{err}");
}

#[test]
fn the_root_is_closed_world_too() {
    // Reflexionsschleife AM, the material finding: an unregistered file
    // at the register's own root must fire — that is where a young
    // standard's artifacts are born. The register file itself is the
    // one root file that never needs a row.
    let root = fixture("root-closed");
    fs::write(root.join("planted.json"), b"{}\n").expect("plant");
    let findings = oh_seal::check(&root).expect("gate runs");
    assert!(
        findings
            .iter()
            .any(|f| f.contains("planted.json") && f.contains("NOT in the register")),
        "an unregistered root artifact must fire: {findings:?}"
    );
    assert!(
        !findings
            .iter()
            .any(|f| f.contains("artifacts.json: artifact file")),
        "the register itself never needs a row: {findings:?}"
    );
}

#[test]
fn a_register_without_a_root_rule_is_itself_a_finding() {
    let root = fixture("no-root-rule");
    let mut register: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("artifacts.json")).expect("read"))
            .expect("parse");
    register["locations"]["dirs"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["path"] != ".");
    std::fs::write(
        root.join("artifacts.json"),
        serde_json::to_string_pretty(&register).unwrap() + "\n",
    )
    .expect("write");
    let findings = oh_seal::check(&root).expect("gate runs");
    assert!(
        findings.iter().any(|f| f.contains("no `.` rule")),
        "a register that leaves its own root open must be a finding: {findings:?}"
    );
}
