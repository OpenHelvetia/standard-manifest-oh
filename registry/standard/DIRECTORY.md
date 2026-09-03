# Directory envelope & liveness probes (v0.2 draft — L0.8)

<!-- language: English -->

**Status:** 0.x draft, no stability promise until ratification by the
standards commission. Hardens the machine entry path from the
cold-start acceptance findings: the directory gains an envelope, a
declared authority rule and per-interface liveness-probe hints — a
cold-start agent never has to guess.

## 1. Envelope

`directory.json` is no longer a bare array:

```json
{
  "version": "0.2",
  "$schema": "https://ld.openhelvetia.swiss/ns/directory/0.2/schema",
  "authority": "manifest",
  "entries": [ … ]
}
```

Schema: [`directory.schema.json`](directory.schema.json)
(2020-12, `additionalProperties: false`; validated as a build gate).

## 2. Authority rule (declared, not implied)

The directory is **derived** data. On ANY divergence between a
directory entry and its manifest, **the manifest governs** — that is
what `"authority": "manifest"` declares machine-readably, and every
entry carries its absolute `manifest` URL (sibling principle), so
the authoritative hop is always one fetch away.

## 3. Liveness-probe hints (`interfaces[].probe`)

The minimal harmless call that proves an interface is alive:

| kind | meaning | expect |
|---|---|---|
| `sparql-ask` | `ASK {}`-class query against the endpoint | `boolean` (a SPARQL result document) |
| `http-get` | plain GET | `ok` (2xx) or `response` |
| `http-head` | HEAD | `ok` or `response` |
| `mcp-initialize` | MCP initialize handshake — **handshake-era servers** | `response` |
| `mcp-discover` | `server/discover` as the first message — **stateless-era servers** | `ok` (2xx) or `response` |

`expect: response` means ANY HTTP answer proves liveness — bot walls
and GET-hostile endpoints (GraphQL 400s on bare GET) included; this
encodes the differentiated findings of the launch link gate.

### Two MCP kinds, because MCP has two eras

MCP split its revisions into a **handshake era** (newest reachable
through `initialize`: `2025-11-25`) and a **stateless era**
(`2026-07-28`, the revision this platform pins), in which there is no
`initialize` method at all and discovery happens through
`server/discover` with a per-request envelope. The two kinds are
therefore not alternatives of taste; each names the era its endpoint
belongs to, and declaring the wrong one produces an answer that proves
nothing.

**What `mcp-discover` sends.** A POST carrying the complete
per-request envelope, every part of which a conformant server rejects
the request without:

- `Accept: application/json, text/event-stream` — both, because the
  server may answer with a stream and refuses the POST with `406`
  before any MCP logic runs when the second is missing;
- `MCP-Protocol-Version: 2026-07-28` and a matching `MCP-Method`
  header;
- a JSON-RPC body whose `params._meta` carries **both**
  `io.modelcontextprotocol/protocolVersion` and
  `io.modelcontextprotocol/clientCapabilities`.

**What counts as alive.** Under `expect: ok` a 2xx answer is required
— and a conformant stateless endpoint gives one, with a body naming
its supported revisions. Under `expect: response` any HTTP answer
counts, as everywhere else in this table.

**What it proves that `mcp-initialize` cannot.** An `mcp-initialize`
that passes is never evidence about the pinned revision, and there are
two ways that plays out. Against a server implementing the stateless
era ALONE, `initialize` is refused — an answer, so `expect: response`
passes, while every real call would fail. Against a server
implementing SEVERAL revisions on one endpoint, `initialize` succeeds
through the **handshake** era, which says nothing about whether
`2026-07-28` is served at all. Both were measured on the wire (E09);
the second against this platform's own gateway, which answers both
eras. `mcp-discover` asks the one question the pinned era actually
answers, so a 2xx is evidence about that era rather than about the
transport or about a different era. The reverse holds too, which is
why `mcp-initialize` stays: against a handshake-era server it is the
honest probe, and `mcp-discover` would be the one asking a question
that era has never heard of.

These semantics are not derived from reading the specification. They
were established on the wire against an independent implementation
and are pinned as cases in `testing/wire` (E09); the checker builds
both requests from one code path, so the two kinds cannot drift apart
in what they send.
Optional `target` names a probe URL when it differs from the
endpoint. The hints live in the manifests (schema: optional
`probe` per interface; context terms `probe/kind/expect/target`) and
pass through into the directory.

## 4. Migration state

The manifest schema and context carry the probe terms; the full
corpus (test corpus and any real entries alike) is migrated with
type-appropriate defaults and passes validation stages 1–2
end-to-end. The E21-gated corpus relocation does not change this
spec — wherever a manifest lives, the same rules apply.

**One migration is declared and not yet done: the MCP era.** §3 above
carries the reason in full and is not restated here — every MCP probe
in the corpus today declares `mcp-initialize`, which names the
handshake era. Whether that is the right kind depends on the endpoint,
and nothing in this repository knows which era a given third-party MCP
endpoint serves. So the migration is **per endpoint and
evidence-based**: an entry moves to `mcp-discover` when its endpoint
has been shown to answer there, never by a bulk edit of the corpus.
Until then the declared kind stays as it is, which is honest — a probe
that names the wrong era proves nothing, and so does a probe changed on
a guess.
