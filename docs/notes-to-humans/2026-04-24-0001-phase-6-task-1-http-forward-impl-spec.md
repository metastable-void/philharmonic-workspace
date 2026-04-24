# Phase 6 Task 1 — `http_forward` implementation spec

**Author**: Claude Code
**Date**: 2026-04-24 (木)
**Audience**: Yuka — review before I archive a Codex prompt
derived from this doc.
**Status**: draft for review
**Crate**: [`philharmonic-connector-impl-http-forward`](https://github.com/metastable-void/philharmonic-connector-impl-http-forward)

## Purpose

Concrete Rust-level spec for implementing `http_forward`, the
generic HTTP-forwarding connector. Complements but doesn't
duplicate
[`docs/design/08-connector-architecture.md`](../design/08-connector-architecture.md)
§"Generic HTTP" — that doc owns the *wire protocol* (config /
request / response / error JSON shapes); this doc owns the
*implementation* (Rust types, module layout, execution flow,
retry algorithm, error mapping, tests).

All three Phase 6 scoping blockers are resolved (see
`2026-04-23-0004` Resolution section): Implementation trait
lives in `philharmonic-connector-impl-api` 0.1.0 (published
2026-04-24 via `f7cc9e4`); `async_trait` crate for the macro;
`reqwest` with `rustls-tls` + tokio for the runtime HTTP stack
(CONTRIBUTING.md §10.9).

## Dependencies

Pinned to latest stable at draft time (2026-04-24; re-verify
via `./scripts/xtask.sh crates-io-versions` before committing
the Cargo.toml):

```toml
[dependencies]
async-trait = "0.1"                        # 0.1.89 published
philharmonic-connector-impl-api = "0.1"    # 0.1.0 — trait + JsonValue/ctx/error re-exports
philharmonic-connector-common = "0.2"      # 0.2.0 — ImplementationError variants
mechanics-config = "0.1"                   # 0.1.0 — HttpEndpoint + PreparedHttpEndpoint
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "gzip", "deflate", "brotli"] }
# CAVEAT: reqwest latest at draft time is 0.13.2; pin to 0.13
# once confirmed compatible with the rest of the stack — Codex
# prompt will verify with crates-io-versions before landing.
tokio = { version = "1", features = ["rt", "macros", "time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
base64 = "0.22"                            # bytes request/response body decoding
url = "2"

[dev-dependencies]
tokio = { version = "1", features = ["rt", "macros", "time", "test-util"] }
wiremock = "0.6"                           # 0.6.5 — deterministic local HTTP mock
```

Notes:

- `reqwest` with `default-features = false` drops native-tls.
  Per CONTRIBUTING.md §10.9, rustls-only; no OpenSSL. `json`
  and compression features are convenience enablers — verify
  each is actually used before landing.
- `tokio` `features = ["rt"]` is deliberately *not*
  `rt-multi-thread`. This crate doesn't spawn its own runtime;
  the connector service binary (Phase 8+) brings one. The
  trait's dyn-compatibility + Send bound (via `async_trait`)
  means the future we return will schedule onto whatever
  runtime the caller provides.
- No workspace-internal crypto deps. Impl crates are
  crypto-free per doc 08 §"Per-implementation crates".

## Crate name consistency

- Package: `philharmonic-connector-impl-http-forward`
  (crate-name on crates.io; already the submodule directory
  name; placeholder published at `0.0.0`, bump to `0.1.0` in
  this work).
- `Implementation::name()` returns: **`"http_forward"`**
  (snake_case — matches the `impl` field in the decrypted
  connector payload per doc 08 §"Encrypted payload contents").

## Module layout

```
philharmonic-connector-impl-http-forward/
├── Cargo.toml
├── CHANGELOG.md
├── LICENSE-APACHE
├── LICENSE-MPL
├── README.md
├── src/
│   ├── lib.rs          # module plumbing + public HttpForward type + trait impl
│   ├── config.rs       # HttpForwardConfig (deser wrapper around HttpEndpoint) + PreparedConfig cache
│   ├── request.rs      # HttpForwardRequest (camelCase ↔ snake_case serde rename)
│   ├── response.rs     # HttpForwardResponse + response-body decoding
│   ├── client.rs       # reqwest::Client construction + single-attempt execute helper
│   ├── retry.rs        # RetryPolicy loop (backoff, Retry-After, max_retry_delay)
│   └── error.rs        # reqwest error → ImplementationError mapping
└── tests/
    ├── happy_path.rs   # wiremock: 2xx json round-trip
    ├── error_cases.rs  # wiremock: every ImplementationError variant
    ├── retry.rs        # wiremock: 5xx retry → success, 429 with Retry-After, exhaustion
    └── request_vectors.rs  # unit: fixed input → fixed outbound HTTP request bytes
```

Rationale: one module per concern, so each stays reviewable
in isolation. `retry.rs` is pure logic (no I/O), fully
unit-testable with a `Duration::ZERO`-stub clock.

## Public surface

```rust
// src/lib.rs
pub use philharmonic_connector_impl_api::{
    async_trait, ConnectorCallContext, Implementation,
    ImplementationError, JsonValue,
};
pub use crate::config::{HttpForwardConfig, PreparedConfig};
pub use crate::request::HttpForwardRequest;
pub use crate::response::HttpForwardResponse;

const NAME: &str = "http_forward";

pub struct HttpForward {
    client: reqwest::Client,
}

impl HttpForward {
    /// Build with the workspace-standard reqwest client
    /// (rustls-tls, connection pooling, no per-call
    /// redirects disabled, reasonable default limits).
    pub fn new() -> Result<Self, ImplementationError> { … }

    /// Alternative constructor for tests that need a
    /// pre-configured client (e.g., with wiremock's mock
    /// server URL resolved against system DNS).
    pub fn with_client(client: reqwest::Client) -> Self { … }
}

#[async_trait]
impl Implementation for HttpForward {
    fn name(&self) -> &str { NAME }

    async fn execute(
        &self,
        config: &JsonValue,
        request: &JsonValue,
        ctx: &ConnectorCallContext,
    ) -> Result<JsonValue, ImplementationError> {
        let cfg: HttpForwardConfig = serde_json::from_value(config.clone())
            .map_err(|e| ImplementationError::InvalidConfig { detail: e.to_string() })?;
        let prepared = cfg.prepare()
            .map_err(|e| ImplementationError::InvalidConfig { detail: e.to_string() })?;

        let req: HttpForwardRequest = serde_json::from_value(request.clone())
            .map_err(|e| ImplementationError::InvalidRequest { detail: e.to_string() })?;

        let response = crate::retry::execute_with_retry(
            &self.client, &prepared, &req, ctx,
        ).await?;

        serde_json::to_value(response)
            .map_err(|e| ImplementationError::Internal { detail: e.to_string() })
    }
}
```

## Types

### `HttpForwardConfig`

```rust
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpForwardConfig {
    pub endpoint: mechanics_config::HttpEndpoint,
}

impl HttpForwardConfig {
    /// Call once at config-ingest time. Caches
    /// `PreparedHttpEndpoint` for the hot path.
    pub fn prepare(&self) -> std::io::Result<PreparedConfig> {
        Ok(PreparedConfig {
            endpoint: self.endpoint.clone(),
            prepared: self.endpoint.prepare_runtime()?,
        })
    }
}

pub struct PreparedConfig {
    pub endpoint: mechanics_config::HttpEndpoint,
    pub prepared: mechanics_config::PreparedHttpEndpoint,
}
```

`deny_unknown_fields` so unexpected keys in the config surface
as `InvalidConfig` rather than silently ignored — the admin
uploading the config should know if they typo'd a field.

### `HttpForwardRequest`

```rust
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HttpForwardRequest {
    #[serde(default)]
    pub url_params: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub queries: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}
```

`rename_all = "camelCase"` maps wire-form `urlParams` /
`queries` / `headers` / `body` to snake_case fields per doc 08
§"Request shape". `deny_unknown_fields` rejects typos with a
clear error.

### `HttpForwardResponse`

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpForwardResponse {
    pub status: u16,
    pub ok: bool,
    pub headers: std::collections::BTreeMap<String, String>,
    pub body: serde_json::Value,
}
```

`BTreeMap` for deterministic header-key ordering in serialized
output (easier to test; easier to diff). Header names
lowercase-normalized per doc 08.

### `Error` (internal, map to `ImplementationError`)

Library-local error enum for clarity inside the crate; the
conversion to `ImplementationError` happens at the boundary.

```rust
// src/error.rs
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("upstream returned non-success status {status}")]
    UpstreamNonSuccess { status: u16, body: serde_json::Value, headers: BTreeMap<String, String> },

    #[error("upstream unreachable: {0}")]
    UpstreamUnreachable(String),

    #[error("upstream timeout")]
    UpstreamTimeout,

    #[error("response too large: limit={limit} actual={actual}")]
    ResponseTooLarge { limit: usize, actual: usize },

    #[error("internal: {0}")]
    Internal(String),
}

impl From<Error> for ImplementationError { … }  // 1:1 mapping
```

The internal enum lets each concrete failure site produce a
specific variant with rich context; the `From` impl flattens
them into the wire-visible `ImplementationError`. Keeps the
crate's internal error flow typed without over-exposing detail
at the trait boundary.

## Execution flow

```text
execute(config, request, ctx) →
  1. Deserialize config → HttpForwardConfig → prepare_runtime()
                                                       ↓ InvalidConfig
  2. Deserialize request → HttpForwardRequest          ↓ InvalidRequest
  3. build_url_prepared(req.url_params, req.queries, &prepared) → absolute URL
                                                       ↓ InvalidRequest
  4. build_headers_prepared(req.headers, &prepared) → layered headers
                                                       ↓ InvalidRequest
  5. Serialize body per endpoint.request_body_type:
       - json  → serde_json::to_vec(&req.body.unwrap_or(Null))
       - utf8  → expect req.body to be a JSON string → .as_str().as_bytes()
       - bytes → expect base64-encoded JSON string → base64::decode → bytes
       - None  → no body
                                                       ↓ InvalidRequest
  6. Build reqwest::Request with method + URL + headers + body + per-attempt timeout
  7. retry::execute_with_retry(...)
       ├─ attempt 1 → reqwest::Client::execute(...)
       │    ├─ success → decode body, filter response headers, return response
       │    ├─ transient error → backoff, retry if retry_on_* allows
       │    └─ permanent error → UpstreamNonSuccess / UpstreamUnreachable / UpstreamTimeout
       ├─ attempt 2..max_attempts
       └─ eventually: total-wall < max_retry_delay_ms, else bail as UpstreamTimeout
  8. Decode response body per endpoint.response_body_type (subject to response_max_bytes cap)
  9. Filter response headers to lowercase(exposed_response_headers) allowlist
 10. Wrap in HttpForwardResponse → serde_json::to_value → return
```

## Retry and backoff algorithm

Driven by `HttpEndpoint::retry_policy` (see
[mechanics-config/src/endpoint/mod.rs](../../mechanics-config/src/endpoint/mod.rs)).

Pseudocode (in `src/retry.rs`):

```rust
let policy = prepared.endpoint.retry_policy();
let max_attempts = policy.max_attempts().max(1);  // at least 1
let overall_deadline = Instant::now() + Duration::from_millis(policy.max_retry_delay_ms());
let base = Duration::from_millis(policy.base_backoff_ms());
let cap = Duration::from_millis(policy.max_backoff_ms());

for attempt in 0..max_attempts {
    let attempt_result = execute_one_attempt(...).await;  // respects per-request timeout_ms
    let err = match attempt_result {
        Ok(response) => return Ok(response),
        Err(e) => e,
    };

    // Is the error class retryable under this policy?
    let retry = match &err {
        Error::UpstreamTimeout if policy.retry_on_timeout() => true,
        Error::UpstreamUnreachable(_) if policy.retry_on_io_errors() => true,
        Error::UpstreamNonSuccess { status, .. } if policy.retry_on_status().contains(status) => true,
        _ => false,
    };
    if !retry || attempt + 1 >= max_attempts { return Err(err); }

    // Compute backoff for this retry.
    let mut delay = (base * 2u32.saturating_pow(attempt as u32)).min(cap);
    // Full jitter: choose uniformly in [0, delay]. Reduces
    // thundering-herd when many executors retry at once.
    delay = Duration::from_millis(rng.random_range(0..=delay.as_millis() as u64));

    // 429-specific overrides.
    if let Error::UpstreamNonSuccess { status: 429, headers, .. } = &err {
        if policy.respect_retry_after() {
            if let Some(retry_after) = parse_retry_after(headers) {
                delay = retry_after;
            }
        } else {
            delay = Duration::from_millis(policy.rate_limit_backoff_ms());
        }
    }

    // Enforce overall deadline.
    let now = Instant::now();
    if now + delay > overall_deadline { return Err(Error::UpstreamTimeout); }
    tokio::time::sleep(delay).await;
}
```

- **Jitter**: full jitter (uniform in `[0, delay]`) is the
  simple, well-studied choice; prevents herd effects without
  needing per-client state. `rand::Rng::random_range` (rand
  0.9+), picked up as a dev-dep if not already transitive.
  If this is contentious, halve-and-random-add also works.
- **Retry-After parsing**: RFC 7231 allows either a number of
  seconds or an HTTP-date. Implement both; fall back to
  `rate_limit_backoff_ms` on parse failure.
- **`max_retry_delay_ms` semantics**: total wall clock from
  first attempt to last attempt including sleeps. If a
  scheduled sleep would overrun, bail as `UpstreamTimeout`
  (documented in doc 08 §"Error cases" as "request exceeded
  `timeout_ms` (including retries)").
- **Per-attempt timeout**: `endpoint.timeout_ms()` drives the
  reqwest per-request timeout. Separately from the
  `max_retry_delay_ms` overall cap.

## Error mapping

| Failure site                                     | Internal `Error` variant            | Wire `ImplementationError`         |
| ---                                              | ---                                 | ---                                |
| Config `serde_json::from_value` fails            | `InvalidConfig`                     | `InvalidConfig { detail }`         |
| `HttpEndpoint::prepare_runtime` fails            | `InvalidConfig`                     | `InvalidConfig { detail }`         |
| Request `serde_json::from_value` fails           | `InvalidRequest`                    | `InvalidRequest { detail }`        |
| `build_url_prepared` / `build_headers_prepared` fails | `InvalidRequest`               | `InvalidRequest { detail }`        |
| Body-type expectation mismatch (utf8 needs string, bytes needs b64 string) | `InvalidRequest` | `InvalidRequest { detail }`        |
| Response body decode (utf8 invalid / base64 encode failure) | `Internal`                 | `Internal { detail }`              |
| reqwest `is_timeout()` on per-attempt           | `UpstreamTimeout`                   | `UpstreamTimeout`                  |
| reqwest `is_connect()` / io::Error / TLS       | `UpstreamUnreachable(reason)`       | `UpstreamUnreachable { detail }`   |
| Non-2xx with `allow_non_2xx_status = false`     | `UpstreamNonSuccess { status, body, headers }` | `UpstreamError { status, body }` (body = JSON-serialized { status, headers, body }; see below) |
| Response body length > `response_max_bytes`     | `ResponseTooLarge { limit, actual }` | `ResponseTooLarge { limit, actual }` |
| retry loop overall-deadline exceeded            | `UpstreamTimeout`                   | `UpstreamTimeout`                  |

Doc 08 §"Error cases" says upstream_error should carry "status,
the exposed response headers, and the decoded body so the
script can still inspect what came back". `ImplementationError::UpstreamError`
has `status: u16` + `body: String` — the string payload carries
a JSON-serialized `{ "status", "headers", "body" }` sub-object.
Script-side inspection uses `JSON.parse(err.body)`. This
matches the wire protocol doc 08 specifies.

**Decision to confirm with Yuka**: is the `body: String` →
JSON-encoded-string idiom acceptable, or should we extend
`ImplementationError::UpstreamError` in `connector-common`
0.3.0 to carry typed `headers` / `body` fields? (0.3.0 bump
would trigger another Gate 1/2 cycle on connector-common.)
Default: stick with `body: String` and JSON-encode — no
breaking change to a crypto crate.

## Response-body size enforcement

Use `reqwest::Response::bytes_stream()` + `futures::StreamExt`
to read chunks, enforcing `response_max_bytes` as an accumulator
check on each chunk rather than buffering the whole thing. If
the accumulator crosses the limit mid-stream, abort and return
`ResponseTooLarge { limit, actual: accumulator }`.

If `response_max_bytes` is `None`, fall back to a framework
default (e.g., 8 MiB) — document the default in the crate's
README. Alternative: require `response_max_bytes` to be set at
config validation time. **Decision to confirm with Yuka**:
framework default, or hard requirement?

## Testing plan

### Unit tests (per module, colocated)

- `config::tests::deserialize_rejects_unknown_fields`
- `config::tests::prepare_runtime_invalid_url_template_rejected`
- `request::tests::deserialize_camelcase_wire_form`
- `request::tests::deserialize_rejects_unknown_fields`
- `response::tests::header_keys_lowercased`
- `retry::tests::exponential_backoff_respects_cap`
- `retry::tests::full_jitter_bounded`
- `retry::tests::retry_after_header_parsed_as_seconds`
- `retry::tests::retry_after_header_parsed_as_http_date`
- `retry::tests::overall_deadline_breaks_retry_loop`
- `retry::tests::non_retryable_status_not_retried`
- `error::tests::every_internal_variant_maps_to_wire`

### Integration tests (tests/, wiremock)

- `happy_path`:
  - `json` request + `json` response + 200 status → round-trip.
  - `utf8` request + `utf8` response.
  - `bytes` request (base64-in) + `bytes` response (base64-out).
- `error_cases`:
  - 404 with `allow_non_2xx_status = false` → `UpstreamError`.
  - 404 with `allow_non_2xx_status = true` → success with status=404, ok=false.
  - Connection refused → `UpstreamUnreachable`.
  - Per-attempt timeout → `UpstreamTimeout`.
  - Response body > `response_max_bytes` → `ResponseTooLarge`.
- `retry`:
  - 500 then 200 under `retry_on_status = [500]` → success on attempt 2.
  - 429 with `Retry-After: 2` → sleeps ~2s → retries → success.
  - 429 with no Retry-After → uses `rate_limit_backoff_ms`.
  - All `max_attempts` exhausted with 500 → final `UpstreamError`.
  - `max_retry_delay_ms` exceeded mid-retry → `UpstreamTimeout`.
- `request_vectors`:
  - For fixed input config + request, assert the exact
    outbound HTTP request: method, URL (after template
    resolution + query emission + percent-encoding), header
    set (baked + override), body bytes. Assertion via
    `wiremock::Mock`'s `and(body_string_contains)` +
    captured-request inspection.

### End-to-end sanity (optional, gated)

- `httpbin_smoke`: `#[ignore]` by default, enabled via
  `HTTPBIN_ENABLED=1` env. Posts to https://httpbin.org/post,
  verifies the echoed request matches what we sent. Useful for
  one-off network validation but not part of CI (flaky + external
  dep).

## Non-goals (v1)

Explicitly deferred or out-of-scope:

- Request/response **streaming**. Bodies are buffered in memory;
  `response_max_bytes` bounds consumption. Streaming would
  break the `JsonValue → JsonValue` trait surface.
- **Connection reuse across different impl instances**. Each
  `HttpForward` holds its own `reqwest::Client`; pooling is
  reqwest-internal to that instance.
- **Custom CA bundles** / client certificates. rustls defaults
  via `webpki-roots`; if an endpoint needs a private CA, that's
  a future `HttpEndpoint` extension.
- **Rate limiting** at the connector service. That's
  connector-service's concern (deferred per ROADMAP) — not
  the impl's.
- **Structured logging** / metrics. Hooks can be added later;
  the v1 spec is silent about telemetry so the trait surface
  stays minimal.
- **Authentication schemes** beyond "bake headers into the
  config". OAuth token refresh, AWS SigV4, etc. are future
  work or dedicated impl crates (e.g.,
  `philharmonic-connector-impl-http-aws-sigv4`).

## Workspace impact

- `philharmonic-connector-impl-http-forward` bumps
  `0.0.0` → `0.1.0`; CHANGELOG updated (dropping the aspirational
  `[0.0.0]` line since the crate was never substantively
  published at that version — same precedent as impl-api).
- No other crate changes. `connector-service`'s dispatch wiring
  (Phase 8+) will consume this crate once the realm binary
  lands.
- Root `Cargo.toml` `[patch.crates-io]` already has the local
  path entry.

## Acceptance criteria (mapping to ROADMAP Phase 6 Task 1)

From ROADMAP.md §"Phase 6 — First implementations" Task 1:

- [x] Config shape reuses `mechanics_config::HttpEndpoint`.
- [x] Request shape `{url_params, query, headers, body}`,
  validated against config's `HttpEndpoint`.
- [x] Response shape `{status, headers, body}`; headers
  filtered to `exposed_response_headers`.
- [x] Error handling: 4xx/5xx returns as response (when
  `allow_non_2xx_status = true`) or as `UpstreamError`
  (otherwise). Network / timeout failures as
  `UpstreamUnreachable` / `UpstreamTimeout`.
- [x] `reqwest` with `rustls-tls`; single reused
  `reqwest::Client`; per-request timeout from
  `HttpEndpoint.timeout_ms`.
- [x] Integration tests against `wiremock`-backed local mock
  (preferred for CI); optional `httpbin.org` gated on env flag.

Plus from the Phase 6 preamble:

- [x] Publishes as `0.1.0` after implementation lands and
  passes Gate-2-equivalent review (non-crypto → no formal Gate
  1/2, but Claude reviews the Codex output end-to-end before
  publish).

## Open questions for Yuka

Three decisions before I draft the Codex prompt:

1. **`body: String` JSON-encoded-string idiom** for
   `UpstreamError` payloads (status + headers + body bundled
   into a JSON string), vs. bumping `connector-common` to
   `0.3.0` with typed `headers` + `body` fields on the
   `UpstreamError` variant. Default: keep `body: String`.

2. **`response_max_bytes` default** when the config omits it.
   Candidate values: 8 MiB (generous, matches modern JSON
   API norms), 1 MiB (tight, matches the example in doc 08),
   or make the field mandatory at config-load time (no
   default; reject configs without it). Default proposal:
   mandatory.

3. **reqwest version pin**: `"0.12"` or `"0.13"`? Latest on
   crates.io is 0.13.2; the rest of the workspace doesn't yet
   have a reqwest dep to align with. Default proposal: pin to
   `"0.13"` (newest stable minor).

Once these are decided + you sign off on this spec, I archive
a Codex prompt under
`docs/codex-prompts/YYYY-MM-DD-NNNN-phase-6-http-forward.md`
that inlines this spec (so Codex reads it in full) and spawn
the Codex session per the `codex-prompt-archive` skill.
