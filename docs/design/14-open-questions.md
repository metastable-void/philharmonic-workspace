# Open Questions

Decisions still pending, organized by urgency. Decisions
resolved during design discussions are listed at the bottom
for reference.

## A note on method

A pattern worth keeping explicit: **state requirements before
choosing a design**. Several earlier leanings (notably PASETO
for token format) were made on aesthetic grounds — "misuse-
resistant by construction" — without checking whether the
construction covered the operations the system actually
needs. It didn't: PASETO has no public-key encryption mode,
which the system requires. When the requirements were stated
first (hybrid PQC encryption, algorithm agility, key-ID-based
rotation), the choice narrowed cleanly to JOSE or COSE, and
then to COSE on the secondary axes.

When revisiting an open question, write down what the design
must do before evaluating candidates.

A parallel pattern also worth keeping explicit: **delete knobs
that nobody asked for**. Several items in earlier drafts of
this file were "configurable per tenant" or "leaning X with
opt-out." Most of those have been collapsed to a committed
default with no knob. Flexibility is not free; every
configuration point is a surface that needs documentation,
testing, and mental maintenance. Commit, ship, iterate if
someone later has a concrete need.

## Blocking v1 implementation

### Per-implementation wire-protocol details

Only one v1 capability still has its wire shape pending:

- **`email_send`** — SMTP submission shape. Tier 2; deferred
  to after Tier 1 closes (i.e., after the embed tract pivot
  lands and Tier 1 publishes as a coherent set). Sketch
  against `lettre`'s submission API when the impl crate
  starts.

Settled since the previous draft:

- **`http_forward`** — reuses `mechanics_config::HttpEndpoint`
  verbatim; full spec in `08-connector-architecture.md`
  §Generic HTTP. Phase 6 Task 1 0.1.0 (2026-04-24).
- **`llm_generate`** — three dialects
  (`openai_native` / `vllm_native` / `tool_call_fallback`)
  with `strict: true` token-level schema enforcement.
  Phase 6 Task 2 `llm_openai_compat` 0.1.0 (2026-04-24).
- **`sql_query`** — driver-native placeholder syntax (`?`
  for MySQL, `$1` for Postgres), dict-per-row response with
  `columns` populated even on empty results, `UpstreamError
  { status: 500 }` sentinel for DB-side errors, no
  connector-level retries. Phase 7 Tier 1 `sql-postgres` +
  `sql-mysql` 0.1.0 (locally ready 2026-04-24; publish held
  pending embed pivot).
- **`embed` vs `vector_search`** — split into two capabilities
  (two crates).
- **`embed`** — local in-process inference with a
  binary-bundled ONNX model (no HuggingFace runtime fetch);
  `EmbedConfig` carries `model_id`; `EmbedRequest` /
  `EmbedResponse` per the spec at
  [`docs/notes-to-humans/2026-04-24-0005-phase-7-tier-1-embed-and-vector-search-spec.md`](../notes-to-humans/2026-04-24-0005-phase-7-tier-1-embed-and-vector-search-spec.md).
  Wire shape locked even though the implementation crate is
  mid-pivot from `fastembed` + `ort` to `tract` +
  `tokenizers` for musl-native pure-Rust inference; pivot
  plan at
  [`docs/notes-to-humans/2026-04-24-0008-phase-7-embed-tract-pivot-plan.md`](../notes-to-humans/2026-04-24-0008-phase-7-embed-tract-pivot-plan.md).
- **`vector_search`** — stateless in-memory cosine kNN,
  corpus-per-request (no persistent state), hundreds-to-
  thousands scale, strings-only id payload. Phase 7 Tier 1
  `vector-search` 0.1.0 (locally ready 2026-04-24; publish
  held with the rest of Tier 1).

## Non-blocking for v1 but important

### Multi-region deployment

Single-region for v1. Multi-region is substantial additional
design and testing surface.

### Distributed tracing beyond spans

`tracing` spans emit locally. OTLP export to a central
collector is deployment configuration — can be added without
code changes. Worth documenting the expected collector shape
once a first deployment sets one up.

### Testing strategy specifics

Integration tests across the full stack (substrate + executor
+ connector + policy + API) require real infrastructure.
testcontainers for MySQL works. Mechanics workers and
connector services need similar — testcontainers running
their binaries, or in-process test implementations.

## Exploratory

### Workflow authoring patterns documentation

Now that a concrete first consumer exists (the chat-app
tenant), patterns worth documenting:

- Chat-session-as-instance pattern.
- 4B-first LLM usage with vector-based classification and
  validation-based fallback.
- Context-size management for long-running instances.
- Instance-scoped ephemeral tokens.

Not a design decision, but worth capturing.

### Stateful connector concerns

Vector stores and SQL databases have persistent state. v1
leans "tenants manage their own state; Philharmonic just
queries." If population-via-workflow becomes important
(e.g., ingesting chat transcripts into a knowledge base),
upsert-style capabilities can be added in v2.

### Admin UI schema-driven form rendering

v1 ships with free-form JSON submission for tenant configs.
A future admin UI can render forms from per-implementation
JSON Schemas published in deployment config. Additive; not
blocking.

## Questions already answered

For reference; these were open at various points and are now
settled.

### Foundations

- **Token format**: COSE (COSE_Sign1 for signed tokens,
  COSE_Encrypt0 for encrypted payloads). PASETO ruled out for
  lacking public-key encryption. JOSE/COSE both qualify; COSE
  won on binary efficiency and IETF PQC work landing there
  first.
- **PQC scheme**: ML-KEM-768 + X25519 hybrid for KEM,
  AES-256-GCM for symmetric encryption. Ed25519 for signing
  with a documented path to hybrid ML-DSA + Ed25519.
- **Content type**: JSON only. No msgpack or protobuf
  negotiation.

### Crates and organization

- **`mechanics-config` extraction**: extract, with wrapper
  newtypes in `mechanics-core` for Boa GC trait impls.
- **Connector crate organization**: split into
  `philharmonic-connector-common`, `philharmonic-connector-
  client`, `philharmonic-connector-router`, `philharmonic-
  connector-service`.
- **Per-implementation crates**: one crate per implementation.
  Naming: `philharmonic-connector-impl-<n>`.
- **Connector router responsibilities**: pure dispatcher for
  v1.
- **`philharmonic-realm` crate**: folded into `connector-
  common`.
- **Policy crate boundaries**: one `philharmonic-policy` crate,
  not split.

### Tenancy and principals

- **Tenant hierarchy**: flat for v1 with customizable roles.
  Hierarchical deferred; adding a parent slot is additive.
- **Tenant subdomain naming rules**: `[a-z0-9]
  [a-z0-9-]{1,62}`, no leading digit, no consecutive hyphens,
  reserved names include realm names plus `admin`, `api`,
  `www`, `app`, `connector`.
- **Deployment-operator endpoint location**: whichever
  ingress the deployment designates — a distinct subdomain,
  a reserved path prefix, or a separate listener, as long
  as it is unambiguously distinct from any tenant's routing
  scope.
- **Principal model details**: persistent identity with
  long-lived API token (SHA-256 hashed in substrate).
  `epoch` scalar included on the entity for future
  self-contained-token migration; unused in v1.
- **Role definition schema**: JSON array of permission atoms.
  No scoped/conditional/ABAC for v1. Array-to-object
  extension remains additive.
- **Permission atom vocabulary**: the list in
  `09-policy-and-tenancy.md` is treated as closed for v1.
  Adjustments (additions, renames) are allowed as development
  surfaces concrete needs, but should go through a deliberate
  doc update rather than accreting silently.
- **Subject claim persistence in step records**: identifier +
  authority only. Full injected claims are never persisted.
  No per-tenant knob.
- **Automatic epoch bump on tenant suspension**: yes,
  automatic. No knob.
- **Minting authority rate limits**: deferred. Per-tenant API
  rate limit on the mint endpoint is the v1 guardrail.
- **Ephemeral token claim size limit**: 4 KB.

### Tokens and revocation

- **Ephemeral token model**: long-lived minting authority
  mints bearer ephemeral tokens with structured subject
  claims; tokens used directly (no session + access-token
  split); epoch-based mass revocation at the minting
  authority; no persistent subject entities.
- **Subject claims exposure to scripts**: ephemeral token
  carries structured claims; passed to the workflow script
  as `subject` alongside `context`, `args`, `input`. Claims
  are free-form for v1.
- **Long-lived token format**: opaque bearer with
  substrate-hash lookup for v1; `Principal.epoch` scalar
  reserved for future self-contained-token migration.

### Workflow layer

- **Chat-session-as-workflow-instance**: one chat session maps
  to one `WorkflowInstance`. Each user message is one
  `execute_step` call.
- **Workflow completion mechanism**: `complete()` engine
  method plus `done: true` script return.
- **Script size limit**: 1 MB (accommodates Webpack-bloated
  bundles within reasonable limits).

### Connector architecture

- **No global endpoint registry**. All per-call-site state
  lives in per-tenant `TenantEndpointConfig` entities.
  Templates reference configs by UUID.
- **Template abstract config shape**:
  `{script_name: config_uuid}` map.
- **Three-way model**: capability (wire shape documentation),
  implementation (Rust code), tenant config (per-tenant
  encrypted blob).
- **Connector service statelessness**: static impl registry
  built at binary startup; operational caches allowed.
- **Per-tenant credentials**: required for v1, embedded in
  the encrypted tenant config blob.
- **Credential encryption in transit**: hybrid PQC KEM per
  realm.
- **Decrypted payload shape**: arbitrary JSON, opaque to the
  framework; Implementations deserialize the `config`
  sub-object into concrete types.
- **Implementation trait**: `execute(config, request, ctx)`
  with `ConnectorCallContext` carrying verified token claims.
- **Token claim set**: `iss, exp, kid, realm, tenant, inst,
  step, config_uuid, payload_hash`. No `impl` claim; dispatch
  uses the `impl` field inside the decrypted payload.
- **Impl references**: tenant-specific (via `config_uuid` in
  token and `impl` field in encrypted payload), not generic
  kinds.
- **HTTP protocol uniformity**: every philharmonic endpoint is
  POST + literal URL + static token/payload headers.
- **Capability definitions as code vs. data**: neither, in the
  global-registry sense — there's no global registry. Wire
  protocols are documented by capability; per-tenant configs
  carry everything operational.

### Encryption

- **Substrate at-rest encryption**: whole-blob SCK encryption
  of `TenantEndpointConfig.encrypted_config`. Operators with
  `endpoint:read_decrypted` permission can retrieve decrypted contents
  via the API for verification.
- **Lowerer transformation**: pure byte forwarding.
  Decrypts SCK ciphertext, re-encrypts byte-identical to
  realm KEM. Only the `realm` field is inspected.
- **Credential encryption key management**: deployment-level
  shared SCK. Per-tenant keys deferred; envelope encryption
  with KMS is a deployment option.
- **Per-config encryption granularity**: each call encrypts a
  fresh payload to its config's realm KEM key.
- **Security boundary between framework and implementation**:
  framework validates cryptography, token claims, realm
  match, impl-registry dispatch; implementation validates
  `config` shape and capability-specific invariants.

### LLM implementations

- **LLM output mode**: schema-only; no free-form (free-form
  is a string schema); no tool calling at the wire protocol
  level.
- **LLM is not a first-class citizen**: LLM implementations
  are HTTP clients with provider-specific domain knowledge.
  No LLM-specific framework mechanisms.
- **LLM implementation set**: three —
  `llm_openai_compat` (covers OpenAI + vLLM + compatible
  gateways), `llm_anthropic`, `llm_gemini`. Each backs as
  many per-tenant configs as needed.
- **vLLM structured output mechanism**: native via top-level
  `structured_outputs: {"json": <schema>}` field in the
  request body. Our HTTP client constructs the body directly
  (no Python `extra_body` layer).

### API layer

- **Authentication mechanisms**: long-lived API tokens and
  ephemeral API tokens. No OAuth/OIDC, no session cookies
  for v1.
- **Rate limiting**: single-node per-tenant token buckets.
  Distributed rate limiting deferred.
- **Pagination**: cursor-based on every list endpoint;
  default page size 50, maximum 200.
- **Idempotency keys**: not in v1. Duplicate creates surface
  as `duplicate_entity` via application-layer uniqueness;
  clients retry by re-reading state.
- **Error envelope**: `{error: {code, message, details?,
  correlation_id}}` with lowercase snake_case codes and a
  settled HTTP-status → code mapping.
- **Connector router responsibilities**: pure dispatcher;
  early verification and rate limiting deferred.
- **Audit log implementation**: substrate-stored as
  `AuditEvent` entities; queried via existing substrate
  mechanisms.

### Observability

- **Correlation ID header**: `X-Correlation-ID`. UUID v4.
  Generated at API ingress if absent; forwarded on every
  hop; recorded on `StepRecord` entities.
- **Logging**: JSON-line via `tracing` +
  `tracing-subscriber` JSON formatter. Required fields: `ts`,
  `level`, `correlation_id`, `crate`, `msg`.
- **Metrics**: Prometheus via `metrics` +
  `metrics-exporter-prometheus`. Naming:
  `philharmonic_<component>_<thing>_<unit>`. No `tenant_id`
  as a label (cardinality); per-tenant observability via log
  fields.

### Simplifications committed by deletion

These were "leaning X with opt-out" or "configurable later"
items that got collapsed to committed defaults with no knob:

- Subject claim persistence (identifier + authority only).
- Automatic epoch bump on tenant suspension.
- Per-minting-authority rate limits (deferred entirely).
- Background reconciler for duplicate-config races (don't
  ship; accept the theoretical race).
- Role schema complexity (simple array).
- Capability/endpoint versioning (no formal versioning; new
  names for breaking changes).
- Payload builder (doesn't exist; lowerer is pure byte
  forwarder).
- Global endpoint registry (doesn't exist; all per-tenant).
- Authentication cache (not needed given SCK-decrypted
  operator-visible configs and the simple lookup pattern).
- Admin UI schema validation (free-form JSON v1; polish
  later).
