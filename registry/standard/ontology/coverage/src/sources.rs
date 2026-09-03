//! Requirement extraction from the coverage sources.
//!
//! The gate's contract (E16 no. 5e: every API/manifest entity has its `oh:`
//! class): every term the published artefacts EMIT must be covered by the
//! ontology. Emitting sources:
//!
//! 1. `context/v1.jsonld` — every mapped term key; `oh:` targets must be
//!    declared, foreign targets must be referenced (imported).
//! 2. `schema/manifest.schema.json`, 3. `directory.schema.json`,
//! 4. `card/agent-card.schema.json` — structural terms: interface `@type`
//!    values, enumerated values, envelope and card fields. Field names are
//!    mapped to ontology terms by the explicit tables below; a field that
//!    reaches no table entry is itself a gate failure, so ADDING a field to
//!    a schema turns the gate red until the ontology (and the table) cover
//!    it. The extractors iterate the schemas' actual JSON — never a
//!    hard-coded field list.
//!
//! Documented exclusions (each justified at its match arm): head metadata
//! (`$schema`, `@context`, `@id`, `@type` keys), JSON-shape helper `$defs`
//! (iri/languageMap/base64url/timestamp...), and the embedded A2A card
//! interior, which A2A 1.0 owns (standards are imported, never duplicated).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::model::OH_NS;

pub const DCT_NS: &str = "http://purl.org/dc/terms/";
pub const DCAT_NS: &str = "http://www.w3.org/ns/dcat#";
pub const SKOS_NS: &str = "http://www.w3.org/2004/02/skos/core#";

/// All coverage requirements extracted from the four sources.
#[derive(Debug, Clone, Default)]
pub struct Requirements {
    /// term IRI -> the source locations emitting it.
    pub terms: BTreeMap<String, BTreeSet<String>>,
    /// (owning `oh:` term, value token) -> the source locations emitting it.
    pub values: BTreeMap<(String, String), BTreeSet<String>>,
    /// Extraction failures (unmapped fields, unexpected structure). Any
    /// entry here fails the gate.
    pub errors: Vec<String>,
}

impl Requirements {
    pub fn add_term(&mut self, iri: impl Into<String>, source: impl Into<String>) {
        self.terms
            .entry(iri.into())
            .or_default()
            .insert(source.into());
    }

    pub fn add_value(
        &mut self,
        owner: impl Into<String>,
        value: impl Into<String>,
        source: impl Into<String>,
    ) {
        self.values
            .entry((owner.into(), value.into()))
            .or_default()
            .insert(source.into());
    }

    fn err(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    /// A term needs a declaration when it lives in the `oh:` namespace;
    /// foreign terms only need to be referenced (imported).
    pub fn needs_declaration(iri: &str) -> bool {
        iri.starts_with(OH_NS)
    }
}

fn oh(local: &str) -> String {
    format!("{OH_NS}{local}")
}

fn dct(local: &str) -> String {
    format!("{DCT_NS}{local}")
}

fn dcat(local: &str) -> String {
    format!("{DCAT_NS}{local}")
}

/// Extract all requirements from the four source documents.
pub fn from_sources(
    context: &Value,
    manifest_schema: &Value,
    directory_schema: &Value,
    card_schema: &Value,
) -> Requirements {
    let mut reqs = Requirements::default();
    context_requirements(&mut reqs, context);
    manifest_requirements(&mut reqs, manifest_schema);
    directory_requirements(&mut reqs, directory_schema);
    card_requirements(&mut reqs, card_schema);
    reqs
}

/// `context/v1.jsonld`: every term key is an emission. A string value that
/// is an absolute IRI declares a prefix (all prefixes in the vendored
/// context are namespace IRIs); every other entry maps a term.
fn context_requirements(reqs: &mut Requirements, context: &Value) {
    let Some(ctx) = context.get("@context").and_then(Value::as_object) else {
        reqs.err("context: missing '@context' object");
        return;
    };
    let mut prefixes = BTreeMap::new();
    for (key, value) in ctx {
        if let Some(s) = value.as_str()
            && (s.starts_with("http://") || s.starts_with("https://"))
        {
            prefixes.insert(key.clone(), s.to_string());
        }
    }
    for (key, value) in ctx {
        if key.starts_with('@') || prefixes.contains_key(key) {
            continue; // JSON-LD keyword (@version, @protected) or prefix.
        }
        let id = match value {
            Value::String(s) => Some(s.clone()),
            Value::Object(o) => o.get("@id").and_then(Value::as_str).map(str::to_string),
            _ => None,
        };
        let Some(id) = id else {
            reqs.err(format!("context: key '{key}' has no usable @id"));
            continue;
        };
        match expand(&id, &prefixes) {
            Some(iri) => reqs.add_term(iri, format!("context key '{key}'")),
            None => reqs.err(format!("context: cannot expand '{id}' (key '{key}')")),
        }
    }
}

fn expand(id: &str, prefixes: &BTreeMap<String, String>) -> Option<String> {
    if id.starts_with("http://") || id.starts_with("https://") {
        return Some(id.to_string());
    }
    let (prefix, local) = id.split_once(':')?;
    prefixes.get(prefix).map(|ns| format!("{ns}{local}"))
}

fn properties<'v>(
    schema: &'v Value,
    at: &str,
    reqs: &mut Requirements,
) -> Option<&'v serde_json::Map<String, Value>> {
    let props = schema.get("properties").and_then(Value::as_object);
    if props.is_none() {
        reqs.err(format!("{at}: expected a 'properties' object"));
    }
    props
}

fn enum_values(sub: &Value, at: &str, reqs: &mut Requirements) -> Vec<String> {
    let values: Option<Vec<String>> = sub.get("enum").and_then(Value::as_array).map(|a| {
        a.iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect()
    });
    match values {
        Some(v) if !v.is_empty() => v,
        _ => {
            reqs.err(format!("{at}: expected a non-empty string 'enum'"));
            Vec::new()
        }
    }
}

/// Shared probe-object walker (identical block in manifest and directory).
fn walk_probe(reqs: &mut Requirements, probe: &Value, src: &str) {
    let Some(props) = properties(probe, &format!("{src} probe"), reqs) else {
        return;
    };
    for (name, sub) in props {
        let at = format!("{src} probe.{name}");
        match name.as_str() {
            "kind" => {
                reqs.add_term(oh("probeKind"), &at);
                for v in enum_values(sub, &at, reqs) {
                    reqs.add_value(oh("probeKind"), v, &at);
                }
            }
            "expect" => {
                reqs.add_term(oh("probeExpect"), &at);
                for v in enum_values(sub, &at, reqs) {
                    reqs.add_value(oh("probeExpect"), v, &at);
                }
            }
            "target" => reqs.add_term(oh("probeTarget"), &at),
            _ => reqs.err(format!(
                "{at}: unmapped probe field — cover it in oh.ttl and map it here"
            )),
        }
    }
}

/// `schema/manifest.schema.json`.
fn manifest_requirements(reqs: &mut Requirements, schema: &Value) {
    let src = "manifest.schema";
    reqs.add_term(oh("Manifest"), format!("{src} root ('@type' const)"));
    if let Some(props) = properties(schema, src, reqs) {
        for name in props.keys() {
            let at = format!("{src} field '{name}'");
            let mapped: Option<String> = match name.as_str() {
                // Head metadata and JSON-LD keywords: never graph content
                // (manifest canonical form rule 8).
                "$schema" | "@context" | "@id" | "@type" => None,
                "title" => Some(dct("title")),
                "description" => Some(dct("description")),
                "keyword" => Some(dcat("keyword")),
                "publisher" => Some(dct("publisher")),
                "license" => Some(dct("license")),
                "landingPage" => Some(dcat("landingPage")),
                "legalBasis" => Some(oh("legalBasis")),
                "conformsTo" => Some(dct("conformsTo")),
                "exactMatch" => Some(format!("{SKOS_NS}exactMatch")),
                "interfaces" => Some(oh("interface")),
                "issued" => Some(dct("issued")),
                "modified" => Some(dct("modified")),
                _ => {
                    reqs.err(format!(
                        "{at}: unmapped manifest field — cover it in oh.ttl and map it here"
                    ));
                    None
                }
            };
            if let Some(iri) = mapped {
                reqs.add_term(iri, at);
            }
        }
    }
    let Some(defs) = schema.get("$defs").and_then(Value::as_object) else {
        reqs.err(format!("{src}: expected a '$defs' object"));
        return;
    };
    for (name, def) in defs {
        match name.as_str() {
            // JSON-shape helpers, not platform entities.
            "iri" | "iriOrArray" | "languageMap" | "languageMapArray" => {}
            "interface" => {
                reqs.add_term(oh("Interface"), format!("{src} $defs.interface"));
                walk_manifest_interface(reqs, def, src);
            }
            _ => reqs.err(format!(
                "{src} $defs.{name}: unmapped definition — cover it in oh.ttl and map it here"
            )),
        }
    }
}

fn walk_manifest_interface(reqs: &mut Requirements, def: &Value, src: &str) {
    let Some(props) = properties(def, &format!("{src} interface"), reqs) else {
        return;
    };
    for (name, sub) in props {
        let at = format!("{src} interface.{name}");
        match name.as_str() {
            "@type" => {
                for v in enum_values(sub, &at, reqs) {
                    reqs.add_term(oh(&v), &at);
                }
            }
            "endpoint" => reqs.add_term(dcat("endpointURL"), &at),
            "docs" => reqs.add_term(oh("documentation"), &at),
            "conformsTo" => reqs.add_term(dct("conformsTo"), &at),
            "tier" => {
                reqs.add_term(oh("tier"), &at);
                for v in enum_values(sub, &at, reqs) {
                    reqs.add_value(oh("tier"), v, &at);
                }
            }
            "auth" => {
                reqs.add_term(oh("auth"), &at);
                reqs.add_term(oh("AuthPolicy"), &at);
                walk_auth(reqs, sub, src);
            }
            "probe" => {
                reqs.add_term(oh("probe"), &at);
                reqs.add_term(oh("Probe"), &at);
                walk_probe(reqs, sub, src);
            }
            _ => reqs.err(format!(
                "{at}: unmapped interface field — cover it in oh.ttl and map it here"
            )),
        }
    }
}

fn walk_auth(reqs: &mut Requirements, auth: &Value, src: &str) {
    let Some(props) = properties(auth, &format!("{src} auth"), reqs) else {
        return;
    };
    for (name, sub) in props {
        let at = format!("{src} auth.{name}");
        match name.as_str() {
            "authType" => {
                reqs.add_term(oh("authType"), &at);
                for v in enum_values(sub, &at, reqs) {
                    reqs.add_value(oh("authType"), v, &at);
                }
            }
            "docs" => reqs.add_term(oh("documentation"), &at),
            _ => reqs.err(format!(
                "{at}: unmapped auth field — cover it in oh.ttl and map it here"
            )),
        }
    }
}

/// `directory.schema.json` (envelope v0.2).
fn directory_requirements(reqs: &mut Requirements, schema: &Value) {
    let src = "directory.schema";
    reqs.add_term(oh("Directory"), format!("{src} root"));
    if let Some(props) = properties(schema, src, reqs) {
        for (name, sub) in props {
            let at = format!("{src} field '{name}'");
            match name.as_str() {
                // Head metadatum, manifest-standard convention.
                "$schema" => {}
                "version" => reqs.add_term(oh("schemaVersion"), &at),
                "authority" => {
                    reqs.add_term(oh("authority"), &at);
                    match sub.get("const").and_then(Value::as_str) {
                        Some(v) => reqs.add_value(oh("authority"), v, &at),
                        None => reqs.err(format!("{at}: expected a string 'const'")),
                    }
                }
                "entries" => reqs.add_term(oh("entry"), &at),
                _ => reqs.err(format!(
                    "{at}: unmapped envelope field — cover it in oh.ttl and map it here"
                )),
            }
        }
    }
    let Some(defs) = schema.get("$defs").and_then(Value::as_object) else {
        reqs.err(format!("{src}: expected a '$defs' object"));
        return;
    };
    for (name, def) in defs {
        match name.as_str() {
            "languageMap" => {} // JSON-shape helper.
            "entry" => {
                reqs.add_term(oh("RegistryEntry"), format!("{src} $defs.entry"));
                walk_directory_entry(reqs, def, src);
            }
            "interface" => {
                reqs.add_term(oh("Interface"), format!("{src} $defs.interface"));
                walk_directory_interface(reqs, def, src);
            }
            "probe" => {
                reqs.add_term(oh("Probe"), format!("{src} $defs.probe"));
                walk_probe(reqs, def, src);
            }
            _ => reqs.err(format!(
                "{src} $defs.{name}: unmapped definition — cover it in oh.ttl and map it here"
            )),
        }
    }
}

fn walk_directory_entry(reqs: &mut Requirements, def: &Value, src: &str) {
    let Some(props) = properties(def, &format!("{src} entry"), reqs) else {
        return;
    };
    for name in props.keys() {
        let at = format!("{src} entry.{name}");
        let mapped: Option<String> = match name.as_str() {
            "slug" => Some(oh("slug")),
            // Projections of manifest fields keep the manifest's terms.
            "name" => Some(dct("title")),
            "description" => Some(dct("description")),
            "publisher" => Some(dct("publisher")),
            "license" => Some(dct("license")),
            "manifest" => Some(oh("manifest")),
            "docs" => Some(oh("documentation")),
            "interfaces" => Some(oh("interface")),
            _ => {
                reqs.err(format!(
                    "{at}: unmapped entry field — cover it in oh.ttl and map it here"
                ));
                None
            }
        };
        if let Some(iri) = mapped {
            reqs.add_term(iri, at);
        }
    }
}

fn walk_directory_interface(reqs: &mut Requirements, def: &Value, src: &str) {
    let Some(props) = properties(def, &format!("{src} interface"), reqs) else {
        return;
    };
    for (name, sub) in props {
        let at = format!("{src} interface.{name}");
        match name.as_str() {
            "type" => {
                for v in enum_values(sub, &at, reqs) {
                    reqs.add_term(oh(&v), &at);
                }
            }
            "endpoint" => reqs.add_term(dcat("endpointURL"), &at),
            "docs" => reqs.add_term(oh("documentation"), &at),
            "tier" => {
                reqs.add_term(oh("tier"), &at);
                for v in enum_values(sub, &at, reqs) {
                    reqs.add_value(oh("tier"), v, &at);
                }
            }
            "probe" => reqs.add_term(oh("probe"), &at),
            _ => reqs.err(format!(
                "{at}: unmapped interface field — cover it in oh.ttl and map it here"
            )),
        }
    }
}

/// `card/agent-card.schema.json` (envelope v0.1).
fn card_requirements(reqs: &mut Requirements, schema: &Value) {
    let src = "card.schema";
    reqs.add_term(oh("AgentCardEnvelope"), format!("{src} root"));
    if let Some(props) = properties(schema, src, reqs) {
        for name in props.keys() {
            let at = format!("{src} field '{name}'");
            let mapped: Option<String> = match name.as_str() {
                // Head metadatum, manifest-standard convention.
                "$schema" => None,
                "card" => Some(oh("card")),
                "authenticity" => Some(oh("authenticity")),
                "attestation" => Some(oh("attestation")),
                _ => {
                    reqs.err(format!(
                        "{at}: unmapped envelope field — cover it in oh.ttl and map it here"
                    ));
                    None
                }
            };
            if let Some(iri) = mapped {
                reqs.add_term(iri, at);
            }
        }
    }
    let Some(defs) = schema.get("$defs").and_then(Value::as_object) else {
        reqs.err(format!("{src}: expected a '$defs' object"));
        return;
    };
    for (name, def) in defs {
        match name.as_str() {
            // The embedded card's interior is owned by A2A 1.0 (E16 no. 6):
            // the standard is imported, never duplicated, so its smoke-check
            // fields (protocolVersion, name, url, version) are deliberately
            // NOT oh: terms — the whole card hangs off oh:card.
            "a2aCard" => {}
            // JSON-shape helpers, not platform entities.
            "base64url" | "timestamp" => {}
            "authenticity" => {
                reqs.add_term(oh("Authenticity"), format!("{src} $defs.authenticity"));
                walk_card_block(reqs, def, "authenticity", src);
            }
            "attestation" => {
                reqs.add_term(oh("Attestation"), format!("{src} $defs.attestation"));
                walk_card_block(reqs, def, "attestation", src);
            }
            "detachedJws" => {
                reqs.add_term(oh("DetachedJws"), format!("{src} $defs.detachedJws"));
                walk_detached_jws(reqs, def, src);
            }
            "keyRef" => {
                reqs.add_term(oh("KeyRef"), format!("{src} $defs.keyRef"));
                walk_key_ref(reqs, def, src);
            }
            _ => reqs.err(format!(
                "{src} $defs.{name}: unmapped definition — cover it in oh.ttl and map it here"
            )),
        }
    }
}

/// Shared walker for the two trust blocks (authenticity / attestation).
fn walk_card_block(reqs: &mut Requirements, def: &Value, block: &str, src: &str) {
    let Some(props) = properties(def, &format!("{src} {block}"), reqs) else {
        return;
    };
    for (name, sub) in props {
        let at = format!("{src} {block}.{name}");
        match name.as_str() {
            "signature" => reqs.add_term(oh("signature"), &at),
            "keyRef" => reqs.add_term(oh("keyRef"), &at),
            "signedAt" => reqs.add_term(oh("signedAt"), &at),
            "verifiedAt" => reqs.add_term(oh("verifiedAt"), &at),
            "expiresAt" => reqs.add_term(oh("expiresAt"), &at),
            "level" => {
                reqs.add_term(oh("attestationLevel"), &at);
                for v in enum_values(sub, &at, reqs) {
                    reqs.add_value(oh("attestationLevel"), v, &at);
                }
            }
            _ => reqs.err(format!(
                "{at}: unmapped trust-block field — cover it in oh.ttl and map it here"
            )),
        }
    }
}

fn walk_detached_jws(reqs: &mut Requirements, def: &Value, src: &str) {
    let Some(props) = properties(def, &format!("{src} detachedJws"), reqs) else {
        return;
    };
    for name in props.keys() {
        let at = format!("{src} detachedJws.{name}");
        match name.as_str() {
            "protected" => reqs.add_term(oh("jwsProtected"), &at),
            // Inside the detached JWS, 'signature' is the base64url value —
            // distinct from the trust blocks' 'signature' (the JWS object).
            "signature" => reqs.add_term(oh("jwsSignature"), &at),
            _ => reqs.err(format!(
                "{at}: unmapped JWS field — cover it in oh.ttl and map it here"
            )),
        }
    }
}

fn walk_key_ref(reqs: &mut Requirements, def: &Value, src: &str) {
    let Some(props) = properties(def, &format!("{src} keyRef"), reqs) else {
        return;
    };
    for name in props.keys() {
        let at = format!("{src} keyRef.{name}");
        match name.as_str() {
            "jwksUri" => reqs.add_term(oh("jwksUri"), &at),
            "kid" => reqs.add_term(oh("kid"), &at),
            _ => reqs.err(format!(
                "{at}: unmapped keyRef field — cover it in oh.ttl and map it here"
            )),
        }
    }
}
