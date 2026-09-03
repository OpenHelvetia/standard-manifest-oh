# Versioning and sealing of published standard artifacts (L0.6, 0.x)

<!-- language: English -->

**Status: 0.x draft, 2026-08-20 — binding for the working corpus now,
normative after Standard-Kommission ratification (Art. 13 Statuten).
Anchors: E16 Ziff. 5c/5d (immutable versioned contexts, RDFC build
fingerprint), E16-K5, the register-seal mechanics pattern
(ENTSCHEIDE), working-rules §4/§7.**

The standard's artifacts — context, schemas, frame, shapes, ontology —
are the contract every consumer builds against. This document fixes
the rules; `artifacts.json` + the `oh-seal` gate enforce them
mechanically on every commit (tools/check.sh).

## The two states

| State | Meaning | Change rule |
|---|---|---|
| `draft` | Pre-publication: nothing external consumes the artifact | Changes are legal, but every change re-records its SHA-256 via `oh-seal --update <path>` **in the same commit** — the gate turns silent drift into a conscious act |
| `published` | Served under its canonical IRI; external consumers exist | **Byte-frozen.** Any modification is a hard gate error. Change = a NEW version (new path, new IRI), never an edit |

Publication is explicit and dated: `oh-seal --publish <path> --date
YYYY-MM-DD` — it refuses stale hashes (sealing binds to consciously
recorded bytes, never to whatever lies on disk) and stamps
`published_on`. Published rows are never removed; a superseded version
keeps its row (and its bytes) forever.

## Closed world

Every file in the artifact locations MUST have a register row —
nothing ships unsealed by omission. The locations are **data, not
code** (Reflexionsschleife lens 4, 2026-08-21): each register carries
its own `locations` object (`dirs` with per-directory extension
filters, plus explicit `files`), and the gate scans exactly what the
register declares. A new artifact location is a rule change: extend
the register's `locations` and this document together. **The root is
closed world too** (Reflexionsschleife AM, material finding): every
register MUST carry a `{"path": ".", "extensions": […]}` rule covering
the artifact extensions its home seals — the gate treats a register
without one as a finding, because the root is exactly where a young
standard's first artifacts are born, and the failure mode was silence.
The register file itself is the one root file that never needs a row. The same gate
now also governs `testing/standard/` via its own register there —
`status: superseded` marks a row byte-frozen by supersession
(no `published_on`; any edit is a hard error, `--update` refuses).

## Version and IRI rules

- **Contexts:** `/ns/manifest/v1` — immutable from first publication,
  served with `Cache-Control: immutable`; extensions become `/v2`,
  never in-place edits (canonical-form rule 1).
- **Schemas:** dated URLs
  (`/ns/manifest/schema/2026-08-15/manifest.schema.json`) — the date
  IS the version; a changed schema gets a new date.
- **Ontology:** `owl:versionIRI` + SemVer inside `oh.ttl`;
  deprecation only with a transition period (E16 Ziff. 5e), never
  removal.
- **Pre-publication amendments** (the one legal in-place change,
  precedent 2026-08-20): allowed only while status is `draft`, and
  only with a dated note in the standard README **and** a `$comment`
  in the artifact itself, plus the same-commit `--update`.

## Relation to the stage-4 fingerprint

Two seals, two objects: the **stage-4 RDFC-1.0/SHA-256 fingerprint**
(oh-validate) seals the *content* of the registry graph — invariant
under formatting and blank-node labels. The **artifact register**
seals the *bytes* of the standard's contract files — exactly what a
consumer's cache or `$schema` reference sees. Both run in
tools/check.sh on every commit; the build fingerprint is published
with the site (rides on L0.6/L0.2 serving).

## Gate usage

```
cargo run --manifest-path registry/standard/seal/Cargo.toml -- --root registry/standard            # check (CI/hook)
cargo run --manifest-path registry/standard/seal/Cargo.toml -- --root registry/standard --update <path>
cargo run --manifest-path registry/standard/seal/Cargo.toml -- --root registry/standard --publish <path> --date YYYY-MM-DD
```
