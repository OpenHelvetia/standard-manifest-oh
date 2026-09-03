# standard-manifest-oh

> **Kurz auf Deutsch.** Das ist der Manifest-Standard von OpenHelvetia: die Form, in der eine Datenquelle für KI-Systeme beschrieben wird — als JSON-Schema, JSON-LD-Kontext, SHACL-Formen und Ontologie —, dazu der Prüfer `oh-validate` mit vier Stufen, das Siegelregister und die Werkzeuge für signierte Agentenkarten. Version 0.1, Entwurf: verbindlich erst nach Ratifikation durch die Standardkommission des Vereins. Anleitung unten auf Englisch.

**standard-manifest-oh** is the manifest standard of the association [OpenHelvetia](https://openhelvetia.swiss): how a data source is described so that an AI system can find, address and cite it. A manifest is one plain JSON document in exactly one normalised shape — a profile on DCAT-AP-CH with an extension for AI interfaces (the `oh:` vocabulary) — that is plain JSON for the person writing it and clean RDF for the machine reading it, thanks to the JSON-LD context.

**Status: version 0.1, draft.** The standard becomes binding only after ratification by the association's standards commission (Art. 13 of the statutes), which is not yet constituted. Everything here is real and tested; nothing here is promised to stay unchanged.

---

## Contents

1. [Before you start](#1-before-you-start)
2. [Get it running in five minutes](#2-get-it-running-in-five-minutes)
3. [What a manifest is](#3-what-a-manifest-is)
4. [The four stages of validation](#4-the-four-stages-of-validation)
5. [The tools](#5-the-tools)
6. [What is in this repository](#6-what-is-in-this-repository)
7. [How it is verified](#7-how-it-is-verified)
8. [When something does not work](#8-when-something-does-not-work)
9. [Where this repository comes from](#where-this-repository-comes-from)
10. [Contributing, security, licence](#contributing-security-licence)

---

## 1. Before you start

| Need | Why | How to get it |
|---|---|---|
| **Rust, stable** (rustc and cargo) | everything here is built from source | <https://rustup.rs> — one command, then open a new terminal and run `cargo --version` |
| **Git** | to clone this repository | macOS: `xcode-select --install`; Linux: your package manager; Windows: <https://git-scm.com> |

The tests need no network. Linux and macOS are what the association builds on; Windows works in principle, use WSL if in doubt.

## 2. Get it running in five minutes

**Clone**

```bash
git clone https://github.com/OpenHelvetia/standard-manifest-oh.git
cd standard-manifest-oh
```

**Run the tests** (offline)

```bash
cargo test --locked --manifest-path registry/standard/oh-validate/Cargo.toml
cargo test --locked --manifest-path registry/standard/seal/Cargo.toml
cargo test --locked --manifest-path registry/standard/ontology/coverage/Cargo.toml
cargo test --locked --manifest-path registry/standard/card/sign/Cargo.toml
cargo test --locked --manifest-path registry/standard/card/conformance/Cargo.toml
```

**Validate the shipped manifests** — the real entries, the examples and the 19-source test corpus, through all four stages:

```bash
cargo run --locked --manifest-path registry/standard/oh-validate/Cargo.toml -- registry/standard/schema/manifest.schema.json registry/entries registry/standard/examples registry/standard/test-corpus --stage2 registry/standard/context/v1.jsonld --stage3 registry/standard/shapes/manifest-v1.ttl --stage4
```

What you should see: `checked N file(s): OK`.

**Validate your own manifest** — write it, then:

```bash
cargo run --locked --manifest-path registry/standard/oh-validate/Cargo.toml -- registry/standard/schema/manifest.schema.json path/to/your-manifest.json --stage2 registry/standard/context/v1.jsonld --stage3 registry/standard/shapes/manifest-v1.ttl
```

Add `--write` to let the validator rewrite the file into the canonical form (key order, indentation, line endings) instead of only reporting the difference.

## 3. What a manifest is

One JSON file per data source, in the registry as a file under version control. Its head names a dated, immutable schema URL and exactly one context; its body says who publishes the source, under which licence, which interfaces it has (SPARQL, REST, download, MCP), how each is authenticated, which access tier it belongs to, and the minimal harmless call that proves the interface is alive (the probe). Titles and descriptions are language maps in the national languages and English.

Start from an example: `registry/standard/examples/fedlex-sparql.json` (a SPARQL endpoint of the Confederation) or `registry/standard/examples/musterwil-mcp.json` (a fictional municipality's MCP server). The binding rules — key order, the one context, dated schema, byte-exact canonical form — are in `registry/standard/README.md`; the URI model in `URI-MODEL.md`; versioning in `VERSIONING.md`; the directory envelope in `DIRECTORY.md`.

## 4. The four stages of validation

| Stage | What it checks | Flag |
|---|---|---|
| 1 | the JSON Schema (`schema/manifest.schema.json`): required fields, shapes, enumerations, unknown keys are errors | always |
| 2 | a JSON-LD round trip against the vendored context: every key maps to a term, nothing is lost or invented | `--stage2 <context>` |
| 3 | the SHACL shapes over the resulting graph: cardinalities, datatypes, allowed values | `--stage3 <shapes>` |
| 4 | the whole graph of all given documents against SPARQL invariants: no two manifests claim one identifier, publishers resolve, and the graph's fingerprint (RDFC-1.0, SHA-256) is reported | `--stage4` |

The canonical form is checked at every stage: a manifest that is valid but not canonical is reported, and `--write` fixes it.

## 5. The tools

| Tool | Purpose | Usage |
|---|---|---|
| `oh-validate` | the four-stage validator | `oh-validate <manifest.schema.json> <file-or-dir>... [--stage2 <context>] [--stage3 <shapes>] [--stage4] [--write]` |
| `oh-seal` | the seal register: records the hash of every published artifact (`artifacts.json`) and refuses drift | `oh-seal [--root <registry/standard dir>]` checks; `--update <path>` re-records a draft; `--publish <path> --date <YYYY-MM-DD>` seals |
| `coverage` | the ontology coverage gate: every term the shapes and examples use exists in `oh.ttl`, and `oh.md` matches its regeneration | `coverage [--root <registry/standard dir>] [--write-doc]` |
| `oh-sign` | agent-card signing: key generation and signing of an envelope | `oh-sign keygen --alg EdDSA\|ES256 --kid <kid> --out <dir>`; `oh-sign sign --envelope <file> --key <private.jwk> --jwks-uri <url> --signed-at <RFC3339>` |
| `card/conformance` | the agent-card conformance suite | `cargo test` in `registry/standard/card/conformance` |

Each binary is run with `cargo run --locked --manifest-path <crate>/Cargo.toml -- <arguments>`.

## 6. What is in this repository

| Path | What |
|---|---|
| `registry/standard/schema/` | the dated JSON Schema of the manifest |
| `registry/standard/context/`, `frame/` | the JSON-LD context and frame |
| `registry/standard/shapes/` | the SHACL shapes |
| `registry/standard/ontology/` | the `oh:` ontology (`oh.ttl`, its rendering `oh.md`) and the coverage gate |
| `registry/standard/directory.schema.json` | the schema of the directory envelope the website and the API both emit |
| `registry/standard/artifacts.json` | the seal register: which artifact is sealed with which hash |
| `registry/standard/oh-validate/`, `seal/`, `card/` | the tools, each a Rust crate with its tests |
| `registry/standard/examples/` | two example manifests |
| `registry/standard/test-corpus/` | 19 real-world manifests of Swiss public data sources, the standard's development and test corpus — not directory entries |
| `registry/entries/` | the directory's real entries, which the validator's tests walk |
| `LICENSE`, `NOTICE` | Apache-2.0 and the attributions |

## 7. How it is verified

The validator's own tests run every stage over entries, examples and test corpus; the seal register is checked against the bytes of every sealed artifact on every commit in the corpus; the coverage gate proves that the ontology covers every term in use (44 of 44 own terms, 11 of 11 foreign terms, 19 of 19 values) and that its rendering is current; the card tools carry a conformance suite over the A2A agent-card schema. Run the five test commands in §2 to see all of it.

## 8. When something does not work

| You see | What it means | What to do |
|---|---|---|
| `error: package … requires rustc 1.xx` | your Rust is too old | `rustup update stable` |
| the validator reports `not canonical` | the file is valid but not in the one normalised shape | run again with `--write` |
| stage 2 names an unknown key | the key is not a term of the context | check the spelling against `schema/manifest.schema.json`; extensions need a context change, not an inline one |
| stage 4 reports a duplicate identifier | two manifests claim the same `@id` | the slug is the identity; rename one |
| `oh-seal` reports drift | a sealed artifact's bytes changed | in the corpus that is a gate; here it tells you which file differs from the register |

## Where this repository comes from

The association develops all its modules in one corpus, on its own GitLab, where every change runs through a gate (formatting, Clippy without warnings, all tests, seal and drift checks). This repository is **assembled from that corpus** by the publication lane (`tools/publish-module.sh` there): it takes the crates and exactly the files their builds and tests need, runs the tests in the assembled tree, and pushes here. Each publication is one commit whose message names the corpus commit.

This copy was published from corpus commit `9a70151` on 2026-09-03.

## Contributing, security, licence

- **Issues** here are welcome: a wrong result, a missing case, an unclear sentence in this README. Please include the command you ran and what came back.
- **Changes** go through the corpus and arrive here with the next publication; a pull request here is read and carried over by hand.
- **Security reports**, in confidence: security@openhelvetia.swiss. The association answers within a working week.
- **Licence:** Apache-2.0 (`LICENSE`, attribution in `NOTICE`).
