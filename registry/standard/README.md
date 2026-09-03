# OpenHelvetia Manifest Standard v0.1 (Draft)

**Status: 0.x draft, 15.08.2026 — binding only after ratification by the
standards commission (Art. 13 statutes). Foundations: decisions E15 and
E16 ([docs/decisions/](../../docs/decisions/index.md)), research
ch. 8–9 ([docs/explanation/research.md](../../docs/explanation/research.md))
— pointer curated 21.08.2026; the former ENTSCHEIDE.md/RECHERCHE.md
targets are stubs since the Diátaxis move.**

<!-- language: English (authoritative since 2026-08-21; German original in
     git history — E20 migration batch 1, prevalence rule flipped) -->

The manifest is the association's first standard product: every entry in
the directory is a versioned manifest file in git (registry as code).
It is a profile on **DCAT-AP-CH** with an **AI-interface extension**
(the `oh:` vocabulary) — and thanks to the JSON-LD context it is plain
JSON for contributors and clean RDF for machines at the same time.

## The canonical form (binding rules)

A manifest is plain JSON in **exactly one** normalized shape:

1. **Head:** `"$schema"` (a dated, immutable schema URL) and
   `"@context"` (exactly one string: `https://ld.openhelvetia.swiss/ns/manifest/v1`).
   No inline contexts, no context arrays — extensions happen through
   new context versions (`/v2`), never through local overrides.
2. **Identity:** `"@id"` is mandatory and a stable HTTPS URI
   (`https://ld.openhelvetia.swiss/registry/{slug}` for
   association-maintained entries; third-party sites use their own
   domain). `"@type"` is mandatory.
3. **Multilingualism:** `title`, `description`, `keyword` are language
   maps (`{"de": …, "fr": …}`) with language codes per Fedlex (de, fr,
   it, rm, en). At least one language; German recommended.
   `title`/`description` carry **one string** per language, `keyword`
   **one array** per language — exactly one shape per field.
4. **References are IRIs:** `publisher`, `license`, `landingPage`,
   `endpoint`, `docs`, `conformsTo`, `legalBasis`, `exactMatch` are
   typed `@id` terms — HTTPS strings that become links in the graph.
5. **No `@vocab`, `@protected` active:** Unknown keys drop out during
   expansion and are a validation error — typos never silently become
   junk IRIs (`additionalProperties: false` in the schema).
6. **Time:** `issued`/`modified` as ISO dates. Citable versions come
   from git tags (`/registry/{slug}/version/{semver}`), never from
   editing history (time model: version.link/JOLux, decision 16).
7. **Byte canonics (since 20.08.2026):** The file is byte-identical to
   its canonical serialization — fixed key order (schema declaration
   order; language maps in Fedlex order de/fr/it/rm/en), 2-space
   indent, LF, trailing newline. The check is part of oh-validate
   (stage 1b); normalization via `--write`. Git diffs thereby carry
   content, never style.
8. **`$schema` is metadata, not graph content:** Before expansion and
   round trip (stage 2), `"$schema"` is removed and afterwards
   prepended again unchanged — it never becomes part of the RDF graph.
   In addition: **filename = slug of the `@id`**, and `@id`s are unique
   registry-wide (both checked by `oh-validate`).

## Validation chain (CI, fully Rust — tool: `oh-validate`; v0.4 covers all stages 1–4 plus slug/uniqueness checks)

> **Stage 4 implemented since 20.08.2026** (`--stage4`, requires
> `--stage2`; runs only on an error-free corpus): All documents are
> merged into ONE registry graph (blank-node labels prefixed per
> document — otherwise generator labels of different documents would
> collide and nodes would silently merge). The graph receives (a) an
> **RDFC-1.0 fingerprint** (SHA-256, Oxigraph =0.5.9 / sha2 =0.11.0) —
> deterministic across runs AND across merge order (suite-pinned) —
> and is checked (b) against the **critical invariants as SPARQL
> queries** (the E16 doubling): every violation names its offending
> nodes precisely — exactly the diagnostics rudof does not deliver for
> nested `sh:node` violations (documented stage-3 finding).

> **Stage 3 implemented since 20.08.2026** (`--stage3
> <shapes/manifest-v1.ttl>`, requires `--stage2`): SHACL validation of
> the graph via rudof (=0.3.8) against the vendored shapes — the
> pipeline expand→flatten→N-Triples fully offline (rudof never parses
> JSON-LD, never resolves the net). Own conformance suite (E16
> obligation, 17 tests): every shape rule has a negative case that
> provably fires. **Documented validator findings:** (1) json-ld emits
> quads only after flattening (node map); (2) rudof reports nested
> `sh:node` violations only as the outer NodeConstraintComponent — the
> precise inner diagnostics come from the stage-4 SPARQL doubling.

> **Stage 2 implemented since 20.08.2026** (`--stage2 <context/v1.jsonld>`):
> Round trip `compact(expand(doc)) == doc` against the vendored
> context, offline loader by construction (foreign IRIs are refused),
> dropped terms are named. **Framing note:** json-ld 0.21.4 implements
> no framing; the frame semantics (`@embed: @always` for `interfaces`)
> is structurally guaranteed (blank nodes) and enforced as an
> invariant — interface nodes with `@id` are an error (schema watch:
> observe json-ld framing support).

1. **jsonschema** against `schema/manifest.schema.json` (structure,
   required fields, `additionalProperties: false`).
2. **json-ld round trip:** `compact(expand(doc), frame) == doc` —
   enforces the one canonical form; dropped-terms check.
3. **SHACL** (rudof, with its own conformance test suite; DCAT-AP-CH
   shapes as the base + own shapes; Jena-Docker as the documented
   fallback).
4. **Whole graph** (Oxigraph): merge of all documents, RDFC-1.0 hash
   as the build fingerprint, critical invariants as the SPARQL
   doubling; the platform's read-only store builds the same graph
   deterministically from git.

Context, frame, schema and shapes live versioned in this repo; all
processors run with an offline loader on the vendored copies — never
net resolution.

## AI-interface extension (`oh:`)

Every interface of the holding is a typed object in `interfaces`:

| `@type` | Meaning |
|---|---|
| `McpInterface` | MCP server (spec version in `conformsTo`) |
| `SparqlInterface` | SPARQL endpoint |
| `RestInterface` | REST API (OpenAPI reference in `docs`) |
| `DownloadInterface` | Dumps/distributions |

Mandatory per interface: `endpoint`, `auth` (`{"authType": "none" | "apikey" |
"oauth2" | "other"}`). Optional: `docs`, `conformsTo`, `tier` (tier
principle, VISION §4: `"base"` — stateless/cheap, `"semantic"`,
`"generative"`).

## URI and namespace conventions (decision 16)

- Canonical data host: `https://ld.openhelvetia.swiss` (mirror:
  `https://w3id.org/openhelvetia/…`).
- Vocabulary: `oh:` = `https://ld.openhelvetia.swiss/schema/` — classes
  UpperCamelCase, properties lowerCamelCase; published as a Turtle
  master with generated documentation (work package `oh:` ontology
  v0.1).
- Context: `https://ld.openhelvetia.swiss/ns/manifest/v1` — immutable
  (`Cache-Control: immutable`), changes only as `/v2`.
- Organisations: `/org/{id}` timeless + `/org/{id}/version/{n}` +
  `/org/{id}/event/{id}` (corrected 20.08.2026 — aligned with
  URI-MODEL.md §2, the governing specification; the emission follows
  it); municipalities via `skos:exactMatch` to
  `https://ld.admin.ch/municipality/{bfs-nr}` — never maintained
  twice.

## Files

> **Pre-publication amendment note (26.08.2026) — the BC remint:** the
> sealed E03 *Revisions-Nachtrag* of 24.08.2026 makes
> `openhelvetia.swiss` the SINGLE public entry and
> **`ld.openhelvetia.swiss`** the home of the permanent identifiers.
> Every `ld.openhelvetia.org` IRI in this standard was therefore
> reminted **in place**: legal precisely because every affected
> artifact is still `draft`, and required to complete BEFORE
> publication — from first publication the byte-freeze binds on the
> NEW identifiers. Amended: `context/v1.jsonld`,
> `schema/manifest.schema.json`, `directory.schema.json`,
> `card/agent-card.schema.json`, `shapes/manifest-v1.ttl` and
> `ontology/oh.ttl` — each carrying the note in the artifact itself
> (`$comment` for JSON, head comment for Turtle). `ontology/oh.md` is
> GENERATED and was regenerated from the amended master rather than
> edited; `frame/v1.frame.jsonld` carries no in-file note because a
> `$comment` key in a JSON-LD frame would be an undefined term, and this
> note is its record. The `examples/` and `test-corpus/` manifests moved
> with the same remint. Every touched row was re-recorded with
> `oh-seal --update` in the same commit. **The stage-4 graph fingerprint
> moves with this change** — the graph carries these IRIs; the current
> value is quoted in the handoff, history stays in git.

> **Pre-publication amendment note (23.08.2026):** `directory.schema.json`,
> `schema/manifest.schema.json`, `ontology/oh.ttl` and
> `shapes/manifest-v1.ttl` were amended **in place** with the probe kind
> `mcp-discover`, while nothing external consumes them yet — the
> immutability rule binds from first publication. `context/v1.jsonld`
> needed no amendment: it names the probe KEYS, never the value set, so
> the closed list lives in the schemas, the ontology and the shapes and
> nowhere else. Grounds: the E09 wire suite established against an
> independent implementation that the pinned MCP revision has no
> `initialize` method, so `mcp-initialize` cannot prove liveness on a
> stateless-era endpoint — the missing piece was vocabulary, not
> checker code. `mcp-initialize` STAYS: it is the honest probe for a
> handshake-era server. Each amended file carries the note as a
> `$comment` (JSON) or a head comment (Turtle); every row was
> re-recorded with `oh-seal --update` in the same commit.

> **Pre-publication amendment note (20.08.2026):** `context/v1.jsonld`
> and `schema/manifest.schema.json` were amended **in place** with the
> probe terms (`probe`/`kind`/`expect`/`target`) while nothing external
> consumes v1 yet — the immutability rule binds from first
> publication. Deliberately NO re-versioning: a `/v2` would have
> churned every `$schema`/`@context` reference for nobody before the
> first consumer. Both files carry the same note as `$comment` in the
> file head; from first publication onward: change = new version,
> never an edit.

- `context/v1.jsonld` — the JSON-LD context (vendored reference copy).
- `frame/v1.frame.jsonld` — the frame for the round-trip check.
- `schema/manifest.schema.json` — JSON Schema 2020-12.
- `oh-validate/` — Rust validator (v0.4: stages 1–4 + slug/uniqueness
  checks; full invocation: `--stage2 ../context/v1.jsonld --stage3
  ../shapes/manifest-v1.ttl --stage4`; debug via `OH_STAGE2_DEBUG=1` /
  `OH_STAGE3_DEBUG=1`).
- `shapes/manifest-v1.ttl` — SHACL shapes v1 (closed node shapes,
  stage 3).
- `examples/` — example manifests (verify endpoint details before
  publication; the examples are work in progress, not assurances).
- `URI-MODEL.md` · `DIRECTORY.md` · `directory.schema.json` · `card/` —
  URI/time model (L0.2), directory envelope with authority rule and
  probe hints (L0.8), agent-card spec (L0.7); all 0.x.
- `VERSIONING.md` · `artifacts.json` · `seal/` — versioning and
  sealing rules of the published artifacts incl. the byte-drift gate
  (`oh-seal`, L0.6): draft changes only with a conscious re-hash in
  the same commit, published artifacts byte-frozen (change = new
  version), closed world over all artifact locations; runs in
  tools/check.sh on every commit.
- `ontology/` — the `oh:` ontology v0.1 (Turtle master, generated
  documentation, imports register, coverage gate; fourth commission
  candidate, L0.5).

## Open points v0.1 → v0.2

*(Curated 21.08.2026 — three of the four original points have since
been BUILT and moved to their delivered homes:)*

- ~~Semantic mapping `oh:Manifest` ↔ `dcat:Dataset`/`dcat:DataService`~~
  — **delivered** in the `oh:` ontology v0.1 (`ontology/oh.ttl`:
  Manifest ⊑ dcat:Dataset, the four interface classes ⊑
  dcat:DataService via oh:Interface; imports referenced, never
  duplicated).
- ~~SHACL shapes first release + rudof conformance test suite~~ —
  **delivered** as validation stage 3 (`shapes/manifest-v1.ttl` +
  the 17-test conformance suite).
- ~~Signature fields (JWS, decision 9)~~ — **delivered at card level**
  (`card/` spec + `card/sign/` signing path with real vectors);
  still open for v0.2: the MANIFEST-level signature fields for
  third-site self-publication (platform Phase 2) and their JSON-LD
  context terms (noted with the card spec's open points).
- Tier/price declaration in detail (zero-tariff base, decision 16
  no. 2) — still open; the board proposal drafts
  (docs/project/vorlagen/) carry the zero-tariff anchoring side.
- **Capability declaration per interface** — found by preparing the
  association's own entry (BD): v0.1 has no way to say *which* tools an
  `McpInterface` serves. There is no `capabilities` field at document
  level or per interface, and both objects are
  `additionalProperties: false`, so an entry cannot name its tool ids
  at all. Today the endpoint answers the question itself
  (`server/discover`, `meta.tools`), which is better than a copied list
  because it cannot drift from what is routed — so this is a real gap
  and not an urgent one. **Adding it is a standard act, not an entry
  act:** a new field needs an `oh:` class, a SHACL rule, a context term
  and directory pass-through (the ontology coverage gate enforces
  exactly that), and the AI-interface extension is a commission
  candidate (E15/E16). Recorded here rather than worked around in a
  manifest.
