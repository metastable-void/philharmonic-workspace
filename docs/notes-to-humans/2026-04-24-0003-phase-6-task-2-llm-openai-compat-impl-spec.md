# Phase 6 Task 2 — `llm_openai_compat` implementation spec

**Author**: Claude Code
**Date**: 2026-04-24 (金)
**Audience**: Yuka — review before I archive a Codex prompt
derived from this doc.
**Status**: **draft — three open questions at the bottom awaiting
resolution** (Q1 jsonschema crate, Q2 non-vLLM fixture policy,
Q3 retry-policy shape). Once resolved, status flips to
"approved for Codex prompt archival".
**Crate**: [`philharmonic-connector-impl-llm-openai-compat`](https://github.com/metastable-void/philharmonic-connector-impl-llm-openai-compat)

## Purpose

Concrete Rust-level spec for implementing `llm_openai_compat`,
the OpenAI-compatible LLM connector. Complements but doesn't
duplicate
[`docs/design/08-connector-architecture.md`](../design/08-connector-architecture.md)
§"LLM — specialized HTTP implementations" and
§"`llm_openai_compat` — config and dialects" — those sections
own the *wire protocol* (normalized `llm_generate` request /
response and the per-dialect native API shapes); this doc owns
the *implementation* (Rust types, module layout, dialect
dispatch, schema validation, error mapping, tests + fixtures).

Builds on Task 1's foundations: `philharmonic-connector-impl-api`
0.1.0 (the `Implementation` trait), `reqwest` 0.13 with rustls
+ tokio for the runtime HTTP stack (CONTRIBUTING.md §10.9),
`#[async_trait]` macro for dyn-compat. Nothing crypto-sensitive
here; same non-crypto dep surface as `http_forward`.

## Dependencies

Pinned to the latest stable at draft time (2026-04-24; re-verify
via `./scripts/xtask.sh crates-io-versions` before committing
the Cargo.toml):

```toml
[dependencies]
async-trait = "0.1"                        # 0.1.89
philharmonic-connector-impl-api = "0.1"    # 0.1.0 — trait + JsonValue/ctx/error re-exports
philharmonic-connector-common = "0.2"      # 0.2.0 — ImplementationError variants (incl. SchemaValidationFailed)
reqwest = { version = "0.13", default-features = false, features = ["rustls-tls", "json", "gzip", "deflate", "brotli"] }
tokio = { version = "1", features = ["rt", "macros", "time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
jsonschema = "0.46"                        # 0.46.2 — Q1 below; contentious
# No direct `url`, no direct `base64`: URLs are joined as strings
# (simple `{base_url}/chat/completions` concat with single-slash
# normalization), and there are no opaque-bytes body fields.

[dev-dependencies]
tokio = { version = "1", features = ["rt", "macros", "time", "test-util"] }
wiremock = "0.6"                           # 0.6.5
```

Notes:

- `reqwest` keeps the same feature set as `http_forward` —
  rustls, json, gzip/deflate/brotli. Same major.minor pin
  (0.13) as http_forward per CONTRIBUTING.md §10.9 workspace-
  consistency rule.
- `jsonschema` is pinned to `"0.46"` tentatively — see Q1
  below. If we go with a different crate, this line changes.
- No `rand` or `httpdate` dep needed unless Q3 resolves with a
  retry policy that uses jitter or parses `Retry-After`. Left
  out of this block; add if Q3 pulls them in.

## Crate name consistency

- Package: `philharmonic-connector-impl-llm-openai-compat`
  (crate-name on crates.io; submodule directory name;
  placeholder published at `0.0.0`, bump to `0.1.0` in this
  work).
- `Implementation::name()` returns: **`"llm_openai_compat"`**
  (snake_case — matches the `impl` field in the decrypted
  connector payload per doc 08 §"Encrypted payload contents").

## Module layout

```
philharmonic-connector-impl-llm-openai-compat/
├── Cargo.toml
├── CHANGELOG.md
├── LICENSE-APACHE
├── LICENSE-MPL
├── README.md
├── src/
│   ├── lib.rs                     # module plumbing + public LlmOpenaiCompat type + trait impl
│   ├── config.rs                  # LlmOpenaiCompatConfig + Dialect enum + validation
│   ├── request.rs                 # LlmGenerateRequest (normalized, snake_case)
│   ├── response.rs                # LlmGenerateResponse + StopReason enum + Usage
│   ├── client.rs                  # reqwest::Client construction + single-attempt POST
│   ├── dialect/
│   │   ├── mod.rs                 # Dialect dispatch trait; per-dialect translate() + extract()
│   │   ├── openai_native.rs       # response_format: json_schema translation
│   │   ├── vllm_native.rs         # structured_outputs: {"json": ...} translation
│   │   └── tool_call_fallback.rs  # synthetic tool + tool_choice + tool-call-args extraction
│   ├── schema.rs                  # output_schema validation via jsonschema
│   ├── retry.rs                   # [optional per Q3] retry loop
│   └── error.rs                   # reqwest + provider-payload errors → ImplementationError
└── tests/
    ├── happy_path.rs              # wiremock: one success per dialect
    ├── error_cases.rs             # wiremock: every ImplementationError variant per dialect
    ├── dialect_openai_native.rs   # request_vectors for openai_native
    ├── dialect_vllm_native.rs     # request_vectors for vllm_native (uses upstream fixtures)
    ├── dialect_tool_call_fallback.rs  # request_vectors for tool_call_fallback
    ├── schema_validation.rs       # output that doesn't match output_schema → SchemaValidationFailed
    ├── stop_reason_normalization.rs  # finish_reason variants → normalized StopReason
    ├── fixtures/                  # response-body fixtures (synthesized; see Testing plan)
    │   ├── openai_native_response.json
    │   ├── vllm_native_response.json
    │   └── tool_call_fallback_response.json
    └── smokes/                    # [#[ignore]]-d, env-gated
        ├── openai_smoke.rs        # OPENAI_SMOKE_ENABLED=1 OPENAI_API_KEY=sk-...
        └── vllm_smoke.rs          # VLLM_SMOKE_ENABLED=1 VLLM_BASE_URL=http://...
```

Rationale:

- `dialect/` as a sub-module keeps the three translators
  side-by-side; each file is a self-contained "here's how this
  provider's native API maps to the normalized request/response."
- `schema.rs` isolates jsonschema usage to one module — swap-out
  if Q1 resolves differently, one file changes.
- Fixtures for request bytes live in `tests/` next to the
  integration test that uses them. Fixtures for upstream
  reference data live at
  [`docs/upstream-fixtures/vllm/`](../upstream-fixtures/vllm/)
  (already committed `2cb0aab`) and are pulled in via
  `include_str!("../../../docs/upstream-fixtures/vllm/...")` at
  test compile time — no copy into the crate tree, so the
  pinned upstream provenance stays the single source of truth.

## Public surface

```rust
// src/lib.rs
pub use philharmonic_connector_impl_api::{
    async_trait, ConnectorCallContext, Implementation,
    ImplementationError, JsonValue,
};
pub use crate::config::{Dialect, LlmOpenaiCompatConfig};
pub use crate::request::{LlmGenerateRequest, Message, Role};
pub use crate::response::{LlmGenerateResponse, StopReason, Usage};

const NAME: &str = "llm_openai_compat";

pub struct LlmOpenaiCompat {
    client: reqwest::Client,
}

impl LlmOpenaiCompat {
    /// Build with the workspace-standard reqwest client.
    pub fn new() -> Result<Self, ImplementationError> { … }

    /// Alternative constructor for tests (e.g. with wiremock's
    /// mock-server URL + a no-op api_key in the config).
    pub fn with_client(client: reqwest::Client) -> Self { … }
}

#[async_trait]
impl Implementation for LlmOpenaiCompat {
    fn name(&self) -> &str { NAME }

    async fn execute(
        &self,
        config: &JsonValue,
        request: &JsonValue,
        ctx: &ConnectorCallContext,
    ) -> Result<JsonValue, ImplementationError> {
        let cfg: LlmOpenaiCompatConfig = serde_json::from_value(config.clone())
            .map_err(|e| ImplementationError::InvalidConfig { detail: e.to_string() })?;

        let req: LlmGenerateRequest = serde_json::from_value(request.clone())
            .map_err(|e| ImplementationError::InvalidRequest { detail: e.to_string() })?;

        // Compile output_schema once; surface a schema-compile error
        // as InvalidRequest (script bug), not SchemaValidationFailed
        // (provider misbehavior).
        let compiled_schema = crate::schema::compile(&req.output_schema)?;

        // Dispatch to the dialect's translator.
        let response = match cfg.dialect {
            Dialect::OpenaiNative      => crate::dialect::openai_native::execute(&self.client, &cfg, &req, ctx).await?,
            Dialect::VllmNative        => crate::dialect::vllm_native::execute(&self.client, &cfg, &req, ctx).await?,
            Dialect::ToolCallFallback  => crate::dialect::tool_call_fallback::execute(&self.client, &cfg, &req, ctx).await?,
        };

        // Enforce output validates against output_schema before
        // handing back. Any dialect may produce off-schema output
        // (tool_call_fallback especially).
        crate::schema::validate(&compiled_schema, &response.output)?;

        serde_json::to_value(response)
            .map_err(|e| ImplementationError::Internal { detail: e.to_string() })
    }
}
```

## Types

### `LlmOpenaiCompatConfig`

```rust
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmOpenaiCompatConfig {
    pub base_url: String,
    pub api_key: String,
    pub dialect: Dialect,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 { 60_000 }  // 60s per doc 08

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dialect {
    OpenaiNative,
    VllmNative,
    ToolCallFallback,
}
```

`deny_unknown_fields` so typos in the config surface as
`InvalidConfig`. `timeout_ms` defaults to 60s per doc 08
(explicit in the spec, not framework-default).

`base_url` is not pre-validated as a URL at deserialize time.
Validation happens at first send — an invalid `base_url`
surfaces as `UpstreamUnreachable` on the first request, which
matches what a script would see with any other misconfigured
HTTP target. Pre-parsing would be a minor nicety but would pull
in `url = "2"` as a dep for no other purpose.

### `LlmGenerateRequest`

```rust
#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LlmGenerateRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub output_schema: serde_json::Value,
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub stop: Option<Vec<String>>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
}
```

`deny_unknown_fields` catches unknown generation knobs per doc
08 §"Request shape" ("Unknown knobs in the request are
rejected"). `content` is `String` only — no multi-part content
parts, no images in v1, per doc 08.

### `LlmGenerateResponse`

```rust
#[derive(serde::Serialize)]
pub struct LlmGenerateResponse {
    pub output: serde_json::Value,
    pub stop_reason: StopReason,
    pub usage: Usage,
}

#[derive(serde::Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ContentFilter,
    Error,
}

#[derive(serde::Serialize, Clone, Copy)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
```

The five-value `StopReason` is the normalized set from doc 08
§"Response shape". Per-dialect `finish_reason` values map into
this via a small normalization table (see `stop_reason_normalization.rs`).

### `Error` (internal, map to `ImplementationError`)

```rust
// src/error.rs
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("upstream returned non-success status {status}")]
    UpstreamNonSuccess { status: u16, body: String },

    #[error("upstream unreachable: {0}")]
    UpstreamUnreachable(String),

    #[error("upstream timeout")]
    UpstreamTimeout,

    #[error("schema validation failed: {0}")]
    SchemaValidationFailed(String),

    #[error("provider payload malformed: {0}")]
    MalformedProviderPayload(String),

    #[error("internal: {0}")]
    Internal(String),
}

impl From<Error> for ImplementationError { … }  // 1:1 mapping
```

`MalformedProviderPayload` (a provider returned us a response
we couldn't parse as the documented envelope — missing
`choices`, empty `content`, etc.) collapses to `Internal` on
the wire rather than `UpstreamError`. Rationale: at that point
we genuinely don't know what happened, and we haven't got a
"status + body" to hand the script that they could do anything
useful with.

## Dialect translation

### `openai_native`

Outbound request to `{base_url}/chat/completions`:

```json
{
  "model": "<model>",
  "messages": [<messages verbatim>],
  "response_format": {
    "type": "json_schema",
    "json_schema": {
      "name": "output",
      "strict": true,
      "schema": <output_schema>
    }
  },
  "max_completion_tokens": <max_output_tokens?>,
  "temperature": <temperature?>,
  "top_p": <top_p?>,
  "stop": <stop?>
}
```

Response extraction: `choices[0].message.content` is a JSON
**string** that must be `serde_json::from_str`-parsed into the
`output` field. `finish_reason` is `stop | length | content_filter
| tool_calls`; map via: `stop → EndTurn`, `length → MaxTokens`,
`content_filter → ContentFilter`, `tool_calls → Error` (native
dialect shouldn't produce tool_calls; if it does, that's a
provider bug). Usage from `usage.prompt_tokens` /
`usage.completion_tokens`.

Header: `Authorization: Bearer <api_key>`. Content-Type: `application/json`.

### `vllm_native`

Outbound request to `{base_url}/chat/completions`:

```json
{
  "model": "<model>",
  "messages": [<messages verbatim>],
  "structured_outputs": { "json": <output_schema> },
  "max_completion_tokens": <max_output_tokens?>,
  "temperature": <temperature?>,
  "top_p": <top_p?>,
  "stop": <stop?>
}
```

`structured_outputs` lives at the **top level** — not under
`extra_body`, which is an OpenAI-Python-client detail that
merges into the top level at the HTTP boundary. Verified
against vLLM's own test suite via
[`docs/upstream-fixtures/vllm/structured_outputs_json_chat_request.json`](../upstream-fixtures/vllm/structured_outputs_json_chat_request.json)
(pinned commit `cf8a613a87264183058801309868722f9013e101`).

Response extraction: same OpenAI chat-completion envelope as
`openai_native` (vLLM mimics it). `finish_reason` values the
same. Usage the same.

Header: `Authorization: Bearer <api_key>`. Some vLLM deployments
don't require auth; missing auth is the deployment's choice,
and we send the header anyway (benign when unused).

### `tool_call_fallback`

Outbound request to `{base_url}/chat/completions`:

```json
{
  "model": "<model>",
  "messages": [<messages verbatim>],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "emit_output",
        "description": "Produce the structured output.",
        "parameters": <output_schema>
      }
    }
  ],
  "tool_choice": {
    "type": "function",
    "function": { "name": "emit_output" }
  },
  "max_completion_tokens": <max_output_tokens?>,
  "temperature": <temperature?>,
  "top_p": <top_p?>,
  "stop": <stop?>
}
```

Response extraction: `choices[0].message.tool_calls[0].function.arguments`
is a JSON string that parses into `output`. `finish_reason` is
expected to be `tool_calls` on the happy path; map
`tool_calls → EndTurn` for this dialect specifically (the
tool-call is the normal completion here). `stop → EndTurn`
(model gave up without calling; output will likely fail schema
validation), `length → MaxTokens`.

If `tool_calls` is empty or absent, that's
`MalformedProviderPayload` → `Internal` on the wire: the server
didn't call our forced tool despite `tool_choice`, and the
resulting "output" from `content` is almost certainly not
schema-shaped. Distinct from `SchemaValidationFailed` because
we don't have an output to validate at all.

Header: `Authorization: Bearer <api_key>`. Content-Type: `application/json`.

## Execution flow

```text
execute(config, request, ctx) →
  1. Deserialize config → LlmOpenaiCompatConfig       ↓ InvalidConfig
  2. Deserialize request → LlmGenerateRequest         ↓ InvalidRequest
  3. schema::compile(&req.output_schema)              ↓ InvalidRequest (script gave a broken schema)
  4. Dispatch to dialect::<variant>::execute(...)
       ├─ translate request → provider-native JSON body
       ├─ POST {base_url}/chat/completions with Authorization header
       ├─ per-attempt timeout from config.timeout_ms
       ├─ [optional Q3 retries]
       └─ on 2xx: parse provider envelope → LlmGenerateResponse
          on non-2xx: UpstreamNonSuccess { status, body }
          on io/tls: UpstreamUnreachable
          on timeout: UpstreamTimeout
  5. schema::validate(&compiled, &response.output)    ↓ SchemaValidationFailed
  6. serde_json::to_value(response) → return
```

Flow is linear: no fan-out, no pipelining. Every step is a
dialect-aware or trait-level concern in isolation. The
per-dialect module owns steps 4a/4b/4e (translate, POST,
parse); the rest is shared.

## Retry (Q3 — contingent)

`LlmOpenaiCompatConfig` currently has no `retry_policy` field
(unlike `HttpEndpoint`). Three options:

1. **No retries in v1.** Network flake / 429 / 5xx surfaces
   directly. Simplest; scripts retry themselves if they want.
2. **Hardcoded minimal retry.** 2 retries on 429 / 5xx /
   network, full-jitter exponential backoff (base 1000ms, cap
   8000ms), respect `Retry-After` on 429. No config surface;
   baked into the impl.
3. **Config-carried retry policy.** Add a `retry_policy` field
   to config (same shape as `EndpointRetryPolicy` from
   mechanics-config, or a subset). Maximum flexibility;
   maximum config surface.

My recommendation is **option 2** (hardcoded minimal). LLM
providers' 429 handling is the biggest driver of script-level
pain, and it's trivially handled at our layer. The config-
carried option is a latent ask that can land in 0.2.0 if
anyone needs it.

Q3 below asks for Yuka's call.

## Schema validation

```rust
// src/schema.rs
pub fn compile(schema: &serde_json::Value) -> Result<jsonschema::Validator, Error> {
    jsonschema::draft202012::new(schema)
        .map_err(|e| Error::InvalidRequest(format!("output_schema invalid: {e}")))
}

pub fn validate(
    compiled: &jsonschema::Validator,
    output: &serde_json::Value,
) -> Result<(), Error> {
    match compiled.validate(output) {
        Ok(()) => Ok(()),
        Err(e) => Err(Error::SchemaValidationFailed(format!("{e}"))),
    }
}
```

- **Draft 2020-12** is the jsonschema crate's most-featured
  draft and matches OpenAI's documented expectation for
  `response_format: json_schema` (OpenAI explicitly cites
  2020-12 compliance).
- Compile once per request (step 3); validate once per
  response (step 5). Compilation is the expensive step; if
  repeated calls against the same config end up with the same
  schema, the caller (connector service) could cache the
  compiled `Validator` — out of scope for this crate, flagged
  as a future optimization.
- **Error messages stay terse.** `jsonschema` produces rich
  error paths like `/work_history/0/duration: 150 > maximum
  100`. We pass the formatted error straight through to the
  script as the `detail` field of `SchemaValidationFailed`
  so scripts can debug their prompts.

## Stop reason normalization

| provider `finish_reason` | `openai_native` → | `vllm_native` → | `tool_call_fallback` → |
| ---                      | ---               | ---             | ---                    |
| `stop`                   | `EndTurn`         | `EndTurn`       | `EndTurn` (fallback, output likely invalid) |
| `length`                 | `MaxTokens`       | `MaxTokens`     | `MaxTokens`            |
| `content_filter`         | `ContentFilter`   | `ContentFilter` | `ContentFilter`        |
| `tool_calls`             | `Error`           | `Error`         | `EndTurn` (the expected happy path) |
| (other)                  | `Error`           | `Error`         | `Error`                |

Any `finish_reason` not in the table maps to `Error` with the
underlying value logged but not surfaced — doc 08 fixes the
normalized set at five values; additions need a doc 08 change.

## Usage normalization

All three dialects of the OpenAI-compatible API return
`usage: { prompt_tokens, completion_tokens, total_tokens }`.
We map:

- `input_tokens = prompt_tokens`
- `output_tokens = completion_tokens`
- `total_tokens` is dropped (redundant; script can sum).

If a provider omits `usage` entirely, we fill with zeros and
let the script notice. Flagging the gap as `MalformedProviderPayload`
is too aggressive — some compatible servers genuinely don't
report usage.

## Error mapping

| Failure site                                        | Internal `Error`                   | Wire `ImplementationError`         |
| ---                                                 | ---                                | ---                                |
| Config `serde_json::from_value` fails               | `InvalidConfig`                    | `InvalidConfig { detail }`         |
| Request `serde_json::from_value` fails              | `InvalidRequest`                   | `InvalidRequest { detail }`        |
| `output_schema` fails to compile                    | `InvalidRequest`                   | `InvalidRequest { detail }`        |
| reqwest `is_timeout()` on per-attempt              | `UpstreamTimeout`                  | `UpstreamTimeout`                  |
| reqwest `is_connect()` / io::Error / TLS          | `UpstreamUnreachable(reason)`      | `UpstreamUnreachable { detail }`   |
| Non-2xx from provider                               | `UpstreamNonSuccess { status, body }` | `UpstreamError { status, body }` |
| Provider envelope missing required fields           | `MalformedProviderPayload(detail)` | `Internal { detail }`              |
| `output` doesn't validate against `output_schema`   | `SchemaValidationFailed(detail)`   | `SchemaValidationFailed { detail }` |
| Output extraction: string parse of inner JSON fails | `MalformedProviderPayload(detail)` | `Internal { detail }`              |

`UpstreamError::body` carries the raw HTTP response body as a
string when non-2xx. Unlike http_forward, there's no wrapping
`{status, headers, body}` sub-object — the LLM provider's
response body already has everything the script needs
(OpenAI returns `{error: {message, type, code}}` on errors;
we hand it through unaltered). Scripts that need the raw
provider error shape do `JSON.parse(err.body).error.message`
etc.

## Testing plan

### Unit tests (per module, colocated)

- `config::tests::deserialize_rejects_unknown_fields`
- `config::tests::dialect_enum_roundtrips_all_three`
- `config::tests::default_timeout_ms_is_60000`
- `request::tests::deserialize_rejects_unknown_generation_knobs`
- `request::tests::role_enum_roundtrips_system_user_assistant`
- `response::tests::stop_reason_serializes_snake_case`
- `schema::tests::invalid_schema_surfaces_as_invalid_request`
- `schema::tests::validation_error_detail_is_readable`
- `dialect::openai_native::tests::translates_basic_request_to_expected_body`
- `dialect::vllm_native::tests::translates_basic_request_to_expected_body`
- `dialect::tool_call_fallback::tests::translates_basic_request_to_expected_body`
- `dialect::*::tests::finish_reason_maps_per_table`  (parametric)
- `error::tests::every_internal_variant_maps_to_wire`

### Integration tests (tests/, wiremock)

Each dialect gets its own `dialect_<name>.rs` request-vector
file. The pattern is: fixed config + fixed request → assert
exact outbound HTTP body bytes. This is the dialect-translation
contract — the piece Codex should not be allowed to get
subtly wrong.

- `happy_path.rs`:
  - One test per dialect: wiremock returns a canned success
    response (from `tests/fixtures/*.json`); verify the
    returned `LlmGenerateResponse` matches expected
    `{output, stop_reason, usage}`.
- `error_cases.rs`:
  - 401 → `UpstreamError { status: 401, body: ... }`.
  - 429 → depends on Q3 resolution (retry and succeed, or surface directly).
  - 500 → `UpstreamError`/`UpstreamTimeout` depending on Q3.
  - Connection refused → `UpstreamUnreachable`.
  - Per-request timeout → `UpstreamTimeout`.
  - Malformed provider envelope (no `choices`) → `Internal`.
- `dialect_openai_native.rs`:
  - Fixed input → exact bytes of the outbound body assert via
    `wiremock::Mock::and(body_string)`.
- `dialect_vllm_native.rs`:
  - **Primary assertion**: outbound body matches
    [`docs/upstream-fixtures/vllm/structured_outputs_json_chat_request.json`](../upstream-fixtures/vllm/structured_outputs_json_chat_request.json)
    byte-for-byte, modulo:
    - `model` value (our test uses whatever the upstream
      fixture declares; fixture has
      `HuggingFaceH4/zephyr-7b-beta`).
    - `messages[1].content` (upstream redacts the inline
      schema; we reconstruct via string interpolation of
      `sample_json_schema.json` into the test prompt).
  - Secondary test: use `sample_json_schema.json` as
    `output_schema` directly, assert the translated
    `structured_outputs.json` sub-object matches
    `sample_json_schema.json` byte-for-byte.
- `dialect_tool_call_fallback.rs`:
  - Fixed input → exact outbound body.
- `schema_validation.rs`:
  - Provider returns output that doesn't match
    `output_schema` → `SchemaValidationFailed`.
  - Error detail includes the offending JSON pointer.
- `stop_reason_normalization.rs`:
  - For each dialect × each provider `finish_reason`, assert
    the normalized `StopReason` per the table above.

### Fixture provenance (see Q2)

- `vllm_native` request fixtures: committed upstream JSON at
  `docs/upstream-fixtures/vllm/`, pinned
  `cf8a613a87264183058801309868722f9013e101`. Tamper-evident.
- `openai_native` + `tool_call_fallback` request fixtures:
  **to-be-synthesized** from OpenAI's public chat-completions
  API docs (no upstream Python source to extract from; tests
  aren't public JSON). Q2 below asks how to anchor these
  against drift.
- Response fixtures for all three dialects: synthesized to
  match the OpenAI chat-completion envelope (stable,
  documented, mimicked by vLLM). Drift-catching falls to the
  smokes below.

### Optional smokes (`#[ignore]`-d, env-gated)

- `tests/smokes/openai_smoke.rs`:
  - Opt-in via `OPENAI_SMOKE_ENABLED=1 OPENAI_API_KEY=sk-...`.
  - Posts a minimal request to the real OpenAI API against a
    small cheap model (`gpt-4o-mini` or equivalent).
  - Asserts the response round-trips through both `openai_native`
    and `tool_call_fallback` dialects.
  - Manual only; never on any CI path.
- `tests/smokes/vllm_smoke.rs`:
  - Opt-in via `VLLM_SMOKE_ENABLED=1 VLLM_BASE_URL=http://...`.
  - Runnable when Yuka is in-network to the Xeon 8259CL
    box. `#[ignore]` keeps `cargo test` green when the box
    isn't reachable.
  - Diffs real-vLLM request/response bytes against the
    committed fixtures; any non-empty diff is either upstream
    drift or cache staleness. Diff output is the bug report.

## Non-goals (v1)

- **Streaming responses.** Bodies are buffered; `tokio::time`
  bounds consumption via `timeout_ms`. Streaming would break
  the `JsonValue → JsonValue` trait surface.
- **Tool calling at the wire protocol level.** Per doc 08:
  "agentic loops are composed in JavaScript." `tool_call_fallback`
  uses tool calling as an internal structured-output transport
  only; `tools` never appears in the normalized wire request.
- **Multi-part content.** `messages[].content` is
  `String`-only. No images, no audio, no mixed parts.
- **Custom CA bundles / client certificates.** rustls +
  webpki-roots.
- **`Retry-After` HTTP-date parsing.** Seconds-only if Q3
  resolves to option 2. HTTP-date is rare on 429 from LLM
  providers.
- **Response caching.** Out of scope; connector-service's
  concern if ever needed.
- **Provider-specific shims** (Anthropic, Gemini, …). Those
  are separate crates (Phase 7).

## Workspace impact

- `philharmonic-connector-impl-llm-openai-compat` bumps
  `0.0.0` → `0.1.0`; CHANGELOG updated (drop the aspirational
  `[0.0.0]` line — same precedent as impl-api and http_forward).
- No other crate changes. Workspace members list + `[patch.crates-io]`
  already have this crate wired in.
- `docs/upstream-fixtures/vllm/` stays under `docs/`; the
  crate references it via relative path at test compile time.

## Acceptance criteria (mapping to ROADMAP Phase 6 Task 2)

- [x] Depends on `philharmonic-connector-impl-api` — no crypto deps.
- [x] Config shape `{base_url, api_key, dialect, timeout_ms}`.
- [x] Dialect enum `openai_native | vllm_native | tool_call_fallback`.
- [x] Request shape normalized `llm_generate` per doc 08.
- [x] Response shape `{output, stop_reason, usage}`.
- [x] Per-dialect translation to provider's wire format
  (openai_native: `response_format: json_schema`; vllm_native:
  top-level `structured_outputs: {"json": ...}`; tool_call_fallback:
  synthetic tool + `tool_choice`).
- [x] Normalize `stop_reason` across dialects (table).
- [x] Normalize usage (`input_tokens`, `output_tokens`).
- [x] Validate output against `output_schema`; return
  `SchemaValidationFailed` on mismatch.
- [x] Testing: `wiremock`-backed integration tests as the
  deterministic primary; `#[ignore]`-d smokes for external
  providers.
- [x] Fixture provenance: vllm_native from committed upstream
  JSON; openai_native + tool_call_fallback from synthesized
  shapes (pending Q2 decision on how to anchor those).
- [ ] Publishes as `0.1.0` (after Codex implementation + Claude
  review + Gate-2-equivalent review).

## Open questions (to resolve before Codex prompt archival)

1. **Q1: `jsonschema` crate choice.**
   Options:
   - **`jsonschema = "0.46"`** (Stranger6667/jsonschema) —
     current draft spec baseline. Draft 2020-12 support,
     moderately fast, actively maintained, ~450k downloads/month.
     Transitive deps include `fancy-regex`, `fraction`,
     `uuid`, `url`. Roughly 15 deps total.
   - **`boon` = "0.6"`** — slimmer alternative; fewer deps
     (~8), JSON-Schema-2020-12 only, comparable speed. Less
     ecosystem familiarity.
   - **Hand-rolled subset validation** — implement only the
     schema keywords OpenAI's `response_format: json_schema`
     accepts (`type`, `properties`, `required`, `items`,
     `additionalProperties`, `enum`, `pattern`, `minimum`/
     `maximum`, `minItems`/`maxItems`, `minProperties`/
     `maxProperties`). ~500 LOC, zero dep cost, brittle
     against schemas that go beyond that subset.

   **My recommendation: `jsonschema = "0.46"`.** It's the
   canonical choice; the transitive weight is modest; hand-
   rolling invites bugs in a security-adjacent code path
   (schema validation is what stops a compromised LLM from
   smuggling structured injection payloads past the script's
   type checks).

2. **Q2: Non-vLLM fixture anchoring.**
   We have tamper-evident vllm_native fixtures from upstream.
   `openai_native` + `tool_call_fallback` have no equivalent
   public-tests source — OpenAI's openai-python repo has
   mocked unit tests but doesn't commit byte-exact wire
   fixtures, and the Azure/OpenAI API reference is in prose.
   Options:
   - **Synthesize-and-smoke.** Write expected request bytes
     by hand against the API docs, commit as fixtures, rely
     on the `#[ignore]`-d `openai_smoke` to catch drift.
     (Current draft position.)
   - **Extract from openai-python unit tests.** Their tests
     under `tests/api_resources/chat/test_completions.py` have
     mock-response bodies but the request bytes are
     constructed in-test from Pydantic. Less clean than vLLM's
     upstream; would need mechanical lift + verification
     against the API docs anyway.
   - **Import a small "golden" capture from a real API run.**
     Tee one real request/response per dialect into a
     check-summed fixture. Higher fidelity but requires a
     one-off live API call; fixture would need periodic
     refresh when models / schemas drift.

   **My recommendation: synthesize-and-smoke.** The
   openai-chat-completions wire contract is ~decade-stable;
   synthesis from the API docs gets us a deterministic CI,
   and the optional smoke catches drift whenever it happens.
   Low fidelity loss.

3. **Q3: Retry policy for v1.**
   See "Retry" section above. Three options: no retries /
   hardcoded minimal / config-carried.

   **My recommendation: hardcoded minimal (option 2).**
   LLM-provider 429s are frequent enough that "ignore and
   tell the script to handle it" is user-hostile, but
   config-carried retry adds surface we don't yet know the
   shape of. Ship basic retries (2 attempts, exp backoff,
   respect `Retry-After` seconds) baked in; promote to
   config-carried in 0.2.0 if anyone complains.

---

Next step: archive a Codex prompt under
`docs/codex-prompts/YYYY-MM-DD-NNNN-phase-6-llm-openai-compat.md`
that inlines this spec (so Codex reads it in full), then
spawn the Codex session per the `codex-prompt-archive` skill.
Only once Q1/Q2/Q3 are resolved.
