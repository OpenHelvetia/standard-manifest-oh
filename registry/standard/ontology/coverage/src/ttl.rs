//! Minimal Turtle reader for the ontology's controlled authoring subset.
//!
//! `oh.ttl` is our own file, written in a documented subset of Turtle (see
//! the header comment there). The subset this reader accepts:
//!
//! - `@prefix` directives,
//! - IRIs (`<...>`) and prefixed names (`oh:Manifest`),
//! - string literals, short (`"..."`) and long (`"""..."""`), with the
//!   escapes `\"`, `\\`, `\n`, `\t`, `\r`, and an optional language tag
//!   (`@en`) or datatype (`^^xsd:string`),
//! - the keywords `a`, `true`, `false`,
//! - the punctuation `;`, `,`, `.` and `#` comments.
//!
//! Blank nodes, collections, `@base`, numeric literals and everything else
//! are OUTSIDE the subset and rejected with a positioned error — drift into
//! syntax this tool cannot see can never pass the coverage gate silently.

use std::collections::BTreeMap;

/// The `rdf:type` IRI emitted for the `a` keyword.
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// The `xsd:boolean` IRI emitted for the bare `true`/`false` tokens.
pub const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

/// A literal object: value plus optional language tag or datatype IRI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Literal {
    pub value: String,
    pub lang: Option<String>,
    pub datatype: Option<String>,
}

/// A triple object: either an IRI or a literal (the subset has no blank nodes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Object {
    Iri(String),
    Literal(Literal),
}

/// One parsed triple; subject and predicate are fully expanded IRIs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Triple {
    pub subject: String,
    pub predicate: String,
    pub object: Object,
}

/// A parsed document: the prefix map plus all triples in file order.
#[derive(Debug, Clone, Default)]
pub struct Document {
    pub prefixes: BTreeMap<String, String>,
    pub triples: Vec<Triple>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DatatypeRaw {
    Iri(String),
    Name(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Iri(String),
    /// A prefixed name or bare keyword (`a`, `true`, `false`).
    Name(String),
    Lit {
        value: String,
        lang: Option<String>,
        datatype: Option<DatatypeRaw>,
    },
    Semicolon,
    Comma,
    Dot,
    PrefixKeyword,
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == ':' || c == '_' || c == '-'
}

/// Tokenize `source`; each token carries the line it starts on (1-based).
fn lex(source: &str) -> Result<Vec<(Tok, usize)>, String> {
    let cs: Vec<char> = source.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    let mut line = 1;
    while i < cs.len() {
        let c = cs[i];
        match c {
            '\n' => {
                line += 1;
                i += 1;
            }
            _ if c.is_whitespace() => i += 1,
            '#' => {
                while i < cs.len() && cs[i] != '\n' {
                    i += 1;
                }
            }
            '<' => {
                let start = line;
                let mut iri = String::new();
                i += 1;
                loop {
                    let Some(&ch) = cs.get(i) else {
                        return Err(format!("line {start}: unterminated IRI"));
                    };
                    if ch == '>' {
                        i += 1;
                        break;
                    }
                    if ch == '\n' || ch == ' ' {
                        return Err(format!("line {start}: whitespace inside IRI"));
                    }
                    iri.push(ch);
                    i += 1;
                }
                toks.push((Tok::Iri(iri), start));
            }
            '"' => {
                let start = line;
                let (value, next, endline) = lex_string(&cs, i, line)?;
                i = next;
                line = endline;
                let mut lang = None;
                let mut datatype = None;
                if cs.get(i) == Some(&'@') {
                    i += 1;
                    let mut tag = String::new();
                    while let Some(&ch) = cs.get(i) {
                        if ch.is_ascii_alphanumeric() || ch == '-' {
                            tag.push(ch);
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    if tag.is_empty() {
                        return Err(format!("line {line}: empty language tag"));
                    }
                    lang = Some(tag);
                } else if cs.get(i) == Some(&'^') && cs.get(i + 1) == Some(&'^') {
                    i += 2;
                    if cs.get(i) == Some(&'<') {
                        let mut iri = String::new();
                        i += 1;
                        while let Some(&ch) = cs.get(i) {
                            if ch == '>' {
                                break;
                            }
                            iri.push(ch);
                            i += 1;
                        }
                        if cs.get(i) != Some(&'>') {
                            return Err(format!("line {line}: unterminated datatype IRI"));
                        }
                        i += 1;
                        datatype = Some(DatatypeRaw::Iri(iri));
                    } else {
                        let mut name = String::new();
                        while let Some(&ch) = cs.get(i) {
                            if is_name_char(ch) {
                                name.push(ch);
                                i += 1;
                            } else {
                                break;
                            }
                        }
                        if name.is_empty() {
                            return Err(format!("line {line}: empty datatype"));
                        }
                        datatype = Some(DatatypeRaw::Name(name));
                    }
                }
                toks.push((
                    Tok::Lit {
                        value,
                        lang,
                        datatype,
                    },
                    start,
                ));
            }
            ';' => {
                toks.push((Tok::Semicolon, line));
                i += 1;
            }
            ',' => {
                toks.push((Tok::Comma, line));
                i += 1;
            }
            '.' => {
                toks.push((Tok::Dot, line));
                i += 1;
            }
            '@' => {
                let mut word = String::new();
                i += 1;
                while let Some(&ch) = cs.get(i) {
                    if ch.is_ascii_alphabetic() {
                        word.push(ch);
                        i += 1;
                    } else {
                        break;
                    }
                }
                if word == "prefix" {
                    toks.push((Tok::PrefixKeyword, line));
                } else {
                    return Err(format!(
                        "line {line}: directive '@{word}' is outside the authoring subset"
                    ));
                }
            }
            '[' | ']' | '(' | ')' | '{' | '}' => {
                return Err(format!(
                    "line {line}: '{c}' (blank node/collection syntax) is outside the authoring subset"
                ));
            }
            _ if is_name_char(c) => {
                let start = line;
                let mut name = String::new();
                while let Some(&ch) = cs.get(i) {
                    if is_name_char(ch) {
                        name.push(ch);
                        i += 1;
                    } else if ch == '.' && cs.get(i + 1).copied().is_some_and(is_name_char) {
                        // A dot inside a local name (e.g. a versioned name);
                        // a trailing dot is statement punctuation instead.
                        name.push(ch);
                        i += 1;
                    } else {
                        break;
                    }
                }
                toks.push((Tok::Name(name), start));
            }
            _ => {
                return Err(format!(
                    "line {line}: character '{c}' is outside the authoring subset"
                ));
            }
        }
    }
    Ok(toks)
}

/// Lex a short or long string starting at `cs[at] == '"'`.
/// Returns (unescaped value, index after the closing quotes, current line).
fn lex_string(cs: &[char], at: usize, mut line: usize) -> Result<(String, usize, usize), String> {
    let start = line;
    let long = cs.get(at + 1) == Some(&'"') && cs.get(at + 2) == Some(&'"');
    let mut i = at + if long { 3 } else { 1 };
    let mut value = String::new();
    loop {
        let Some(&ch) = cs.get(i) else {
            return Err(format!("line {start}: unterminated string"));
        };
        match ch {
            '\\' => {
                let esc = cs
                    .get(i + 1)
                    .ok_or_else(|| format!("line {line}: dangling escape"))?;
                let unescaped = match esc {
                    '"' => '"',
                    '\\' => '\\',
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    other => {
                        return Err(format!(
                            "line {line}: escape '\\{other}' is outside the authoring subset"
                        ));
                    }
                };
                value.push(unescaped);
                i += 2;
            }
            '"' if long => {
                if cs.get(i + 1) == Some(&'"') && cs.get(i + 2) == Some(&'"') {
                    return Ok((value, i + 3, line));
                }
                value.push('"');
                i += 1;
            }
            '"' => return Ok((value, i + 1, line)),
            '\n' if long => {
                line += 1;
                value.push('\n');
                i += 1;
            }
            '\n' => return Err(format!("line {start}: newline inside short string")),
            _ => {
                value.push(ch);
                i += 1;
            }
        }
    }
}

/// Expand a prefixed name against the prefix map.
fn resolve(name: &str, prefixes: &BTreeMap<String, String>, line: usize) -> Result<String, String> {
    let Some((prefix, local)) = name.split_once(':') else {
        return Err(format!("line {line}: '{name}' is not a prefixed name"));
    };
    let Some(ns) = prefixes.get(prefix) else {
        return Err(format!("line {line}: unknown prefix '{prefix}:'"));
    };
    Ok(format!("{ns}{local}"))
}

/// Parse the controlled Turtle subset into a [`Document`].
pub fn parse(source: &str) -> Result<Document, String> {
    let toks = lex(source)?;
    let mut doc = Document::default();
    let mut p = 0;

    let expect = |toks: &[(Tok, usize)], p: usize, what: &str| -> Result<(Tok, usize), String> {
        toks.get(p)
            .cloned()
            .ok_or_else(|| format!("unexpected end of file, expected {what}"))
    };

    while p < toks.len() {
        let (tok, line) = toks[p].clone();
        if tok == Tok::PrefixKeyword {
            let (name_tok, l) = expect(&toks, p + 1, "prefix name")?;
            let Tok::Name(name) = name_tok else {
                return Err(format!("line {l}: expected prefix name after @prefix"));
            };
            let Some(prefix) = name.strip_suffix(':') else {
                return Err(format!("line {l}: prefix name '{name}' must end with ':'"));
            };
            let (iri_tok, l2) = expect(&toks, p + 2, "prefix IRI")?;
            let Tok::Iri(ns) = iri_tok else {
                return Err(format!("line {l2}: expected IRI after prefix name"));
            };
            let (dot, l3) = expect(&toks, p + 3, "'.'")?;
            if dot != Tok::Dot {
                return Err(format!("line {l3}: expected '.' after @prefix directive"));
            }
            doc.prefixes.insert(prefix.to_string(), ns);
            p += 4;
            continue;
        }

        // Statement: subject, then predicate-object groups.
        let subject = match tok {
            Tok::Iri(iri) => iri,
            Tok::Name(name) => resolve(&name, &doc.prefixes, line)?,
            other => {
                return Err(format!(
                    "line {line}: unexpected token {other:?} as subject"
                ));
            }
        };
        p += 1;
        'predicates: loop {
            let (ptok, pline) = expect(&toks, p, "predicate")?;
            let predicate = match ptok {
                Tok::Name(n) if n == "a" => RDF_TYPE.to_string(),
                Tok::Name(n) => resolve(&n, &doc.prefixes, pline)?,
                Tok::Iri(iri) => iri,
                other => {
                    return Err(format!(
                        "line {pline}: unexpected token {other:?} as predicate"
                    ));
                }
            };
            p += 1;
            loop {
                let (otok, oline) = expect(&toks, p, "object")?;
                let object = match otok {
                    Tok::Iri(iri) => Object::Iri(iri),
                    Tok::Name(n) if n == "true" || n == "false" => Object::Literal(Literal {
                        value: n,
                        lang: None,
                        datatype: Some(XSD_BOOLEAN.to_string()),
                    }),
                    Tok::Name(n) => Object::Iri(resolve(&n, &doc.prefixes, oline)?),
                    Tok::Lit {
                        value,
                        lang,
                        datatype,
                    } => {
                        let datatype = match datatype {
                            None => None,
                            Some(DatatypeRaw::Iri(iri)) => Some(iri),
                            Some(DatatypeRaw::Name(n)) => Some(resolve(&n, &doc.prefixes, oline)?),
                        };
                        Object::Literal(Literal {
                            value,
                            lang,
                            datatype,
                        })
                    }
                    other => {
                        return Err(format!(
                            "line {oline}: unexpected token {other:?} as object"
                        ));
                    }
                };
                doc.triples.push(Triple {
                    subject: subject.clone(),
                    predicate: predicate.clone(),
                    object,
                });
                p += 1;
                let (sep, sline) = expect(&toks, p, "',', ';' or '.'")?;
                match sep {
                    Tok::Comma => {
                        p += 1;
                    }
                    Tok::Semicolon => {
                        p += 1;
                        // Tolerate a trailing ';' before the closing '.'.
                        if toks.get(p).map(|(t, _)| t) == Some(&Tok::Dot) {
                            p += 1;
                            break 'predicates;
                        }
                        continue 'predicates;
                    }
                    Tok::Dot => {
                        p += 1;
                        break 'predicates;
                    }
                    other => {
                        return Err(format!(
                            "line {sline}: expected ',', ';' or '.', found {other:?}"
                        ));
                    }
                }
            }
        }
    }
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "@prefix oh: <https://ld.openhelvetia.swiss/schema/> .\n\
                        @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n";

    #[test]
    fn parses_predicate_and_object_lists() {
        let src =
            format!("{HEAD}oh:Manifest a owl:Class ;\n  rdfs:seeAlso oh:Interface , oh:Probe .\n");
        let doc = parse(&src).expect("parse");
        assert_eq!(doc.triples.len(), 3);
        assert_eq!(doc.triples[0].predicate, RDF_TYPE);
        assert_eq!(
            doc.triples[2].object,
            Object::Iri("https://ld.openhelvetia.swiss/schema/Probe".into())
        );
    }

    #[test]
    fn parses_long_strings_language_tags_and_booleans() {
        let src = format!(
            "{HEAD}oh:X a owl:Class ;\n  rdfs:comment \"\"\"line one\nline two\"\"\"@en ;\n  owl:deprecated true .\n"
        );
        let doc = parse(&src).expect("parse");
        let Object::Literal(comment) = &doc.triples[1].object else {
            panic!("expected literal");
        };
        assert_eq!(comment.value, "line one\nline two");
        assert_eq!(comment.lang.as_deref(), Some("en"));
        let Object::Literal(flag) = &doc.triples[2].object else {
            panic!("expected literal");
        };
        assert_eq!(flag.datatype.as_deref(), Some(XSD_BOOLEAN));
    }

    #[test]
    fn parses_typed_literals() {
        let src = format!(
            "{HEAD}oh:X rdfs:label \"0.1.0\"^^<http://www.w3.org/2001/XMLSchema#string> .\n"
        );
        let doc = parse(&src).expect("parse");
        let Object::Literal(lit) = &doc.triples[0].object else {
            panic!("expected literal");
        };
        assert_eq!(
            lit.datatype.as_deref(),
            Some("http://www.w3.org/2001/XMLSchema#string")
        );
    }

    #[test]
    fn rejects_blank_nodes_and_collections() {
        let err = parse(&format!("{HEAD}oh:X rdfs:range [ a owl:Class ] .\n")).unwrap_err();
        assert!(err.contains("outside the authoring subset"), "{err}");
        let err = parse(&format!("{HEAD}oh:X owl:oneOf (\"a\" \"b\") .\n")).unwrap_err();
        assert!(err.contains("outside the authoring subset"), "{err}");
    }

    #[test]
    fn rejects_unknown_prefixes() {
        let err = parse("foo:X a foo:Y .\n").unwrap_err();
        assert!(err.contains("unknown prefix"), "{err}");
    }
}
