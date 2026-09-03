# oh-coverage — ontology coverage gate and doc generator

<!-- language: English -->

**Status: 0.x draft, 2026-08-20. Purpose: masterplan L0.5, E16 no. 5e —
"Ontologie-Deckung als CI-Test": every API/manifest entity has its `oh:`
class.**

## Purpose

The `coverage` binary checks two gates and exits non-zero when either is
red:

1. **Coverage gate.** Every term EMITTED by the published artefacts —
   `context/v1.jsonld` (all mapped keys), `schema/manifest.schema.json`,
   `directory.schema.json`, `card/agent-card.schema.json` (interface
   `@type` values, enumerated values, envelope and card fields) — must be
   covered by `ontology/oh.ttl`: `oh:` terms declared, foreign terms
   referenced (imported, never re-declared), enum values documented as
   `Allowed value '<v>': ...` `skos:note`s on the owning term or its named
   range datatype. Schema fields are mapped by explicit tables in
   `src/sources.rs`; a field with no mapping is itself a failure, so adding
   a schema field turns CI red until the ontology covers it. Declared terms
   no source emits are listed: linked support vocabulary (URI-model
   classes, value datatypes) passes; a term with no link into the emitted
   core is an **orphan warning** (never a failure).
2. **Doc-drift gate.** `ontology/oh.md` is generated from `oh.ttl`; the
   binary regenerates it in memory and fails when the committed file
   differs.

## Invocation

```
cargo run --offline -- [--root <registry/standard dir>] [--write-doc]
cargo test --offline
```

Default `--root` is resolved from the crate location
(`registry/standard/ontology/coverage` -> `registry/standard`).
`--write-doc` rewrites `oh.md` instead of drift-checking it; run it after
every `oh.ttl` change and commit both files together.

## Limits

- The Turtle reader (`src/ttl.rs`) parses only the documented authoring
  subset of `oh.ttl` (no blank nodes, no collections, no `@base`);
  anything outside the subset is a hard error, never silently skipped.
  Consequence: `owl:oneOf` value lists are deferred to ontology 0.2.
- The embedded A2A card interior (`$defs.a2aCard`) is excluded from
  emission by design: A2A 1.0 owns those fields (E16 no. 6 — standards
  imported, never duplicated).
- JSON-shape helper `$defs` (`iri`, `languageMap`, `base64url`,
  `timestamp`, ...) and head metadata (`$schema`, `@context`, `@id`,
  `@type`) are documented exclusions, not entities.

## Tests

`tests/coverage_gate.rs`: full coverage and doc match on the real sources;
the gate fires on a fixture Turtle file with one declaration removed
(`tests/fixtures/oh-missing-probe-target.ttl`); doc drift fires on a
doctored doc (`tests/fixtures/oh-doctored.md`); orphans warn without
failing. Parser/model/doc unit tests live inline in their modules.
