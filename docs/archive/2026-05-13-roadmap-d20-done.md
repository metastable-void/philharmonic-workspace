# 2026-05-13 ROADMAP rewrite — D20 done, §3.G trimmed

Pre-trim verbatim text of `docs/ROADMAP.md` §3.G as it stood
after D20 landed but before its substantive scope notes were
archived out.

Trimmed on 2026-05-13 because:

1. **D20 landed** — `mechanics-http-client` 0.1.0 published
   to crates.io. All four reqwest call sites migrated; runtime
   tree of all three release bins is clean of `reqwest`,
   `rustls-platform-verifier`, `rustls-native-certs`, `ring`.
2. **The "deferred to a follow-up session" caveat dissolved**
   — the cascade-bumped crates' deps moved from path-only to
   version-spec (`mechanics-http-client = "0.1"`), and each
   is independently publishable.
3. **The pre-implementation "Original scope notes" subsection
   served its purpose** — it's referenced verbatim only for
   historic context now; the implementation matches it
   closely enough that the live ROADMAP can drop the bulk
   detail and point here.

Prior trim archive:
[`2026-05-12-roadmap-d17-done-d7-spec-d18-added.md`](2026-05-12-roadmap-d17-done-d7-spec-d18-added.md).

---

## Verbatim §3.G — HTTP-client transport + TLS trust posture

### G. HTTP-client transport + TLS trust posture (1 dispatch) — DONE

Surfaced 2026-05-12 after the ring-removal work (commit
`18f1bb2`); design pivoted 2026-05-13 from "runtime-bypass
reqwest's `rustls-platform-verifier`" to a structural fix.

**The problem.** The workspace's TLS trust-store posture is
inconsistent: sqlx (Postgres/MySQL connectors,
philharmonic-api's MySQL substrate store) verifies against
the bundled Mozilla CA bundle via `webpki-roots`; reqwest
(every outbound HTTP path — mechanics-core's endpoint client,
http_forward, llm_openai_compat, and the upcoming Tier 3 LLM
connectors) verifies against the host OS trust store via
`rustls-platform-verifier` + `rustls-native-certs`. Operator
consequences: a tenant-installed corporate CA gets picked up
for HTTP outbound but not for SQL outbound; air-gapped
environments need different mitigations for each path; the
HTTP trust set drifts on OS package updates while SQL trust
is frozen at compile time.

**The earlier plan** (archived) was to keep reqwest and call
`ClientBuilder::use_preconfigured_tls(webpki_roots_config)` at
every construction site. reqwest 0.13.3's public `rustls`
feature unconditionally pulls `dep:rustls-platform-verifier`,
so the dead crate stays compiled into the binary even though
the runtime path never invokes it. Acceptable trade-off in
isolation, but reqwest is a convenience layer the framework
has outgrown — the connector impls each carry near-duplicate
client-builder + error-classification + body-reading code,
and the workspace's serious-frameworking direction is to own
this surface rather than depend on a thick general-purpose
HTTP client.

**The locked direction** (2026-05-13, Yuka): build a small
in-house HTTP-client crate `mechanics-http-client` that wraps
`hyper-rustls` + `webpki-roots` with a reqwest-shaped
convenience API. Every outbound HTTP path in the workspace
migrates to it; reqwest is dropped from the four affected
crates;`rustls-platform-verifier` and `rustls-native-certs`
exit the runtime dep tree as a natural consequence.

- **D20 — DONE 2026-05-13** Built `mechanics-http-client`
  and migrated every reqwest call site to it.

  **Outcome.** All three release binaries
  (`philharmonic-api-server`, `mechanics-worker`,
  `philharmonic-connector-bin`) now have a runtime tree free
  of `reqwest`, `rustls-platform-verifier`,
  `rustls-native-certs`, and `ring`. TLS provider is
  `aws-lc-rs` 1.16.3; trust store is `webpki-roots` 1.0.7.
  Cascaded version bumps shipped in this dispatch:
  `mechanics-core` 0.4.1 → 0.5.0 (rename
  `ReqwestEndpointHttpClient` → `DefaultEndpointHttpClient`),
  `mechanics` 0.4.2 → 0.5.0 (mechanics-core dep bump),
  `philharmonic` 0.2.0 → 0.3.0 (mechanics + connector dep
  bumps), `philharmonic-connector-impl-http-forward`
  0.1.0 → 0.2.0, `philharmonic-connector-impl-llm-openai-compat`
  0.1.2 → 0.2.0. `mechanics-http-client` itself is published
  here at `0.0.1` (workspace path-dep; crates.io bootstrap
  publish deferred to a follow-up session, at which point the
  five bumped crates can each pick up a path-and-version dep
  and become independently publishable).

  **Original scope notes:**

  **Crate placement.** `mechanics-http-client` lives in the
  Mechanics family (same independence rule as the rest of
  mechanics — MUST NOT depend on any `philharmonic-*` crate;
  Philharmonic crates depend on it, never the reverse). Lives
  as a workspace submodule mirroring the existing Mechanics
  submodule layout. crates.io reservation as a `0.0.0`
  placeholder before substantive content lands, then patch /
  minor bumps as the API stabilises. The "mechanics-" prefix
  signals ownership; the crate itself is general-purpose and
  could be consumed by anyone, but the Mechanics family
  conventions (no Philharmonic-internal references in docs /
  CHANGELOG / module names) apply.

  **API shape.** Reqwest-like convenience subset that covers
  exactly what the workspace's call sites need today:

  - `Client` / `ClientBuilder` with timeout / pool sizing /
    user agent / default headers.
  - `RequestBuilder` chainable: `.timeout()`, `.header()`,
    `.body()`, `.json()`, `.bearer_auth()`, `.send().await`.
  - `Response` with `.status()`, `.headers()`, `.bytes()`,
    `.text()`, `.json::<T>()`, `.chunk()` (streaming).
  - Body decompression: gzip, deflate, brotli (transparent on
    response).
  - Error model: thiserror-derived `Error` enum with
    `Timeout`, `Unreachable`, `Tls`, `Decode`, `Status`,
    `Cancelled` variants. Each call site re-maps these into
    its own crate's error.
  - TLS: hyper-rustls + webpki-roots only, baked at compile
    time; aws-lc-rs as the rustls crypto provider.
  - HTTP/1.1 and HTTP/2 via ALPN (matching the existing
    reqwest-based behavior).
  - No multipart, no cookies, no proxy, no redirect-following
    knobs in v1. Add later if a call site needs them.

  **Migration sites** (4 production + the test fleet):

  - `mechanics-core/src/internal/pool/api.rs:119` and the
    underlying `ReqwestEndpointHttpClient` transport in
    `mechanics-core/src/internal/http/transport.rs` — the
    bulk of the porting work, since this is the script-level
    `endpoint(...)` HTTP path.
  - `bins/philharmonic-api-server/src/executor.rs:24` — the
    mechanics-worker dispatch path.
  - `philharmonic-connector-impl-http-forward/src/client.rs` —
    the generic-HTTP connector.
  - `philharmonic-connector-impl-llm-openai-compat/src/client.rs`
    — the OpenAI-compatible LLM connector.
  - `mechanics-core/src/internal/pool/tests/{mod,queue,lifecycle}.rs`
    — 4 test-side `reqwest::Client::new()` sites in tests.

  After migration: drop `reqwest` from each of the four
  production crates' `[dependencies]`.
  `rustls-platform-verifier` and `rustls-native-certs` should
  no longer appear in `cargo tree -p <bin> --features https
  -e normal` for the three release bins.

  **Version bumps** (published crates touched):

  - `mechanics-core` 0.4.1 → 0.5.0 (minor; not a SemVer-
    visible API change strictly — the public surface stays —
    but switching the HTTP transport is operator-visible
    behavior worth flagging).
  - `philharmonic-connector-impl-http-forward` 0.1.0 → 0.2.0
    (same reasoning).
  - `philharmonic-connector-impl-llm-openai-compat` 0.1.2 →
    0.2.0 (same).
  - `mechanics-http-client` (new): published as `0.0.1`
    initial substantive version, after a `0.0.0` name
    reservation.

  **Hard constraints:**

  - No `philharmonic-*` dep on `mechanics-http-client`.
    Mechanics family stays independent.
  - aws-lc-rs is the sole crypto provider. No `ring`.
  - Trust store is webpki-roots only. No native-certs, no
    platform-verifier — those should literally not appear in
    the runtime dep tree after the migration.
  - Existing wire behavior preserved: HTTP/1.1 + HTTP/2 ALPN,
    same body decompression set, same error-classification
    semantics that the connector impls' tests assert against.
  - No public API change on the four migrated crates beyond
    error-variant additions, which take a patch/minor bump as
    appropriate.

  **Implementation approach:** Claude direct, this session
  (user override of the default Codex-dispatch path). The
  pre-existing D20 Codex prompt archive at
  [`docs/codex-prompts/2026-05-12-0002-d20-webpki-roots-only-tls-01.md`](codex-prompts/2026-05-12-0002-d20-webpki-roots-only-tls-01.md)
  is **superseded** — it described the runtime-bypass shape
  that this revision replaces. Pre-revision §3.G text
  archived alongside.

---

## Cross-references

- Live ROADMAP §3.G — trimmed `DONE` entry:
  [`docs/ROADMAP.md` §3.G](../ROADMAP.md#g-http-client-transport--tls-trust-posture-1-dispatch--done)
- Pre-revision pre-pivot §3.G content (runtime-bypass plan):
  [`2026-05-11-roadmap-completed-arc-trim.md`](2026-05-11-roadmap-completed-arc-trim.md)
  (search for §3.G).
- Superseded D20 Codex prompt:
  [`docs/codex-prompts/2026-05-12-0002-d20-webpki-roots-only-tls-01.md`](../codex-prompts/2026-05-12-0002-d20-webpki-roots-only-tls-01.md).
