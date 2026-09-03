//! Conformance tests for the agent-card signature envelope v0.1.
//!
//! Covered guarantees:
//! 1. the schema itself is a valid JSON Schema draft 2020-12 document,
//! 2. both valid example vectors pass schema validation,
//! 3. the deliberately merged vector fails — the regression tripwire for
//!    E09 no. 5 («Zwei Felder, nie eines»): authenticity and attestation
//!    must remain two separate top-level structures, and the schema's
//!    `additionalProperties: false` must actually reject the merged shape.
//!
//! The vectors are structural: signatures are well-formed base64url but not
//! cryptographically valid. Cryptographic verification is out of scope here
//! (open point 0.1 → 0.2 in the spec README).

use card_conformance::{example, schema};

/// Compile the envelope schema once per test; compilation itself already
/// rejects malformed schema constructs.
fn validator() -> jsonschema::Validator {
    jsonschema::validator_for(&schema()).expect("agent-card.schema.json must compile")
}

/// The schema must declare draft 2020-12 and validate against its
/// meta-schema (the jsonschema crate bundles the meta-schemas, so this
/// check runs fully offline).
#[test]
fn schema_is_valid_draft_2020_12() {
    let schema = schema();
    assert_eq!(
        schema["$schema"],
        serde_json::json!("https://json-schema.org/draft/2020-12/schema"),
        "the envelope schema must declare draft 2020-12 explicitly"
    );
    jsonschema::draft202012::meta::validate(&schema)
        .expect("agent-card.schema.json must be valid against the draft 2020-12 meta-schema");
}

/// `card-minimal.json`: authenticity only — a card that is operator-signed
/// but not (yet) Verein-verified. This is a normal lifecycle state and must
/// validate without an attestation block.
#[test]
fn minimal_vector_passes() {
    let errors: Vec<String> = validator()
        .iter_errors(&example("card-minimal"))
        .map(|err| format!("{} at {}", err, err.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "card-minimal.json must validate: {errors:?}"
    );
}

/// `card-attested.json`: both trust blocks present — operator-signed and
/// Verein-attested, each with its own key reference and JWS.
#[test]
fn attested_vector_passes() {
    let errors: Vec<String> = validator()
        .iter_errors(&example("card-attested"))
        .map(|err| format!("{} at {}", err, err.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "card-attested.json must validate: {errors:?}"
    );
}

/// `card-invalid-merged.json`: operator signature and Verein verification
/// merged into a single `authenticity` block. The schema must reject it,
/// and the rejection must point into `/authenticity` — proving that the
/// mechanism is the block's `additionalProperties: false`, not some
/// unrelated defect in the vector.
#[test]
fn merged_vector_fails_inside_authenticity() {
    let validator = validator();
    let merged = example("card-invalid-merged");
    assert!(
        !validator.is_valid(&merged),
        "the merged two-fields-in-one vector must fail validation (E09 no. 5)"
    );
    let paths: Vec<String> = validator
        .iter_errors(&merged)
        .map(|err| err.instance_path().to_string())
        .collect();
    assert!(
        paths.iter().any(|path| path.starts_with("/authenticity")),
        "the failure must come from the merged authenticity block, got: {paths:?}"
    );
}
