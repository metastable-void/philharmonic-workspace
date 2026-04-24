# Phase 6 Task 1 — `http_forward` implementation (initial dispatch)

**Date:** 2026-04-24
**Slug:** `phase-6-http-forward`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

First substantive Phase-6 connector implementation: `http_forward`,
the generic HTTP-forwarding connector. It is the simplest
implementation and the canary for the connector-impl contract
landed in impl-api 0.1.0 (Phase 6 Task 0) — all three scoping
blockers are resolved (Option B trait location, `async_trait`
macro, `reqwest` + `rustls-tls` + tokio runtime stack).

Non-crypto task: no Gate 1/2, no key material, no COSE, no
protocol primitives. Doc 08 §"Generic HTTP" owns the wire
protocol; the implementation spec drafted for this dispatch
owns the Rust-level shape.

## References (read before coding)

- **Authoritative implementation spec** (drives this dispatch):
  `docs/notes-to-humans/2026-04-24-0001-phase-6-task-1-http-forward-impl-spec.md`.
  All three open questions in that doc are resolved inline; if
  the body of this prompt contradicts the spec, the spec wins.
- `docs/design/08-connector-architecture.md`:
  - §"Implementation trait" + §"Why `async_trait` (in 2026)"
    — trait shape + rationale.
  - §"Per-implementation crates" — dep-surface rules (depend on
    `connector-impl-api` + whatever external crates the impl
    needs; no crypto).
  - §"Generic HTTP" — full wire protocol for `http_forward`
    (config / request / response / error shapes, with the
    `HttpEndpoint` JSON example, per-body-type decoding rules,
    every `ImplementationError` variant mapping).
- `ROADMAP.md` §"Phase 6 — First implementations" — acceptance
  criteria for Task 1.
- `CONTRIBUTING.md`:
  - §10.3 no panics in library `src/`.
  - §10.4 libraries take bytes, not file paths.
  - §10.9 HTTP-client rule (`reqwest` + `rustls-tls` + tokio
    for runtime crates; `ureq` forbidden in runtime deps;
    workspace-wide reqwest version consistency).
  - §4 git workflow, §5 script wrappers, §11 pre-landing.
- `philharmonic-connector-impl-api` 0.1.0 — source the trait
  and re-exports from there (`Implementation`, `async_trait`,
  `ConnectorCallContext`, `ImplementationError`, `JsonValue`).
- `mechanics-config` 0.1.0 — `HttpEndpoint`, `PreparedHttpEndpoint`,
  `HttpEndpoint::prepare_runtime()`, `build_url_prepared()`,
  `build_headers_prepared()`. DO NOT reinvent; use the exported
  methods.

If anything in this prompt contradicts the docs above, the
docs win. Flag any contradiction and stop rather than guessing.

## Crate state (starting point)

- `philharmonic-connector-impl-http-forward` — currently a
  0.0.0 placeholder submodule at
  `philharmonic-connector-impl-http-forward/`. Has
  `Cargo.toml` (placeholder), `src/lib.rs` (placeholder
  comment), `README.md`, `CHANGELOG.md`, `LICENSE-*`,
  `.gitignore`.
- Never published substantively to crates.io (verified HTTP
  404 via `./scripts/xtask.sh crates-io-versions --
  philharmonic-connector-impl-http-forward`). Drop any
  aspirational `[0.0.0] Name reservation` entry from the
  CHANGELOG when bumping — same precedent as impl-api (see
  its `CHANGELOG.md` as a pattern).
- Workspace-internal `[patch.crates-io]` entry for this crate
  already exists in root `Cargo.toml`; no workspace-root
  changes are needed.

Target state after this dispatch: `0.1.0`, Implementation
trait implemented, wiremock-backed integration tests passing,
pre-landing green, working tree dirty (Claude commits +
publishes after review).

## Scope

### In scope

1. Populate `philharmonic-connector-impl-http-forward/Cargo.toml`:
   - Version `0.0.0` → `0.1.0`.
   - `[dependencies]` per the spec's §"Dependencies" block.
   - `[dev-dependencies]` per the same block (`tokio` with
     `test-util`, `wiremock`).
   - Keep `[profile.release]` section as-is.
2. Implement the module layout the spec lays out:
   - `src/lib.rs` — module plumbing + public `HttpForward` type +
     `impl Implementation for HttpForward`.
   - `src/config.rs` — `HttpForwardConfig` + `PreparedConfig`
     (with mandatory `response_max_bytes` validation per the
     spec's Q2 resolution).
   - `src/request.rs` — `HttpForwardRequest` with camelCase ↔
     snake_case serde rename.
   - `src/response.rs` — `HttpForwardResponse` + body decoding
     per `EndpointBodyType`.
   - `src/client.rs` — `reqwest::Client` constructor + one
     `execute_one_attempt` helper.
   - `src/retry.rs` — retry loop implementing
     `EndpointRetryPolicy` semantics (full-jitter exponential
     backoff, Retry-After parsing, `max_retry_delay_ms` overall
     cap).
   - `src/error.rs` — internal `Error` enum + `From<Error> for
     ImplementationError` (including the `body: String`
     JSON-encoded `{status, headers, body}` payload shape per
     the spec's Q1 resolution).
3. Unit tests colocated with each module per the spec's test
   list (validate unknown-field rejection, camelCase
   deserialization, header-key lowercasing, backoff formulae,
   Retry-After parsing both forms, deadline breaks, error
   mapping).
4. Integration tests under `tests/`:
   - `happy_path.rs` — json / utf8 / bytes round-trips.
   - `error_cases.rs` — every `ImplementationError` variant
     triggered and verified.
   - `retry.rs` — 5xx retry-to-success, 429 with and without
     `Retry-After`, attempt-exhaustion, overall deadline.
   - `request_vectors.rs` — fixed input config + request →
     exact outbound HTTP request (method, URL, headers, body
     bytes) via wiremock request capture.
5. Populate `CHANGELOG.md` with a `[0.1.0] - 2026-04-24` entry
   describing the initial substantive release. Drop the
   aspirational `[0.0.0]` line (see precedent in
   `philharmonic-connector-impl-api/CHANGELOG.md`).
6. `src/lib.rs` doc comment on the crate root (multi-paragraph
   rustdoc; matches the density of
   `philharmonic-connector-impl-api/src/lib.rs`).

### Out of scope (flag; do NOT implement)

- Any change to `philharmonic-connector-impl-api`,
  `philharmonic-connector-common`, `philharmonic-connector-service`,
  or the wire protocol in doc 08. If you think the trait or
  error types need adjustment, stop and flag.
- Streaming request/response bodies (buffered only per the
  spec's "Non-goals (v1)").
- Custom CA bundles, client certs, OAuth token refresh, any
  auth beyond "bake the Authorization header into the
  config". Explicitly Non-goals in the spec.
- Metrics / structured logging / tracing hooks.
- `cargo publish`, `git tag`, any commit or push — Claude
  handles those post-review.
- Workspace-root `Cargo.toml` edits — already in place.

### Decisions fixed upstream (do NOT deviate)

From the spec's §"Decisions (resolved 2026-04-24)":

1. **`ImplementationError::UpstreamError.body: String`** carries
   a JSON-serialized `{"status": <u16>, "headers": {…},
   "body": <JsonValue>}` sub-object. Do not reshape; do not
   add typed fields to `connector-common`'s error enum.
2. **`response_max_bytes` is mandatory at config load**. A
   config without the field must fail deserialization /
   validation with `ImplementationError::InvalidConfig {
   detail: "missing response_max_bytes" }` before any HTTP
   call. No framework default, no fallback.
3. **`reqwest = "0.13"`** (not 0.12). `http_forward` is the
   first reqwest consumer in the workspace; keep the pin
   consistent with what CONTRIBUTING.md §10.9 now formalizes.

## Workspace conventions (authoritative:
`CONTRIBUTING.md`, `docs/design/13-conventions.md`)

- Edition 2024, MSRV 1.88.
- License `Apache-2.0 OR MPL-2.0`.
- `thiserror` for library errors; no `anyhow` in library code.
- **No panics in library `src/`.** No `.unwrap()` /
  `.expect()` on `Result` / `Option`, no `panic!` /
  `unreachable!` / `todo!` / `unimplemented!` on reachable
  paths, no unbounded indexing, no unchecked integer
  arithmetic, no lossy `as` casts on untrusted widths.
  Tests and `#[cfg(test)]` helpers can `.unwrap()` freely.
- **Library crates take bytes, not file paths.** The trait
  hands this impl decrypted `JsonValue`s; never read from
  disk or the environment.
- **No `unsafe`.** This crate has no call for it; flag any
  temptation.
- **Rustdoc on every `pub` item.** Module-level rustdoc on
  each file covers the module's purpose in 1–3 paragraphs.
- **Re-export discipline** (CONTRIBUTING.md §10.6): re-export
  types from direct deps that appear in your public API.
  `HttpForward` appears; `Implementation` /
  `ImplementationError` / `ConnectorCallContext` / `JsonValue`
  / `async_trait` all come from `connector-impl-api` — keep
  consumers able to depend on just this crate for the common
  case.
- **Use `./scripts/*.sh` wrappers**, not raw `cargo`. See
  Pre-landing below.

## HTTP client (CONTRIBUTING.md §10.9)

- `reqwest` with `default-features = false` + features
  `["rustls-tls", "json", "gzip", "deflate", "brotli"]`.
  No `native-tls`. No `ureq` in this crate.
- Single `reqwest::Client` per `HttpForward` instance; reuse
  across calls (connection pooling, TLS session reuse).
- Per-request timeout from `HttpEndpoint::timeout_ms()`.
  Overall wall-time cap (across retries) from
  `retry_policy.max_retry_delay_ms()`.
- Response body size enforcement: streamed accumulator check
  (`bytes_stream()` + per-chunk length tracking) against
  `response_max_bytes`. Not buffer-then-check.

## Retry / backoff (from the spec)

Full-jitter exponential backoff:

```
delay_n = rand_uniform(0, min(base * 2^n, cap))
```

where `base = retry_policy.base_backoff_ms()` and
`cap = retry_policy.max_backoff_ms()`.

429 overrides:
- If `retry_policy.respect_retry_after()` and `Retry-After`
  header parses (seconds OR HTTP-date per RFC 7231), use that.
- Else use `retry_policy.rate_limit_backoff_ms()`.

Overall-deadline guard: if the scheduled sleep would push
wall-clock past `max_retry_delay_ms` from the first attempt,
return `ImplementationError::UpstreamTimeout` instead of
sleeping. The overall deadline also breaks the retry loop
between attempts.

Retry eligibility per error class:
- `UpstreamTimeout` → retry iff `retry_on_timeout`.
- `UpstreamUnreachable` → retry iff `retry_on_io_errors`.
- `UpstreamNonSuccess { status, .. }` → retry iff `status` is
  in `retry_on_status`.
- Every other internal error class → do NOT retry.

## Pre-landing

```sh
./scripts/pre-landing.sh philharmonic-connector-impl-http-forward
```

This runs fmt + check + clippy (with `-D warnings`) + test
for the crate and its deps. Must pass clean. Do NOT run raw
`cargo fmt` / `cargo check` / `cargo clippy` / `cargo test` —
the script normalizes flag choices and wires `CARGO_TARGET_DIR`
correctly.

If pre-landing fails, fix the problem and re-run. If the
failure is in a dep you didn't touch, flag and stop.

## Git

You do NOT commit, push, branch, tag, or publish. Leave the
working tree dirty in the `philharmonic-connector-impl-http-forward`
submodule. Claude runs `scripts/commit-all.sh` after reviewing
your output and then `scripts/publish-crate.sh` once ready.

Read-only git is fine (`git log`, `git diff`, `git show`,
`git blame`, `git status`, `git rev-parse`).

## Deliverables

1. `philharmonic-connector-impl-http-forward/Cargo.toml`
   populated with deps + dev-deps per the spec; version bumped
   to 0.1.0.
2. `philharmonic-connector-impl-http-forward/src/` — all 7
   modules (`lib.rs`, `config.rs`, `request.rs`, `response.rs`,
   `client.rs`, `retry.rs`, `error.rs`) with full
   implementation + colocated unit tests.
3. `philharmonic-connector-impl-http-forward/tests/` — 4
   integration test files against wiremock (happy, errors,
   retry, request_vectors).
4. `philharmonic-connector-impl-http-forward/CHANGELOG.md` —
   `[0.1.0] - 2026-04-24` entry; aspirational `[0.0.0]`
   entry dropped.
5. Optional: `philharmonic-connector-impl-http-forward/README.md`
   slight expansion beyond the placeholder-template (one-
   paragraph "what this does" above the existing
   "Contributing" section is enough).

Working tree: dirty. Do not commit.

## Structured output contract

Return in your final message:

1. **Summary** (3–6 sentences): what landed, what tests pass,
   any deviations from the spec and why.
2. **Files touched**: bulleted list of absolute paths relative
   to the workspace root.
3. **Verification results**:
   - Output of `./scripts/pre-landing.sh philharmonic-connector-impl-http-forward`
     (pass/fail + notable warnings).
   - Integration test count (passed / failed / ignored).
4. **Residual risks / TODOs**: anything you'd flag for
   Claude's post-review pass, including anything you thought
   the spec under-specified that required judgement.
5. **Git state**: `git -C philharmonic-connector-impl-http-forward
   status --short` output; confirm you did not commit or push.
6. **Dep versions used**: exact resolved versions of
   `reqwest`, `tokio`, `wiremock`, plus any transitive dep
   whose version was surprising or pinned specifically.

## Default follow-through policy

- If pre-landing fails, fix the cause and re-run — don't
  return a red tree.
- If a spec detail is genuinely ambiguous, pick the
  interpretation that minimizes the public API surface and
  matches doc 08's wire protocol most closely, then flag
  which one you picked under "Residual risks".
- If any dep refuses to resolve (e.g., `wiremock 0.6` pulls
  in a conflicting transitive), try the next-older compatible
  minor before flagging — record what you tried and what
  worked.

## Completeness contract

"Done" means:
- All 7 `src/` modules exist with full implementation (not
  `todo!()` placeholders, not partial `unimplemented!()`).
- All 4 `tests/` files exist and run green against wiremock.
- Pre-landing passes clean (`-D warnings` enforces no clippy
  warnings).
- `cargo test -p philharmonic-connector-impl-http-forward`
  reports all tests passing, none ignored-by-default except
  an optional `httpbin_smoke` test behind an env flag (if you
  add one).
- CHANGELOG and Cargo.toml version are consistent (`0.1.0`).

## Verification loop

Before returning, in order:

```sh
./scripts/pre-landing.sh philharmonic-connector-impl-http-forward
cargo test -p philharmonic-connector-impl-http-forward --all-targets
git -C philharmonic-connector-impl-http-forward status --short
git -C . status --short
```

Expected:
- pre-landing: "all checks passed".
- cargo test: all passing.
- submodule status: dirty files for Cargo.toml, CHANGELOG,
  src/*, tests/* (and maybe README).
- workspace status: `modified: philharmonic-connector-impl-http-forward`
  (the pointer bump Claude will commit later); no other
  workspace-root changes.

## Missing-context gating

If any of the following apply, **stop and flag** instead of
guessing:

- A required doc (spec, ROADMAP, doc 08, CONTRIBUTING, or
  impl-api source) is missing or unreadable.
- A dep version you need has been yanked from crates.io or is
  missing.
- `mechanics-config::HttpEndpoint` exposes a method the spec
  names but you can't find (suggests a doc-code drift — flag
  rather than reinvent).
- `connector-impl-api`'s public surface does not match what the
  spec says it exports.

## Action safety

- No `cargo publish`, no `git push`, no branch creation, no
  tags. Claude owns those.
- No `rm -rf` or destructive file ops outside the crate
  directory.
- No edits outside `philharmonic-connector-impl-http-forward/`
  — the workspace root is stable for this task.
- If you need to run ad-hoc `cargo` commands during
  exploration, OK; before final return, run through the
  pre-landing script so the verification matches what Claude
  will re-run.

---

## Outcome

Pending — will be updated after Codex run completes.
