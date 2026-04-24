# Phase 6 Task 2 — `llm_openai_compat` implementation (initial dispatch)

**Date:** 2026-04-24
**Slug:** `phase-6-llm-openai-compat`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Second Phase-6 connector implementation: `llm_openai_compat`,
the OpenAI-compatible LLM connector covering OpenAI itself,
vLLM, Together, Groq, OpenRouter, and any other server that
speaks the OpenAI `/chat/completions` wire protocol.
`http_forward` (Task 1) shipped at 0.1.0 earlier the same
day; this is the capstone dispatch that unblocks the Phase 6
acceptance criteria.

Non-crypto task: no Gate 1/2, no key material, no COSE.
Doc 08 §"llm_openai_compat — config and dialects" owns the
wire protocol; the implementation spec drafted for this
dispatch owns the Rust-level shape.

## References (read before coding)

- **Authoritative implementation spec** (drives this
  dispatch):
  [`docs/notes-to-humans/2026-04-24-0003-phase-6-task-2-llm-openai-compat-impl-spec.md`](../notes-to-humans/2026-04-24-0003-phase-6-task-2-llm-openai-compat-impl-spec.md).
  All three open questions resolved inline. If the body of
  this prompt contradicts the spec, the spec wins.
- [`docs/design/08-connector-architecture.md`](../design/08-connector-architecture.md):
  - §"Implementation trait" + §"Why `async_trait` (in 2026)"
    — trait shape + rationale.
  - §"Per-implementation crates" — dep-surface rules (depend
    on `connector-impl-api` + whatever external crates the
    impl needs; no crypto).
  - §"LLM — specialized HTTP implementations" + §"The
    `llm_generate` wire protocol" — normalized request /
    response shapes.
  - §"`llm_openai_compat` — config and dialects" — config
    shape + per-dialect translation rules.
- [`ROADMAP.md`](../../ROADMAP.md) §"Phase 6 — First
  implementations" → Task 2 — acceptance criteria for this
  impl.
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md):
  - §10.3 no panics in library `src/`.
  - §10.4 libraries take bytes, not file paths.
  - §10.9 HTTP-client rule (`reqwest` + `rustls-tls` + tokio
    for runtime crates; `ureq` forbidden in runtime deps;
    workspace-wide reqwest version consistency — must match
    `http_forward`'s 0.13 pin).
  - §4 git workflow, §5 script wrappers, §11 pre-landing.
- `philharmonic-connector-impl-api` 0.1.0 — source the trait
  + re-exports (`Implementation`, `async_trait`,
  `ConnectorCallContext`, `ImplementationError`, `JsonValue`).
- `philharmonic-connector-impl-http-forward` 0.1.0 —
  reference implementation of the trait contract at one
  layer of remove (HTTP without LLM specifics). Read the
  crate's `src/lib.rs` and `src/retry.rs` for the trait-impl
  shape and the full-jitter retry pattern; do not
  re-derive. Same workspace, same conventions.
- Committed upstream fixtures:
  - [`docs/upstream-fixtures/vllm/`](../upstream-fixtures/vllm/)
    — pinned to vLLM commit
    `cf8a613a87264183058801309868722f9013e101`. Source of
    truth for `vllm_native` request bytes.
  - [`docs/upstream-fixtures/openai-chat/`](../upstream-fixtures/openai-chat/)
    — captured 2026-04-24 against real OpenAI API (model
    `gpt-4o-mini-2024-07-18`). Source of truth for
    `openai_native` + `tool_call_fallback` request AND
    response bytes.

If anything in this prompt contradicts the docs above, the
docs win. Flag any contradiction and stop rather than
guessing.

## Crate state (starting point)

- `philharmonic-connector-impl-llm-openai-compat` —
  currently a 0.0.0 placeholder submodule at
  `philharmonic-connector-impl-llm-openai-compat/`. Has
  `Cargo.toml` (placeholder `[dependencies]` empty),
  `src/lib.rs` (empty or single-line placeholder), `README.md`,
  `CHANGELOG.md`, `LICENSE-*`, `.gitignore`.
- Never published substantively (verified via
  `./scripts/xtask.sh crates-io-versions --
  philharmonic-connector-impl-llm-openai-compat` at spec-
  drafting time). Drop any aspirational `[0.0.0] Name
  reservation` entry from the CHANGELOG when bumping — same
  precedent as impl-api and http_forward.
- Workspace-internal `[patch.crates-io]` entry for this
  crate already exists in root `Cargo.toml`; no
  workspace-root edits are needed.

Target state after this dispatch: `0.1.0`, Implementation
trait implemented, deterministic wiremock-backed tests +
request-vector byte-assertions for all three dialects
passing, pre-landing green, working tree dirty in the
submodule (Claude commits + publishes after review).

## Scope

### In scope

1. Populate `philharmonic-connector-impl-llm-openai-compat/Cargo.toml`:
   - Version `0.0.0` → `0.1.0`.
   - `[dependencies]` per the spec's §"Dependencies" block
     (async-trait 0.1, impl-api 0.1, connector-common 0.2,
     reqwest 0.13 with the same feature set `http_forward`
     uses, tokio 1 with `rt` / `macros` / `time`, serde 1,
     serde_json 1, thiserror 2, jsonschema 0.46).
   - `[dev-dependencies]`: tokio with `test-util`, wiremock 0.6.
   - Keep `[profile.release]` block as-is (workspace's
     published-crate template).
2. Implement the module layout the spec lays out:
   - `src/lib.rs` — crate-root rustdoc + module plumbing +
     public `LlmOpenaiCompat` type + `impl Implementation
     for LlmOpenaiCompat`.
   - `src/config.rs` — `LlmOpenaiCompatConfig` + `Dialect`
     enum (`openai_native`, `vllm_native`,
     `tool_call_fallback`); `timeout_ms` default = 60_000.
   - `src/request.rs` — `LlmGenerateRequest` + `Message` +
     `Role` (snake_case rename; `deny_unknown_fields`).
   - `src/response.rs` — `LlmGenerateResponse` + `StopReason`
     (5-variant normalized enum) + `Usage`.
   - `src/client.rs` — shared `reqwest::Client` construction;
     single-attempt POST to `{base_url}/chat/completions`;
     `Authorization: Bearer <api_key>`.
   - `src/dialect/mod.rs` — per-dialect dispatch glue.
   - `src/dialect/openai_native.rs` — `response_format:
     {"type":"json_schema", "json_schema": {"name":"output",
     "strict": true, "schema": <output_schema>}}` translation +
     response extraction from `choices[0].message.content`
     (parse inner JSON string).
   - `src/dialect/vllm_native.rs` — top-level
     `structured_outputs: {"json": <output_schema>}`
     translation (NOT under `extra_body`); same OpenAI
     chat-completion response envelope as openai_native.
   - `src/dialect/tool_call_fallback.rs` — `tools` +
     `tool_choice` translation with `strict: true` on the
     function; response extraction from
     `choices[0].message.tool_calls[0].function.arguments`.
   - `src/schema.rs` — compile + validate helpers over
     `jsonschema = "0.46"`'s Draft 2020-12 validator.
   - `src/retry.rs` — **hardcoded minimal** retry (3
     attempts total = 2 retries after first); 429 / 5xx /
     network errors; full-jitter exp backoff base 1000ms
     cap 8000ms; honor `Retry-After` on 429 (seconds format
     only — HTTP-date is out-of-scope per spec non-goals).
     No `retry_policy` field on config in v1.
   - `src/error.rs` — internal `Error` enum + `From<Error>
     for ImplementationError`; `MalformedProviderPayload` →
     `Internal`, `UpstreamNonSuccess` → `UpstreamError`.
3. Unit tests colocated with each module per the spec's
   test list (dialect translation shapes, deny_unknown_fields,
   Role/StopReason round-trips, default timeout, error
   mapping, retry math).
4. Integration tests under `tests/`:
   - `happy_path.rs` — one wiremock-backed success test per
     dialect, response fixtures loaded from the committed
     `docs/upstream-fixtures/openai-chat/*/response.json`
     (for openai_native + tool_call_fallback); for
     vllm_native, **synthesize** a response that matches
     OpenAI chat-completion envelope (same keys, same
     nesting, different `id` / `created` values). Commit
     that synthesized vLLM-response fixture under
     `tests/fixtures/vllm_native_response.json` with a
     short comment in the same directory's `README.md`
     explaining it's synthesized (upstream only commits
     requests).
   - `error_cases.rs` — every `ImplementationError` variant
     triggered: InvalidConfig (missing field), InvalidRequest
     (schema compile fails), UpstreamError (401, 500),
     UpstreamUnreachable (connection refused), UpstreamTimeout
     (per-attempt timeout), SchemaValidationFailed
     (provider output doesn't match schema), Internal
     (malformed provider envelope).
   - `dialect_openai_native.rs` — **byte-exact** assertion:
     given a fixed config + fixed request + committed schema,
     the outbound HTTP body matches
     `docs/upstream-fixtures/openai-chat/openai_native/request.json`
     modulo (a) the user prompt content (rehydrate from the
     fixture), (b) whitespace (use canonicalized JSON or
     `serde_json::Value` equality).
   - `dialect_vllm_native.rs` — **byte-exact** assertion
     against
     `docs/upstream-fixtures/vllm/structured_outputs_json_chat_request.json`,
     modulo `messages[1].content` (upstream redacted the
     inline schema with a placeholder; reconstruct via
     string interpolation of
     `docs/upstream-fixtures/vllm/sample_json_schema.json`
     at test time) and modulo `model` (upstream uses
     `HuggingFaceH4/zephyr-7b-beta`; use whatever the
     fixture declares verbatim).
   - `dialect_tool_call_fallback.rs` — **byte-exact**
     assertion against
     `docs/upstream-fixtures/openai-chat/tool_call_fallback/request.json`.
   - `schema_validation.rs` — provider returns off-schema
     output → `SchemaValidationFailed`; error detail includes
     a readable path.
   - `stop_reason_normalization.rs` — parametric: each
     dialect × each provider `finish_reason` (`stop`,
     `length`, `content_filter`, `tool_calls`, `other`)
     maps per the spec's table. Critically for
     tool_call_fallback: `stop` IS the happy path (reports
     `EndTurn`).
   - `smokes/openai_smoke.rs` — `#[ignore]`-d, gated on
     `OPENAI_SMOKE_ENABLED=1 OPENAI_API_KEY=...`. One real
     call through openai_native (cheap model like
     `gpt-4o-mini`) + one through tool_call_fallback;
     assert the response round-trips as expected.
   - `smokes/vllm_smoke.rs` — `#[ignore]`-d, gated on
     `VLLM_SMOKE_ENABLED=1 VLLM_BASE_URL=http://...`. One
     real call through vllm_native; diff outbound request
     bytes against the committed vLLM fixture.
5. Load committed fixture files at test compile time via
   `include_str!("../../../docs/upstream-fixtures/...")`.
   Do NOT copy them into the crate's own `tests/fixtures/`
   — the `docs/upstream-fixtures/` tree is the single
   source of truth with pinned upstream SHAs. The sole
   exception: the synthesized vLLM response fixture lives
   in `tests/fixtures/` (since there's no upstream source
   for it).
6. Populate `CHANGELOG.md` with a `[0.1.0] - 2026-04-24`
   entry describing the initial release. Drop the
   aspirational `[0.0.0]` line.
7. Crate-root rustdoc on `src/lib.rs` matching the density
   of `philharmonic-connector-impl-api/src/lib.rs` and
   `philharmonic-connector-impl-http-forward/src/lib.rs`:
   what the crate does, which three dialects it covers, the
   dispatch model, usage snippet showing `LlmOpenaiCompat::new()`
   into the `Implementation` trait.
8. Minor `README.md` expansion (1-paragraph "what this
   does" above the existing Contributing section).

### Out of scope (flag; do NOT implement)

- Any change to `philharmonic-connector-impl-api`,
  `philharmonic-connector-common`, `philharmonic-connector-service`,
  `mechanics-config`, or the wire protocol in doc 08. If
  you think the trait or error types need adjustment, stop
  and flag.
- Additional connector impls (llm_anthropic, llm_gemini) —
  those are Phase 7.
- Support for `/v1/responses` (OpenAI's newer endpoint) —
  deferred per the spec's decision trail.
- Streaming responses.
- Multi-part content parts (images / audio / mixed) — doc
  08 restricts `messages[].content` to `String`-only for v1.
- Tool calling at the wire protocol level — `tool_call_fallback`
  uses tool calling as an internal structured-output
  transport only; `tools` never appears in the normalized
  wire request.
- Custom CA bundles, client certs, auth schemes beyond
  `Authorization: Bearer <api_key>`.
- Any `retry_policy` config field — retries are hardcoded
  in v1 per spec Q3.
- HTTP-date parsing of `Retry-After` (seconds format only).
- Response caching / metrics / structured logging / tracing
  hooks.
- `cargo publish`, `git tag`, `git push`, any commit —
  Claude handles those post-review.
- Workspace-root `Cargo.toml` edits — already in place.
- Editing any fixture under `docs/upstream-fixtures/` —
  those are committed and immutable for this task.

### Decisions fixed upstream (do NOT deviate)

From the spec's §"Decisions (resolved 2026-04-24)":

1. **`jsonschema = "0.46"`** (Stranger6667/jsonschema),
   Draft 2020-12. Not boon, not hand-rolled.
2. **Fixtures come from `docs/upstream-fixtures/`** via
   `include_str!`. The vLLM tree is pinned to
   `cf8a613a87264183058801309868722f9013e101`; the
   openai-chat tree is a real-API capture against
   `gpt-4o-mini-2024-07-18`. Both trees are tamper-
   evident anchors for drift checks.
3. **Retry: hardcoded minimal** — 3 attempts total (= 2
   retries), full-jitter exp backoff base 1000ms cap 8000ms,
   honor `Retry-After` seconds, HTTP-date out of scope. No
   `retry_policy` config field.
4. **`strict: true` everywhere** — on
   `response_format.json_schema` (openai_native) AND on
   `tools[0].function` (tool_call_fallback). Project-wide
   discipline. Schemas that use strict-incompatible features
   (`pattern`, `minimum`/`maximum`, `minItems`/`maxItems`,
   `minProperties`/`maxProperties`, `format`,
   `minLength`/`maxLength`, `multipleOf`) will fail at
   OpenAI with a 400 that we surface as UpstreamError.
5. **`tool_call_fallback` stop_reason**: `"stop"` is the
   happy path (empirically verified — OpenAI reports
   `finish_reason: "stop"` when `tool_choice` forces a
   specific function and the model complies), maps to
   `EndTurn`.
6. **reqwest version**: `"0.13"` (same major.minor as
   `http_forward`). CONTRIBUTING.md §10.9 workspace-wide
   consistency rule.

## Workspace conventions (authoritative:
`CONTRIBUTING.md`, `docs/design/13-conventions.md`)

- Edition 2024, MSRV 1.88.
- License `Apache-2.0 OR MPL-2.0`.
- `thiserror` for library errors; no `anyhow` in library
  code.
- **No panics in library `src/`**. No `.unwrap()` /
  `.expect()` on `Result` / `Option`, no `panic!` /
  `unreachable!` / `todo!` / `unimplemented!` on reachable
  paths, no unbounded indexing, no unchecked integer
  arithmetic, no lossy `as` casts on untrusted widths.
  Tests and `#[cfg(test)]` helpers can `.unwrap()` freely.
- **Library crates take bytes, not file paths.** The trait
  hands this impl decrypted `JsonValue`s; never read from
  disk or the environment.
- **No `unsafe`**. This crate has no call for it; flag any
  temptation.
- **Rustdoc on every `pub` item.** Module-level rustdoc on
  each file covers the module's purpose in 1–3 paragraphs.
- **Re-export discipline** (CONTRIBUTING.md §10.6): re-export
  types from direct deps that appear in your public API.
  `LlmOpenaiCompat` appears; `Implementation` /
  `ImplementationError` / `ConnectorCallContext` / `JsonValue`
  / `async_trait` come from `connector-impl-api` — keep
  consumers able to depend on just this crate for the common
  case.
- **Use `./scripts/*.sh` wrappers**, not raw `cargo`. See
  Pre-landing below.

## HTTP client (CONTRIBUTING.md §10.9)

- `reqwest` with `default-features = false` + features
  `["rustls-tls", "json", "gzip", "deflate", "brotli"]`
  (same as `http_forward`). No `native-tls`. No `ureq`.
- Single `reqwest::Client` per `LlmOpenaiCompat` instance;
  reuse across calls (connection pooling, TLS session reuse).
- Per-request timeout from `LlmOpenaiCompatConfig::timeout_ms`
  (default 60_000).

## Schema validation

- Compile `output_schema` once per `execute()` call via
  `jsonschema::draft202012::new(schema)`. Compile failure →
  `InvalidRequest` (script gave a broken schema).
- Validate the parsed `output` once per success response.
  Validation failure → `SchemaValidationFailed` with the
  formatted `jsonschema` error as `detail` (rich JSON-pointer
  paths make scripts debuggable).

## Retry (hardcoded, from the spec)

```
max_attempts = 3     // i.e. 2 retries after the initial try
base = 1000ms
cap = 8000ms

for attempt in 0..max_attempts:
    result = one_attempt(...).await
    match result:
        Ok -> return Ok
        Err @ retryable(e) if attempt + 1 < max_attempts:
            delay = uniform(0, min(base * 2^attempt, cap))
            if let UpstreamNonSuccess(429, hdrs) = e:
                if let Some(secs) = parse_retry_after_seconds(hdrs):
                    delay = secs
            tokio::time::sleep(delay).await
            continue
        Err(e) -> return Err(e)
```

`retryable(e)` is true iff:
- `UpstreamNonSuccess(status)` where `status == 429` or
  `500 <= status < 600`.
- `UpstreamUnreachable(_)` (network / io / TLS).
- `UpstreamTimeout`.

Not retryable: `InvalidConfig`, `InvalidRequest`,
`UpstreamNonSuccess(4xx != 429)`, `SchemaValidationFailed`,
`MalformedProviderPayload`, `Internal`.

Full-jitter via `rand::random_range(0..=max)` or similar
(use whichever rand crate version is already in the
dependency tree — likely transitive via reqwest). Don't add
a direct `rand` dep unless nothing transitive is usable.

## Pre-landing

```sh
./scripts/pre-landing.sh philharmonic-connector-impl-llm-openai-compat
```

This runs fmt + check + clippy (`-D warnings`) + test for
the crate and its deps. Must pass clean. Do NOT run raw
`cargo fmt` / `cargo check` / `cargo clippy` / `cargo test`
— the script normalizes flag choices and wires
`CARGO_TARGET_DIR` correctly.

If pre-landing fails, fix the problem and re-run. If the
failure is in a dep you didn't touch, flag and stop.

## Git

You do NOT commit, push, branch, tag, or publish. Leave the
working tree dirty in the
`philharmonic-connector-impl-llm-openai-compat` submodule.
Claude runs `scripts/commit-all.sh` after reviewing your
output and then `scripts/publish-crate.sh` once ready.

Read-only git is fine (`git log`, `git diff`, `git show`,
`git blame`, `git status`, `git rev-parse`).

## Deliverables

1. `philharmonic-connector-impl-llm-openai-compat/Cargo.toml`
   populated with deps + dev-deps per the spec; version
   bumped to 0.1.0.
2. `philharmonic-connector-impl-llm-openai-compat/src/` —
   all modules per the spec's §"Module layout"
   (`lib.rs`, `config.rs`, `request.rs`, `response.rs`,
   `client.rs`, `dialect/{mod, openai_native, vllm_native,
   tool_call_fallback}.rs`, `schema.rs`, `retry.rs`,
   `error.rs`) with full implementation + colocated unit
   tests.
3. `philharmonic-connector-impl-llm-openai-compat/tests/` —
   integration test files per the spec's §"Integration
   tests": `happy_path.rs`, `error_cases.rs`,
   `dialect_openai_native.rs`, `dialect_vllm_native.rs`,
   `dialect_tool_call_fallback.rs`, `schema_validation.rs`,
   `stop_reason_normalization.rs`, `smokes/openai_smoke.rs`
   (`#[ignore]`-d), `smokes/vllm_smoke.rs` (`#[ignore]`-d).
4. `philharmonic-connector-impl-llm-openai-compat/tests/fixtures/vllm_native_response.json`
   — synthesized response fixture matching the OpenAI chat-
   completion envelope (upstream vLLM only commits requests,
   not responses). One-paragraph `tests/fixtures/README.md`
   explaining the synthesis.
5. `philharmonic-connector-impl-llm-openai-compat/CHANGELOG.md`
   — `[0.1.0] - 2026-04-24` entry; aspirational `[0.0.0]`
   entry dropped.
6. `philharmonic-connector-impl-llm-openai-compat/README.md`
   — one-paragraph expansion above the existing Contributing
   section (what the crate does, three-dialect coverage,
   link back to doc 08).

Working tree: dirty. Do not commit.

## Structured output contract

Return in your final message:

1. **Summary** (3–6 sentences): what landed, what tests
   pass, any deviations from the spec and why.
2. **Files touched**: bulleted list of absolute paths
   relative to the workspace root.
3. **Verification results**:
   - Output of `./scripts/pre-landing.sh
     philharmonic-connector-impl-llm-openai-compat`
     (pass/fail + notable warnings).
   - Test counts (unit / integration, passed / failed /
     ignored-by-default).
4. **Residual risks / TODOs**: anything you'd flag for
   Claude's post-review pass, including anything you thought
   the spec under-specified that required judgement.
5. **Git state**: `git -C philharmonic-connector-impl-llm-openai-compat
   status --short` output; confirm you did not commit or
   push.
6. **Dep versions used**: exact resolved versions of
   `reqwest`, `tokio`, `wiremock`, `jsonschema`, plus any
   transitive dep whose version was surprising or pinned
   specifically (e.g. `rand` if any).

## Default follow-through policy

- If pre-landing fails, fix the cause and re-run — don't
  return a red tree.
- If a spec detail is genuinely ambiguous, pick the
  interpretation that minimizes the public API surface and
  matches doc 08's wire protocol most closely, then flag
  which one you picked under "Residual risks".
- If `jsonschema = "0.46"` has a breaking API that the spec
  didn't anticipate, try the latest 0.4x compatible minor
  before flagging. Record what you tried.
- If `wiremock 0.6` pulls in a conflicting transitive, try
  the next-older compatible minor before flagging. Record
  what you tried.
- `tool_call_fallback` happy path reports `finish_reason:
  "stop"`, not `"tool_calls"` — the captured fixture at
  `docs/upstream-fixtures/openai-chat/tool_call_fallback/response.json`
  is the ground truth here; trust it over any secondary
  documentation.

## Completeness contract

"Done" means:

- All `src/` modules (including the `dialect/` sub-tree)
  exist with full implementation (no `todo!()` placeholders,
  no partial `unimplemented!()`).
- All integration test files exist and run green against
  wiremock; `smokes/*` are `#[ignore]`-d.
- Pre-landing passes clean (`-D warnings` enforces no clippy
  warnings).
- `cargo test -p
  philharmonic-connector-impl-llm-openai-compat --all-targets`
  reports all non-ignored tests passing.
- CHANGELOG and Cargo.toml version are consistent (`0.1.0`).
- All three dialects produce outbound bytes that match
  their committed fixture (byte-exact equality on the
  serde_json::Value level, modulo the documented carve-outs
  for each dialect's fixture).

## Verification loop

Before returning, in order:

```sh
./scripts/pre-landing.sh philharmonic-connector-impl-llm-openai-compat
cargo test -p philharmonic-connector-impl-llm-openai-compat --all-targets
git -C philharmonic-connector-impl-llm-openai-compat status --short
git -C . status --short
```

Expected:

- pre-landing: "all checks passed".
- cargo test: all non-ignored tests passing; `#[ignore]`-d
  smokes skipped.
- submodule status: dirty files for Cargo.toml, CHANGELOG,
  src/*, tests/* (and maybe README).
- workspace status: `modified:
  philharmonic-connector-impl-llm-openai-compat` (the
  pointer bump Claude will commit later); no other
  workspace-root changes.

## Missing-context gating

If any of the following apply, **stop and flag** instead of
guessing:

- A required doc (spec, ROADMAP, doc 08, CONTRIBUTING,
  impl-api source, http_forward source as reference) is
  missing or unreadable.
- A fixture file under `docs/upstream-fixtures/` is
  missing, unreadable, or has content that contradicts the
  spec.
- A dep version you need has been yanked from crates.io or
  is missing.
- `jsonschema = "0.46"`'s actual API shape differs enough
  from the spec's `jsonschema::draft202012::new(schema)` /
  `Validator::validate(&output)` assumption that the spec
  is materially wrong.
- `connector-impl-api`'s public surface doesn't match what
  the spec says it exports.

## Action safety

- No `cargo publish`, no `git push`, no branch creation, no
  tags. Claude owns those.
- No `rm -rf` or destructive file ops outside the crate
  directory.
- No edits outside
  `philharmonic-connector-impl-llm-openai-compat/`. The
  workspace root, `docs/upstream-fixtures/`, and every
  other submodule are stable for this task.
- If you need to run ad-hoc `cargo` commands during
  exploration, OK; before final return, run through the
  pre-landing script so the verification matches what
  Claude will re-run.

---

## Outcome

**Halted 2026-04-24 ~18:40 JST (task id `a350df678cb92cf70`,
stopped via `TaskStop`).**

Codex had progressed substantially before the halt: modified
`Cargo.toml`, `CHANGELOG.md`, `README.md`, and `src/lib.rs`
in the submodule; added all module files (`src/client.rs`,
`src/config.rs`, `src/dialect/` subtree, `src/error.rs`,
`src/request.rs`, `src/response.rs`, `src/retry.rs`,
`src/schema.rs`) plus a self-added `src/types.rs` that wasn't
in the spec, plus a `tests/` directory. Parent tree: modified
`Cargo.lock` and the submodule pointer.

**Reason for halt**: item 5 of "In scope" was structurally
wrong — instructed fixtures to be loaded via
`include_str!("../../../docs/upstream-fixtures/...")` from
the submodule's `tests/*.rs`, which escapes the submodule
boundary. That breaks (a) standalone clone + build of the
impl repo (GitHub Actions CI in the submodule repo has no
access to the parent's `docs/upstream-fixtures/` tree), (b)
`cargo publish --dry-run`'s "file is outside package
directory" check, and (c) the published `.crate` tarball —
downstream consumers couldn't build tests. Surfaced by Yuka
mid-dispatch.

**Resolution**: redispatch as **round 02** with fixtures
copied into the submodule's own `tests/fixtures/` tree. See
[`2026-04-24-0002-phase-6-llm-openai-compat-02.md`](./2026-04-24-0002-phase-6-llm-openai-compat-02.md)
for the round-02 prompt and outcome. Partial work from this
round is left in the submodule working tree as a starting
state that round 02 can pick up and adapt (or replace) as
needed.
