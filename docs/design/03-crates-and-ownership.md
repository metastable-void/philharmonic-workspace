# Crates and Ownership

## Currently published with substantive content

- **`philharmonic-types`** (v0.3.4) — cornerstone vocabulary.
- **`philharmonic-store`** (v0.1.0) — storage substrate traits.
- **`philharmonic-store-sqlx-mysql`** (v0.1.0) — SQL backend.
- **`mechanics-config`** (v0.1.0) — Boa-free schema types
  (`MechanicsConfig`, `HttpEndpoint`, supporting types) extracted
  from `mechanics-core` so the lowerer can consume schema without
  pulling in Boa / reqwest / tokio.
- **`mechanics-core`** (v0.3.0) — JS executor library (Boa-backed).
- **`mechanics`** (v0.3.0) — HTTP service wrapping `mechanics-core`.
- **`philharmonic-policy`** (v0.1.0) — tenants, principals,
  per-tenant endpoint configs (`TenantEndpointConfig`) with SCK
  AES-256-GCM at-rest encryption, roles, role memberships,
  minting authorities, audit events; `pht_` long-lived API
  token format.
- **`philharmonic-connector-common`** (v0.2.0 on crates.io,
  published 2026-04-23) — shared vocabulary for the connector
  layer: COSE token and payload types (`ConnectorTokenClaims`,
  `ConnectorSignedToken`, `ConnectorEncryptedPayload`), realm
  model (`RealmId`, `RealmPublicKey`, `RealmRegistry`),
  `ConnectorCallContext` (verified claims delivered to
  implementations), and the shared `ImplementationError`
  taxonomy. Types-only; crypto construction lives in
  `philharmonic-connector-client` and
  `philharmonic-connector-service`. 0.2.0 adds an `iat` claim
  to `ConnectorTokenClaims` (Wave A Gate-2 follow-up) and
  publishes with the rest of the connector triangle after Wave B.
- **`philharmonic-workflow`** (v0.1.0) — orchestration engine.
  Three entity kinds (`WorkflowTemplate`, `WorkflowInstance`,
  `StepRecord`) with append-only revision-based state evolution.
  `SubjectContext` / `SubjectKind` for caller attribution,
  reusing `philharmonic-policy`'s `Tenant` and
  `MintingAuthority` markers. Async trait boundaries
  (`StepExecutor`, `ConfigLowerer`) keep the engine transport-
  and lowerer-naive. `WorkflowEngine<S, E, L>` implements the
  nine-step execution sequence, five-state lifecycle with
  terminal-state immutability, and architecturally-enforced
  step-record audit discipline (persisted subject drops
  `claims` and `tenant_id` by type construction).

## Connector triangle (published 2026-04-23)

- **`philharmonic-connector-client`** — the lowerer. Produces
  concrete `MechanicsConfig` values by fetching
  `TenantEndpointConfig` entities via policy, decrypting each
  with the substrate credential key, re-encrypting the
  byte-identical plaintext to the destination realm's KEM
  public key, and minting signed authorization tokens binding
  the payload hash to the step context. Implements
  `ConfigLowerer` from `philharmonic-workflow`. Wave A mint
  path (COSE_Sign1) landed 2026-04-22; Wave B encrypt path
  (hybrid ML-KEM-768 + X25519 + AES-256-GCM COSE_Encrypt0)
  landed 2026-04-23. Published as `0.1.0` on 2026-04-23.
- **`philharmonic-connector-router`** — pure HTTP dispatcher
  binary library. Routes requests by realm. Landed and
  published as `0.1.0` on 2026-04-23 alongside the rest of the
  triangle.
- **`philharmonic-connector-service`** — service framework for
  per-realm connector service binaries. Hosts the
  `Implementation` trait, token verification, payload
  decryption, dispatch. Wave A verify path landed 2026-04-22;
  Wave B decrypt path + `verify_and_decrypt` chain landed
  2026-04-23. Published as `0.1.0` on 2026-04-23.

## Connector implementations (Phase 6 — published 2026-04-24)

- **`philharmonic-connector-impl-api`** — non-crypto
  trait-only crate hosting the `#[async_trait] Implementation`
  trait plus re-exports of `ConnectorCallContext`,
  `ImplementationError`, `JsonValue`, and the `async_trait`
  macro. Published as `0.1.0` on 2026-04-24 (Phase 6 Task 0).
- **`philharmonic-connector-impl-http-forward`** — generic
  HTTP-forwarding connector. Reuses
  `mechanics_config::HttpEndpoint` for config; handles
  per-body-type decoding, full-jitter exponential retry with
  Retry-After support, `response_max_bytes` enforcement via
  streamed accumulator. Published as `0.1.0` on 2026-04-24
  (Phase 6 Task 1).
- **`philharmonic-connector-impl-llm-openai-compat`** —
  OpenAI-compatible LLM connector covering OpenAI itself +
  vLLM + Together + Groq + OpenRouter via three dialects
  (`openai_native` / `vllm_native` / `tool_call_fallback`),
  all with `strict: true` token-level schema enforcement.
  Published as `0.1.0` on 2026-04-24 (Phase 6 Task 2).

## Published as 0.0.0 placeholders (Phase 7+ and beyond)

- **`philharmonic`** — meta-crate placeholder.
- **`philharmonic-api`** — public HTTP API (Phase 8+).
- Per-implementation crates pending for Phase 7. Crates.io
  still shows the `0.0.0` placeholders, but local
  implementation state varies:
  - **Tier 1 implementations exist locally at 0.1.0 but are
    not yet published**:
    - `philharmonic-connector-impl-sql-postgres` —
      compile-clean, green.
    - `philharmonic-connector-impl-sql-mysql` — compile-
      clean, green.
    - `philharmonic-connector-impl-vector-search` —
      compile-clean, green.
    - `philharmonic-connector-impl-embed` — round-01
      `fastembed` + `ort` code committed as a checkpoint
      but rejected as a library choice (glibc-only ort
      prebuilts vs. our musl deployment targets); rewrite
      with `tract` + `tokenizers` is the next embed
      Codex dispatch (plan at
      [`docs/notes-to-humans/2026-04-24-0008-phase-7-embed-tract-pivot-plan.md`](../notes-to-humans/2026-04-24-0008-phase-7-embed-tract-pivot-plan.md)).

    Tier 1 publishes as a coherent set once the embed
    tract rewrite lands.
  - **Tier 2** (deferred until Tier 1 closes):
    - `philharmonic-connector-impl-email-smtp`.
  - **Tier 3** (deferred until on or after 2026-05-07,
    post-Golden-Week 2026):
    - `philharmonic-connector-impl-llm-anthropic`.
    - `philharmonic-connector-impl-llm-gemini`.

The single `philharmonic-connector` crate in the earlier sketch
has been split into four (common / client / router / service)
to give each connector responsibility clean boundaries. The
name `philharmonic-connector` itself may be kept as a meta-crate
placeholder or released.

The `philharmonic-realm` name is released: realm vocabulary
folds into `philharmonic-connector-common`. Realms don't warrant
a separate crate.

## Dependency graph

### Current

```
philharmonic-types               (no philharmonic deps)
philharmonic-store               → philharmonic-types
philharmonic-store-sqlx-mysql    → philharmonic-store, philharmonic-types
mechanics-core                   (no philharmonic deps; depends on Boa)
mechanics                        → mechanics-core
```

### Planned

```
# Schema extraction (mechanics-core reorg):

mechanics-config                 (no philharmonic deps, no Boa)
mechanics-core                   → mechanics-config, boa_engine
                                   (wrapper newtypes impl Boa GC traits)
mechanics                        → mechanics-core

# Policy and workflow:

philharmonic-policy              → philharmonic-types,
                                   philharmonic-store
philharmonic-workflow            → philharmonic-types,
                                   philharmonic-store,
                                   philharmonic-policy
                                   (defines StepExecutor and
                                   ConfigLowerer traits;
                                   WorkflowInstance has
                                   tenant entity slot)

# Connector layer:

philharmonic-connector-common    → philharmonic-types,
                                   mechanics-config
                                   (COSE formats, realm model,
                                   ConnectorCallContext)

philharmonic-connector-client    → philharmonic-connector-common,
                                   philharmonic-types,
                                   philharmonic-store,
                                   philharmonic-policy,
                                   philharmonic-workflow,
                                   mechanics-config
                                   (implements ConfigLowerer)

philharmonic-connector-router    → philharmonic-connector-common,
                                   philharmonic-types

philharmonic-connector-service   → philharmonic-connector-common,
                                   philharmonic-types
                                   (Implementation trait,
                                   verification, decryption,
                                   dispatch)

# Connector implementations (one crate per implementation):

philharmonic-connector-impl-*    → philharmonic-connector-service,
                                   philharmonic-connector-common,
                                   (per-implementation deps:
                                   reqwest, sqlx, lettre, etc.)

# API layer:

philharmonic-api                 → philharmonic-types,
                                   philharmonic-store,
                                   philharmonic-workflow,
                                   philharmonic-policy,
                                   philharmonic-connector-client,
                                   philharmonic-connector-common
```

### Key points about the dependency graph

**`philharmonic-workflow` does not depend on `mechanics-core` or
`mechanics`.** Workflow code reaches the executor via the
`StepExecutor` trait; the HTTP-client implementation of that
trait lives in the API binary or a separate glue crate.

**`philharmonic-connector-client` does not depend on
`mechanics-core`.** The lowerer produces `MechanicsConfig` values
via `mechanics-config`, which is Boa-free. This is the main
reason the schema extraction is worth doing.

**`philharmonic-connector-router` has minimal dependencies.** It
needs only the realm model from `connector-common` and basic HTTP
infrastructure. No policy, no store, no workflow. A pure
dispatcher.

**`philharmonic-connector-service` does not depend on the client.**
Client and service communicate through the COSE token and payload
formats defined in `connector-common`. Each side implements its
half of the protocol independently.

**Per-implementation crates depend only on the service framework
and common types.** An implementation crate carries its own
external dependencies (HTTP clients, database drivers, vector
store clients) but stays within the connector layer. It does not
depend on the workflow, policy, or store crates.

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
- MSRV 1.88.

Documented in each crate's `Cargo.toml` via `rust-version = "1.88"`.

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
