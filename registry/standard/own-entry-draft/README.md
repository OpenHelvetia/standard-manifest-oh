# Entries no. 1 and no. 2 — the association's own MCP servers

<!-- language: English -->

> **Listed on 02.09.2026 (E21 Nachtrag).** The two manifests moved to
> `registry/entries/`; this directory holds no manifests any more. The
> document below stays as the record of why each field says what it
> says and as the home of the gated tool tables (the oh-validate suite
> reads them against the listed entries). Sentences about «parking» and
> «waiting for the endpoint» describe the state before 02.09.2026: the
> association's own infrastructure is always an entry, and what the
> directory says about it is measured — «never checked» until the first
> recorded probe.


**Two manifests are parked here** — the fedlex domain server (entry
no. 1, prepared at BD) and the LINDAS domain server (entry no. 2,
prepared at BX). Everything the first half of this document says about
*why* a manifest waits here applies to both, unchanged; the two entries
differ only in what they describe, and entry no. 2 has its own section
at the end. Both name the SAME endpoint, because both are mounted
behind the one gateway — which is the point of a gateway, and is
asserted by a gate rather than trusted.

**Anchors: sealed E21 (clauses 1–3) and `testing/standard/CONTRACT.md`
§2b.** Masterplan **L1.5** (the self-publication half) and **L2.3** (the
serving half). Serving shape and publication act:
[`docs/how-to/entry-one-publication.md`](../../../docs/how-to/entry-one-publication.md).

**These are NOT directory entries.** Each is a complete, validated
manifest waiting for one thing: an endpoint that answers publicly. It becomes an
entry the day it does, by the association itself — admission route
no. 1, self-publication, the same route open to everyone (E21;
CONTRACT §2b).

## Why the file is HERE and not in `registry/entries/`

`registry/entries/` holds real entries only, and an entry is real when
its endpoint answers. Parking the manifest in the standard's own tree
follows the precedent E21 clause 3 set when the 19 prepared manifests
moved to `test-corpus/`: a clearly non-registry location under
`registry/standard/`.

Two gates keep that true while it waits
(`oh-validate/tests/own_entry_draft.rs`, run by `tools/check.sh`):

- the draft is **never also** in `registry/entries/` — by filename and
  by `@id`, because publication is a *move* and a copy would list the
  entry while this file still says it is not live;
- `registry/entries/` holds **no manifests at all** — the honest empty
  state, measured rather than believed.

The manifest itself runs the **full four-stage chain on every commit**,
in the same invocation as the test corpus, so it cannot rot while it
waits.

## What the entry says, and why each field says it

| Field | Value | Grounds |
|---|---|---|
| `@id` | `…/registry/openhelvetia-fedlex-mcp` | Registry slug; filename = slug (canonical form rule 8). |
| `publisher` | `…/org/openhelvetia` | The association's timeless identity IRI — the same one the DCAT catalogue and the `/org/` time-model graph already use. |
| `license` | `dcat-ap.ch/…/terms_open` | The licence of what is served. The underlying Fedlex data is `terms_open`, and this service adds no restriction to it — the minimal, non-expansive claim. **Flagged:** the association's own *service terms* are not separately decided; that is a Plattformreglement question (X.2), and this field does not pre-empt it. |
| `legalBasis` | **absent** | Deliberately. The field is for a statutory basis, and this service has none: a private association's voluntary open-infrastructure service rests on no legal act. Naming the *Publikationsgesetz* here would claim the Confederation's basis for our service. |
| `conformsTo` | the dated manifest schema | The graph-visible conformance claim. `$schema` carries the same URL but is stripped before expansion (canonical form rule 8), so in RDF only `conformsTo` says it. |
| `landingPage` | `…/en/developers/` | The developer section beneath the one entry (E03 Nachtrag) — where a human reads what this server is. English as the machine default, matching the root facade's `publish` pointer. |
| `interfaces[0].endpoint` | `https://mcp.openhelvetia.swiss/mcp` | **Not a placeholder — the real one.** See below. |
| `interfaces[0].conformsTo` | MCP `2026-07-28` | The revision the gateway pins and the wire suite proves against an independent implementation. |
| `interfaces[0].auth` | `none` | The base tier is account-free (E16 «reading open», VISION §3 zero tariff). The RFC 9728 chain is *declared* for the day a tier above base needs it; when that day comes the auth declaration changes as a versioned edit. |
| `interfaces[0].tier` | `base` | The thirty-five tools are stateless navigation and structured query — the base tier by the VISION §4 *Stufen-Prinzip*. |
| `interfaces[0].probe` | `mcp-discover` / `ok` | The first manifest in the corpus to declare `mcp-discover`, and it may: `DIRECTORY.md` §4 requires per-endpoint evidence for that kind, and this endpoint is ours — the gateway pins the stateless revision, which has no `initialize` at all. |
| `issued` | the drafting date | `dct:issued` is when the manifest was issued, which is now. `modified` is stamped only if the document changes before publication. |

### The endpoint is the real identifier, on purpose

An entry that named an invented stand-in would put a **second** URL for
this service into the corpus — and there is exactly one by construction:
`RESOURCE_IDENTIFIER` in `mcp/gateway`, from which the RFC 9728
protected-resource metadata is generated and against which the web
repository's `site-config.json` is held equal by a cross-repository
test. RFC 8707 fixes that identifier because tokens are issued for it.

So the draft carries it, a third gate asserts the two never part
company, and «not live» is expressed where it cannot be forgotten: by
the file's **location**, not by a string somebody has to remember to
replace.

## The thirty-five tools

Named, never aspirational: these are the ids the gateway's capability
register actually routes, in `<domain>.<verb_object>` form (E16 Ziff. 1).
Eight close the bitemporal citation loop (the v0 spine); twelve are
the navigator surface added at BQ; thirteen complete it at BR — tables,
citations, version comparison, treaties, consultations, the Official
Compilation and the Federal Gazette; two close the citation chain at BT
— a quote proven against the read text, the canonical label of a place
(`mcp/servers/fedlex/TOOLSET-v1.md`).

| Tool | What it answers |
|---|---|
| `fedlex.resolve_sr` | SR number → ELI, with titles, status and visible predecessor matches |
| `fedlex.list_versions` | all dated consolidations of an act, future ones included |
| `fedlex.resolve_consolidation_at` | the consolidation governing at a date (the bitemporal core) |
| `fedlex.check_in_force` | in force at a date? `false` is a valid answer, not an error |
| `fedlex.read_article` | eId-precise element text of a dated consolidation (Akoma Ntoso), path eIds included |
| `fedlex.get_citations` | impact relations, direction-typed |
| `fedlex.get_law_metadata` | ELI → JOLux profile with provenance |
| `fedlex.search_law` | enactment search |
| `fedlex.get_structure` | the outline of one consolidation — sections and articles with eId, num and heading |
| `fedlex.search_text` | where a word occurs inside one consolidation; every hit names its article (a hint) |
| `fedlex.read_document` | a whole small act as capped Markdown, with a continuation offset |
| `fedlex.get_references` | the references an act's text makes, linked ELI where the corpus links it (a hint) |
| `fedlex.get_modifications` | the amendment notes per element of a consolidation, plus the mod blocks of amending acts |
| `fedlex.list_annexes` | the annexes of a consolidation with titles and the path eIds that read them |
| `fedlex.get_article_history` | which amendments and consolidations changed one article, with dates |
| `fedlex.get_subdivisions` | the subdivisions the JOLux graph knows — a gap catalogue, not an outline |
| `fedlex.get_taxonomy` | the systematic classification: notation, labels, the branch chain to the SR root |
| `fedlex.list_expressions` | the language versions and manifestations (XML, PDF) of one consolidation — PDF-only visible before a read |
| `fedlex.resolve_vocabulary_label` | a Fedlex vocabulary term by label or IRI (a hint) |
| `fedlex.find_related_topic` | acts in the same field of law via the legal taxonomy (a hint) |
| `fedlex.extract_tables` | the tables of a consolidation or of one element (annex limit values, tariffs) as header and rows |
| `fedlex.parse_reference` | a citation in plain text taken apart into act, article eId and path proposal (a hint) |
| `fedlex.compare_versions` | what changed between two consolidations: added, removed, changed paragraphs with wording |
| `fedlex.explore_node` | the edges of a JOLux node, both directions, capped — a debugging view (a hint) |
| `fedlex.detect_foreign_content` | foreign-language sections and `<foreign>` islands of a consolidation |
| `fedlex.find_treaties` | treaty processes by a title word, partner country or bilaterality (a hint) |
| `fedlex.get_treaty_info` | the profile of a treaty process |
| `fedlex.get_consultations` | the consultation procedures of an act's drafts or of one draft (a hint) |
| `fedlex.get_consultation_documents` | the position statements and result reports of one consultation |
| `fedlex.check_quote` | whether a quote's wording stands in the norm text of the element that was read — never whether it is true |
| `fedlex.cite` | the canonical Fundstelle of an eId («Art. 7 Abs. 1 Bst. b LSV») with abbreviation, SR and title |
| `fedlex.get_oc_act` | the legally binding AS/RO publication behind a consolidation |
| `fedlex.get_memorial` | the AS/BBl issue an oc publication appeared in, with its acts |
| `fedlex.get_fga_documents` | the Federal Gazette documents of an act's genesis |
| `fedlex.get_drafts` | the legislative drafts an act came from, with the Curia Vista number |

**The manifest does not list them, and that is a standard gap, not an
omission.** Manifest v0.1 has no `capabilities` field — not at document
level, not per interface — and both objects are
`additionalProperties: false`. Inventing one here would be a change to
the *standard* (a new field needs an `oh:` class, a SHACL rule, a
context term and directory pass-through — the ontology coverage gate
enforces exactly that), and the AI-interface extension is a commission
candidate (E15/E16), not something one entry's preparation decides.

What fills the gap today is better than a copied list anyway: **the
endpoint answers the question itself.** A machine that reaches it gets
the ids from `server/discover` and `meta.tools`, generated from the same
register the router uses, so they cannot drift from what is served. The
gap is recorded as an open point for the standard's v0.2
(`registry/standard/README.md`).

## Publication

One command's worth of verification, then a move. The steps, with the
verification for each and the honest-failure path — if the probe fails,
the entry does not ship — are in
[`docs/how-to/entry-one-publication.md`](../../../docs/how-to/entry-one-publication.md).

---

# Entry no. 2, prepared — the association's own LINDAS MCP server

**Prepared at BX**, the day the server was built
(`mcp/servers/lindas/`, report
[BX](../../../docs/project/reports/2026-08-30-bx-lindas-server.md)).
Same route, same location, same two gates: admission route no. 1,
self-publication, parked until the endpoint answers.

## What differs from entry no. 1, field by field

| Field | Value | Grounds |
|---|---|---|
| `@id` | `…/registry/openhelvetia-lindas-mcp` | Registry slug; filename = slug. |
| `license` | `admin.ch/gov/en/start/terms-and-conditions.html` | **The honest field, and the one that took a measurement.** Entry no. 1 says `terms_open` because the Fedlex data is `terms_open`. Here no such statement exists: none of the 44 cubes states a licence in the graph at any distance the probe looked (§2b.1 of the assessment), the I14Y record for a data SERVICE has no licence field at all, and the only written frame is the federal terms-and-conditions notice the operator's site points to — so that document is what the field names. It is not a grant of openness and the manifest never says it is; the description says «not stated at the source» in both languages, and so does every answer the server gives (`provenance.licence`). |
| `description` | carries source, access and licence in prose | **A standard gap, named as one.** Manifest v0.1 has no field for the ACCESS right, none for the source's own registration, and none for the tool ids — and both objects are `additionalProperties: false`. So the three statements the masterplan's decision requires live in the description: source attribution (Swiss Federal Archives as operator, Federal Chancellery as publisher), access «public (I14Y data service `c9cf11b6-d165-4498-92fc-d51167def66c`, `accessRights: PUBLIC` of the EU vocabulary)», licence «not stated at the source». Inventing fields would be a change to the *standard*, which is a commission decision and not one entry's to make (the same gap entry no. 1 records for capabilities). |
| `interfaces[0].endpoint` | `https://mcp.openhelvetia.swiss/mcp` | The SAME endpoint as entry no. 1: both domains are mounted behind the one gateway, which is where policy lives (E11/E16). Two entries, one door — and the gate below now holds BOTH drafts against the gateway's RFC 8707 resource identifier. |
| `interfaces[0].tier` | `base` | Eight stateless tools over structured public data; no index of its own. |
| `interfaces[0].probe` | `mcp-discover` / `ok` | Same evidence as entry no. 1: our endpoint, our pinned stateless revision. |
| `issued` | `2026-08-30` | The day the manifest was written. |

## The eight tools

The ids the gateway routes, in `<domain>.<verb_object>` form (E16
Ziff. 1) — held against the committed tool inventory by the same test
that holds entry no. 1's list
(`mcp/servers/lindas/TOOLSET-v0.md` carries the contract behind them).

| Tool | What it answers |
|---|---|
| `lindas.list_cubes` | the 44 served cubes with name, family, version, status and observation count — a published cube holding nothing says so |
| `lindas.find_cube` | the cube behind a question, by a word of its name (a hint) |
| `lindas.describe_cube` | what a cube DECLARES: its profile and the dimensions of its SHACL shape, with the honest note that the record carries more |
| `lindas.dimension_values` | the values one dimension takes, labelled — so a filter names an IRI instead of a word (a hint) |
| `lindas.observations` | the rows themselves, filtered, capped, every cell saying whether its value is stated |
| `lindas.list_versions` | the versions of a cube family — and nothing in the graph links an old version to a new one, so nothing claims it |
| `lindas.describe` | everything the store says about one IRI, counted and capped |
| `lindas.resolve_label` | an IRI of ANY host, asked of the ONE endpoint, in one language with a fallback (a hint) |

**What the server will not do, and the entry does not promise:** no
join across cubes, no graph enumeration (measured: 90 s without an
answer where the typed query answers in 223 ms), no derived verdict —
the Ständemehr is read from the row it is stated in, where counting
cantons would give a different number.
