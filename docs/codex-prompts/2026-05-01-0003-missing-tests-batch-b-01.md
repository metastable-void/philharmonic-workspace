# Missing tests batch B: security headers, lowerer, executor, SinglePool (initial dispatch)

**Date:** 2026-05-01
**Slug:** `missing-tests-batch-b`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Four items from the remaining-TODOs list (#7, #9, #10, #13) need
unit tests in the API server bin crate and the store-sqlx-mysql
crate.

## References

- `docs/notes-to-humans/2026-05-01-0001-remaining-todos.md` items 7, 9, 10, 13
- `bins/philharmonic-api-server/src/security_headers.rs`
- `bins/philharmonic-api-server/src/lowerer.rs`
- `bins/philharmonic-api-server/src/executor.rs`
- `philharmonic-store-sqlx-mysql/src/connection.rs`

## Context files pointed at

- `bins/philharmonic-api-server/src/security_headers.rs`
- `bins/philharmonic-api-server/src/lowerer.rs`
- `bins/philharmonic-api-server/src/executor.rs`
- `philharmonic-store-sqlx-mysql/src/connection.rs`

## Outcome

Pending — will be updated after Codex run.

---

## Prompt (verbatim)

<task>
Add unit tests for four modules across two crates.

## 1. Security headers middleware (item #7)

File: `bins/philharmonic-api-server/src/security_headers.rs`

Read the file to find what headers the `inject` middleware adds.
Add a `#[cfg(test)] mod tests` block that:

- Constructs a minimal axum test setup with the middleware
- Sends a request through it
- Asserts the response contains:
  - `X-Content-Type-Options: nosniff`
  - `X-Frame-Options: DENY`
  - `Cache-Control: no-store`
  - `Referrer-Policy: strict-origin-when-cross-origin`
  - `Permissions-Policy` (any value — just assert the header exists)

Use `axum::Router::new().route("/", get(|| async { "ok" }))`
with the middleware layered on, and `axum::body::to_bytes` or
similar to drive the request. The bin crate already depends on
`axum`, `tokio`, etc.

You'll need to add `axum = { version = "0.8", features = ["macros"] }`
to the bin's dev-dependencies if the `get` macro import isn't available.
Or use `axum::routing::get` directly.

## 2. ConnectorConfigLowerer unit test (item #9)

File: `bins/philharmonic-api-server/src/lowerer.rs`

Read the file to understand `ConnectorConfigLowerer`. It wraps
`philharmonic-connector-client` for COSE_Sign1 + COSE_Encrypt0.
Test with known inputs:

- Create a `LowererSigningKey` from a fixed 32-byte seed
- Create a `RealmPublicKeyRegistry` with a test ML-KEM + X25519
  keypair (generate deterministically or use fixed bytes)
- Call `lower()` with a test `TenantEndpointConfig` (or whatever
  the input type is)
- Assert the output is `Ok(...)` and the lowered config bytes
  are non-empty

If the `lower` method requires types that are hard to construct,
test the simpler public methods or just verify construction
succeeds without panicking. Read the code first.

NOTE: This test touches crypto primitives. Do NOT invent new
crypto constructions. Only use the existing public API of
`philharmonic-connector-client`. If the lowerer's `lower()` method
requires a realm key, generate one using the crate's own API.

## 3. HttpStepExecutor test (item #10)

File: `bins/philharmonic-api-server/src/executor.rs`

Read the file. `HttpStepExecutor` does an HTTP POST to the
connector service URL. Use `wiremock` for a mock HTTP server:

- Add `wiremock = "0.6"` to the bin's `[dev-dependencies]`
- Start a `MockServer`
- Create an `HttpStepExecutor` pointing at the mock's URI
- Call `execute()` with a test input
- Assert the mock received a POST with:
  - `Authorization: Bearer <hex-encoded token>`
  - `X-Encrypted-Payload: <hex-encoded payload>` (or however
    the headers are named — read the code)

If `execute()` requires complex crypto inputs (COSE tokens),
just verify that the method constructs and sends an HTTP request
without panicking. Focus on the HTTP dispatch behavior, not the
crypto.

If `wiremock` is too heavy, use `tokio::net::TcpListener` as a
minimal server that reads the request headers.

## 4. SinglePool::connect() error path (item #13)

File: `philharmonic-store-sqlx-mysql/src/connection.rs`

Add a test that verifies `SinglePool::connect("invalid://bad-url")`
returns an error (not a panic). This should be a simple unit test:

```rust
#[tokio::test]
async fn connect_bad_url_returns_error() {
    let result = SinglePool::connect("invalid://not-a-real-host:99999/fake").await;
    assert!(result.is_err());
}
```

The test should NOT require a running MySQL server. The connection
attempt to a bogus URL should fail quickly with a `StoreError`.

## Verification

After implementing:

1. Run `./scripts/pre-landing.sh` — it auto-detects touched crates
   and runs fmt + check + clippy (-D warnings) + test.
2. Do NOT run raw `cargo fmt/check/clippy/test` — use the scripts.
3. If `pre-landing.sh` finds issues, fix them and re-run.
4. Use `./scripts/build-status.sh` if cargo seems stuck.

## Git rules

- Commit via `./scripts/commit-all.sh "<message>"` ONLY.
- Do NOT run `./scripts/push-all.sh` or `cargo publish`.
- Do NOT run raw `git commit` / `git add` / `git push`.
</task>

<default_follow_through_policy>
If a step produces warnings, errors, or unexpected output, address
them immediately before proceeding to the next step.
</default_follow_through_policy>

<completeness_contract>
The task is complete when:
1. All four test groups are implemented
2. `./scripts/pre-landing.sh` passes cleanly
3. Changes are committed via `./scripts/commit-all.sh`
</completeness_contract>

<verification_loop>
After each significant code change:
1. Run `./scripts/pre-landing.sh`
2. If it fails, fix and re-run
3. Only commit after a clean pass
</verification_loop>

<missing_context_gating>
If you cannot find a type, method, or pattern referenced in this
prompt, grep for it before inventing alternatives.
</missing_context_gating>

<action_safety>
- Never run `./scripts/push-all.sh`
- Never run `cargo publish`
- Never run raw git commands
- Never modify files outside the scope listed above
</action_safety>

<structured_output_contract>
When done, report:
- Summary: what was implemented
- Files touched: list with brief description of changes
- Verification: pre-landing.sh output (pass/fail)
- Git state: commit SHA, branch, pushed=no
</structured_output_contract>
