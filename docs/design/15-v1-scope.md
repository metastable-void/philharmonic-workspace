# v1 Scope

What a first shipping release includes, and what's deferred.

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
  (2026-04-22). Shared connector-layer vocabulary:
  `ConnectorTokenClaims`, `ConnectorCallContext`, realm model
  (`RealmId`, `RealmPublicKey`, `RealmRegistry`), thin COSE
  wrapper types (`ConnectorSignedToken`,
  `ConnectorEncryptedPayload`), `ImplementationError`. Types-
  only; crypto construction lives in Phase 5.

### Needs implementation work

- `philharmonic-workflow` — design complete, code not started.
  Depends on `philharmonic-policy` for `Tenant` entity marker.
- `philharmonic-connector-client` — the lowerer; implements
  `ConfigLowerer`.
- `philharmonic-connector-router` — pure HTTP dispatcher
  binary.
- `philharmonic-connector-service` — framework for per-realm
  connector service binaries.
- Per-implementation crates. Naming:
  `philharmonic-connector-impl-<name>`.
- `philharmonic-api` — public HTTP API.

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

Four crates plus per-implementation crates:

- `philharmonic-connector-common` — shared types (COSE
  formats, realm model, `ConnectorCallContext`).
- `philharmonic-connector-client` — the lowerer; SCK
  decryption, realm KEM re-encryption, token minting.
- `philharmonic-connector-router` — pure HTTP dispatcher per
  realm.
- `philharmonic-connector-service` — service framework; hosts
  `Implementation` trait, verification, decryption,
  impl-registry dispatch.
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
  blobs under a deployment-level substrate credential key
  (SCK); whole blob encrypted, including realm and impl.
- Lowerer as pure byte forwarder — decrypts SCK blob,
  re-encrypts byte-identical to realm KEM.
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
- `TenantEndpointConfig` entity kind — minimal: tenant slot,
  `display_name`, `encrypted_config`, `key_version`,
  `is_retired`. Realm, impl name, and credentials all inside
  the encrypted blob.
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
- Per-minting-authority rate limits (single per-tenant API
  rate limit on the mint endpoint is the v1 guardrail).
- Persistence of full injected subject claims in audit
  records.
- Background reconciler for duplicate-config races (accept
  the theoretical race; lowerer tie-breaks by latest-updated
  non-retired).

## API layer

### Ships with v1

- Tenant-scoped subdomain routing
  (`<tenant>.api.our-domain.tld`).
- Deployment-operator endpoints at
  `https://admin.our-domain.tld/` (distinct subdomain; no
  ambiguity about cross-tenant requests).
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

### Supported for v1

Standard three-tier SaaS:

- MySQL-family database (MySQL 8, MariaDB 10.5+, Aurora
  MySQL, TiDB).
- API + workflow engine tier (horizontally scaled behind load
  balancer or anycast). API processes host the
  `WorkflowEngine` with plugged-in lowerer
  (`connector-client`) and HTTP executor client (pointing at
  mechanics fleet).
- Mechanics worker fleet (horizontally scaled).
- Connector service fleet per realm (horizontally scaled).
- Connector router per realm (thin dispatcher).

Subdomains:

- `https://<tenant>.api.our-domain.tld/` — API.
- `https://<tenant>.app.our-domain.tld/` — Web UI.
- `https://<realm>.connector.our-domain.tld/` — connector
  router per realm.
- `https://admin.our-domain.tld/` — deployment-operator
  endpoints.

Wildcard HTTPS certificates cover the subdomain patterns.

Single region, single AZ (or multi-AZ within region).

### Not supported for v1

- Multi-region deployments.
- Edge-distributed connector services.
- In-process "all-in-one" mode.
- Arbitrary operator-chosen URL layouts (the subdomain
  structure above is load-bearing; operators deploy using
  this shape).

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

## Critical v1 path

Order of work, with parallelism noted.

1. **Close remaining design questions** (mostly done):

   - **Still open:** per-implementation wire-protocol details
     for SQL (row shape, parameter binding, timeouts, errors),
     email (SMTP submission shape), and vector-search (output
     vector format, nearest-neighbor result shape). Resolve
     when each implementation begins.
   - **Already settled:** permission atom vocabulary (closed
     for v1; adjustable via deliberate doc updates — see
     `14-open-questions.md`); `http_forward` wire protocol
     (reuses `mechanics_config::HttpEndpoint`; full spec in
     `08-connector-architecture.md`); embedding and vector
     search split into two capabilities; no global endpoint
     registry; `TenantEndpointConfig` minimal shape; per-tenant
     free-form JSON configs; SCK restored with operator-visible
     decryption; `Principal.epoch` reserved; token claim set;
     simplified role schema; committed simplifications list in
     `14-open-questions.md`.

2. **Claim remaining crate names** as 0.0.0 stubs on
   crates.io:

   - `mechanics-config`
   - `philharmonic-policy`
   - `philharmonic-workflow`
   - `philharmonic-connector-common`
   - `philharmonic-connector-client`
   - `philharmonic-connector-router`
   - `philharmonic-connector-service`
   - `philharmonic-api`
   - `philharmonic-connector-impl-http-forward` etc.

3. **Extract `mechanics-config`** from `mechanics-core`.

4. **Implement `philharmonic-policy`** (parallelizable with
   step 3). Entity kinds, basic queries, SCK-based
   encrypt/decrypt of `TenantEndpointConfig`.

5. **Implement `philharmonic-connector-common`** (after step
   4 for `Tenant` marker). COSE format types, realm model,
   `ConnectorCallContext`.

6. **Implement `philharmonic-workflow`** (needs policy for
   `Tenant`, needs common for `ConfigLowerer` trait
   signatures to be stable).

7. **Implement connector layer** (parallel):

   - `philharmonic-connector-client` (the lowerer).
   - `philharmonic-connector-service` (framework).
   - `philharmonic-connector-router` (minimal).

8. **Implement per-implementation crates** in parallel. Order
   within this step (generic HTTP as baseline, LLM as first
   specialization):

   - `http_forward` — the baseline HTTP implementation.
   - LLM × 3 — `llm_openai_compat` (covering OpenAI + vLLM)
     is first priority as it unblocks the chat-app
     end-to-end path; then `llm_anthropic`, `llm_gemini`.
   - `sql_postgres`, `sql_mysql`.
   - `email_smtp`.
   - `embed` + `vector_search` (at least one impl of each).

9. **Implement `philharmonic-api`** (needs everything above).

10. **End-to-end integration testing** via testcontainers.

11. **Deploy reference deployment.**

Steps 7 and 8 have significant parallelism. Steps 1–6 are
largely sequential.

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
- Arbitrary deployment-operator-chosen URL layouts.
