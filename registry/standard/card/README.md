# OpenHelvetia Agent-Card Signature Package v0.1 (Draft)

<!-- language: English -->

**Status: 0.x draft, 2026-08-20 — no stability promise until ratification by the
standards commission (Art. 13 statutes). This spec belongs to the AI-interface
line, the fourth commission candidate (E16, consequence 3). Foundations:
decisions E09 (signature work package), E14 (organisation scope, succession),
E16 no. 6 (bootstrap facades).**

The signature package defines the envelope that carries an A2A 1.0 agent card
together with its trust statements. It is the `registry/standard` deliverable of
E09's consequence «Card-Schema mit getrennten Feldern Authentizität/
Attestierung».

## Scope

This spec defines: the envelope format (schema, `agent-card.schema.json`), the
two separated trust fields and their exact semantics, the JWS profile and
canonicalization, key discovery, and the revocation/drift lifecycle as
normative prose. It does **not** define the agent card's content — the card is
owned by A2A 1.0 and is strictly validated against the A2A 1.0 schema as a
separate, mandatory step (E16 no. 6: the served card is «streng 1.0-validiert,
JWS», build-generated from the root `/.well-known/openhelvetia.json`). Checker
behaviour (daily checks, quarantine mechanics) is implemented in
`registry/checker`; this spec fixes only what those checks must mean.

## Placement: what is served, what is stored

- `/.well-known/agent-card.json` serves the **pure A2A 1.0 card** — foreign
  A2A clients see nothing OpenHelvetia-specific. Its native A2A `signatures`
  member is emitted at build time from the envelope's authenticity block (the
  JWS is placement-independent, see «JWS profile» below).
- The **envelope** (this spec) is the registry's exchange and record format:
  it embeds the card and carries the two trust blocks. The root well-known
  points to both artefacts (Fächer-Modell, E16 no. 6). Registry-as-Code: the
  envelope is the versioned file in Git.

## Two fields, never one

E09 no. 5, verbatim: «Zwei Felder, nie eines: ‹Der Betreiber hat diese Card
signiert› (**Authentizität**) ist etwas anderes als ‹Der Verein hat den
Betreiber geprüft› (**Attestierung**).»

| | `authenticity` | `attestation` |
|---|---|---|
| Who signs | The **operator**, with a key published under the operator's own domain | The **Verein** (attestor), with the association's key |
| What it proves | This exact card was signed by whoever controls the domain-anchored key: content integrity plus **key continuity** | The Verein has verified the operator **out of band** at a stated level, at a stated time, with a stated expiry |
| What it does *not* prove | Legal entity. E09 no. 3: «Eine Signatur beweist Schlüsselkontinuität, nicht Rechtsträgerschaft» — the key→legal-entity binding is checked outside the system | Card content freshness — attestation refers to the card that was current when verified |
| Lifecycle | Changes with every card edit; a newly signed card is a **re-verification event** (E09 no. 2) | Changes only by an act of the Verein: verification, expiry, withdrawal |
| Revocation surface | Operator's well-known (JWKS rotation/removal) | Verein's attestor key and attestation records |

Why the two must never collapse into one field:

1. **Different signers, different claims.** One field would assert «signed and
   verified» as a single bit; the registry could no longer represent the
   normal states «signed but not (yet) verified» and «verified, then
   re-signed» — the exact states the lifecycle below runs on.
2. **Different lifecycles.** Because every newly signed card is a
   re-verification event, attestation state must be able to *lag* authenticity
   state. A merged field either forces the Verein to countersign every
   operator edit (making the association a bottleneck and its signature a lie
   about what it checked) or lets an operator edit silently void — or worse,
   silently carry over — the verification claim.
3. **Different revocation surfaces.** A compromised operator key must not
   invalidate the Verein's verification history, and a rotated attestor key
   must not un-sign every operator's card. Merging the fields merges the blast
   radii. (Cf. E14: «eine Widerrufsfläche, nicht zwei» — per identity system;
   here there are deliberately two identity systems, so two surfaces, cleanly
   separated rather than blended.)

For operators without their own domain or key management, the Verein verifies
identity out of band and **countersigns** (E09 no. 4, a board-proposal-level
governance decision — the Verein becomes a liable trust anchor). In the
envelope this is an `attestation` with level `countersigned`, never a fake
`authenticity`: the invariant «every entry is signed» holds without
fabricating operator key continuity that does not exist.

## JWS profile

**Serialization: detached, not compact.** The signature travels *with* the
card while the payload stays the card itself — the mechanism A2A 1.0 uses for
its agent-card signatures (a `signatures` member carried in the card; the
payload is reconstructed from the card, not embedded).

- Compact serialization (`header.payload.signature`) would base64url-embed the
  card: the served document becomes an opaque blob, or the card is duplicated
  — once readable, once inside the JWS — with two sources of truth that can
  drift apart. Both outcomes break the manifest principle that the served
  JSON *is* the canonical, human-readable artefact (`curl` + `jq`, meaningful
  Git diffs, Registry-as-Code).
- Detached serialization keeps exactly one card. Each signature block carries
  only `protected` (the base64url-encoded protected header) and `signature`;
  the verifier reconstructs the JWS signing input from the card bytes it
  actually received. Verification therefore **binds to the exact content
  served**: any change to the card breaks the signature, while re-serializing
  the same data (whitespace, key order) does not — see canonicalization.

**Canonicalization: JCS (RFC 8785).** The signed byte string is the JCS
canonical form of the JSON data, so the signature covers the data model, not
one accidental serialization. Rationale: JCS is a published, deterministic
canonicalization; it matches the A2A 1.0 card-signature practice, which makes
the envelope's authenticity JWS **placement-independent** — byte-identical
whether verified against the envelope's embedded `card` or against the served
`/.well-known/agent-card.json` with its `signatures` member stripped; and it
is implementable in pure Rust without a JSON-LD stack. Constraint inherited
from JCS: numbers must be IEEE-754-double representable — card and envelope
fields are strings, objects and small integers, so this costs nothing.

**Signing inputs (normative):**

- `authenticity.signature` — JWS signing input built from
  `JCS(card)`, where `card` is the embedded card object. The embedded card
  MUST NOT contain a `signatures` member (the schema forbids it); the build
  step emits the same JWS into the served card's `signatures` member, where
  A2A verifiers compute the identical payload (card minus `signatures`).
- `attestation.signature` — JWS signing input built from
  `JCS({"card": <card>, "attestation": <attestation without "signature">})`.
  The attestor thereby signs its own claims (`level`, `verifiedAt`,
  `expiresAt`, `keyRef`) **bound to the exact card it verified** — an
  attestation cannot be replayed onto a later card revision.

**Algorithms:** `ES256` and `EdDSA` (Ed25519). `alg: "none"` and all
symmetric (HS*) algorithms are rejected. The protected header MUST contain
`alg` and `kid`; `kid` MUST equal `keyRef.kid`.

## Key discovery and anchor

Keys are discovered via the organisation domain's well-known (E09 no. 1:
«Schlüssel via `/.well-known/` der Organisations-Domain; Identität verankert
in DNS und TLS. Einstiegshürde = Datei unter URL»). `keyRef.jwksUri` MUST be
an HTTPS URL under the operator's (resp. the Verein's) own domain, serving a
JWK Set; `keyRef.kid` selects the key. For OpenHelvetia-hosted organisations
the JWKS is the `jwks_uri` of the root `/.well-known/openhelvetia.json` (E16
no. 6). The anchor is DNS plus TLS — and *only* that: the out-of-band
key→legal-entity binding is what `attestation` records, never something the
authenticity signature claims by itself.

## Revocation, drift, lifecycle

The threat model (E09 no. 6): the real risk is not a forged card at
submission, but the legitimate card whose **endpoint drifts later**. A
signature at registration time cannot cover that; the daily checks must.

- **Revocation = a well-known change.** Removing or replacing a key in the
  domain's JWKS revokes it; there is no separate revocation protocol. The
  change becomes effective through the **daily checks** — clients and the
  registry learn of a compromise because the checker re-reads the well-known
  every day, not because anyone pushes a message.
- **Drift detection.** The daily check re-fetches the served card and the
  JWKS and compares canonical hashes (JCS) against the registry record. Any
  divergence — card content, key set, endpoint behaviour — is drift and
  opens an event with notification to the operator.
- **Grace and quarantine, never binary.** Listing state is a gradient with
  explicit intermediate states (Karenz/Quarantäne), never a binary
  listed/delisted flip: «ein rotiertes Zertifikat wirft keine Gemeinde aus
  dem Verzeichnis» (E09 no. 2). A signature that stops verifying moves the
  entry into grace with notification; unresolved grace escalates to
  quarantine (visibly marked, not silently removed); resolution restores the
  entry. These states are **registry-derived state, not card fields** —
  dynamic facts never live in static cards (E16 no. 6).
- **A newly signed card is a re-verification event** — not a mere recompute.
  New authenticity resets the check clock and re-opens the attestation
  question; the existing attestation remains on record but refers to the
  prior card revision until the Verein re-verifies (visually distinct from
  «verified, passed» — E14(b): a gradient, not a grey badge).

## Anchor renewal for dying domains (E14)

The domain anchor breaks by Swiss certainty, not by accident: municipal
mergers happen yearly (plus cantonal reorganisations and admin.ch subdomain
moves). If organisations A and B merge into C, two legal entities and two
domains disappear — and with them the anchors of their entries (E14(c)).
Therefore, from day one:

- Every organisation has a **stable internal ID separate from the display
  slug**, and the registry keeps a **predecessor/successor relation** (E14).
- **Anchor renewal rule:** the successor organisation publishes its own
  well-known anchor under the new domain and re-signs the inherited cards
  with its key (`authenticity` under the new anchor). The registry links old
  and new entries via the predecessor/successor relation; the old entries
  enter grace — not deletion — with a successor pointer. Because a signature
  proves key continuity and not legal succession, succession is confirmed as
  an attestation act: the Verein verifies the successor out of band and
  issues a new `attestation`. A dying domain is thus an orderly relay
  hand-over, never a trust rupture and never a silent delisting.

## Files

- `agent-card.schema.json` — JSON Schema 2020-12 for the envelope
  (`$id`: `https://ld.openhelvetia.swiss/ns/card/0.1/schema`).
- `examples/card-minimal.json` — authenticity only (signed, not yet
  verified): the minimal valid envelope.
- `examples/card-attested.json` — both fields: signed by the operator,
  attested by the Verein. **Its attestation `keyRef.jwksUri` still names
  `openhelvetia.org`, and that is deliberate** (BC remint, 26.08.2026):
  the ES256 attestation signature covers the `keyRef`, the private
  attestor key was never committed (only the public JWKS is), and so the
  vector cannot be re-signed on the new host. It is frozen evidence, like
  a sealed *Wortlaut* — the bytes record what was signed. The two places
  where the host is a live claim rather than frozen evidence DID move:
  `card-invalid-merged.json` (validated structurally, no signature over
  it) and `sign/tests/signing.rs` (signs at runtime with generated keys).
  Re-minting this vector needs a new attestor keypair, which is a key
  ceremony act (L0.7), not a sweep.
- `examples/card-invalid-merged.json` — **deliberately invalid**: the two
  fields merged into one block. It exists as a regression tripwire for E09
  no. 5 — the schema's `additionalProperties: false` must actually reject
  the merged shape, and the conformance suite proves it keeps doing so.
- `conformance/` — Rust conformance suite (serde_json + jsonschema): the two
  valid vectors pass, the merged vector fails, the schema itself is valid
  draft 2020-12.
- `sign/` — **the signing path (`oh-sign`, since 20.08.2026):** JCS
  (RFC 8785, serde_jcs =0.2.0) + detached JWS, EdDSA (ed25519-dalek
  =3.0.0) and ES256 (p256 =0.14.0), kid bound to keyRef.kid, offline
  verification against caller-provided JWK Sets (fetching is the
  checker's daily job). CLI: keygen / sign / attest / verify /
  verify-served. 14-test conformance suite (round trips both
  algorithms, JCS covers the data model — reserialization never breaks
  a signature, any content change does —, placement independence both
  directions, attestation replay guard, kid/alg profile rules).
- `examples/keys/` — the PUBLIC test JWK Sets for the vectors.

The example signature values are **real cryptographic vectors** (since
20.08.2026; previously structural-only): signed with deterministic TEST
keys (fixed seeds, committed in `sign/tests/signing.rs` — never real
trust anchors), reproducible via the documented regen test, and
verified by the house harness (tools/check.sh) on every commit.
Domains use `.example` (RFC 2606).

## Open points 0.1 → 0.2

- ~~Cryptographic test vectors with published test keys~~ — **done
  20.08.2026** (`sign/`, see Files).
- Multi-signature during key rotation overlap (two authenticity signatures,
  old and new key).
- JSON-LD context for the envelope (`oh:` mapping; the manifest signature
  fields noted in `registry/standard/README.md` land together with this).
- Attestation level vocabulary alignment with the checker's verification
  gradient (E14(b)) once `registry/checker` fixes its levels.
- Countersigning governance (E09 no. 4) is a pending board proposal; level
  `countersigned` is specified here, its operational rules are not.
