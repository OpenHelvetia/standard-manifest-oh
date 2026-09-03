//! Ontology model extracted from the parsed `oh.ttl` triples.
//!
//! The model answers the three questions the gate asks: which `oh:` terms
//! are DECLARED (subject of an `a owl:Class` / property / datatype triple),
//! which foreign IRIs are REFERENCED (object position — imports are never
//! re-declared, so object position is the only legal place for them), and
//! which enumerated values each term documents (machine-readable
//! `skos:note "Allowed value '<v>': ..."` annotations).

use std::collections::{BTreeMap, BTreeSet};

use crate::ttl::{Document, Object, RDF_TYPE};

/// The `oh:` namespace; also the ontology header's own IRI.
pub const OH_NS: &str = "https://ld.openhelvetia.swiss/schema/";

pub const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
pub const OWL_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
pub const OWL_DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
pub const OWL_ANNOTATION_PROPERTY: &str = "http://www.w3.org/2002/07/owl#AnnotationProperty";
pub const RDFS_DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";
pub const OWL_ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";

pub const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
pub const RDFS_COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
pub const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
pub const RDFS_SUBPROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
pub const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
pub const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
pub const RDFS_SEE_ALSO: &str = "http://www.w3.org/2000/01/rdf-schema#seeAlso";
pub const OWL_DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";
pub const OWL_EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
pub const OWL_EQUIVALENT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
pub const OWL_DEPRECATED: &str = "http://www.w3.org/2002/07/owl#deprecated";
pub const OWL_VERSION_IRI: &str = "http://www.w3.org/2002/07/owl#versionIRI";
pub const OWL_VERSION_INFO: &str = "http://www.w3.org/2002/07/owl#versionInfo";
pub const SKOS_NOTE: &str = "http://www.w3.org/2004/02/skos/core#note";
pub const OH_IMPORTS_TERM: &str = "https://ld.openhelvetia.swiss/schema/importsTerm";

/// The declaration types the gate recognizes as "this term is declared".
pub const DECLARATION_KINDS: [&str; 5] = [
    OWL_CLASS,
    OWL_OBJECT_PROPERTY,
    OWL_DATATYPE_PROPERTY,
    OWL_ANNOTATION_PROPERTY,
    RDFS_DATATYPE,
];

/// Everything the doc generator and the gate need to know about one term.
#[derive(Debug, Clone, Default)]
pub struct TermData {
    pub kinds: BTreeSet<String>,
    pub label: Option<String>,
    pub comment: Option<String>,
    /// `skos:note` literals in file order (carry the allowed-value docs).
    pub notes: Vec<String>,
    pub subclass_of: Vec<String>,
    pub subproperty_of: Vec<String>,
    pub equivalent: Vec<String>,
    pub domains: Vec<String>,
    pub ranges: Vec<String>,
    pub disjoint_with: Vec<String>,
    pub see_also: Vec<String>,
    pub deprecated: bool,
}

impl TermData {
    /// True when the term carries one of the recognized declaration types.
    pub fn is_declared(&self) -> bool {
        self.kinds
            .iter()
            .any(|k| DECLARATION_KINDS.contains(&k.as_str()))
    }
}

/// Metadata of the `owl:Ontology` header node.
#[derive(Debug, Clone, Default)]
pub struct OntologyHeader {
    pub label: Option<String>,
    pub comment: Option<String>,
    pub version_iri: Option<String>,
    pub version_info: Option<String>,
    /// Foreign terms listed under `oh:importsTerm`, in file order.
    pub imports: Vec<String>,
}

/// The extracted ontology model.
#[derive(Debug, Clone, Default)]
pub struct Model {
    pub prefixes: BTreeMap<String, String>,
    pub header: OntologyHeader,
    /// All `oh:` term subjects (the ontology header node is kept separate).
    pub terms: BTreeMap<String, TermData>,
    /// Every foreign IRI appearing in object position.
    pub referenced: BTreeSet<String>,
    /// subject IRI -> allowed-value tokens parsed from its skos:notes.
    pub allowed_values: BTreeMap<String, BTreeSet<String>>,
    /// `oh:` IRIs used in predicate position (in use even without emission).
    pub predicates_used: BTreeSet<String>,
    /// Undirected adjacency between `oh:` terms (any triple linking two).
    pub edges: BTreeMap<String, BTreeSet<String>>,
}

impl Model {
    /// The declared `oh:` terms (subject of a recognized declaration).
    pub fn declared(&self) -> BTreeSet<&str> {
        self.terms
            .iter()
            .filter(|(_, d)| d.is_declared())
            .map(|(iri, _)| iri.as_str())
            .collect()
    }

    /// Compact an IRI to `prefix:local` form; the longest matching
    /// namespace (= shortest remaining local name) wins.
    pub fn compact(&self, iri: &str) -> String {
        let mut best: Option<(&str, &str)> = None;
        for (prefix, ns) in &self.prefixes {
            if let Some(local) = iri.strip_prefix(ns.as_str())
                && best.is_none_or(|(_, b): (&str, &str)| local.len() < b.len())
            {
                best = Some((prefix, local));
            }
        }
        match best {
            Some((prefix, local)) => format!("{prefix}:{local}"),
            None => format!("<{iri}>"),
        }
    }
}

/// Parse the machine-readable value token out of an allowed-value note:
/// `Allowed value 'none': ...` -> `none`.
pub fn allowed_value_token(note: &str) -> Option<&str> {
    let rest = note.strip_prefix("Allowed value '")?;
    let end = rest.find('\'')?;
    Some(&rest[..end])
}

/// Build the [`Model`] from a parsed document.
pub fn build(doc: &Document) -> Result<Model, String> {
    let mut model = Model {
        prefixes: doc.prefixes.clone(),
        ..Model::default()
    };
    let mut saw_header = false;

    for t in &doc.triples {
        let subject_is_oh_term = t.subject.starts_with(OH_NS) && t.subject != OH_NS;

        // Referenced foreign IRIs and the oh:-to-oh: adjacency (objects).
        if let Object::Iri(o) = &t.object {
            if o.starts_with(OH_NS) && o != OH_NS {
                if subject_is_oh_term {
                    model
                        .edges
                        .entry(t.subject.clone())
                        .or_default()
                        .insert(o.clone());
                    model
                        .edges
                        .entry(o.clone())
                        .or_default()
                        .insert(t.subject.clone());
                }
            } else if !o.starts_with(OH_NS) {
                model.referenced.insert(o.clone());
            }
        }
        if t.predicate.starts_with(OH_NS) && t.predicate != OH_NS {
            model.predicates_used.insert(t.predicate.clone());
        }

        // Ontology header node.
        if t.subject == OH_NS {
            saw_header = true;
            let h = &mut model.header;
            match (t.predicate.as_str(), &t.object) {
                (RDFS_LABEL, Object::Literal(l)) => h.label = Some(l.value.clone()),
                (RDFS_COMMENT, Object::Literal(l)) => h.comment = Some(l.value.clone()),
                (OWL_VERSION_IRI, Object::Iri(iri)) => h.version_iri = Some(iri.clone()),
                (OWL_VERSION_INFO, Object::Literal(l)) => h.version_info = Some(l.value.clone()),
                (OH_IMPORTS_TERM, Object::Iri(iri)) => h.imports.push(iri.clone()),
                (RDF_TYPE, Object::Iri(k)) if k == OWL_ONTOLOGY => {}
                _ => {}
            }
            continue;
        }
        if !subject_is_oh_term {
            return Err(format!(
                "foreign subject '{}' — oh.ttl must not declare or annotate foreign terms",
                t.subject
            ));
        }

        let data = model.terms.entry(t.subject.clone()).or_default();
        match (t.predicate.as_str(), &t.object) {
            (RDF_TYPE, Object::Iri(kind)) => {
                data.kinds.insert(kind.clone());
            }
            (RDFS_LABEL, Object::Literal(l)) => data.label = Some(l.value.clone()),
            (RDFS_COMMENT, Object::Literal(l)) => data.comment = Some(l.value.clone()),
            (SKOS_NOTE, Object::Literal(l)) => {
                if let Some(token) = allowed_value_token(&l.value) {
                    model
                        .allowed_values
                        .entry(t.subject.clone())
                        .or_default()
                        .insert(token.to_string());
                }
                data.notes.push(l.value.clone());
            }
            (RDFS_SUBCLASS_OF, Object::Iri(o)) => data.subclass_of.push(o.clone()),
            (RDFS_SUBPROPERTY_OF, Object::Iri(o)) => data.subproperty_of.push(o.clone()),
            (OWL_EQUIVALENT_CLASS | OWL_EQUIVALENT_PROPERTY, Object::Iri(o)) => {
                data.equivalent.push(o.clone());
            }
            (RDFS_DOMAIN, Object::Iri(o)) => data.domains.push(o.clone()),
            (RDFS_RANGE, Object::Iri(o)) => data.ranges.push(o.clone()),
            (OWL_DISJOINT_WITH, Object::Iri(o)) => data.disjoint_with.push(o.clone()),
            (RDFS_SEE_ALSO, Object::Iri(o)) => data.see_also.push(o.clone()),
            (OWL_DEPRECATED, Object::Literal(l)) => data.deprecated = l.value == "true",
            _ => {}
        }
    }

    if !saw_header {
        return Err(format!("no owl:Ontology header found for <{OH_NS}>"));
    }
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ttl;

    #[test]
    fn allowed_value_tokens_are_parsed() {
        assert_eq!(
            allowed_value_token("Allowed value 'sparql-ask': send an ASK."),
            Some("sparql-ask")
        );
        assert_eq!(allowed_value_token("Some other note"), None);
    }

    #[test]
    fn foreign_subjects_are_rejected() {
        let src = "@prefix dcat: <http://www.w3.org/ns/dcat#> .\n\
                   @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                   dcat:Dataset a owl:Class .\n";
        let doc = ttl::parse(src).expect("parse");
        let err = build(&doc).unwrap_err();
        assert!(err.contains("foreign subject"), "{err}");
    }
}
