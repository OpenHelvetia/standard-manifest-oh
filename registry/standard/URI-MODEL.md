# URI home & time model (v0.x draft — L0.2)

<!-- language: English -->

**Status:** 0.x draft, no stability promise until ratification by the
standards commission. Writes out E16 Ziff. 5a/5b; served
**statically first** per the phase cut (E16 K5) — the initial ld.*
content is build-generated under `/ns/…` on the one public entry, with
the immutable cache plan (launch runbook §3).

> **Identifier home revised (E03 *Revisions-Nachtrag*, sealed
> 24.08.2026; reminted corpus-wide 26.08.2026, assignment BC):** the
> canonical host is **`ld.openhelvetia.swiss`**. The remint was legal
> because every affected artifact was still `draft`, and it had to
> complete before publication — from first publication the byte-freeze
> binds on the new identifiers.

## 1. URI home

Canonical namespace: **`https://ld.openhelvetia.swiss/`**, mirrored
from day 1 via `https://w3id.org/openhelvetia` (302 — outage
insurance; masterplan L0.3). Cool-URIs rules apply: identity URIs
are timeless, nothing meaningful ever 404s, and **content
negotiation** serves HTML, Turtle and JSON-LD under the SAME URI
(static phase: pre-generated variants + negotiation at the edge;
the URL paths under `/ns/…` are the exact bytes of the vendored
artifacts).

## 2. Identity, version and event URIs (version.link/JOLux pattern)

- **`/org/{id}`** — timeless identity of an organisation. `{id}` is
  the stable internal organisation ID (E14: separate from the
  display slug; predecessor/successor relation).
- **`/org/{id}/version/{n}`** — citable, immutable version of the
  organisation description.
- **`/org/{id}/event/{id}`** — change events with an
  `org:ChangeEvent` mapping (municipal mergers and registry history
  are modelled identically; the E14 anchor-renewal rule hangs off
  these events).
- **`/registry/{slug}`** — timeless identity of a directory entry;
  **`/registry/{slug}/version/{semver}`** — citable entry versions,
  derived from git tags of the registry repo (registry-as-code: the
  tag IS the version event).
- **Municipalities and other federal entities are never maintained
  twice:** `skos:exactMatch` links to `ld.admin.ch` URIs; our URIs
  add only what the platform itself asserts.

## 3. Namespace artifacts (`/ns/…`, immutable)

- `/ns/manifest/v1` — the vendored JSON-LD context (versioned,
  `@protected`, no `@vocab`; a term change is a version event).
- `/ns/manifest/schema/{date}/manifest.schema.json` — the dated,
  immutable manifest JSON Schema URL: the exact `$schema` value every
  corpus manifest carries (canonical form rule 1). The emitted path is
  derived from the schema's `$id`, never invented.
- `/ns/manifest/schema/manifest.schema.json` — undated convenience
  path to the current schema, byte-identical to the dated emission.

**Serving rule (build-enforced, general):** every schema URL referenced
by a corpus file must have an emitted path; a reference without one —
typically a future dated version that is not the vendored schema — is
a build error, never a silently dangling `$schema`.
- `/ns/card/0.1/schema` — the agent-card envelope schema (L0.7).
- `/ns/directory/0.2/schema` — the directory envelope schema
  (L0.8, DIRECTORY.md).

Published `/ns/` artifacts are immutable per version (cache
`immutable, max-age=31536000`); corrections create a NEW version
URI, never a changed old one (E16 Z5c/d; RDFC build fingerprints).

## 4. Emitted organisation graph (static phase)

The first time-model instance data is the association itself, emitted
by site-gen (crate-owned static content, byte-identical across the
resource paths, deterministic across builds):

- `/org/openhelvetia` — timeless identity (`oh:Organization`;
  `org:resultedFrom` → founding event).
- `/org/openhelvetia/version/1` — citable version
  (`oh:OrganizationVersion`; `oh:versionOf` → identity).
- `/org/openhelvetia/event/founding` — the founding event
  (`oh:ChangeEvent` ⊑ `org:ChangeEvent`).

Static cut: each resource URI is materialized as `.ttl` / `.jsonld`
siblings (e.g. `/org/openhelvetia.ttl`), every file carrying the SAME
complete organisation graph — dereferencing any of the three resources
yields its full immediate context. The two serializations are
test-pinned isomorphic (oxigraph). Municipalities keep the
`skos:exactMatch` pattern of §2 and are NOT emitted: no municipality
data exists, and none is invented.

Open (external, launch runbook): the `ld.openhelvetia.swiss`
DNS/serving switch-on, which activates the extensionless URIs with
content negotiation at the edge; the emitted paths live on both
dispatch domains, ready to be fronted.
