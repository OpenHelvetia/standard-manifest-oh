//! Test support for the agent-card signature-envelope conformance suite.
//!
//! The crate carries no production code: the specification artefacts are the
//! schema (`../agent-card.schema.json`) and the example vectors
//! (`../examples/`). This library only loads those files from their canonical
//! locations, so the tests in `tests/` stay free of path plumbing. Everything
//! is read from disk relative to `CARGO_MANIFEST_DIR` — no network, ever.

use std::fs;
use std::path::{Path, PathBuf};

/// Root of the signature-package spec (`registry/standard/card/`),
/// i.e. the parent directory of this conformance crate.
pub fn spec_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("conformance crate must live inside the spec directory")
        .to_path_buf()
}

/// Load and parse a JSON file relative to the spec root.
///
/// Panics with a readable message on I/O or parse errors — in a test-support
/// crate a panic is the correct failure mode, it surfaces directly as a
/// failing test.
pub fn load_json(relative: &str) -> serde_json::Value {
    let path = spec_dir().join(relative);
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("invalid JSON in {}: {err}", path.display()))
}

/// The envelope schema, `agent-card.schema.json`.
pub fn schema() -> serde_json::Value {
    load_json("agent-card.schema.json")
}

/// An example vector from `examples/` by file stem,
/// e.g. `example("card-minimal")`.
pub fn example(stem: &str) -> serde_json::Value {
    load_json(&format!("examples/{stem}.json"))
}
