# Imported vocabularies — documented term subsets

<!-- language: English -->

**Status: part of the `oh:` ontology v0.1 (0.x draft, masterplan L0.5).
As of: 2026-08-20.**

E16 no. 5e: standards are **imported, never duplicated** — `oh.ttl` contains
no triple with a foreign term as its subject; alignments run from `oh:`
terms to the vocabularies below (`rdfs:subClassOf`, `rdfs:subPropertyOf`,
`rdfs:range`, `rdfs:seeAlso`). This directory holds **no copies of foreign
ontology files**: vendored copies of imported term subsets exist only as
documentation, i.e. as the tables below. The machine-readable mirror of
this list is the `oh:importsTerm` block in the `oh.ttl` ontology header;
the coverage gate (`ontology/coverage`) checks that every foreign term the
JSON-LD context emits is actually referenced there.

## DCAT 3 (`dcat:` = `http://www.w3.org/ns/dcat#`)

Canonical spec: <https://www.w3.org/TR/vocab-dcat-3/>

| Term | Referenced as |
|---|---|
| `dcat:Dataset` | superclass of `oh:Manifest` |
| `dcat:DataService` | superclass of `oh:Interface` |
| `dcat:Catalog` | superclass of `oh:Directory` |
| `dcat:CatalogRecord` | superclass of `oh:RegistryEntry` |
| `dcat:record` | superproperty of `oh:entry` |
| `dcat:keyword` | manifest `keyword` (context-mapped, used as-is) |
| `dcat:landingPage` | manifest `landingPage` (context-mapped); see-also of `oh:documentation` |
| `dcat:endpointURL` | interface `endpoint` (context-mapped, used as-is) |
| `dcat:endpointDescription` | see-also of `oh:documentation` |
| `dcat:servesDataset` | see-also of `oh:interface` (DCAT's service-to-dataset direction) |

## Dublin Core Terms (`dct:` = `http://purl.org/dc/terms/`)

Canonical spec: <https://www.dublincore.org/specifications/dublin-core/dcmi-terms/>

| Term | Referenced as |
|---|---|
| `dct:title` | manifest `title` / directory entry `name` (context-mapped, used as-is) |
| `dct:description` | manifest and entry `description` (context-mapped, used as-is) |
| `dct:publisher` | manifest and entry `publisher` (context-mapped, used as-is) |
| `dct:license` | manifest and entry `license` (context-mapped, used as-is) |
| `dct:issued` | manifest `issued` (context-mapped, used as-is) |
| `dct:modified` | manifest `modified` (context-mapped, used as-is) |
| `dct:conformsTo` | manifest/interface `conformsTo` (context-mapped, used as-is) |
| `dct:isVersionOf` | superproperty of `oh:versionOf` |

## PROV-O (`prov:` = `http://www.w3.org/ns/prov#`)

Canonical spec: <https://www.w3.org/TR/prov-o/>

| Term | Referenced as |
|---|---|
| `prov:Entity` | superclass of `oh:AgentCardEnvelope` |
| `prov:Attribution` | superclass of `oh:Authenticity` and `oh:Attestation` |
| `prov:qualifiedAttribution` | superproperty of `oh:authenticity` and `oh:attestation` |
| `prov:Agent`, `prov:wasAttributedTo` | the unqualified side of the two attributions (imported for consumers; no own subterms in v0.1) |
| `prov:Activity`, `prov:wasAssociatedWith`, `prov:actedOnBehalfOf` | the BYO-agent audit backbone (E16 no. 2): every agent action a `prov:Activity`; imported for the L2/L3 modules, no own subterms in v0.1 |

## SKOS (`skos:` = `http://www.w3.org/2004/02/skos/core#`)

Canonical spec: <https://www.w3.org/TR/skos-reference/>

| Term | Referenced as |
|---|---|
| `skos:exactMatch` | manifest `exactMatch` (context-mapped, used as-is; municipalities link to `ld.admin.ch`, never maintained twice) |
| `skos:note` | carrier of the machine-readable "Allowed value" documentation on the named value datatypes |

## W3C Organization Ontology (`org:` = `http://www.w3.org/ns/org#`)

Canonical spec: <https://www.w3.org/TR/vocab-org/>

| Term | Referenced as |
|---|---|
| `org:Organization` | superclass of `oh:Organization` |
| `org:ChangeEvent` | superclass of `oh:ChangeEvent` (municipal mergers and registry history modelled identically, E16 no. 5b) |
| `org:changedBy`, `org:resultedFrom` | the event-to-organisation links of the E14 predecessor/successor relation (used as-is, no own subterms in v0.1) |

## ELI (`eli:` = `http://data.europa.eu/eli/ontology#`)

Canonical spec: <https://eur-lex.europa.eu/eli-register/about.html>
(ontology served at <http://data.europa.eu/eli/ontology>)

| Term | Referenced as |
|---|---|
| `eli:LegalResource` | range of `oh:legalBasis` (Fedlex ELI URIs) |

## cube.link (`cube:` = `https://cube.link/`)

Canonical spec: <https://cube.link/>

| Term | Referenced as |
|---|---|
| `cube:Cube`, `cube:Observation` | the L3 semantic view: condensed benchmark aggregates are published as cube.link cubes (E16 no. 4c); imported for the Phase-3 benchmark module, no own subterms in v0.1 |

## Meta-vocabularies

`owl:` (<https://www.w3.org/TR/owl2-overview/>), `rdfs:`
(<https://www.w3.org/TR/rdf-schema/>) and `xsd:`
(<https://www.w3.org/TR/xmlschema11-2/>) are used as the definition
language of `oh.ttl` itself (declaration types, labels/comments,
alignments, literal datatypes) — they are not domain imports and carry no
`oh:importsTerm` entries.

## Instance-data usage (scope note, 2026-08-20)

The register above documents the ONTOLOGY's vocabulary alignments and
the terms the published schema artifacts map to. Platform-emitted
INSTANCE data may additionally use well-known foreign terms without an
`oh:importsTerm` entry, provided the emission documents its vocabulary
at the emission site and re-declares nothing. First case (audit of
assignment G): the `/org/` time-model graph uses `dct:identifier`
(the association's UID) and `dct:date` (event date) — documented in
URI-MODEL.md §4. If an instance-data term later enters the ontology's
own modelling, it moves into this register with an `oh:importsTerm`
entry (candidate for ontology 0.2).
