# Crates and Ownership

This document describes the **architectural** crate layout —
what each crate owns and which other crates it depends on. It
does **not** pin versions; use `./scripts/crate-version.sh
--all` for current local versions and `./scripts/xtask.sh
crates-io-versions -- <crate>` for crates.io state.

## Substantive crates

The workspace is divided into the following groups. Each
crate is published on crates.io as a substantive
implementation unless otherwise noted.

### Cornerstone and storage

- **`philharmonic-types`** — cornerstone vocabulary
  (`Entity`/slot model, `Uuid`, `EntityId`, scalar/content
  types). Many crates depend on it; revisions follow the
  strict-end of the workspace versioning discipline.
- **`philharmonic-store`** — storage substrate traits:
  `EntityStore`, `EntityStream`, `EntityRevision`. Defines
  the interface; no concrete backends.
- **`philharmonic-store-sqlx-mysql`** — sqlx-MySQL backend
  for `philharmonic-store`. Carries schema migrations.

### Execution substrate

- **`mechanics-config`** — Boa-free schema types
  (`MechanicsConfig`, `HttpEndpoint`, URL/header/retry
  supporting types) plus structural validation. Depends on
  `serde`/`serde_json` only; no Boa, no philharmonic crates.
  Exists so the lowerer (in `philharmonic-api-server`) can
  produce `MechanicsConfig` values without pulling in Boa.
- **`mechanics-core`** — JS executor library wrapping Boa.
  Depends on `mechanics-config` and adds Boa GC trait wrapper
  newtypes. No philharmonic dependencies.
- **`mechanics`** — HTTP worker binary wrapping
  `mechanics-core`. One of the deployment binaries.

### Policy and workflow

- **`philharmonic-policy`** — tenants, principals,
  per-tenant endpoint configs (`TenantEndpointConfig`) with
  SCK AES-256-GCM at-rest encryption (credentials only —
  the `implementation` name is a plaintext content slot;
  see doc 09), roles, role memberships, minting
  authorities, audit events; `pht_` long-lived API token
  format.
- **`philharmonic-workflow`** — orchestration engine. Three
  entity kinds (`WorkflowTemplate`, `WorkflowInstance`,
  `StepRecord`) with append-only revision-based state
  evolution. `SubjectContext` / `SubjectKind` for caller
  attribution, reusing `philharmonic-policy`'s `Tenant` and
  `MintingAuthority` markers. Async trait boundaries
  (`StepExecutor`, `ConfigLowerer`) keep the engine
  transport- and lowerer-naive. `WorkflowEngine<S, E, L>`
  implements the execution sequence, the five-state
  lifecycle with terminal-state immutability, and the
  architecturally-enforced step-record audit discipline
  (persisted subject drops `claims` and `tenant_id` by type
  construction).

### Connector framework

- **`philharmonic-connector-common`** — shared vocabulary
  for the connector layer: COSE token and payload types
  (`ConnectorTokenClaims`, `ConnectorSignedToken`,
  `ConnectorEncryptedPayload`), realm model (`RealmId`,
  `RealmPublicKey`, `RealmRegistry`),
  `ConnectorCallContext` (verified claims delivered to
  implementations), and the shared `ImplementationError`
  taxonomy. Types and crypto contract only.
- **`philharmonic-connector-client`** — crypto/minting
  primitives: COSE_Sign1 token minting and COSE_Encrypt0
  payload encryption (hybrid ML-KEM-768 + X25519 +
  AES-256-GCM). Pure crypto library; **does not** read
  policy storage and **does not** implement
  `ConfigLowerer`. The full lowerer (which fetches
  `TenantEndpointConfig`, decrypts SCK, assembles
  `{realm, impl, config}`, and calls the connector-client
  primitives) lives in the API server binary.
- **`philharmonic-connector-router`** — pure HTTP
  dispatcher library used by the deployment binary to
  forward by-realm requests upstream.
- **`philharmonic-connector-service`** — service framework
  for connector service binaries: token verification,
  payload decryption, `ConnectorCallContext` construction.
  **Does not** host the `Implementation` trait registry —
  the registry/dispatch lives in the deployment binary that
  embeds the framework plus the implementations it serves.
- **`philharmonic-connector-impl-api`** — non-crypto
  trait-only crate hosting `#[async_trait] Implementation`
  plus re-exports of `ConnectorCallContext`,
  `ImplementationError`, `JsonValue`, and the `async_trait`
  macro.

### Connector implementations

Substantive (production):

- **`philharmonic-connector-impl-http-forward`** — generic
  HTTP-forwarding connector. Reuses
  `mechanics_config::HttpEndpoint` for config.
- **`philharmonic-connector-impl-llm-openai-compat`** —
  OpenAI-compatible LLM connector covering OpenAI / vLLM /
  compatible gateways with `openai_native`, `vllm_native`,
  and `tool_call_fallback` dialects.
- **`philharmonic-connector-impl-sql-postgres`** —
  sqlx-postgres-backed `sql_query`.
- **`philharmonic-connector-impl-sql-mysql`** —
  sqlx-mysql-backed `sql_query`.
- **`philharmonic-connector-impl-vector-search`** —
  stateless in-memory cosine kNN `vector_search`,
  corpus-per-request.
- **`philharmonic-connector-impl-embed`** — pure-Rust
  `tract` + `tokenizers` `embed` with a default-bundled
  BAAI/bge-m3 ONNX model gated behind the
  `bundled-default-model` Cargo feature.
- **`philharmonic-connector-impl-email-smtp`** — `email_smtp`
  over SMTP submission via `lettre` on rustls (aws-lc-rs +
  webpki-roots; no `native-tls`, no `ring`). Port 25 rejected
  at config validation; `connection_mode ∈ {starttls, smtps,
  auto}` with port-driven defaults and 587-then-465
  auto-discovery in `auto`; four-valued `tls_strictness`
  (`strict` / `sloppy` / `opportunistic` /
  `opportunistic_sloppy`); minimal MIME envelope fixing
  (`MIME-Version` / `Date` / `Message-Id` / `Content-Type` /
  CRLF normalisation) with no security-relevant header
  rewriting.
- **`philharmonic-connector-impl-dns`** — `dns_query` via the
  host's stub resolver through the in-tree `mechanics-dns`
  crate (no direct `hickory-*` dep). `IN`-class only;
  per-endpoint `allowed_types` / `allowlist_zones` /
  `blocklist_zones` policy gates fire **before** any DNS
  packet leaves the process (denied queries have no
  observable network side-effect); per-call timeout clamped
  to `[100, 60000]` ms with a 5000 ms default; responses
  return RDATA in DNS presentation form.

Placeholders (deferred Tier 3 — names reserved on crates.io
as `0.0.x` placeholders, no substantive implementation yet):

- **`philharmonic-connector-impl-llm-anthropic`** (Tier 3).
- **`philharmonic-connector-impl-llm-gemini`** (Tier 3).

### Workspace internal

- **`inline-blob`** — proc-macro emitting `static [u8; N]`
  items into `.lrodata.<name>` ELF sections (with anchor in
  `.lbss.<name>`) so multi-gigabyte blobs can be
  `include_bytes!`-d into ELF binaries without triggering
  rust-lld's small-code-model 32-bit relocation overflow.
  Consumed by `philharmonic-connector-impl-embed`.
- **`mechanics-http-client`** — workspace's single outbound
  HTTP client. `hyper-rustls` + `webpki-roots` +
  `aws-lc-rs`; optional `http3` feature for opportunistic
  HTTP/3 (HTTPS RR discovery + Alt-Svc caching + fallback).
  Added 2026-05-13 (D20 / D22 client). All runtime
  outbound-HTTP call sites in the three release bins go
  through this crate; `reqwest` is no longer in the
  workspace dep tree. H3 failure-convergence discipline:
  every H3 attempt is wrapped in an RAII cancellation guard
  that inserts a negative-cache entry on Drop if the attempt
  future is cancelled before completing (so an outer
  endpoint timeout that aborts an in-flight H3 request still
  marks the origin H3-unhealthy), and negative-cache
  insertion also evicts the origin's `Alt-Svc` entry (so a
  server that disabled H3 after previously advertising it
  doesn't get re-tried via the stale Alt-Svc route every
  time the negative-cache window expires). H3 streamed
  response bodies retain the h3 `SendRequest` owner until
  body completion/drop so response streaming does not
  prematurely close the underlying H3 connection.
- **`mechanics-http-server`** — opt-in HTTP/3 (QUIC)
  listener + Alt-Svc tower middleware that runs alongside
  the existing hyper-driven HTTP/1.1+HTTP/2 listener.
  Caller-supplied cert chain + private key; `aws-lc-rs`
  rustls provider only; activation gated by
  `bind_h3: Option<SocketAddr>`. Added 2026-05-13 (D22
  server-lib).
- **`dockerlet`** — dev-tooling: minimal Docker
  test-container helper, thin wrapper over `bollard` with
  deliberately narrow features (Unix socket only, no
  `home`, no `ssl_providerless`, no `rustls-native-certs`).
  Used by SQL connector + e2e integration tests as a
  lightweight alternative to `testcontainers` (which was
  evicted from the workspace dep tree during the §3.J
  cleanup pass). Added 2026-05-13 (D23). `libc::atexit`
  cleanup hook + Docker `auto_remove: true` so containers
  don't leak across test runs.
- **`mechanics-h3-quinn`** — vendored fork of upstream
  `h3-quinn 0.0.10` with the `quinn` dep pinned to drop the
  upstream `rustls-ring` default. Eliminates the last
  `ring` wrapper exception from the workspace's TLS
  posture. **Unique shape**: lives in-tree as a workspace
  member, NOT a git submodule, but IS published to
  crates.io (first crate in the workspace with this
  shape). Maintained via the `./scripts/xtask.sh
  vendor-upstream` bin (reads `vendor/vendor.toml`; 3-day
  release-age cooldown; SHA-256 verify against crates.io
  index). The hand-written `Cargo.toml` is preserved
  across re-vendor; `src/` is overwritten from upstream
  tarballs. Added 2026-05-14. Consumed by
  `mechanics-http-client` and `mechanics-http-server` via
  the cargo `package = "mechanics-h3-quinn"` rename so
  consumer code keeps writing `use h3_quinn::*`.
- **`mechanics-dns`** —
  hickory-resolver wrapper with the shared
  [Cloudflare fallback resolver set](08-connector-architecture.md#cloudflare-fallback-resolver-set)
  for `/etc/resolv.conf` ENOENT. Provides generic DNS
  query, HTTPS-RR lookup, and A/AAAA lookup APIs;
  `IN`-class only. Same in-tree
  non-submodule + published shape as `mechanics-h3-quinn`
  (lives at `./mechanics-dns/` in the parent repo; no
  separate git submodule lifecycle). Consumed by
  `mechanics-http-client` (replacing its previous
  `tokio::net::lookup_host` and inline
  `hickory_resolver::TokioResolver` calls) and by
  `philharmonic-connector-impl-dns` (D19) when D19
  dispatches. Added 2026-05-15 under ROADMAP §3.L / D26.

### API and meta

- **`philharmonic-api`** — public HTTP API library (axum
  routes, middleware, executor wiring). Consumed by the API
  binary.
- **`philharmonic`** — meta-crate / WebUI asset host.
- **In-tree binaries** (under `bins/`, never published):
  `philharmonic-api-server`, `philharmonic-connector`, and
  the workspace-internal `xtask/` crate. Under
  [§02 Bins are thin](02-design-principles.md#bins-are-thin)
  these own only Clap CLI + `main()` glue. Shared
  deployment helpers live at `philharmonic::server`
  (feature-gated by `server` / `server-key-material` /
  `server-https`; the last is separate from the
  mechanics-runtime `https` feature). The mechanics-worker
  workflow executor lives at
  `philharmonic_api::MechanicsWorkerExecutor` behind the
  `mechanics-worker-executor` feature (forwarded via the
  meta-crate's `api-mechanics-worker-executor`). Remaining
  extraction candidates:
  `bins/philharmonic-api-server/src/{lowerer,embed_job,scope}.rs`
  (the SCK-touching paths are crypto-review-aware and want
  their own slice).
  Slice log at [ROADMAP §3.K](../ROADMAP.md#k-audit--refactor-in-flight-yuka-direct-codex-dispatch).

### Naming history

The single `philharmonic-connector` crate in early sketches
was split into the framework crates above (`-common`,
`-client`, `-router`, `-service`, `-impl-api`) so each
responsibility has a clean boundary. The
`philharmonic-realm` name is released; realm vocabulary
folds into `philharmonic-connector-common`.

## Dependency graph

```
philharmonic-types                (no philharmonic deps)
philharmonic-store                → philharmonic-types
philharmonic-store-sqlx-mysql     → philharmonic-store, philharmonic-types

mechanics-config                  (no philharmonic deps, no Boa)
mechanics-core                    → mechanics-config, boa_engine
                                    (wrapper newtypes impl Boa GC traits)
mechanics (bin)                   → mechanics-core

mechanics-dns                     → hickory-resolver
mechanics-http-client             → mechanics-dns

philharmonic-policy               → philharmonic-types,
                                    philharmonic-store

philharmonic-workflow             → philharmonic-types,
                                    philharmonic-store,
                                    philharmonic-policy
                                    (defines StepExecutor and
                                    ConfigLowerer traits;
                                    WorkflowInstance has tenant
                                    entity slot)

philharmonic-connector-common     → philharmonic-types
                                    (COSE formats, realm model,
                                    ConnectorCallContext)

philharmonic-connector-client     → philharmonic-connector-common,
                                    philharmonic-types
                                    (crypto primitives only)

philharmonic-connector-router     → philharmonic-connector-common,
                                    philharmonic-types

philharmonic-connector-service    → philharmonic-connector-common,
                                    philharmonic-types
                                    (verification, decryption,
                                    framework — does NOT host the
                                    Implementation registry)

philharmonic-connector-impl-api   → philharmonic-connector-common
                                    (Implementation trait, no crypto)

philharmonic-connector-impl-*     → philharmonic-connector-impl-api,
                                    philharmonic-connector-common,
                                    (per-implementation deps:
                                    mechanics-http-client, sqlx,
                                    lettre, tract, …)

philharmonic-api                  → philharmonic-types,
                                    philharmonic-store,
                                    philharmonic-workflow,
                                    philharmonic-policy,
                                    philharmonic-connector-client,
                                    philharmonic-connector-common
                                    (axum API library; routes, middleware)

bins/philharmonic-api-server      → philharmonic-api,
                                    philharmonic-connector-router,
                                    mechanics-config
                                    (lowerer / embed-job / scope
                                    currently live here; Audit &
                                    refactor sweep extracts them
                                    into libraries — see "Bins are
                                    thin" principle)

bins/philharmonic-connector       → philharmonic-connector-service,
                                    philharmonic-connector-impl-*
                                    (registers and dispatches
                                    Implementations; per-realm)

philharmonic                      → philharmonic-api
                                    (WebUI assets bundled here)
```

### Key points about the dependency graph

**`philharmonic-workflow` does not depend on `mechanics-core` or
`mechanics`.** Workflow code reaches the executor via the
`StepExecutor` trait; the HTTP-client implementation lives in the
API binary.

**`philharmonic-connector-client` does not depend on
`mechanics-core`** and does not depend on
`philharmonic-policy` or `philharmonic-store` either. It is a
pure crypto library — minting and encrypting given inputs.
Reading `TenantEndpointConfig`, decrypting SCK, and assembling
the connector payload is the API server binary's job.

**`philharmonic-connector-router` has minimal dependencies.** It
needs only the realm model from `connector-common` and basic
HTTP infrastructure. No policy, no store, no workflow. A pure
dispatcher.

**`philharmonic-connector-service` does not depend on the
client.** Client and service communicate through the COSE token
and payload formats defined in `connector-common`. Each side
implements its half of the protocol independently.

**`philharmonic-connector-service` does not host the
`Implementation` trait registry.** The framework provides
verification + decryption + `ConnectorCallContext`; the
deployment binary that consumes the framework decides which
`Implementation`s are registered and dispatches accordingly.

**Per-implementation crates depend only on
`-impl-api` + `-common`.** They carry their own external
dependencies (HTTP clients, database drivers, ONNX runtimes)
but stay within the connector layer — no workflow, policy, or
store dependencies.

## Why the connector split

Four crates instead of one is more moving parts, but each has a
clear single responsibility and the dependency graph stays clean:

- **`common`** owns shared vocabulary — the wire contracts, the
  data shapes. Changing a contract is a `common` release; both
  sides of the contract pick it up. Narrow dependency footprint.
- **`client`** is the lowerer. Depends on storage (to fetch
  tenant endpoint configs), policy (to check permissions and
  decrypt), workflow (to implement `ConfigLowerer`), and
  `mechanics-config` (to produce configs). Heavy dependency
  footprint — appropriate for a crate that bridges several
  concerns.
- **`router`** is a dispatcher. Minimal dependencies; cleanly
  deployable as a small binary.
- **`service`** is the framework for service binaries. Depends on
  `common` and not much else. Per-implementation crates build on
  top.

Without the split, a single `philharmonic-connector` crate would
force every consumer (including the minimal router) to carry the
full dependency closure. The split lets each consumer carry only
what it needs.

## Per-implementation crate naming

Connector implementation crates use
`philharmonic-connector-impl-<name>`. Consistent with the rest of
the connector namespace, unambiguous about what the crates are
for, and unencumbered by the historical "mechanics" prefix that
implies JS-executor concerns these crates don't have.

## Licensing

All crates: `Apache-2.0 OR MPL-2.0`.

The dual license gives consumers choice: Apache-2.0 (standard
permissive free software license with patent grants) or MPL-2.0
(file-level copyleft, FSF-compatible, GPL-2.0+ compatible via
the secondary license clause). This combination covers more
deployment scenarios than the common `Apache-2.0 OR MIT` while
keeping every crate firmly in the free software / FLOSS category
(both licenses on the FSF's approved list).

## Repository structure

One-crate-per-repo under `github.com/metastable-void/*`. Each
crate has its own issue tracker, CI, release cycle. Cross-crate
refactors require coordinated PRs, which the cornerstone's
versioning discipline mostly absorbs.

With the connector split and per-implementation crates, the
workspace has significantly more repositories. The one-repo-per-
crate discipline scales through tooling (consistent CI templates,
shared release scripts) rather than by merging crates into
monorepos.

## Versioning

Semantic versioning with pre-1.0 caveats.

- **Patch (0.x.y → 0.x.(y+1))** — additive changes, bug fixes,
  documentation.
- **Minor (0.x.y → 0.(x+1).0)** — changes to existing APIs. In
  pre-1.0 versioning, minor bumps may break consumers.
- **Major (0.x.y → 1.0.0)** — stability boundary.

The cornerstone (`philharmonic-types`) is on the strict end of
this discipline because many crates depend on it. Breaking
changes are announced and bundled. Other crates can be more
relaxed about minor releases since they have fewer dependents.

`philharmonic-connector-common` is near the strict end once
implementations depend on it: changing a wire format affects
every client, router, service, and implementation simultaneously.

## Edition and MSRV

- Edition 2024.
- Workspace baseline MSRV: **1.88**.
- Documented exceptions: `inline-blob` and
  `philharmonic-connector-impl-embed` declare
  `rust-version = "1.89"` because they require
  language/library features (large array literals and
  associated `slice` APIs) introduced in 1.89.

Documented in each crate's `Cargo.toml` via `rust-version`. A
workspace-wide bump is the right long-term answer; until then,
the two 1.89 crates are the single source of truth for the
exception, and any new crate should default to the 1.88
baseline.

## Build targets

Production targets `x86_64-unknown-linux-musl` for static linking.
Library crates build for whatever target the consumer chooses.
Binary crates (`mechanics`, future API service, future connector
router and connector service binaries) ship as statically-linked
musl binaries suitable for minimal container images.

Connector service binaries are per-realm: one static binary per
realm, bundling the connector-service framework plus all
implementation crates configured for that realm. Different realms
can include different implementations.
