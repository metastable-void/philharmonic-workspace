# v1 Scope

What a first shipping release includes, and what's deferred.

**Phase 9 complete (2026-04-30).** All items in "Core
infrastructure" and "API + deployment" below are shipped.
25 crates on crates.io, three bin targets with key generation,
musl static builds, Docker compose, full-pipeline e2e tests.
Remaining: Tier 2 SMTP + Tier 3 LLM providers (post-Golden-Week).

## Core infrastructure

### Already complete

- `philharmonic-types` — published, stable, 99%+ documented.
- `philharmonic-store` — published, stable.
- `philharmonic-store-sqlx-mysql` — published, 28 passing
  integration tests.
- `mechanics-config` — published `0.1.0` (2026-04-21). Schema
  types (`MechanicsConfig`, `HttpEndpoint`, etc.) extracted
  from `mechanics-core` so the lowerer stays Boa-free.
- `mechanics-core` — published, substantial implementation.
- `mechanics` HTTP service — published.
- `philharmonic-policy` — published `0.1.0` (2026-04-22). All
  seven entity kinds (`Tenant`, `Principal`,
  `TenantEndpointConfig`, `RoleDefinition`, `RoleMembership`,
  `MintingAuthority`, `AuditEvent`), permission evaluation
  with three-way tenant binding, SCK AES-256-GCM at-rest
  encryption, `pht_` long-lived API token format. Yuka's
  two-gate crypto review protocol (Gate-1 approach approval
  + Gate-2 code review) satisfied.
- `philharmonic-connector-common` — published `0.1.0`
  (2026-04-22); `0.2.0` prepared in-tree (adds `iat` claim to
  `ConnectorTokenClaims` per Wave A Gate-2 follow-up), publishes
  with the rest of the connector triangle after Wave B end-to-
  end. Shared connector-layer vocabulary:
  `ConnectorTokenClaims`, `ConnectorCallContext`, realm model
  (`RealmId`, `RealmPublicKey`, `RealmRegistry`), thin COSE
  wrapper types (`ConnectorSignedToken`,
  `ConnectorEncryptedPayload`), `ImplementationError`. Types-
  only; crypto construction lives in Phase 5.
- `philharmonic-workflow` — published `0.1.0` (2026-04-22).
  Orchestration engine with three entity kinds
  (`WorkflowTemplate`, `WorkflowInstance`, `StepRecord`),
  `SubjectContext` threading, `StepExecutor` / `ConfigLowerer`
  async trait boundaries, `WorkflowEngine<S, E, L>` with
  nine-step execution sequence and terminal-state-immutable
  five-state lifecycle. Step-record subject content is
  architecturally confined to `kind` + `id` + `authority_id`
  (no `claims`, no `tenant_id`) via a separate
  `StepRecordSubject` persistence type.

### Implemented and published

- `philharmonic-connector-client` — crypto/minting library
  (COSE_Sign1 token minting, COSE_Encrypt0 payload
  encryption). Published.
- `philharmonic-connector-impl-api` — `Implementation` trait
  contract. Published.
- `philharmonic-connector-router` — pure HTTP dispatcher
  binary. Published.
- `philharmonic-connector-service` — service framework; token
  verification, payload decryption, `ConnectorCallContext`
  construction. Published.
- Per-implementation crates
  (`philharmonic-connector-impl-<name>`). Published.
- `philharmonic-api` — public HTTP API. Published.

### Deferred or not planned

- `philharmonic-store-mem` — quality-of-life, not blocking.
- Alternative storage backends for the substrate — no current
  need.
- Streaming support — no clear driver.
- Replay/determinism — design allows it; implement later if
  needed.
- Hierarchical tenancy — entity model doesn't foreclose adding
  a parent slot later.

## Workflow layer

### Ships with v1

- Three entity kinds: `WorkflowTemplate`, `WorkflowInstance`,
  `StepRecord`. Both `WorkflowTemplate` and `WorkflowInstance`
  carry tenant entity slots.
- Five-status lifecycle state machine with all documented
  transitions.
- `create_instance`, `execute_step`, `complete`, `cancel`
  engine methods, all accepting a `SubjectContext` parameter.
- Four-field script argument: `{context, args, input, subject}`.
- Step records carry subject content for audit attribution.
  Subject content records identifier + minting authority only
  — full injected claims are not persisted, by design.
- Step records carry correlation ID for cross-crate log
  correlation.
- Abstract config as a JSON map of `{script_name: config_uuid}`,
  opaque to the workflow engine and interpreted by the
  lowerer.
- `StepExecutor` trait abstracting executor transport.
- `ConfigLowerer` trait abstracting lowering; lowerer
  receives `SubjectContext` for future consumers.
- `done: true` script completion convention.
- Single-step orchestration (caller drives the loop).
- Template source code size capped at 1 MB (accommodates
  Webpack-bloated scripts within limits).

### Doesn't ship with v1

- Retry policy (caller's responsibility).
- Multi-step auto-advancement (caller or scheduler).
- Cross-instance coordination.

## Connector layer

### Model

- **Capability**: named wire-protocol shape. Documentation
  and schema, not a runtime entity.
- **Implementation**: Rust code speaking one category of
  external service; crate-level shipping decision.
- **Tenant endpoint config**: per-tenant
  `TenantEndpointConfig` entity holding the encrypted call-
  site blob (realm + impl + credentials + constraints).

No deployment-wide endpoint registry. Per-call-site state
lives entirely in per-tenant config entities. Templates
reference configs by UUID.

### Crate split

Five crates plus per-implementation crates:

- `philharmonic-connector-common` — shared types (COSE
  formats, realm model, `ConnectorCallContext`).
- `philharmonic-connector-client` — crypto/minting library;
  COSE_Sign1 token minting, COSE_Encrypt0 payload encryption.
- `philharmonic-connector-router` — pure HTTP dispatcher per
  realm.
- `philharmonic-connector-impl-api` — `Implementation` trait
  contract; non-crypto, no key material.
- `philharmonic-connector-service` — service framework; token
  verification, payload decryption, `ConnectorCallContext`
  construction. Does not host the `Implementation` trait
  registry; dispatch lives in the deployment binary.
- Per-implementation crates, one per implementation.

### Ships with v1

- Capability / implementation / tenant-config model.
- COSE_Sign1 connector authorization tokens carrying `realm`
  + `tenant` + `inst` + `step` + `config_uuid` +
  `payload_hash`, no `impl` claim.
- COSE_Encrypt0 encrypted payloads per config per call, via
  per-realm hybrid PQC KEM (ML-KEM-768 + X25519 +
  AES-256-GCM).
- Substrate at-rest encryption for `TenantEndpointConfig`
  credential blobs under a deployment-level substrate
  credential key (SCK). `display_name` and `implementation`
  are plaintext content slots; `encrypted_config` holds the
  connector-implementation-specific config only.
- Lowerer as payload assembler — reads plaintext
  `implementation`, resolves a realm via deployment config,
  decrypts SCK blob, assembles `{realm, impl, config}`, and
  COSE_Encrypt0-encrypts that to the realm KEM.
- Per-realm static binary for connector services, bundling
  the service framework plus the implementations configured
  for that realm.
- Pure-dispatcher connector router.
- `Implementation` trait with
  `execute(config, request, ctx)` shape and a
  `ConnectorCallContext` carrying verified token claims.
- Arbitrary JSON payload shape, opaque to the framework;
  Implementations deserialize `config` into their own
  concrete types.
- Free-form JSON admin submission; no schema validation at
  the API layer.

### Shipping implementations

All implementations share the `Implementation` trait contract
and plug into the connector framework identically. Groupings
below are by external-service protocol and domain knowledge —
not by architectural privilege. Generic HTTP is listed first
because it's the baseline from which LLM implementations are
specializations; **LLM connectors are not first-class citizens
of this system**, they are HTTP clients with LLM-provider
domain knowledge.

**Generic HTTP** — one implementation, the baseline:

- `http_forward` — backs many configs, including transactional
  email providers (SendGrid, Postmark, AWS SES) which are HTTP
  configs, not separate email implementations.

**LLM — specialized HTTP** — three implementations covering
the major provider API shapes. Structurally identical to
`http_forward`; separate crates because the translation code
volume justifies a dedicated implementation per provider.

- `llm_openai_compat` — OpenAI Chat Completions API shape;
  covers OpenAI itself, vLLM (which has native structured
  outputs via `structured_outputs: {"json": <schema>}` as a
  top-level body field), and other compatible gateways. Config
  selects the structured-output dialect.
- `llm_anthropic` — Anthropic Messages API (native structured
  output).
- `llm_gemini` — Google Gemini API (native `responseSchema`).

All three implement the `llm_generate` capability.

**SQL** — per database family:

- `sql_postgres`
- `sql_mysql`

Wire protocol: parameterized SQL plus parameter values, rows
as structured data. Not HTTP; the implementations speak the
database wire protocol directly.

**Email**:

- `email_smtp` — SMTP to configured submission servers.

Transactional email providers are served by `http_forward`-
backed configs.

**Embedding and vector search** — two capabilities, separate
crates (split, settled):

- `embed` — text in, vector out. At least one implementation.
- `vector_search` — vector in, nearest-neighbor results out.
  At least one implementation.

### Doesn't ship with v1

- Free-form LLM output (free-form text expressed as `{"type":
  "string"}` schema).
- Tool calling / agentic patterns at the connector level
  (composed in JS scripts instead).
- Streaming LLM or SQL responses.
- Upsert capabilities for vector stores (tenants manage state
  out-of-band for v1).
- Schema-driven admin UI for configs (free-form JSON for now).

## Policy and tenancy

### Ships with v1

- `Tenant` entity kind (flat namespace).
- `Principal` entity kind (long-lived API token credential,
  SHA-256 hashed in substrate; `epoch` scalar reserved for
  future self-contained-token migration, unused in v1).
- `TenantEndpointConfig` entity kind — tenant slot,
  `display_name`, `implementation`, `encrypted_config`,
  `key_version`, `is_retired`. The `implementation` name is a
  plaintext content slot; only credentials and connector-
  specific config are inside the encrypted blob; realm is
  derived from `implementation` via the deployment's
  connector-router map.
- `RoleDefinition` entity kind (tenant-scoped).
- `RoleMembership` entity kind.
- `MintingAuthority` entity kind (tenant-scoped, with
  permission envelope, minting constraints, and `epoch`
  scalar for mass revocation).
- `AuditEvent` entity kind.
- Permission atom vocabulary (`workflow:*`, `endpoint:*`,
  `tenant:*`, `mint:*` families).
- Simple JSON array role-definition schema.
- SCK-based at-rest encryption with rotation via
  `key_version` scalar.
- Tenant suspension automatically bumps every minting
  authority's epoch.
- Injected ephemeral-token claims capped at 4 KB.
- Tenant subdomain naming rules enforced (`[a-z0-9]
  [a-z0-9-]{1,62}`, no leading digit, no consecutive hyphens,
  reserved names including realm names, `admin`, `api`,
  `www`, `app`, `connector`).

### Doesn't ship with v1

- Hierarchical tenancy.
- Per-tenant encryption keys.
- Per-ephemeral-token revocation lists.
- OAuth 2.0 / OIDC federation.
- Distributed rate limiting (v1 ships per-tenant and
  per-minting-authority single-node token buckets — see doc
  10 — but cross-node coordination is deferred).
- Persistence of full injected subject claims in audit
  records.
- Background reconciler for duplicate-config races (accept
  the theoretical race; lowerer tie-breaks by latest-updated
  non-retired).

## API layer

### Ships with v1

- Tenant-scoped request routing via a deployment-supplied
  `request → tenant-id` resolver. Subdomain-per-tenant,
  path-prefix-per-tenant, single-tenant, and mTLS-per-tenant
  are all supported shapes (see `10-api-layer.md`).
- Deployment-operator endpoints distinguishable from tenant
  requests by the resolver (distinct subdomain, reserved
  path prefix, or separate listener — the deployment's
  choice).
- Long-lived API token authentication (opaque bearer strings
  with substrate-hash lookup).
- Ephemeral API token authentication (COSE_Sign1, up to 24h
  lifetime, with structured subject claims, instance-scope
  support, and epoch-based revocation).
- Minting endpoint for ephemeral tokens.
- Workflow CRUD endpoints.
- Endpoint config CRUD endpoints (add/rotate/retire/read;
  decrypted read requires `endpoint:read_decrypted` permission for
  operator verification).
- Principal and role management endpoints (CRUD).
- Minting authority management endpoints (CRUD + epoch bump).
- Simple per-tenant rate limiting via single-node token
  buckets.
- Audit log access endpoint.
- URL-path versioning (`/v1/...`).
- JSON-only content type (no msgpack/protobuf negotiation).
- Cursor-based pagination on every list endpoint; default
  page size 50, maximum 200. Opaque cursor string.
- Structured error envelope
  (`{error: {code, message, details?, correlation_id}}`)
  with a settled HTTP status → code vocabulary. See
  `11-security-and-cryptography.md`.
- JSON-line structured logging via `tracing`.
- Prometheus metrics via `metrics`.
- Correlation ID propagation via `X-Correlation-ID` header.
- Web UI / admin UI using the same API-token mechanism
  (separate project).

### Doesn't ship with v1

- OAuth 2.0 / OIDC federation.
- Session-based authentication for Web UI.
- Outbound webhooks.
- Distributed rate limiting.
- Streaming endpoints.
- GraphQL or gRPC.
- Atomic create-instance-and-mint-token endpoint.
- Idempotency keys (clients retry on `duplicate_entity` by
  re-reading state).

## Deployment topology

Philharmonic is a framework; topology is the deployment's
choice. v1 scope covers *what the framework must make
possible*, not *which specific shape every deployment must
use*.

### Supported for v1

Process-layer shapes supported end-to-end:

- MySQL-family database (MySQL 8, MariaDB 10.5+, Aurora
  MySQL, TiDB), as the one storage substrate implementation
  shipping in v1.
- API + workflow engine tier (horizontally scaled). API
  processes host the `WorkflowEngine` with plugged-in
  lowerer (`connector-client`) and HTTP executor client
  (pointing at mechanics fleet).
- Mechanics worker fleet (horizontally scaled).
- Connector service fleet per realm (horizontally scaled).
- Connector router per realm (thin dispatcher).
- Alternate topology: a deployment can collapse layers —
  one binary hosting API + workflow engine + mechanics +
  connectors — for single-user or single-tenant use. The
  crate boundaries support this directly.

URL / routing shapes supported end-to-end:

- Subdomain-per-tenant with wildcard HTTPS cert.
- Path-prefix-per-tenant on a single certificate.
- Single-tenant (one fixed tenant ID).
- mTLS-per-tenant (tenant ID in client-cert CN/SAN).
- Any other `request → tenant-id` resolver the deployment
  supplies.

Region layout for v1: single region. Deployments can be
single-AZ or multi-AZ within that region.

### Not supported for v1

- Multi-region deployments.
- Edge-distributed connector services.
- Storage backends other than the MySQL-family one shipping
  in v1 (the `philharmonic-store` trait surface supports
  additional backends; adding one is explicitly outside v1
  scope).

## Security

### v1 security features

- **Cryptographic primitives**: ML-KEM-768 + X25519 hybrid
  for KEM, AES-256-GCM for symmetric encryption, Ed25519 for
  signing, SHA-256 for hashing.
- **Token format**: COSE (COSE_Sign1 for signed tokens,
  COSE_Encrypt0 for encrypted payloads).
- **Three token systems**: connector authorization tokens
  (lowerer-signed, minutes, one per config per step),
  ephemeral API tokens (API-signed, up to 24h), long-lived
  API tokens (opaque bearer, hashed in substrate).
- **Encryption at rest** for per-tenant config blobs as whole
  ciphertexts; SCK in deployment secret storage.
- **Per-realm KEM keys** for payload encryption; realm
  isolation limits blast radius.
- **Per-config encryption granularity** — each call encrypts
  a fresh payload to its config's realm KEM key.
- **Epoch-based mass revocation** for ephemeral tokens.
- **Automatic epoch bump** on tenant suspension.
- **TLS for all HTTP hops.**
- **Payload-hash binding** in COSE_Sign1 claims prevents
  token-payload mix-and-match.
- **Impl-registry dispatch** at the connector service — only
  impls registered in the realm binary are accepted.
- **Defense in depth**: TLS → token signature → realm match
  → payload hash binding → payload decryption → inner-realm
  match → impl registry dispatch → implementation-side
  validation → per-tenant rate limiting.
- **Observability**: correlation IDs, structured logs,
  Prometheus metrics, cross-crate consistent conventions.

### Not in v1

- Per-token revocation lists.
- HSM-backed signing keys (operational choice deployments
  can make).
- Per-tenant encryption keys.
- Hybrid PQ signing (Ed25519 only for v1; path to hybrid
  ML-DSA + Ed25519 is clear via COSE's algorithm agility).
- Automated key rotation procedures (manual rotation
  documented).

## Current state (Phase 9 complete, 2026-05-02)

The critical path below is preserved as a historical record.
For current implementation status, the authoritative sources
are the workspace `README.md` (phase status) and
[`docs/ROADMAP.md`](../ROADMAP.md) (current and remaining
work).

Summary of where v1 stands:

- Cornerstone, storage, execution substrate, policy, and
  workflow crates are published and in production use.
- Connector framework + Phase 6 implementations
  (`http_forward`, `llm_openai_compat`) are published and
  exercised by the reference deployment.
- Phase 7 Tier 1 implementations (`sql_postgres`,
  `sql_mysql`, `embed`, `vector_search`) are published.
- Phase 7 Tier 2/3 (`email_smtp`, `llm_anthropic`,
  `llm_gemini`) are deferred post-Golden-Week 2026; their
  crate names are reserved as `0.0.x` placeholders on
  crates.io.
- `philharmonic-api` and the three deployment binaries
  (`philharmonic-api-server`, `mechanics-worker`,
  `philharmonic-connector`) are published and operational.

## Historical v1 implementation path

The original critical path for getting from "design docs only"
to "shipped v1." Kept here for context on how the work was
sequenced; **not** an authoritative open-task list. Items
marked stale in the audit (e.g. "extract mechanics-config",
"claim remaining crate names as 0.0.0 stubs") have already
landed.

1. **Close remaining design questions** (now mostly done; the
   audit and doc reconciliation passes settled
   `TenantEndpointConfig` shape, lowerer semantics,
   `http_forward` wire protocol, `embed` / `vector_search`
   split, etc.). Per-implementation wire-protocol details
   still pending only for the deferred Tier 3 LLM impls.

2. **Claim remaining crate names** on crates.io. Done — every
   v1 crate name is on crates.io as either a substantive
   implementation (`0.1.0+`) or a published placeholder
   (`0.0.x`); see "Phase/status language" in the workspace
   `README.md`.

3. **Extract `mechanics-config`** from `mechanics-core`. Done.

4. **Implement `philharmonic-policy`**. Done.

5. **Implement `philharmonic-connector-common`**. Done.

6. **Implement `philharmonic-workflow`**. Done.

7. **Implement connector layer**. Done — connector-client
   (crypto primitives), connector-router (dispatcher), and
   connector-service (framework) all published.

8. **Implement per-implementation crates** in parallel.
   Phase 6 (`http_forward`, `llm_openai_compat`) and Phase 7
   Tier 1 (`sql_postgres`, `sql_mysql`, `embed`,
   `vector_search`) done. Tier 2 (`email_smtp`) and Tier 3
   (`llm_anthropic`, `llm_gemini`) deferred post-GW.

9. **Implement `philharmonic-api`**. Done.

10. **End-to-end integration testing**. Done via
    testcontainers and full-pipeline e2e tests.

11. **Deploy reference deployment.** Done — reference
    deployment operational with the OpenAI-compatible LLM
    workflow verified through the WebUI on 2026-05-02.

## Out of scope entirely

- Alternative JavaScript engines (Boa only).
- Alternative languages for workflow scripts (JS only).
- Alternative storage backends to MySQL-family for the
  substrate.
- Replay and determinism.
- Multi-region anything.
- Tool calling / agentic LLM patterns at the connector level.
- Streaming.
- Hierarchical tenancy.
- Session + access-token authentication pattern.
- ~~Arbitrary deployment-operator-chosen URL layouts.~~
  Partially in scope: the `RequestScopeResolver` trait lets
  deployments supply custom scope resolution (subdomain,
  header, path-based). Arbitrary API path customization
  (changing `/v1/...` prefixes) is not supported.
