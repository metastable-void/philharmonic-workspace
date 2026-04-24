# Phase 6 Task 1 — `http_forward` Codex output review

**Author**: Claude Code
**Date**: 2026-04-24 (金), JST ~17:00
**Codex session**: `task-mock8uqm-7in7om` (session id
`019dbe49-92c4-7302-9d05-92ba6f801cf8`)
**Prompt archive**: [`docs/codex-prompts/2026-04-24-0001-phase-6-http-forward.md`](../codex-prompts/2026-04-24-0001-phase-6-http-forward.md)
**Spec**: [`2026-04-24-0001-phase-6-task-1-http-forward-impl-spec.md`](2026-04-24-0001-phase-6-task-1-http-forward-impl-spec.md)
**Verdict**: **approved for commit + publish**, with six
small flags below (all non-blocking).

## Summary

Codex delivered a complete `philharmonic-connector-impl-http-forward`
0.1.0 per the spec: 7 src modules + 4 wiremock-backed
integration test suites. `./scripts/pre-landing.sh` passes
(`=== pre-landing: all checks passed ===`), and `cargo test
-p philharmonic-connector-impl-http-forward --all-targets`
reports **30 tests passed / 0 failed / 0 ignored** (13 unit
in lib + 8 error_cases + 3 happy_path + 1 request_vectors +
5 retry). Code is idiomatic Rust 2024; no panics in library
`src/`; no `unsafe`; no `anyhow`. All three pre-resolved
decisions honored: UpstreamError carries a JSON-encoded
`{status, headers, body}` string in `body: String` (Q1);
`response_max_bytes` is mandatory at config load with
`InvalidConfig { detail: "missing response_max_bytes" }`
(Q2); reqwest pinned to `0.13` with `rustls` feature (Q3).

## Module-by-module review

### `src/lib.rs`

Clean trait impl. `HttpForward::new()` + `with_client()` give
a pluggable Client for tests. `ctx` is threaded through to
`retry::execute_with_retry` but not read there (underscore-
prefixed) — fine as a forward hook for future metrics /
logging. Public surface re-exports `async_trait`,
`ConnectorCallContext`, `Implementation`,
`ImplementationError`, `JsonValue` from
`connector-impl-api`, matching the re-export discipline in
CONTRIBUTING.md §10.6.

### `src/config.rs`

`HttpForwardConfig` is a thin wrapper around
`mechanics_config::HttpEndpoint` (single field
`endpoint`), with `serde(deny_unknown_fields)` so typo'd
admin configs surface as `InvalidConfig`. `prepare()` does
three things in order: (1) reject if `response_max_bytes`
missing (Q2); (2) call `HttpEndpoint::validate_config()`;
(3) cache the returned `PreparedHttpEndpoint` in
`PreparedConfig`. Minor: the `response_max_bytes` check is
duplicated across lines 22–27 and 32–35 — first branch
rejects up front, second unwraps for the struct field.
Cosmetic; could collapse into a single unwrap-or-error after
the first check. **Not a bug.**

### `src/request.rs`

`HttpForwardRequest` deserializes the camelCase wire form
with `serde(rename_all = "camelCase")`; `deny_unknown_fields`
rejects typos. `encode_request_body()` handles all three
`EndpointBodyType` variants plus the method-doesn't-support-
body case. **Minor spec-drift flag 🚩**: when
`request_body_type` is `utf8` or `bytes` and the request
carries `body: null` (explicit JSON null, deserialized as
`Some(JsonValue::Null)`), the code errors with
`InvalidRequest` rather than treating null as "no body" per
doc 08 §"Request shape" bullet "Missing / `null` → no body
sent". For `json` type, `body: null` encodes to the literal
bytes `"null"` with `Content-Type: application/json`, again
rather than sending no body. Practical impact is low (most
HTTP servers accept `"null"` identically to no-body; `utf8`/
`bytes` users rarely send `null`), but the behavior diverges
from doc 08's three-way rule. Easy follow-up: in each arm of
`encode_request_body`, treat `Some(JsonValue::Null)` the same
as `None`.

### `src/response.rs`

Response shaping is clean: `BTreeMap<String, String>` for
deterministic header ordering, lowercase normalization on
exposed names, multi-value join with `", "`. Body-size
enforcement is belt-and-suspenders: short-circuit on
`Content-Length` when the server sends one, otherwise
accumulator check on each streamed chunk. The streamed path
uses `futures_util::StreamExt::next()` on
`reqwest::Response::bytes_stream()`. Rust 2024 let-chains
used correctly in the `content_length > limit` branch.
Empty JSON body correctly decodes to `JsonValue::Null`.

### `src/client.rs`

Single `reqwest::Client` per `HttpForward` instance, built
with default `Client::builder()` (features set in
`Cargo.toml`, not programmatically). Per-request timeout
applied via `builder.timeout(...)` from
`HttpEndpoint::timeout_ms()`. Content-Type default
(`application/json` / `text/plain; charset=utf-8` /
`application/octet-stream`) is only emitted if no caller-
provided `Content-Type` exists — so baked-in or
script-override headers take precedence. Non-2xx handling
routes through `allow_non_2xx_status`: `false` → error with
status+headers+body+retry_after; `true` → normal response
with `ok: false`. **Minor flag 🚩**: `reqwest::Url::parse(&url)`
(line 34) is redundant — `build_url` returns a validated
String, and `client.request(method, url)` accepts `&str` via
the `IntoUrl` trait. Dropping the parse call saves one
allocation per request. Non-blocking.

### `src/retry.rs`

Full-jitter exponential backoff: `delay_n = uniform(0,
min(base * 2^n, cap))`. Per-429 overrides: if
`respect_retry_after && Retry-After` parseable (seconds OR
HTTP-date via `httpdate` crate), use it; else use
`rate_limit_backoff_ms`. Overall-deadline enforcement via
`sleep_would_exceed_deadline(now, delay, deadline)` — pre-
sleep check so we don't over-sleep the budget. Per-error-
class retry eligibility matches the spec (timeout →
`retry_on_timeout`, unreachable → `retry_on_io_errors`,
non-success status → `retry_on_status.contains(status)`;
everything else is terminal). Deterministic unit tests with
seeded `StdRng`. **Minor flag 🚩**: lines 103–104 cap the
per-retry delay at `max_retry_delay_ms` (the overall wall-
time budget), which is slightly more restrictive than spec —
a large Retry-After exceeding the overall budget would be
capped at the budget, then `sleep_would_exceed_deadline`
would still break. Redundant but harmless.

### `src/error.rs`

Internal `Error` enum with `thiserror::Error`; `From<Error>
for ImplementationError` does the mapping. `UpstreamError`
variant carries `body: String` that holds
`serde_json::to_string(json!({"status", "headers", "body"}))`
— exactly the Q1 resolution. Every variant covered in the
`every_internal_variant_maps_to_wire` test. Fallback if the
JSON encoding itself fails: `Internal { detail: ... }` —
defensive and correct.

## Deviations from the spec (flags for follow-up, not blockers)

1. **`reqwest` feature name**: spec wrote `"rustls-tls"`,
   Codex used `"rustls"` in `Cargo.toml`. Both are valid
   features on `reqwest = "0.13.2"` and both select the
   rustls TLS stack (no native-tls / no OpenSSL / musl-
   static-linkable). Outcome matches CONTRIBUTING.md §10.9;
   no action required beyond optionally updating the spec
   doc to reflect current reqwest 0.13 feature naming.

2. **`body: null` handling** (see request.rs §above). Three-
   way spec rule "Missing / `null` → no body sent" is
   only honored for the "Missing" case; explicit `null`
   either errors (`utf8`/`bytes`) or encodes as `"null"`
   bytes (`json`). Low-severity spec-drift; easy follow-up.

3. **`mechanics-config` prepared-builder visibility**:
   `HttpEndpoint::build_url_prepared` and
   `build_headers_prepared` are `pub(crate)` in
   mechanics-config 0.1.0, so impl-http-forward cannot call
   the prepared path directly — it instead calls the public
   `build_url` / `build_headers`, each of which re-runs
   `prepare_runtime()` internally. Correctness is fine;
   performance hit is one url-template parse + allowlist
   HashSet rebuild per call, dwarfed by the actual network
   I/O but non-zero. Worth a mechanics-config 0.2.0 bump to
   expose the prepared methods publicly when the next
   substantive mechanics-config change lands. Tracked
   separately; not blocking Phase 6 Task 1.

4. **Unused `url` crate**: `Cargo.toml` declares `url = "2"`
   but no source file uses it directly — `url::*`/`use url`
   absent, and the only `Url` reference is
   `reqwest::Url::parse` which uses reqwest's transitive
   `url`. Dead direct dep. Either remove the line or keep
   for forward use; trivial cleanup.

5. **`POSIX_CHECKLIST.md` file-mode change**: workspace-root
   file changed from `0755` → `0644` during Codex's run
   (probably an incidental filesystem op from cargo
   packaging or wiremock test runs). `0644` is actually the
   semantically-correct mode — the file is markdown, not an
   executable — so accept the change as a coincidental
   cleanup rather than restore `0755`.

6. **Workspace `Cargo.lock` is legitimately modified**
   despite Codex's summary claiming otherwise. The 172-line
   diff adds entries for reqwest + hyper + tower + tokio +
   wiremock + aws-lc-rs + their transitives — required for
   reproducible workspace builds now that impl-http-forward
   is a real consumer. This diff must land alongside the
   impl crate's changes; any attempt to "restore" Cargo.lock
   would break `cargo check --workspace`. Codex's stated
   restoration was incorrect (the file is still modified);
   accept the diff as required.

## Conformance with acceptance criteria

From ROADMAP.md Phase 6 Task 1:

| Criterion                                                             | Status |
| ---                                                                   | ---    |
| Config reuses `mechanics_config::HttpEndpoint`                        | ✓      |
| Request shape `{url_params, query, headers, body}` validated          | ✓      |
| Response `{status, headers, body}`, exposed-header allowlist applied | ✓      |
| 4xx/5xx as response when `allow_non_2xx_status=true`; else error     | ✓      |
| Network / timeout → `UpstreamUnreachable` / `UpstreamTimeout`         | ✓      |
| `reqwest` with rustls-TLS; single reused `Client`                     | ✓      |
| Per-request timeout from `HttpEndpoint.timeout_ms`                    | ✓      |
| Wiremock-backed integration tests (CI-deterministic)                  | ✓      |
| Crate ready to publish as `0.1.0`                                     | ✓      |

From the spec's decision list:

| Decision                                                              | Status |
| ---                                                                   | ---    |
| Q1 `body: String` JSON-encoded `{status, headers, body}`              | ✓      |
| Q2 `response_max_bytes` mandatory; absence → `InvalidConfig`          | ✓      |
| Q3 `reqwest = "0.13"` (with rustls feature set)                       | ✓      |

## Next steps

1. Commit Codex output + Cargo.lock via
   `./scripts/commit-all.sh`. The submodule commit carries
   the impl; the parent commit carries the submodule pointer
   + Cargo.lock diff + (optionally) `POSIX_CHECKLIST.md`
   mode restoration.
2. Update `## Outcome` section of
   [`docs/codex-prompts/2026-04-24-0001-phase-6-http-forward.md`](../codex-prompts/2026-04-24-0001-phase-6-http-forward.md)
   with the Codex result summary + link to this review.
3. Decide whether to land the six small flags above as
   follow-ups or include in this commit. Recommendation:
   fix #5 (POSIX_CHECKLIST.md mode) in the same commit (trivial),
   defer the others to small fix-forward commits after
   publish — none of them block 0.1.0's correctness.
4. Once landed, run `./scripts/publish-crate.sh
   philharmonic-connector-impl-http-forward` to publish
   0.1.0 via the `pub-fresh` alias path.

Gate-2-equivalent review complete (non-crypto task; no
formal Gate). No red lines; ready to publish.
