//! Canonical byte formatting of manifest files (stage 1b).
//!
//! «Genau eine normierte Gestalt» reaches down to the bytes: a
//! manifest file must equal its canonical serialization — fixed key
//! order, two-space indent, LF, trailing newline. This ended the
//! corpus drift of 20.08. (hand-written inline arrays vs. a
//! migration's serde pretty-print) and makes Git diffs carry content,
//! never style.
//!
//! Key order is the schema's declaration order; language maps follow
//! the Fedlex code order de/fr/it/rm/en. Unknown keys (impossible
//! after stage 1, defensive anyway) sort alphabetically at the end.

use serde_json::Value;

const TOP: &[&str] = &[
    "$schema",
    "@context",
    "@id",
    "@type",
    "title",
    "description",
    "keyword",
    "publisher",
    "license",
    "landingPage",
    "legalBasis",
    "conformsTo",
    "exactMatch",
    "interfaces",
    "issued",
    "modified",
];
const INTERFACE: &[&str] = &[
    "@type",
    "endpoint",
    "docs",
    "conformsTo",
    "auth",
    "tier",
    "probe",
];
const AUTH: &[&str] = &["authType", "docs"];
const PROBE: &[&str] = &["kind", "expect", "target"];
const LANG: &[&str] = &["de", "fr", "it", "rm", "en"];

/// Which ordering applies inside the object reached via `key`.
fn order_for(parent: Order, key: &str) -> Order {
    match (parent, key) {
        (Order::Top, "interfaces") => Order::Interface,
        (Order::Top, "title" | "description" | "keyword") => Order::Lang,
        (Order::Interface, "auth") => Order::Auth,
        (Order::Interface, "probe") => Order::Probe,
        _ => Order::Unknown,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Order {
    Top,
    Interface,
    Auth,
    Probe,
    Lang,
    Unknown,
}

impl Order {
    fn keys(self) -> &'static [&'static str] {
        match self {
            Order::Top => TOP,
            Order::Interface => INTERFACE,
            Order::Auth => AUTH,
            Order::Probe => PROBE,
            Order::Lang => LANG,
            Order::Unknown => &[],
        }
    }
}

/// The canonical serialization of a manifest document.
pub fn canonical(doc: &Value) -> String {
    let mut out = String::new();
    write_value(doc, Order::Top, 0, &mut out);
    out.push('\n');
    out
}

/// `Some(message)` when the raw file bytes differ from the canonical
/// serialization.
pub fn check(raw: &str, doc: &Value) -> Option<String> {
    (raw != canonical(doc)).then(|| {
        "file is not in canonical formatting (fixed key order, 2-space \
         indent, LF, trailing newline) — run oh-validate with --write \
         to normalize"
            .to_string()
    })
}

fn write_value(v: &Value, order: Order, indent: usize, out: &mut String) {
    match v {
        Value::Object(map) => {
            if map.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            let mut keys: Vec<&String> = map.keys().collect();
            let rank = |k: &str| {
                order
                    .keys()
                    .iter()
                    .position(|o| *o == k)
                    .unwrap_or(usize::MAX)
            };
            keys.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.cmp(b)));
            for (i, key) in keys.iter().enumerate() {
                pad(indent + 1, out);
                out.push_str(&serde_json::to_string(key).expect("string serializes"));
                out.push_str(": ");
                write_value(&map[key.as_str()], order_for(order, key), indent + 1, out);
                if i + 1 < keys.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            pad(indent, out);
            out.push('}');
        }
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                pad(indent + 1, out);
                write_value(item, order, indent + 1, out);
                if i + 1 < items.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            pad(indent, out);
            out.push(']');
        }
        scalar => out.push_str(&serde_json::to_string(scalar).expect("scalar serializes")),
    }
}

fn pad(indent: usize, out: &mut String) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Value {
        serde_json::json!({
            "issued": "2026-08-20",
            "@id": "https://x/y",
            "title": {"en": "T", "de": "T"},
            "interfaces": [{"auth": {"authType": "none"}, "@type": "RestInterface"}]
        })
    }

    #[test]
    fn canonical_is_idempotent() {
        let c = canonical(&doc());
        let reparsed: Value = serde_json::from_str(&c).unwrap();
        assert_eq!(c, canonical(&reparsed));
    }

    #[test]
    fn key_order_follows_the_schema_declaration() {
        let c = canonical(&doc());
        let id = c.find("\"@id\"").unwrap();
        let title = c.find("\"title\"").unwrap();
        let interfaces = c.find("\"interfaces\"").unwrap();
        let issued = c.find("\"issued\"").unwrap();
        assert!(id < title && title < interfaces && interfaces < issued);
        // Language maps follow the Fedlex order, not alphabet:
        assert!(c.find("\"de\"").unwrap() < c.find("\"en\"").unwrap());
        // Interface keys reorder too:
        assert!(c.find("\"@type\": \"RestInterface\"").unwrap() < c.find("\"auth\"").unwrap());
    }

    #[test]
    fn check_flags_non_canonical_bytes() {
        let raw = serde_json::to_string(&doc()).unwrap(); // compact form
        let parsed: Value = serde_json::from_str(&raw).unwrap();
        assert!(check(&raw, &parsed).is_some());
        let canon = canonical(&parsed);
        assert!(check(&canon, &parsed).is_none());
    }
}
