//! Ontology coverage gate for the `oh:` platform ontology (masterplan
//! L0.5; E16 no. 5e: "Ontologie-Deckung als CI-Test").
//!
//! The library extracts (a) the terms DECLARED by `ontology/oh.ttl` and
//! (b) the terms EMITTED by the platform's published artefacts — the
//! JSON-LD context and the three JSON schemas — and checks that every
//! emitted term is covered. It also regenerates `oh.md` from the Turtle
//! master so the binary can fail on documentation drift.
//!
//! See `src/ttl.rs` for the documented Turtle authoring subset and
//! `src/sources.rs` for the emission tables and exclusions.

pub mod doc;
pub mod model;
pub mod report;
pub mod sources;
pub mod ttl;

use std::path::{Path, PathBuf};

/// The source files, resolved relative to `registry/standard/`.
#[derive(Debug, Clone)]
pub struct Paths {
    pub ontology_ttl: PathBuf,
    pub ontology_doc: PathBuf,
    pub context: PathBuf,
    pub manifest_schema: PathBuf,
    pub directory_schema: PathBuf,
    pub card_schema: PathBuf,
}

impl Paths {
    /// Resolve the standard layout below the `registry/standard/` root.
    pub fn under(standard_root: &Path) -> Self {
        Paths {
            ontology_ttl: standard_root.join("ontology/oh.ttl"),
            ontology_doc: standard_root.join("ontology/oh.md"),
            context: standard_root.join("context/v1.jsonld"),
            manifest_schema: standard_root.join("schema/manifest.schema.json"),
            directory_schema: standard_root.join("directory.schema.json"),
            card_schema: standard_root.join("card/agent-card.schema.json"),
        }
    }
}

/// Model + requirements loaded from disk, ready for checking.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub model: model::Model,
    pub requirements: sources::Requirements,
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    serde_json::from_str(&read(path)?)
        .map_err(|e| format!("invalid JSON in {}: {e}", path.display()))
}

/// Load and parse all sources.
pub fn load(paths: &Paths) -> Result<Loaded, String> {
    let ttl_source = read(&paths.ontology_ttl)?;
    let document =
        ttl::parse(&ttl_source).map_err(|e| format!("{}: {e}", paths.ontology_ttl.display()))?;
    let model = model::build(&document)?;
    let requirements = sources::from_sources(
        &read_json(&paths.context)?,
        &read_json(&paths.manifest_schema)?,
        &read_json(&paths.directory_schema)?,
        &read_json(&paths.card_schema)?,
    );
    Ok(Loaded {
        model,
        requirements,
    })
}
