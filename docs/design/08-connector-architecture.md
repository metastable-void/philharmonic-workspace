# Connector Architecture

The connector layer routes script-originated calls to external
services, encrypts sensitive configuration end-to-end, and
isolates implementation code by realm. Primary architectural
decisions are settled.

## Core model

### Capabilities, implementations, and tenant configs

Three concepts, each with a specific job:

**Capabilities** are named wire-protocol shapes. A capability
declares "what a script sends and what it gets back" via JSON
Schema for request and response. `llm_generate`, `sql_query`,
`http_forward`, `embed`, `vector_search`, `email_send` are
capabilities. Capabilities are documentation and schema, not
runtime entities; scripts bind to them by the shape of the
messages they send and receive.

**Implementations** are Rust code that speaks to one category
of external service. `llm_openai_compat`, `llm_anthropic`,
`http_forward`, `sql_postgres` are implementations. Each lives
in its own crate; each is a crate-level shipping decision
bundled into connector service binaries at build time. Adding
a new implementation requires Rust work.

**Tenant endpoint configs** are per-tenant `TenantEndpointConfig`
entities in the substrate. Each entity holds an encrypted blob
describing one call-site: which realm to route to, which
implementation handles it, what credentials and per-call
configuration to supply. Adding or changing a call-site is a
tenant admin action — create a new entity or append a revision
to an existing one. No deployment-wide registry to update.

The relationships:

- One config → one implementation (the `impl` field inside the
  encrypted blob names it).
- One config → one realm (the `realm` field inside names it).
- Multiple configs can share an implementation — two configs
  both backed by `llm_openai_compat` with different base URLs
  and API keys is a normal case.
- A config's shape is private to the tenant and the
  implementation. The admin writes arbitrary JSON that the
  implementation knows how to consume; the substrate and the
  framework never inspect it.

### No global endpoint registry

Earlier drafts envisioned a deployment-level registry of named
endpoints that tenants would configure against. That's gone.
The only per-call-site state is the tenant's
`TenantEndpointConfig` entity. Workflow templates reference
configs by entity UUID. Human-visible names are a display-layer
concern only; the substrate's identity model (UUIDs) is the
identity model.

Consequences:

- Adding a call-site is a substrate write, not a deployment
  config change. Self-service for tenant admins.
- The deployment-wide static config shrinks to: the realm
  registry (realm names → KEM public keys), the impl crate
  bundling per realm binary, and the operational knobs.
- Two tenants can have configs pointing at the same external
  service with independent credentials, display names, and
  lifecycles. There's no shared object they both reference.

### Endpoints as the executor-level unit

The word **endpoint** in this document means an entry in the
executor's `MechanicsConfig` map — a URL, a token header, an
encrypted-payload header, ready to POST to. Endpoints exist
only at the executor level, at step-execution time. They're
built by the lowerer from the step's tenant configs and
discarded after the step completes.

Scripts call endpoints by their script-local name (a key in
the `MechanicsConfig` map). The lowerer established the
name → config-UUID mapping when it built the map, reading the
template's abstract config.

### Template abstract config

A workflow template's abstract config is a map from
script-local name to `TenantEndpointConfig` entity UUID:

```json
{
  "chat_llm": "01K9S4Z7X0P2YH8D6TC3B9MRFV",
  "user_db":  "01K9S4Z7X1QR3JNKWG2A8EHVBT"
}
```

The template author chooses script-local names; the script
uses those names. The UUIDs pin specific configs, which means
rotating a config (new revision, same UUID) propagates
automatically; retiring a config breaks the template at its
next step, which is the correct failure mode.

### Uniform HTTP

The executor-to-connector protocol is uniformly HTTP POST.
Scripts call endpoints by name; the runtime sends a POST to a
literal URL (the realm's connector router URL) with:

- The script's JSON body as the request body.
- `Authorization: Bearer <COSE_Sign1 token>` header.
- `X-Encrypted-Payload: <COSE_Encrypt0 payload>` header (or
  equivalent header name).

Every philharmonic endpoint in `MechanicsConfig` has the same
shape: POST, literal URL, static token and payload headers,
no URL slots, no query parameters. `mechanics-core`'s richer
`HttpEndpoint` features go unused in philharmonic deployments;
they exist for `mechanics-core` consumers outside philharmonic.

Only the script's JSON body and the connector authorization
token travel in cleartext (inside TLS). Credentials, routing
information, implementation name, and all other sensitive
context ride in the encrypted payload.

### Implementations are structurally equivalent; LLM is not privileged

All implementations are the same kind of thing: Rust code
satisfying the `Implementation` trait, making calls to one
category of external service. They differ only in the external
protocol they speak (HTTP for most, SQL wire protocol for
databases, SMTP for email) and in the domain knowledge they
encode.

**LLM connectors are not a first-class citizen of this
system.** The framework has no LLM-specific code paths, no
LLM-specific entity kinds, no LLM-specific token claims, no
LLM-specific anything. `llm_openai_compat` is structurally
identical to `http_forward` — both are HTTP clients with
domain knowledge for their respective external services. What
distinguishes an LLM implementation from the generic HTTP
forwarder is only the domain knowledge it carries: how to
translate the normalized `llm_generate` wire protocol to a
provider's native API shape, how to parse that provider's
structured-output mechanism, how to report token usage.

The reason LLM implementations are separate crates rather than
configurations of `http_forward` is practical, not
architectural: translating between the normalized protocol and
each provider's API involves enough real code that it's cleaner
as a dedicated implementation than as a declarative
configuration. The same applies to SQL and SMTP. None of these
categories is privileged; they all plug in through the same
trait.

The practical consequence: resist the temptation to add
LLM-specific mechanisms. If a feature seems to require LLM
awareness at the framework, capability, or token level, the
design has probably gone wrong — either the feature should
live inside the implementation's domain knowledge, or the
capability's wire protocol should be extended in a way that
applies to any implementation.

## Crate organization

Five crates plus per-implementation crates:

### `philharmonic-connector-common`

Shared vocabulary. Depended on by every other connector crate.

- COSE token structure types and token claim definitions.
- Realm model (realm identifiers, realm KEM public key
  registry entries).
- `ConnectorCallContext` (verified token claims delivered to
  implementations).
- `ImplementationError` and other shared error types.

The encrypted payload itself is treated as arbitrary JSON by
this crate — no shared envelope type.

No heavy dependencies. `philharmonic-types` and
`mechanics-config` are the main deps.

### `philharmonic-connector-client`

The lowerer: produces concrete `MechanicsConfig` values from a
workflow template's abstract config at step-execution time.

- Implements the `ConfigLowerer` trait defined in
  `philharmonic-workflow`.
- Resolves `script_name → config_uuid` from the template's
  abstract config.
- Consults `philharmonic-policy` to fetch
  `TenantEndpointConfig` entities by UUID.
- Decrypts each config's blob with the substrate credential
  key (SCK).
- Reads the `realm` field from the decrypted blob to pick the
  target realm KEM key.
- Re-encrypts the decrypted blob — byte-identical — to the
  realm's KEM public key via COSE_Encrypt0.
- Mints one signed token per config, binding
  instance/step/tenant/config context to the payload hash via
  COSE_Sign1.
- Assembles the `MechanicsConfig` map the executor consumes.

Depends on `connector-common`, `philharmonic-store`,
`philharmonic-policy`, `philharmonic-workflow`, and
`mechanics-config`. Crucially: **no dependency on
`mechanics-core`**. The lowerer stays Boa-free.

The lowerer's transformation of the per-tenant blob is
**minimal**: decrypt from SCK, inspect only the `realm` field,
re-encrypt as-is to the realm KEM. No field extraction,
substitution, synthesis, or reshaping. The admin's submitted
JSON is the payload, round-tripped through two encryption
boundaries.

### `philharmonic-connector-router`

Pure HTTP dispatcher. Deployed as a small static binary per
realm.

- Terminates TLS for `<realm>.connector.our-domain.tld`.
- Forwards requests to connector services in the realm.
- Load-balances across connector service instances.
- No token verification (services do this).
- No payload decryption (services do this).
- No rate limiting (deferred).
- No policy (not its concern).

Minimal dependencies: `connector-common` for routing model
types, plus standard HTTP infrastructure.

### `philharmonic-connector-impl-api`

Trait-only contract that per-implementation crates and the
service framework agree on. Non-crypto; no key material; no
network.

- `Implementation` trait (one `async fn execute` method + a
  `name` accessor — full signature in the §"Implementation
  trait" below).
- Re-exports `ConnectorCallContext` and `ImplementationError`
  from `connector-common` so impl crates depend on exactly one
  connector crate at call-site granularity.

Depends on `connector-common` and `async-trait`. Nothing else
from the philharmonic namespace; nothing crypto-sensitive.
This separation means: updating the `Implementation` trait
surface never touches the crypto-reviewed
`connector-service` crate, and impl crates don't transitively
pull crypto code into their dependency tree.

Versioning discipline: the trait is an API seam between the
service framework and every in-tree / third-party impl crate.
A breaking change here forces a coordinated bump across every
impl crate; a non-breaking extension (default-method, new
optional re-export) is the typical release. Bumps to this
crate don't trigger the crypto review protocol (the crate
holds no keys and speaks no wire format).

### `philharmonic-connector-service`

Framework for per-realm connector service binaries.

- HTTP listener.
- COSE_Sign1 signature verification.
- COSE_Encrypt0 payload decryption with the realm private key.
- Token claim checking (expiry, realm, payload hash).
- Implementation registry built at binary startup from linked
  impl crates (holds `Box<dyn Implementation>` values from
  `connector-impl-api`).
- Dispatch to the implementation named by the decrypted
  payload's `impl` field.
- Response path back to the executor.

Per-realm binaries link the service framework together with
the specific implementation crates configured for that realm.
Depends on `connector-common` (crypto primitives, token
claims), `connector-impl-api` (the trait it dispatches to),
and little else from the philharmonic namespace.

### Per-implementation crates

One crate per implementation. Each implements the
`Implementation` trait from `connector-impl-api`. Each carries
its own external dependencies (reqwest, sqlx, lettre,
qdrant-client, etc.).

Implementation crates depend only on `connector-impl-api`
(and transitively `connector-common`); they don't see
workflow, policy, storage, or any crypto concerns directly.
The service framework in `connector-service` hands them only
decrypted plaintext + verified context.

Naming convention: `philharmonic-connector-impl-<name>`.

## Wire protocol

### The COSE formats

Both signed tokens and encrypted payloads use COSE:

- **COSE_Sign1** for the authorization token in the
  `Authorization` header.
- **COSE_Encrypt0** for the encrypted payload in the
  `X-Encrypted-Payload` header.

COSE was chosen for algorithm agility (clean path to PQ
signing), hybrid PQC KEM support, key-ID-based rotation, and
binary efficiency in HTTP headers. See
`11-security-and-cryptography.md` for the full cryptographic
design.

### Token claims

The signed token's COSE payload carries exactly:

- `iss` — issuer (the lowerer / deployment).
- `exp` — expiry (short-lived, minutes).
- `kid` — signing key ID (for rotation overlap periods).
- `realm` — destination realm name. The connector router uses
  this to route; the connector service uses it to sanity-check
  that the request reached the correct binary.
- `tenant` — tenant UUID.
- `inst` — workflow instance UUID.
- `step` — step sequence number within the instance.
- `config_uuid` — `TenantEndpointConfig` UUID. Used for audit
  correlation; not used for dispatch.
- `payload_hash` — SHA-256 hash of the COSE_Encrypt0 payload
  bytes. Binds the token to its payload and prevents
  mix-and-match attacks.

Dispatch is on the `impl` field inside the decrypted payload,
not a token claim. The token records identity and binding; the
payload records everything else.

### Encrypted payload contents

The COSE_Encrypt0 payload is **arbitrary JSON**. It carries
whatever the admin submitted for this config. The conventional
top-level shape:

```json
{
  "realm": "llm",
  "impl": "llm_openai_compat",
  "config": {
    "base_url": "https://api.openai.com/v1",
    "api_key": "sk-...",
    "model": "gpt-4o",
    "structured_output_dialect": "openai_native",
    ...
  }
}
```

- `realm` must match the token's `realm` claim (framework
  checks; mismatch is a rejection).
- `impl` is looked up in the connector service's
  implementation registry; unknown names are rejected.
- `config` is passed opaquely to the implementation; its shape
  is a private contract between the admin (who wrote it) and
  the implementation (which deserializes it, typically via
  serde into a concrete struct).

Identity fields (tenant, instance, step, config UUID) are not
duplicated in the payload. They live in the token claims and
are delivered to the implementation as `ConnectorCallContext`.

The framework never inspects `config`. Deserialization
failures inside the implementation are normal error paths
(bad config → implementation returns `ImplementationError`).

### Payload encryption

Encryption uses COSE_Encrypt0 with a hybrid construction:

- ML-KEM-768 encapsulation to the realm's ML-KEM public key
  produces one shared secret.
- X25519 ECDH with the realm's X25519 public key produces
  another shared secret.
- Both secrets are combined via HKDF-SHA256 to derive the
  symmetric key.
- The payload is encrypted under this key with AES-256-GCM.

Encryption happens per config per call. A step that touches
three configs produces three independently-encrypted payloads
and three tokens.

## Connector services

### Per-realm binaries

One static binary per realm. Contains:

- The `connector-service` framework.
- The implementation crates configured for this realm.
- An in-memory implementation registry: `impl_name → handler`.
- The long-lived realm KEM private keys (loaded at startup
  from deployment secret storage).
- The lowerer's public signing key (for token verification).

Different realms can include different implementations — an
LLM-only realm bundles only LLM implementations; a
general-purpose realm bundles everything.

### Statelessness

No cross-request state beyond:

- The realm private key (long-lived; changes only on rotation).
- The lowerer's public signing key (updated on rotation).
- The implementation registry (static, built at startup).
- Operational caches (HTTP connection pools, database
  connection pools).

No per-request state survives the request. No credential cache
— credentials arrive via the encrypted payload on each request.

### Security boundary

The framework and the Implementation split responsibilities:

**Framework (`connector-service`) validates:**

- COSE_Sign1 signature against the lowerer's public key
  (looked up by `kid`).
- Token `exp` not passed.
- `payload_hash` claim matches SHA-256 of the encrypted
  payload bytes.
- Token `realm` claim matches this binary's realm.
- Decrypted payload's `realm` field matches the token's
  `realm` claim (belt-and-suspenders; AEAD already binds the
  ciphertext to the realm key, but cheap to re-check).
- Decrypted payload's `impl` field is registered in this
  binary's implementation registry.

Once these pass, the payload's `config` sub-object is passed
to the implementation. The implementation deserializes and
validates the config's shape; failures surface as error
responses to the executor.

**Implementation validates:**

- `config` shape — typically by serde-deserializing into a
  concrete Rust struct.
- Capability-specific invariants — allowed methods, URL
  allowlists, body size limits, etc.
- The script's request body against the capability's wire
  protocol.

The framework never looks inside `config`.

### Request handling

1. Router receives request, forwards to a connector service
   instance in the realm.
2. Service verifies the COSE_Sign1 token signature against the
   lowerer's public key (looked up by `kid`).
3. Service checks token claims: `exp`, `realm`,
   `payload_hash`.
4. Service decrypts the COSE_Encrypt0 payload with the realm
   private key (looked up by `kid`).
5. Service parses the decrypted JSON, extracts `realm`, `impl`,
   `config`. Checks decrypted `realm` matches token `realm`.
   Looks up `impl` in the implementation registry; rejects if
   unknown.
6. Service dispatches to the implementation, passing the
   `config` sub-object, the script's request body, and a
   `ConnectorCallContext` built from the verified token claims.
7. Implementation deserializes `config`, validates, does its
   work (calls external service, formats response).
8. Response returns to the executor through the router.

### Implementation trait

Defined in the dedicated trait-only crate
`philharmonic-connector-impl-api`. The crypto-reviewed
`connector-service` crate depends on it for dispatch; impl
crates depend on it for the trait definition. Both routes
converge on the same symbol without either depending on the
other, and without pulling the crypto crate's dep surface into
impl crates.

```rust
#[async_trait]
pub trait Implementation: Send + Sync {
    fn name(&self) -> &str;

    async fn execute(
        &self,
        config: &JsonValue,
        request: &JsonValue,
        ctx: &ConnectorCallContext,
    ) -> Result<JsonValue, ImplementationError>;
}

pub struct ConnectorCallContext {
    pub tenant_id: Uuid,
    pub instance_id: Uuid,
    pub step_seq: u64,
    pub config_uuid: Uuid,
    pub issued_at: UnixMillis,
    pub expires_at: UnixMillis,
}
```

`ConnectorCallContext` and `ImplementationError` originate in
`connector-common` and are re-exported by `connector-impl-api`,
so impl crates can work against one direct dependency
(`connector-impl-api`) rather than juggling the common crate
alongside. The async-trait macro ships from the separate
`async-trait` crate; `connector-impl-api` re-exports that too.

Three inputs:

- `config` — the decrypted `config` sub-object from the
  encrypted payload. Implementations deserialize it into a
  concrete struct before use.
- `request` — the script's cleartext JSON body. Passed through
  unchanged.
- `ctx` — verified token claims. Useful for logging, metrics,
  and per-tenant behavior. The framework has already verified
  everything in the context; implementations consume it as
  trusted metadata.

`ConnectorCallContext` uses `Uuid` rather than typed entity IDs
to keep `connector-common` free of dependencies on
`philharmonic-policy` or `philharmonic-workflow`.

#### Why `async_trait` (in 2026)

The trait is declared with the `#[async_trait]` macro from the
`async-trait` crate rather than with native `async fn` in
traits (stabilized in Rust 1.75). The choice is deliberate
because of subtle compatibility friction that the native
mechanism still has in 2026:

- **Dyn-compatibility (object safety).** The service
  framework's implementation registry holds impls as
  `Box<dyn Implementation>` (or `Arc<dyn Implementation>`) in
  a `HashMap<impl_name, _>`. Native `async fn` in traits
  doesn't produce a dyn-compatible trait on its own — the
  return-position `impl Future` makes the method un-object-safe
  without either rewriting every method as
  `fn foo(&self, …) -> Pin<Box<dyn Future<Output = …> + Send + 'a>>`
  by hand, or opting into the `trait_variant::make` workaround
  with its own macro surface. `#[async_trait]` does that Box-
  allocated Pin rewrite for us, uniformly, at the trait
  declaration site.
- **`Send`-bound inference on returned futures.** Native async
  methods produce opaque futures whose `Send`-ness is inferred
  from captured state and is **not** advertised in the trait
  method signature. Every caller that wants to move the future
  across threads has to add `where T::Output: Send` bounds or
  use the `trait_variant` pattern to re-expose a `Send` bound.
  `#[async_trait]` hard-codes `+ Send` on the boxed future,
  which is what the service framework (tokio-multi-threaded
  request handling) requires anyway.
- **Cost is negligible for this workload.** `async_trait`'s
  per-call cost is one `Box` allocation for the future. Every
  `Implementation::execute` call is dominated by external I/O
  (HTTP, SQL, SMTP, LLM-provider APIs); a heap allocation on
  that scale does not show up.
- **Ecosystem alignment.** As of 2026, `async_trait` is still
  the idiomatic choice across the Rust async ecosystem for
  dyn-compatible traits with async methods (tower services,
  axum extractors, actor frameworks). New contributors meet
  it on familiar terms.
- **Migration is mechanical.** If/when a future Rust release
  makes native async-fn-in-traits fully dyn-compatible without
  macro gymnastics, swapping `#[async_trait]` out is a
  trait-declaration-plus-impl-site mechanical change. The
  macro does not lock the ecosystem in.

Pinning: `async-trait = "0.1"`. The crate has been at 0.1.x
with an exceptionally stable surface for years. `connector-impl-api`
re-exports `async_trait::async_trait` so impl crates use the
same macro version the trait was declared with, avoiding any
multi-version drift.

## v1 implementation set

Implementations in the v1 set, organized by external-service
protocol. All share the same `Implementation` trait contract.
Generic HTTP is listed first because it's the baseline from
which LLM implementations are specializations.

### Generic HTTP

One implementation: **`http_forward`**.

Forwards HTTP requests to config-specified targets with
credential injection. Used for pass-through forwarding,
templated forwarding, and webhooks. Many configs share this
one implementation. Transactional email providers (SendGrid,
Postmark, AWS SES) are served by configs pointing at
`http_forward`, not by separate email implementations.

#### Config shape

**Principle.** `http_forward` does not invent its own endpoint
schema. It reuses `mechanics_config::HttpEndpoint` verbatim —
the same structure `mechanics-core` uses for JS-side HTTP
endpoint definitions. One schema for "how to describe an HTTP
call-site" across the whole system: validation, URL templating,
query emission, header allowlisting, retry policy, body typing,
and size caps all come from that shared type.

The `config` sub-object contains exactly one key, `endpoint`,
whose value is an `HttpEndpoint`:

```json
{
  "realm": "http",
  "impl": "http_forward",
  "config": {
    "endpoint": {
      "method": "POST",
      "url_template": "https://api.example.com/users/{user_id}/events",
      "url_param_specs": {
        "user_id": {
          "default": null,
          "min_bytes": 1,
          "max_bytes": 64
        }
      },
      "query_specs": [
        { "type": "const",   "key": "api_version", "value": "2024-01" },
        { "type": "slotted", "key": "trace_id",    "slot": "trace_id",
          "mode": "optional", "default": null,
          "min_bytes": null, "max_bytes": 128 }
      ],
      "headers": {
        "Authorization": "Bearer sk-...",
        "Content-Type": "application/json"
      },
      "overridable_request_headers": ["Idempotency-Key"],
      "exposed_response_headers": ["X-Request-Id"],
      "request_body_type":  "json",
      "response_body_type": "json",
      "response_max_bytes": 1048576,
      "timeout_ms": 30000,
      "allow_non_2xx_status": false,
      "retry_policy": {
        "max_attempts": 3,
        "base_backoff_ms": 200,
        "max_backoff_ms": 5000,
        "max_retry_delay_ms": 30000,
        "rate_limit_backoff_ms": 1000,
        "retry_on_io_errors": true,
        "retry_on_timeout": true,
        "respect_retry_after": true,
        "retry_on_status": [429, 500, 502, 503, 504]
      }
    }
  }
}
```

Field semantics follow `mechanics_config::HttpEndpoint`
exactly. Summarized for reference:

- **`method`**: HTTP verb (`GET`, `POST`, etc.).
- **`url_template`**: absolute URL with `{slot}` placeholders.
  Must not carry a fragment or pre-baked query string.
- **`url_param_specs`**: per-slot constraints — optional
  `default`, `min_bytes`, `max_bytes` (UTF-8 byte lengths).
  Every slot appearing in `url_template` must be declared.
- **`query_specs`**: array of query emission rules.
  - `{"type":"const","key":..,"value":..}` emits a fixed pair.
  - `{"type":"slotted","key":..,"slot":..,"mode":..,
    "default":..,"min_bytes":..,"max_bytes":..}` reads from
    the request's `queries[slot]` under one of four modes:
    `required`, `required_allow_empty`, `optional`,
    `optional_allow_empty`.
- **`headers`**: baked-in request headers. This is where
  credentials live — the script never sees the API key.
- **`overridable_request_headers`**: case-insensitive
  allowlist of header names the script may set on a per-call
  basis. Names outside this list are rejected.
- **`exposed_response_headers`**: case-insensitive allowlist
  of response headers surfaced to the script. Everything else
  is dropped before returning.
- **`request_body_type`**: `json` | `utf8` | `bytes`. Controls
  how the script-provided `body` is serialized and which
  `Content-Type` the client defaults to. Optional; defaults
  to `json`.
- **`response_body_type`**: `json` | `utf8` | `bytes`.
  Defaults to `json`.
- **`response_max_bytes`**: optional per-endpoint response-size
  cap. When absent, the framework default applies.
- **`timeout_ms`**: optional per-endpoint request timeout.
- **`allow_non_2xx_status`**: when `false` (default), non-2xx
  responses surface as `upstream_error`. When `true`, the
  response is returned normally and the script inspects
  `status`/`ok`.
- **`retry_policy`**: `EndpointRetryPolicy` — `max_attempts`,
  `base_backoff_ms`, `max_backoff_ms`, `max_retry_delay_ms`,
  `rate_limit_backoff_ms`, `retry_on_io_errors`,
  `retry_on_timeout`, `respect_retry_after` (honors
  `Retry-After` on 429), `retry_on_status` (list of HTTP
  statuses eligible for retry).

The connector service deserializes the `endpoint` object and
calls `HttpEndpoint::prepare_runtime` at load time; invalid
configs fail (with an `io::Error`) before any external call
happens, and the returned `PreparedHttpEndpoint` is cached for
reuse across requests to the same config.

#### Request shape (from script)

Fields are camelCase over the wire (matching the JS surface
that `mechanics-core` already exposes) and `serde`-rename to
snake_case internally.

```json
{
  "urlParams": { "user_id": "u_12345" },
  "queries":  { "trace_id": "req-abc-123" },
  "headers":  { "Idempotency-Key": "req-abc-123" },
  "body":     { ... }
}
```

- **`urlParams`**: values for `url_template` slots. Resolved
  through `url_param_specs` (default / min / max byte length).
  Unknown slot keys are rejected. Percent-encoded during URL
  construction.
- **`queries`**: values for `slotted` query rules. Keys outside
  the declared slot set are rejected.
- **`headers`**: values for headers in
  `overridable_request_headers`. Matching is case-insensitive;
  duplicates are rejected. Unknown names are rejected.
- **`body`**: request body. Interpreted according to
  `request_body_type`:
  - `json` → any JSON value; serialized as the request body
    with `Content-Type: application/json`.
  - `utf8` → JSON string; sent as a UTF-8 `text/plain` body.
  - `bytes` → base64-encoded JSON string; sent as a raw
    `application/octet-stream` body.
  - Missing / `null` → no body sent.

Fields with defaults in `url_param_specs` / `query_specs` may
be omitted from the request.

#### Response shape

```json
{
  "status":  200,
  "ok":      true,
  "headers": { "X-Request-Id": "xyz" },
  "body":    { ... }
}
```

- **`status`**: HTTP status code (integer).
- **`ok`**: convenience flag, `true` iff `status` is in
  2xx. (Mirrors `mechanics-core`'s
  `EndpointResponse.ok`.)
- **`headers`**: only response headers listed in
  `exposed_response_headers`, normalized to lowercase names.
- **`body`**: decoded according to `response_body_type`:
  - `json` → parsed JSON value (or `null` if the response was
    empty).
  - `utf8` → UTF-8 string.
  - `bytes` → base64-encoded string.

#### Error cases

Mapped to `ImplementationError` variants:

- **`upstream_error`** — non-2xx response, and
  `allow_non_2xx_status` is `false`. Carries the upstream
  `status`, the exposed response headers, and the decoded body
  so the script can still inspect what came back.
- **`upstream_unreachable`** — transport error; all retries
  exhausted. Carries the underlying error kind.
- **`upstream_timeout`** — request exceeded `timeout_ms`
  (including retries, subject to `max_retry_delay_ms`).
- **`response_too_large`** — response body exceeded
  `response_max_bytes`.
- **`invalid_request`** — the script's request didn't match
  the endpoint schema (missing required slot, header not in
  the override allowlist, body type mismatch, etc.).
- **`invalid_config`** — the `endpoint` object failed
  validation at load time. Surfaces as a deserialization
  failure rather than a runtime error.

Scripts distinguish "upstream returned a response I should
handle" from "the call never completed" by whether the
implementation returned a response object or raised an error
— identical to the error model for JS-side
`mechanics-core` HTTP endpoint calls.

### LLM — specialized HTTP implementations

LLM implementations are HTTP clients with domain knowledge for
specific LLM provider APIs. Architecturally not privileged
above `http_forward`; separate crates only because translating
the normalized `llm_generate` wire protocol to each provider's
native API involves enough code (request translation, response
parsing, structured-output mechanism handling, usage
accounting) to be cleaner as a dedicated implementation than
as a declarative `http_forward` configuration.

Three implementations cover the major provider API shapes:
`llm_openai_compat`, `llm_anthropic`, `llm_gemini`. All three
implement the `llm_generate` capability with the same
normalized wire protocol described below.

#### The `llm_generate` wire protocol

The capability's wire protocol is an OpenAI-like-but-minimal
normalization: familiar enough for anyone who's touched an LLM
API, stripped of provider-specific extensions. Concretely it
carries messages + output schema + generation knobs in, and
structured output + stop reason + token usage out.

**Request shape (from script):**

```json
{
  "model": "gpt-4o-mini",
  "messages": [
    { "role": "system", "content": "..." },
    { "role": "user",   "content": "..." },
    { "role": "assistant", "content": "..." }
  ],
  "output_schema": { "type": "object", "properties": { ... }, "required": [...] },
  "max_output_tokens": 1024,
  "temperature": 0.2,
  "top_p": 1.0,
  "stop": ["\n\n"]
}
```

- `model` — required. Script picks which model. Implementations
  don't silently substitute.
- `messages` — required. Array of `{role, content}` with roles
  `system`, `user`, `assistant`. Single-string content only
  (no multi-part content parts; no images in v1).
- `output_schema` — required. JSON Schema describing the
  expected output shape. Free-form text is expressed as
  `{"type": "string"}`.
- `max_output_tokens`, `temperature`, `top_p`, `stop` —
  optional generation knobs. Implementations forward them to
  the provider with any needed per-provider mapping. Unknown
  knobs in the request are rejected.

Model, temperature, and all other generation parameters are
script-controlled. The config carries credentials, endpoint
routing, and dialect; not per-call parameters.

**Response shape:**

```json
{
  "output": { ... matches output_schema ... },
  "stop_reason": "end_turn",
  "usage": {
    "input_tokens": 412,
    "output_tokens": 87
  }
}
```

- `output` — the structured output produced by the model,
  already matching `output_schema`. Implementations do the
  JSON-parse-and-validate step before returning.
- `stop_reason` — normalized values: `end_turn`, `max_tokens`,
  `stop_sequence`, `content_filter`, `error`. Provider-
  specific values are mapped.
- `usage` — token counts. Both fields required on success.

**Error cases:**

- `schema_validation_failed` — provider returned output that
  didn't match `output_schema`. Rare with native guided
  decoding; possible with tool-call-fallback dialect.
- `upstream_error` — provider returned an error response.
  Details in the envelope.
- `upstream_unreachable`, `upstream_timeout` — network or
  timeout failures.
- `invalid_request` — malformed request (bad schema, unknown
  knob, etc.).

No tool calling at the wire protocol level. Agentic loops are
composed in JavaScript by the workflow author using structured
output at each iteration. This keeps control flow in the
workflow script (deterministic, auditable, testable) rather
than delegated to the LLM.

This is distinct from the internal mechanism an implementation
uses to transport a structured-output request. When
`llm_openai_compat` falls back to the tool-calling dialect for
a server that supports no native structured-output mechanism,
it's using tool calling as an internal implementation detail.
Invisible to the script; the `llm_generate` capability never
exposes tool-calling semantics to workflow authors.

**Schema-only output** — no free-form mode. Free-form text is
expressed as a schema like `{"type": "string"}`.

#### `llm_openai_compat` — config and dialects

Covers endpoints backed by the OpenAI service itself and any
OpenAI-compatible server (vLLM, Together, Groq, OpenRouter's
compat mode, local compatible servers).

```json
{
  "realm": "llm",
  "impl": "llm_openai_compat",
  "config": {
    "base_url": "https://api.openai.com/v1",
    "api_key": "sk-...",
    "dialect": "openai_native",
    "timeout_ms": 60000
  }
}
```

- `base_url` — required. Chat-completions endpoint lives at
  `<base_url>/chat/completions`.
- `api_key` — required. Sent as `Authorization: Bearer <key>`.
- `dialect` — required. One of:
  - `openai_native` — native `response_format: json_schema` in
    the request body. Preferred when the endpoint points at
    OpenAI itself or a server that faithfully implements the
    same `response_format` contract.
  - `vllm_native` — native structured outputs via a top-level
    `structured_outputs: {"json": <schema>}` field in the
    request body. Enforced at the token level via vLLM's
    guided decoding (xgrammar / outlines backends). Reliable
    even on small open-weight models.
  - `tool_call_fallback` — declares a single synthetic "tool"
    whose input schema is `output_schema`, forces the model
    to call it via `tool_choice`, extracts the tool-call
    arguments as the output. Fallback dialect for servers
    that support neither native path. Relies only on the
    OpenAI tool-calling contract.
- `timeout_ms` — optional, default 60s.

The dialect is an endpoint-level decision in static
configuration because it's a property of the target server,
not of the tenant or the script. Tenants don't choose the
dialect; they choose the endpoint, which pins a dialect.

#### `llm_anthropic` — config

Uses Anthropic's Messages API and its native structured-output
mechanism.

```json
{
  "realm": "llm",
  "impl": "llm_anthropic",
  "config": {
    "base_url": "https://api.anthropic.com",
    "api_key": "sk-ant-...",
    "anthropic_version": "2023-06-01",
    "timeout_ms": 60000
  }
}
```

#### `llm_gemini` — config

Uses Gemini's native `responseSchema` in `generationConfig`.

```json
{
  "realm": "llm",
  "impl": "llm_gemini",
  "config": {
    "base_url": "https://generativelanguage.googleapis.com/v1",
    "api_key": "...",
    "timeout_ms": 60000
  }
}
```

### SQL

Per database family: at minimum **`sql_postgres`** and
**`sql_mysql`**. Both implement the `sql_query` capability.

Wire protocol: parameterized SQL string plus positional
parameter values, rows as dict-per-row maps. Not HTTP; the
implementations speak the database wire protocol directly via
`sqlx`.

#### Config shape

```json
{
  "realm": "sql",
  "impl": "sql_postgres",
  "config": {
    "connection_url": "postgres://user:pass@host:5432/db",
    "max_connections": 10,
    "default_timeout_ms": 30000,
    "default_max_rows": 10000
  }
}
```

- `connection_url` — required. Full `sqlx` connection string.
  Credentials embedded here; never visible to scripts.
- `max_connections` — per-tenant pool size.
- `default_timeout_ms`, `default_max_rows` — query-level
  defaults that scripts may override downward but not upward.

The same config shape applies to `sql_mysql` with a
`mysql://...` connection URL. Different drivers, same wire
protocol.

#### Request shape

```json
{
  "sql": "SELECT id, name, created_at FROM users WHERE tenant = ? AND created_at > ?",
  "params": ["t_abc", "2025-01-01T00:00:00Z"],
  "max_rows": 500,
  "timeout_ms": 5000
}
```

- `sql` — arbitrary SQL, required. Parameter placeholders use
  `sqlx`'s native syntax for the target driver: `?` for
  MySQL, `$1, $2, ...` for Postgres. Scripts write SQL for
  the target driver rather than a portable dialect; the
  implementations don't translate.
- `params` — positional parameter values. JSON types map to
  SQL types via `sqlx`'s normal bindings: strings, numbers,
  booleans, null. Dates/times as RFC 3339 strings. The
  implementation binds parameters with `sqlx::query().bind()`
  semantics, which is always safely-parameterized; **no
  string interpolation ever**.
- `max_rows`, `timeout_ms` — optional overrides, clamped to
  the config's defaults.

#### Response shape

```json
{
  "rows": [
    { "id": "u_1", "name": "Alice", "created_at": "2025-03-01T12:00:00Z" },
    { "id": "u_2", "name": "Bob",   "created_at": "2025-03-02T09:30:00Z" }
  ],
  "row_count": 2,
  "columns": [
    { "name": "id",         "sql_type": "text" },
    { "name": "name",       "sql_type": "text" },
    { "name": "created_at", "sql_type": "timestamptz" }
  ],
  "truncated": false
}
```

- `rows` — dict-per-row. Column name as key, value as the
  SQL-to-JSON-mapped value. Column ordering is preserved via
  the separate `columns` array for scripts that need it.
- `row_count` — rows returned. Equals `rows.length`.
- `columns` — column metadata, populated even when `rows` is
  empty so scripts can introspect the result shape.
- `truncated` — `true` if `max_rows` clipped the result set.

For INSERT/UPDATE/DELETE: `rows` is empty, `columns` is empty,
`row_count` reflects rows affected.

#### SQL-to-JSON type mapping

- Integer types → JSON number (with i64 range checking;
  overflow returns `upstream_error`).
- Floating-point → JSON number.
- Decimal/numeric → JSON string (avoids float precision loss).
- Boolean → JSON boolean.
- Text/varchar → JSON string.
- Bytea/blob → JSON string (base64-encoded).
- Date, time, timestamp → JSON string (RFC 3339).
- NULL → JSON null.
- Arrays (Postgres) → JSON array.
- JSON/JSONB columns → JSON value verbatim.

#### Error cases

- `invalid_sql` — SQL syntax error, unknown column, etc.
  Database-reported errors surface here with the database's
  own message.
- `parameter_mismatch` — number of `params` doesn't match the
  placeholders in `sql`.
- `upstream_error` — database error during execution
  (constraint violation, deadlock, permission denied).
- `upstream_timeout` — query exceeded `timeout_ms`.
- `upstream_unreachable` — couldn't reach the database.

### SMTP

One implementation: **`email_smtp`**. Submits messages via SMTP
to configured submission servers.

Transactional email providers are served by `http_forward`-
backed configs, not distinct email implementations.

### Embedding and vector search

**Decision (settled): split.** Two capabilities in separate
crates.

- **`embed`** — text in, vector out. Implementations: local
  embedding model, hosted APIs, provider-bundled.
- **`vector_search`** — vector in, nearest neighbors out.
  Implementations: Qdrant; possibly others.

At least one implementation of each ships in v1. Splitting
keeps the two concerns independent: an embedding change
doesn't force a vector-store release and vice versa, and
tenants can mix embedders with vector stores freely.

### State management for stateful external services

Vector stores and SQL databases have persistent state. v1
position: **tenants manage state out-of-band.** Philharmonic's
vector and SQL capabilities are query-shaped; populating a
vector store or migrating a schema happens outside
Philharmonic via native tooling.

## Deployment topology

- `https://<realm>.connector.our-domain.tld/` — per-realm
  connector router URL.
- Wildcard HTTPS certificate for `*.connector.our-domain.tld`.
- Connector router per realm, small static binary.
- Connector services behind each router, horizontally scaled.
- Implementations bundled into each realm's service binary at
  build time. No runtime plug-ins.

Typical realm layout: `llm`, `sql`, `http`, `email`, `vector`,
or combined into fewer realms as operationally convenient.
Each realm has its own KEM keypair; realm granularity is a
blast-radius decision.

## Admin UI

Admin-submitted configs are **free-form JSON** for v1. No
schema validation at the API layer, no per-impl form schemas
in the admin UI. Operators read docs or copy working examples.
A typo like `"lm_openai_compat"` fails at the first call with
an explicit "unknown impl" error from the connector service —
loud, late, safe.

This is the v0 UX. Schema-driven form rendering can be added
later as an additive feature without rearchitecture.

## What the connector layer doesn't do

- **Execute business logic.** That's in scripts.
- **Hold authoritative state.** That's in the substrate.
- **Validate config shape.** Configs are free-form JSON;
  implementations validate when they deserialize.
- **Mint API authentication tokens.** Those come from the API
  layer.
- **Know about workflows.** Connector services see only
  authenticated requests with decrypted configuration.
- **Inspect payload contents at the framework level.** The
  framework's validation stops at token claims and the
  top-level `realm`/`impl` fields.

## Open questions

Narrow and specific:

- **Embedding and `vector_search` split versus unified
  capability.**
- **Per-implementation wire-protocol details** — exact request
  and response JSON shapes for SQL row data, HTTP templating
  syntax, embedding output format.

Everything else in this area is settled.
