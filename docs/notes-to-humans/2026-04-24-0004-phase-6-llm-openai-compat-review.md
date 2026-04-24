# Phase 6 Task 2 review — `llm_openai_compat` 0.1.0 (Codex round 02)

**Date**: 2026-04-24 (金)
**Reviewer**: Claude Code
**Subject**: submodule `philharmonic-connector-impl-llm-openai-compat`
  at `7bb5b61`, parent pointer bumped in workspace `b3548e1`.
**Dispatch history**:
- Round 01 (`c01d59e`) halted mid-flight after Yuka spotted a
  structural flaw: item 5 instructed `include_str!` paths that
  escape the submodule boundary, breaking standalone clone +
  `cargo publish`.
- Round 02 (`a9da557`) re-issued with the fixture-location
  correction. Codex initially regressed (wrote the same
  `../../docs/upstream-fixtures/...` paths again), then
  self-corrected during the same run — observed via
  `scripts/codex-logs.sh` while I was preparing a maintenance
  fix. Final tree is clean and publishable.

## TL;DR

Ship it. 41 tests pass, pre-landing green, `cargo package
--list` confirms all 10 fixture files + both smoke files are
included in the tarball. Crate is self-contained for
standalone build + `cargo publish`. Six non-blocking flags
below, none of which gate 0.1.0.

## Conformance with spec + ROADMAP

- [x] `philharmonic-connector-impl-api` 0.1.0 trait impl
  present via `#[async_trait] impl Implementation for
  LlmOpenaiCompat`. Returns `&str` for `name()`; async
  `execute` takes `&JsonValue` + `&JsonValue` + `&ConnectorCallContext`
  per the impl-api contract.
- [x] Config shape `{base_url, api_key, dialect,
  timeout_ms}` with `deny_unknown_fields` and `timeout_ms`
  default 60_000 (matches doc 08 and spec).
- [x] Dialect enum covers `openai_native` | `vllm_native` |
  `tool_call_fallback` with `snake_case` serde rename.
- [x] Request shape matches normalized `llm_generate` from
  doc 08: `model`, `messages`, `output_schema`, optional
  `max_output_tokens` / `temperature` / `top_p` / `stop`.
  Message has `role: Role` (enum System/User/Assistant) +
  `content: String` (single-part only, per doc 08 v1).
- [x] Response shape `{output, stop_reason, usage}` with
  `StopReason` normalized to the 5-value enum and `Usage`
  carrying `input_tokens` + `output_tokens`.
- [x] Per-dialect translation implemented in `src/dialect/`:
  `openai_native.rs` uses `response_format: {type:
  "json_schema", json_schema: {name: "output", strict:
  true, schema}}`; `vllm_native.rs` uses top-level
  `structured_outputs: {"json": <schema>}`;
  `tool_call_fallback.rs` uses synthetic `tools[0].function`
  with `strict: true` + `tool_choice` forcing
  `emit_output`. All three assert byte-for-byte against
  the committed fixtures.
- [x] Stop-reason normalization per the spec's table — `"stop"`
  maps to `EndTurn` for all three dialects (including the
  empirically-correct tool_call_fallback happy path).
- [x] Usage normalization: `prompt_tokens` → `input_tokens`,
  `completion_tokens` → `output_tokens`.
- [x] Schema compile + validate via `jsonschema = "0.46"`
  Draft 2020-12 (`src/schema.rs`).
- [x] Retry: hardcoded minimal 3 attempts, full-jitter
  exponential backoff base 1000ms / cap 8000ms, Retry-After
  seconds (HTTP-date out of scope per spec non-goals).
  Retryable: 429 / 5xx / network / timeout. Not retryable:
  InvalidConfig / InvalidRequest / 4xx-not-429 /
  SchemaValidationFailed / MalformedProviderPayload /
  Internal. (`src/retry.rs`, 5 unit tests.)
- [x] Error mapping via internal `Error` enum with
  `From<Error> for ImplementationError`.
  `MalformedProviderPayload → Internal`, `UpstreamNonSuccess
  → UpstreamError { status, body }`.
- [x] Tests: 41 passing (24 lib unit + 8 error_cases + 3
  happy_path + 2 dialect_openai_native + 1 dialect_vllm_native
  + 1 dialect_tool_call_fallback + 1 schema_validation + 1
  stop_reason_normalization + 0 helpers). 3 `#[ignore]`-d
  smokes (2 openai + 1 vllm). Pre-landing green.
- [x] Fixtures: all under submodule's `tests/fixtures/` tree
  as real byte-exact copies of `docs/upstream-fixtures/`.
  `cargo package --list` confirms each is in the tarball.
- [x] Strict discipline: `strict: true` on `response_format.json_schema`
  AND on `tools[0].function`. OpenAI-strict-compatible
  `sample_json_schema.json` used throughout.
- [x] `CHANGELOG.md` bumped to `[0.1.0] - 2026-04-24`,
  aspirational `[0.0.0]` line dropped.
- [x] Crate-root rustdoc on `src/lib.rs` summarises the three
  dialects + validation + retry model.
- [x] `README.md` expanded minimally.
- [x] MSRV 1.88, edition 2024, license `Apache-2.0 OR
  MPL-2.0`, no panics in library `src/`, no `unsafe`, no
  `anyhow`.

## Module inventory

```
src/ (9 files, 1362 LOC + dialect/)
├── lib.rs              # 102 LOC — trait impl, re-exports, crate docs
├── config.rs           # 82  LOC — LlmOpenaiCompatConfig + Dialect + default_timeout_ms
├── request.rs          # 83  LOC — LlmGenerateRequest + Message + Role
├── response.rs         # 72  LOC — LlmGenerateResponse + StopReason + Usage
├── client.rs           # 77  LOC — reqwest::Client build + execute_one_attempt
├── retry.rs            # 188 LOC — execute_with_retry + full-jitter + SplitMix64 jitter draw
├── schema.rs           # 47  LOC — compile + validate via jsonschema 0.46 Draft 2020-12
├── error.rs            # 135 LOC — internal Error enum + From<Error> for ImplementationError
├── types.rs            # 40  LOC — Codex-added: shared provider-envelope types for dialect modules
└── dialect/
    ├── mod.rs                 # 69  LOC — build_request_body + extract_response dispatch
    ├── openai_native.rs       # 153 LOC — response_format: json_schema translation + extraction
    ├── vllm_native.rs         # 139 LOC — top-level structured_outputs: {json} translation + extraction
    └── tool_call_fallback.rs  # 175 LOC — synthetic tools + tool_choice translation + extraction
```

`src/types.rs` wasn't in the spec — Codex added it on its own
for shared provider-envelope types (`ChatCompletionResponse`,
`ChatCompletionChoice`, `ChatCompletionMessage`, etc.) that
the three dialect modules parse. Round 02 explicitly said
"if it makes sense as a shared-types module, keep it." It
does; kept.

## Non-blocking flags

Numbered for easy reference; none of these gate the 0.1.0
publish. All can land as follow-ups.

1. **Hand-rolled SplitMix64 for jitter**
   (`src/retry.rs:98–118`). Codex bypassed both the spec's
   suggestion to use a transitive `rand` crate and also the
   simpler "nanoseconds % modulus" approach; wrote a full
   SplitMix64 PRNG with a SystemTime + counter seed. Not
   crypto-sensitive (just backoff jitter) and SplitMix64 is
   a well-known PRNG, so functionally fine. But hand-rolling
   primitives goes against the workspace's general
   "RustCrypto only, or flag" instinct even for non-crypto
   PRNGs. Follow-up candidate: swap for `fastrand = "2"`
   (zero-dep, ~200 LOC, widely used) or cut to a simple LCG
   inline with a 5-line `fn`. Not urgent.

2. **Cargo.lock unchanged in parent commit.** Round 01 had
   dirtied `Cargo.lock`; round 02's final state ended up
   matching the previously-committed lock (same dep set
   resolved the same way, evidently). Net zero change —
   flagging for transparency.

3. **`MAX_ATTEMPTS` / `BASE_BACKOFF_MS` / `CAP_BACKOFF_MS`
   are file-private constants** in `src/retry.rs`. That's
   fine for v1 (spec said hardcoded) but if/when a future
   0.2.0 promotes them to config, the knobs already have
   the right shape — just `pub`-ify and thread through
   `LlmOpenaiCompatConfig`.

4. **No docstring on `Implementation` trait impl method
   `execute`**. The trait's own rustdoc in impl-api covers
   the contract, but a 2–3 line `/// ... implements
   dispatch → schema-compile → retry → response-extract →
   output-validate` on the impl would be a nicety. Not
   required.

5. **Smoke tests were flattened during review.** Round 02
   put them at `tests/smokes/openai_smoke.rs` +
   `tests/smokes/vllm_smoke.rs` per the spec's file-tree
   block; cargo's integration-test harness only picks up
   `.rs` files directly under `tests/` (not nested
   subdirectories), so the three `#[ignore]`-d smoke fns
   were orphaned. I moved them up to `tests/openai_smoke.rs`
   and `tests/vllm_smoke.rs` during the review pass (matches
   http_forward's flat layout); `cargo test -- --ignored
   --list` now discovers all three. This is a spec bug on
   my side — the module-layout block suggested the
   subdirectory. Not a Codex error. Follow-up: update the
   spec doc next time I touch it.

6. **`tests/helpers.rs` appears as an empty test crate** in
   the test report (0 tests). That's cargo's behaviour for
   shared helper modules in tests/ — unavoidable without
   moving to `tests/common/mod.rs` layout. Harmless. Noting
   for readers who see "0 passed" and wonder.

## Files touched this session (this task)

Parent:
- `docs/codex-prompts/2026-04-24-0002-phase-6-llm-openai-compat.md`
  (round 01 archive; `c01d59e`).
- `docs/codex-prompts/2026-04-24-0002-phase-6-llm-openai-compat-02.md`
  (round 02 archive; `a9da557`).
- `philharmonic-connector-impl-llm-openai-compat` submodule
  pointer bump from `eb264d1` → `7bb5b61` (`b3548e1`).

Submodule (`7bb5b61`):
- `Cargo.toml`, `CHANGELOG.md`, `README.md` updated.
- `src/lib.rs`, `src/client.rs`, `src/config.rs`, `src/dialect/*.rs`
  (4), `src/error.rs`, `src/request.rs`, `src/response.rs`,
  `src/retry.rs`, `src/schema.rs`, `src/types.rs` — new.
- `tests/dialect_openai_native.rs`,
  `tests/dialect_tool_call_fallback.rs`,
  `tests/dialect_vllm_native.rs`, `tests/error_cases.rs`,
  `tests/happy_path.rs`, `tests/helpers.rs`,
  `tests/openai_smoke.rs`, `tests/schema_validation.rs`,
  `tests/stop_reason_normalization.rs`,
  `tests/vllm_smoke.rs` — new.
- `tests/fixtures/README.md` + 9 byte-exact fixture-tree
  copies (vllm/: 2 files; openai-chat/: 5 files; vllm_native_response.json).

## Verification

```
./scripts/pre-landing.sh philharmonic-connector-impl-llm-openai-compat
  ↳ === pre-landing: all checks passed ===

cargo test -p philharmonic-connector-impl-llm-openai-compat --all-targets
  ↳ 41 passed / 0 failed / 0 ignored / 0 measured

cargo test -p philharmonic-connector-impl-llm-openai-compat --all-targets -- --ignored --list
  ↳ openai_smoke: openai_native_smoke, tool_call_fallback_smoke (2 tests)
     vllm_smoke:   vllm_native_smoke                            (1 test)

cargo package --list -p philharmonic-connector-impl-llm-openai-compat
  ↳ Includes src/ (10 files), tests/ (10 integration files), tests/fixtures/ (10 files).
    No workspace-escape paths. Ready for pub-fresh at Yuka's signal.
```

## Recommendation

**Publish 0.1.0 via `./scripts/publish-crate.sh
philharmonic-connector-impl-llm-openai-compat` using the
`pub-fresh` alias (same mechanism http_forward used today).
All acceptance criteria for Phase 6 met; both 0.1.0-impl
crates shipped same-day.** Follow-up flags are all landable
post-publish without breaking the 0.1.0 surface.
