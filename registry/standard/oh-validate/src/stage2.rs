//! Stage 2 of the validation chain (E16 Ziff. 5c): the JSON-LD
//! round-trip gate. `compact(expand(doc))` against the vendored,
//! versioned context must reproduce the document byte-for-byte in
//! structure — this is what enforces the ONE canonical manifest form.
//! On failure the gate names the dropped or altered terms instead of
//! failing silently (the `@protected`/no-`@vocab` design makes typos
//! surface here rather than becoming junk IRIs).
//!
//! Offline guarantee: expansion resolves the document's `@context`
//! IRI through [`VendoredLoader`], which serves exactly the vendored
//! context file and REFUSES every other IRI. Network resolution is
//! impossible by construction (E16: "Kontexte gevendort, Offline-
//! Loader, nie Netz-Auflösung").
//!
//! Framing note (crate gap, Schema-Watch): json-ld 0.21.4 implements
//! no JSON-LD framing. The frame's only semantic for manifests
//! (`interfaces: {"@embed": "@always"}`) is guaranteed structurally
//! instead: interface nodes are blank nodes (no `@id`) and therefore
//! stay embedded through expand→compact. [`check_frame_invariant`]
//! refuses any interface node carrying an `@id` so that this
//! guarantee can never silently erode; if such nodes ever become
//! legal, real framing support is required first.

use std::fmt;

use anyhow::{Context as _, Result};
use json_ld::iref::{Iri, IriBuf};
use json_ld::syntax::{Parse, Print};
use json_ld::{JsonLdProcessor, LoadError, LoadingResult, RemoteDocument};

/// The canonical context IRI every manifest must use (mirrors the
/// `const` in `schema/manifest.schema.json`).
pub const CONTEXT_IRI: &str = "https://ld.openhelvetia.swiss/ns/manifest/v1";

/// Error returned for any IRI that is not the vendored context.
#[derive(Debug)]
struct OfflineOnly;

impl fmt::Display for OfflineOnly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "offline loader: only the vendored context IRI is servable; \
             network resolution is disabled by construction"
        )
    }
}

impl std::error::Error for OfflineOnly {}

/// Serves exactly one vendored context document; refuses everything
/// else. This is the E16 offline loader.
pub struct VendoredLoader {
    iri: IriBuf,
    document: json_ld::syntax::Value,
}

impl VendoredLoader {
    /// Reads the vendored context from disk. `context_path` points at
    /// the repository copy (`registry/standard/context/v1.jsonld`).
    pub fn from_file(context_path: &std::path::Path) -> Result<Self> {
        let raw = std::fs::read_to_string(context_path)
            .with_context(|| format!("reading vendored context {}", context_path.display()))?;
        let (document, _) = json_ld::syntax::Value::parse_str(&raw)
            .map_err(|e| anyhow::anyhow!("parsing vendored context: {e}"))?;
        Ok(Self {
            iri: IriBuf::new(CONTEXT_IRI.to_owned()).expect("canonical context IRI is valid"),
            document,
        })
    }
}

impl json_ld::Loader for VendoredLoader {
    async fn load(&self, url: &Iri) -> LoadingResult {
        if url == self.iri.as_iri() {
            Ok(RemoteDocument::new(
                Some(self.iri.clone()),
                Some("application/ld+json".parse().expect("valid mime")),
                self.document.clone(),
            ))
        } else {
            Err(LoadError::new(url.to_owned(), OfflineOnly))
        }
    }
}

/// One stage-2 finding, printable in the oh-validate error style.
#[derive(Debug)]
pub struct RoundtripError(pub String);

impl fmt::Display for RoundtripError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Runs the stage-2 gate on one raw manifest document.
///
/// `raw` is the file content exactly as read (stage 1 has already
/// validated it against the JSON schema). Returns the list of
/// findings; empty means the document IS in the canonical form.
pub fn check(raw: &str, loader: &VendoredLoader) -> Result<Vec<RoundtripError>> {
    let mut findings = Vec::new();

    let original: serde_json::Value =
        serde_json::from_str(raw).context("stage 2: re-parsing document")?;

    // The frame invariant must hold before the round-trip proves
    // anything (see module docs).
    if let Some(msg) = check_frame_invariant(&original) {
        findings.push(RoundtripError(msg));
        return Ok(findings);
    }

    // `$schema` is metadata, never graph content (standard README
    // rule 8): strip it before expansion, compare without it.
    let mut body = original.clone();
    if let Some(obj) = body.as_object_mut() {
        obj.remove("$schema");
    }

    let (ld_value, _) = json_ld::syntax::Value::parse_str(
        &serde_json::to_string(&body).context("stage 2: serializing body")?,
    )
    .map_err(|e| anyhow::anyhow!("stage 2: json-syntax parse: {e}"))?;

    // Deliberately NO base IRI: the canonical form requires absolute
    // IRIs everywhere (schema `pattern ^https://`), and a base would
    // make compaction relativize `@id` against it — exactly the kind
    // of silent shape change this gate exists to refuse.
    let input = RemoteDocument::new(
        None::<IriBuf>,
        Some("application/ld+json".parse().expect("valid mime")),
        ld_value,
    );

    let context_ref: json_ld::RemoteContextReference = json_ld::RemoteDocumentReference::iri(
        IriBuf::new(CONTEXT_IRI.to_owned()).expect("valid IRI"),
    );

    let compacted = futures::executor::block_on(async { input.compact(context_ref, loader).await })
        .map_err(|e| anyhow::anyhow!("stage 2: expand/compact failed: {e}"))?;

    let output: serde_json::Value = serde_json::from_str(&compacted.pretty_print().to_string())
        .context("stage 2: parsing compaction output")?;

    // Compare structurally, ignoring `@context` (the library may
    // inline it; stage 1 already pins the input value to the exact
    // canonical IRI via the schema `const`).
    let lhs = without_context(&body);
    let rhs = without_context(&output);

    if lhs != rhs {
        let dropped = key_diff(&lhs, &rhs);
        let gained = key_diff(&rhs, &lhs);
        let mut msg = String::from("round-trip drift: compact(expand(doc)) != doc");
        if !dropped.is_empty() {
            msg.push_str(&format!("; dropped terms: {}", dropped.join(", ")));
        }
        if !gained.is_empty() {
            msg.push_str(&format!("; gained terms: {}", gained.join(", ")));
        }
        if dropped.is_empty() && gained.is_empty() {
            msg.push_str("; same keys, differing values (shape or typing drift)");
        }
        if std::env::var_os("OH_STAGE2_DEBUG").is_some() {
            eprintln!(
                "--- input ---\n{}\n--- output ---\n{}",
                serde_json::to_string_pretty(&lhs).unwrap_or_default(),
                serde_json::to_string_pretty(&rhs).unwrap_or_default()
            );
        }
        findings.push(RoundtripError(msg));
    }

    Ok(findings)
}

/// The structural stand-in for the frame's `@embed: @always` (see
/// module docs): interface nodes must be blank nodes.
fn check_frame_invariant(doc: &serde_json::Value) -> Option<String> {
    let interfaces = doc.get("interfaces")?.as_array()?;
    for (i, node) in interfaces.iter().enumerate() {
        if node.get("@id").is_some() {
            return Some(format!(
                "interfaces[{i}] carries an @id: the structural embedding \
                 guarantee is void and real JSON-LD framing would be \
                 required — json-ld 0.21.4 does not implement framing \
                 (Schema-Watch entry)"
            ));
        }
    }
    None
}

fn without_context(v: &serde_json::Value) -> serde_json::Value {
    let mut c = v.clone();
    if let Some(obj) = c.as_object_mut() {
        obj.remove("@context");
    }
    c
}

/// All keys (recursively, dot-annotated) present in `a` but not `b`.
fn key_diff(a: &serde_json::Value, b: &serde_json::Value) -> Vec<String> {
    fn collect(v: &serde_json::Value, prefix: &str, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Object(map) => {
                for (k, val) in map {
                    let path = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{prefix}.{k}")
                    };
                    out.push(path.clone());
                    collect(val, &path, out);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    collect(item, prefix, out);
                }
            }
            _ => {}
        }
    }
    let mut ka = Vec::new();
    let mut kb = Vec::new();
    collect(a, "", &mut ka);
    collect(b, "", &mut kb);
    ka.retain(|k| !kb.contains(k));
    ka.sort();
    ka.dedup();
    ka
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_diff_names_missing_paths() {
        let a: serde_json::Value =
            serde_json::json!({"title": {"de": "x"}, "issued": "2026-08-15"});
        let b: serde_json::Value = serde_json::json!({"title": {"de": "x"}});
        let diff = key_diff(&a, &b);
        assert_eq!(diff, vec!["issued".to_string()]);
    }

    #[test]
    fn frame_invariant_refuses_identified_interfaces() {
        let doc = serde_json::json!({
            "interfaces": [{"@type": "RestInterface", "@id": "https://x/y"}]
        });
        let msg = check_frame_invariant(&doc).expect("must refuse");
        assert!(msg.contains("framing"));
    }

    #[test]
    fn frame_invariant_accepts_blank_interfaces() {
        let doc = serde_json::json!({
            "interfaces": [{"@type": "RestInterface", "endpoint": "https://x/y"}]
        });
        assert!(check_frame_invariant(&doc).is_none());
    }
}
