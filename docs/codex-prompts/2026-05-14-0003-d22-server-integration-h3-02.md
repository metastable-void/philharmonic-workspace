# D22 server-integration — mhs HTTP/3 body streaming (round 02)

**Date:** 2026-05-14 (JST)
**Slug:** `d22-server-integration-h3`
**Round:** 02 — extends round 01 to remove the body-buffering
limitation in mhs's HTTP/3 service shape.
**Subagent:** `codex:codex-rescue`

## Motivation

Round 01 (`-01.md`) landed the wiring but left a known
incompleteness called out in its Outcome:

> The new axum adapter buffers response bodies up to
> 16 MiB because the current `Http3Server` public service shape
> is `Request<()> -> Response<Bytes>`. Consequently, the
> `mechanics` h3-side request handler can only support
> body-less/request-metadata paths until the mhs public API
> grows request-body streaming.

Yuka's 2026-05-14 directive: **"streaming must be supported in
mhs h3; too incomplete."** Round 02 grows mhs's `Http3Server`
public API to support proper request-body recv and response-body
send streaming, then updates the axum_compat adapter to use the
streaming shape (no buffer cap). After this round, HTTP/3 servers
built on mhs handle large request and response bodies without
memory bloat — same as the TCP+TLS path.

## References (read in this order)

1. `docs/codex-prompts/2026-05-14-0003-d22-server-integration-h3-01.md`
   — round 01 prompt + Outcome. Read in full; round 02 inherits
   its operational discipline.
2. `mechanics-http-server/src/`:
   - `lib.rs` — public re-exports.
   - `server.rs` — `Http3Server`, `Http3Handle`, the current
     service signature at line 57:
     `Service<Request<()>, Response = Response<Bytes>>`.
   - `axum_compat.rs` — round 01's adapter; buffers response
     bodies up to `DEFAULT_H3_RESPONSE_BODY_LIMIT_BYTES` (16
     MiB). Currently feeds empty request bodies.
3. h3 crate at
   `~/.cargo/registry/src/.../h3-0.0.8/src/server/{request,stream}.rs`
   — the actual upstream API:
   - `RequestResolver::resolve_request() -> (Request<()>,
     RequestStream<C::BidiStream, B>)`.
   - `RequestStream::recv_data() -> Result<Option<impl Buf>>`
     for chunk-by-chunk request-body recv.
   - `RequestStream::send_response(Response<()>)`, then
     `RequestStream::send_data(B)` for chunk-by-chunk response-
     body send.
   - `RequestStream::recv_trailers()` /
     `send_trailers()` / `finish()`.
4. `http-body` crate (already in the workspace tree via axum's
   transitive deps) — the `Body` trait abstracts streaming
   request / response bodies. axum's `axum::body::Body`
   implements it. This is the right abstraction for mhs's new
   service shape.
5. `CONTRIBUTING.md` §§3.1, 4, 5, 10.3, 10.9, 11.

## Goal — high level

Change mhs's `Http3Server::start` service shape to support
streaming bodies in both directions. The new shape, in
http-body terms:

```rust
pub fn start<S, RespBody>(
    self,
    service: S,
    tls_cert_chain: Vec<rustls::pki_types::CertificateDer<'static>>,
    tls_private_key: rustls::pki_types::PrivateKeyDer<'static>,
) -> Result<Http3Handle>
where
    S: Service<Request<RequestBody>, Response = Response<RespBody>>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    S::Error: std::fmt::Display + Send + Sync + 'static,
    RespBody: http_body::Body<Data = Bytes> + Send + 'static,
    RespBody::Error: std::fmt::Display + Send + Sync + 'static,
```

Where:

- `RequestBody` is a new mhs-public type that wraps the h3
  `RequestStream`'s recv side and implements `http_body::Body`.
  It drains `recv_data()` chunks (and optionally
  `recv_trailers()`) as `http_body::Frame<Bytes>`.
- `RespBody` is any user-supplied `http_body::Body` that yields
  `Bytes` data frames. mhs drives it via the standard
  `poll_frame` pattern, calling `RequestStream::send_data(...)`
  for each data frame chunk and `send_trailers(...)` for the
  trailers frame.
- The exact `RequestBody` shape — owned vs. borrowed, naming —
  is Codex's call. Keeping it owned + `Send` simplifies the
  service-future signature.

Codex picks the precise type names. Suggested:
- `mechanics_http_server::H3RequestBody` (or `Body`) for the
  request side.
- The response side stays generic over `RespBody:
  http_body::Body<Data = Bytes>` — no new mhs type needed.

## What changes in mhs

1. **`Http3Server::start` signature** — switches from
   `Request<()> -> Response<Bytes>` to the streaming shape
   above. The accept loop's per-request work changes to:
   - Resolve `(Request<()>, RequestStream)` from h3.
   - Construct an `H3RequestBody` wrapping the
     `RequestStream`'s recv half.
     - Note: h3's `RequestStream` is a unified handle for both
       directions. Splitting into recv-half + send-half is a
       design decision — `RequestStream::split()` exists in
       newer h3 versions; check h3 0.0.8's API and either use
       a split helper if present, or wrap the unified
       `RequestStream` in an `Arc<Mutex<_>>`-like primitive,
       or pass the whole `RequestStream` around via a small
       state machine. Pick the cleanest.
   - Build `Request<H3RequestBody>` from the resolved headers
     and the body wrapper.
   - Call `service.call(req)` to get `Response<RespBody>`.
   - Send response headers via
     `RequestStream::send_response(Response::from_parts(parts,
     ()))`.
   - Drive `RespBody`'s `poll_frame` loop: for each
     `Frame::data(bytes)`, call
     `RequestStream::send_data(bytes)`. For
     `Frame::trailers(map)`, call `send_trailers(map)`. Then
     `finish()`.
   - All work async; backpressure flows naturally since
     `send_data` is async and the service's body polling is
     pull-based.

2. **New `H3RequestBody` type** (or whatever Codex names it):
   - Implements `http_body::Body<Data = Bytes, Error = SomeErr>`.
   - `poll_frame` calls into h3's `recv_data` (async) under the
     hood. Since `http_body::Body::poll_frame` is sync-poll-shaped
     and h3's `recv_data` is async, the wrapper needs a
     `tokio::pin!` / `Pin<Box<dyn Future>>` pattern to bridge
     them. The standard `http_body::Body` impls for axum and
     hyper do exactly this — Codex can crib from those.
   - The error type should be a small mhs-defined enum or
     `Box<dyn Error + Send + Sync>` for caller flexibility.
   - Trailers support: if h3's `recv_trailers()` is non-trivial
     to integrate, optional for round 02. Document if so.

3. **`axum_compat::router_into_h3_service`** — drops the
   buffering. Now that mhs accepts `RespBody:
   http_body::Body`, the adapter just plumbs axum's `Body`
   (which implements `http_body::Body`) through. Similarly,
   the adapter feeds axum's Router an `axum::Body` constructed
   from mhs's `H3RequestBody` (via `axum::body::Body::new(...)`
   wrap). No body-size caps anywhere. Update the docs to
   reflect.

4. **mhs tests** — extend the existing end-to-end h3 test to
   verify:
   - Large request body (e.g. 32 MiB random bytes) round-trips.
   - Large response body streams correctly (e.g. 32 MiB).
   - Trailers (optional; nice to have).

5. **`DEFAULT_H3_RESPONSE_BODY_LIMIT_BYTES`** and its
   `router_into_h3_service_with_response_limit` variant —
   round 02 removes them since there's no longer a buffer
   cap to set. If keeping the helper for API stability is
   easier (deprecated stub forwarding to the streaming
   adapter), that's acceptable; flag in residual risks.
   Preferred: delete the now-meaningless helper and bump.
   The buffer-cap constant is also gone.

6. **CHANGELOG** under mhs 0.1.3:
   - Breaking: `Http3Server::start` service shape changed
     from `Service<Request<()>, Response = Response<Bytes>>`
     to the streaming `http_body::Body`-based shape.
     `axum_compat::router_into_h3_service` is unchanged at
     the call site (still takes `axum::Router`) but no
     longer buffers.
   - Removed: `DEFAULT_H3_RESPONSE_BODY_LIMIT_BYTES` constant
     and the `_with_response_limit` variant (the cap is no
     longer meaningful).
   - Added: `H3RequestBody` (or whatever Codex names it)
     public type implementing `http_body::Body`.

## What does NOT change in mhs

- `AltSvcLayer` and the `alt_svc` module — unchanged.
- `Http3ServerConfig`, `Http3Handle`, `Http3Server::new` —
  unchanged.
- `default_zero_rtt_methods`, `is_zero_rtt_safe` — unchanged.
- The accept loop's connection-acceptance logic + zero-RTT
  replay safety — unchanged.
- TLS plumbing — unchanged.

## What changes downstream (mechanics, bins)

For the workspace's three release bins and `mechanics`:

- Both go through `axum_compat::router_into_h3_service` at
  the API surface — the streaming change is internal to mhs.
  Their consumers shouldn't need updates.
- But: any downstream test (in mhs or elsewhere) that asserted
  the buffer-cap behaviour needs to be updated to the new
  streaming behaviour.
- If `mechanics` or any bin directly used
  `DEFAULT_H3_RESPONSE_BODY_LIMIT_BYTES` or the `_with_response_limit`
  variant, that breaks — verify by grep and migrate.

No `mechanics` version bump expected unless its public API
shifts (unlikely; the round-01 wiring goes through
`router_into_h3_service` which keeps its outer shape).
`mechanics-http-server 0.1.2 → 0.1.3`.

Bins are `publish = false`; no bumps.

## Non-goals

- No mhc changes.
- No new feature gating on mhs (`axum` stays a hard dep; if a
  future round wants axum-optional, that's a separate
  discussion).
- No connector-router h3.
- No publishing during the Codex round (Claude publishes if
  needed post-review).
- No webui changes.
- No design-doc rewrites.
- No mechanics module redesign (D18 is the next dispatch
  after this one).

## Commit discipline (binding)

Same as round 01:

- **Codex does NOT commit.** No `./scripts/commit-all.sh`,
  no `git commit` / `git add` / `git push` / `git stash`.
  Read-only `git status` / `git diff` / `git log` are fine.
- **Codex does NOT publish.** No `./scripts/publish-crate.sh`,
  no `cargo publish`.
- **Codex CAN run `./scripts/pre-landing.sh`** at the end of
  the dispatch.
- Every cargo invocation needs `CARGO_TARGET_DIR=target-main`.
- Per-crate `cargo check -p <crate> --all-targets` after each
  crate's edits.

## Outcome

Pending — will be updated after Codex round 02 run.

---

<task>
Make mhs's HTTP/3 service shape properly streaming (both
request and response bodies). The round 01 wiring buffers
response bodies up to 16 MiB and feeds empty request bodies;
neither is acceptable for production HTTP/3 server use. After
this round, mhs's `Http3Server` accepts any service whose
request/response shape uses `http_body::Body`, and the existing
axum_compat adapter streams without caps.

**Authoritative references:**

1. `docs/codex-prompts/2026-05-14-0003-d22-server-integration-h3-01.md`
   — round 01 prompt + Outcome.
2. `mechanics-http-server/src/{lib.rs,server.rs,axum_compat.rs}`
   — the current public surface.
3. `~/.cargo/registry/src/.../h3-0.0.8/src/server/{request,stream}.rs`
   — h3's RequestResolver / RequestStream APIs.
4. `http-body` crate's `Body` trait (transitive via axum).

**Concrete tasks:**

1. **New mhs public type `H3RequestBody`** (or equivalent
   name): wraps h3's request-recv half. Implements
   `http_body::Body<Data = Bytes, Error = SomeErr>`. Polls
   `RequestStream::recv_data()` under the hood; bridges
   async-recv to sync-poll-shape via the standard
   `Pin<Box<dyn Future>>` pattern (axum/hyper crib it the same
   way).

2. **`Http3Server::start` signature change**:
   ```rust
   pub fn start<S, RespBody>(
       self,
       service: S,
       tls_cert_chain: Vec<rustls::pki_types::CertificateDer<'static>>,
       tls_private_key: rustls::pki_types::PrivateKeyDer<'static>,
   ) -> Result<Http3Handle>
   where
       S: Service<Request<H3RequestBody>, Response = Response<RespBody>>
           + Clone + Send + 'static,
       S::Future: Send + 'static,
       S::Error: std::fmt::Display + Send + Sync + 'static,
       RespBody: http_body::Body<Data = Bytes> + Send + 'static,
       RespBody::Error: std::fmt::Display + Send + Sync + 'static,
   ```
   The accept loop:
   - Resolve `(Request<()>, RequestStream)` from h3.
   - Wrap `RequestStream`'s recv half in `H3RequestBody`.
   - Build `Request<H3RequestBody>` from the resolved
     headers + body wrapper.
   - Call `service.call(req)` → `Response<RespBody>`.
   - `RequestStream::send_response(headers-only Response)`.
   - Poll `RespBody::poll_frame` in a loop: data frames →
     `send_data(bytes)`, trailers frame → `send_trailers(map)`.
   - `finish()`.
   - All async; backpressure flows naturally.

3. **`axum_compat::router_into_h3_service` rewrite**: drop the
   16 MiB buffer cap. The adapter:
   - Receives an `axum::Router`.
   - Returns a service that maps `Request<H3RequestBody>` to
     `Request<axum::body::Body>` (wrapping the H3RequestBody
     in axum's Body), calls the Router, and returns the
     Router's `Response<axum::body::Body>` directly — axum's
     Body is already `http_body::Body<Data = Bytes>` so it
     drops straight through to mhs's response-streaming.
   - No `DEFAULT_H3_RESPONSE_BODY_LIMIT_BYTES`. Delete the
     constant. Delete `router_into_h3_service_with_response_limit`
     (or have it forward to `router_into_h3_service` and
     ignore its limit arg, with a deprecation comment).

4. **mhs test extension**: the existing end-to-end h3 test
   (probably in `mechanics-http-server/src/tests.rs` or similar
   internal location) should grow assertions for:
   - Large request body round-trip (e.g. send 8 MiB random
     bytes, server echoes them, client verifies bytes-for-bytes
     match).
   - Large response body stream (e.g. server returns 8 MiB
     of incrementing data; client reads it chunked).
   - Body framing — number of `recv_data` calls / `send_data`
     calls roughly matches expectation (don't assert exact
     counts; assert "more than one chunk" to confirm streaming
     happened).

   8 MiB is enough to demonstrate streaming without taxing
   the test runner.

5. **mhs CHANGELOG entry under 0.1.3**: document the breaking
   change clearly.

6. **`mechanics-http-server` version bump 0.1.2 → 0.1.3**.

7. **Workspace verification**:
   - Per-crate `cargo check -p mechanics-http-server
     --all-targets`: PASS.
   - `cargo check -p mechanics --all-targets`: PASS (mechanics
     consumes mhs).
   - `cargo check -p philharmonic-api-server --all-targets`:
     PASS (one of the three bins).
   - `./scripts/pre-landing.sh`: PASS.
   - `cargo deny check bans`: PASS.

<action_safety>
- **Codex does NOT commit.** No git-write. Leave dirty tree
  for Claude.
- **Codex does NOT publish.**
- **Codex CAN run `pre-landing.sh`** at the end.
- Every cargo via `CARGO_TARGET_DIR=target-main`.
- POSIX-ish host.
- Run `./scripts/xtask.sh calendar-jp` at session start and
  before returning. If JST outside 10:00-19:00 (ext 21:00),
  note in the reply.
- TLS posture: rustls + aws-lc-rs + webpki-roots. Don't
  introduce ring / native-tls / platform-verifier /
  native-certs.
</action_safety>

<missing_context_gating>
Before starting, run `./scripts/status.sh` — the parent
should be clean (round 01 was already committed). If the
workspace has unrelated dirty work, STOP and report.

Read h3 0.0.8's actual `RequestStream` API in
`~/.cargo/registry/src/.../h3-0.0.8/src/server/stream.rs`.
The prompt sketches the API shape from a quick read; if the
actual API differs (e.g. `split()` doesn't exist; `recv_data`
returns a different shape), the upstream code wins.

If `http_body::Body` integration runs into a non-trivial
upstream-API limitation that prevents clean streaming, stop,
document, and report INCOMPLETE. Don't paper over with a
half-stream / partial-buffer hybrid — that's the round 01
shape we're explicitly fixing.
</missing_context_gating>

<default_follow_through_policy>
Land the streaming mhs Http3Server shape + the axum_compat
rewrite + extended tests + version bump + CHANGELOG in this
single round. Don't stop after the type changes and report
"tests pending" — the test is what proves streaming works.
</default_follow_through_policy>

<completeness_contract>
"Complete" means all of:

1. `Http3Server::start` accepts a streaming service shape via
   `http_body::Body`.
2. `H3RequestBody` (or equivalent) public type implements
   `http_body::Body`.
3. `axum_compat::router_into_h3_service` has no buffering;
   the 16 MiB cap is gone.
4. mhs end-to-end test asserts large bidirectional bodies
   stream correctly.
5. mhs CHANGELOG documents the breaking change under 0.1.3.
6. `mechanics-http-server 0.1.2 → 0.1.3`.
7. `pre-landing.sh` clean.
8. `cargo deny check bans` clean.
9. Banned-dep tree-invert checks clean.
10. `## Outcome` of this `-02.md` updated.

If any of 1–8 incomplete, report INCOMPLETE clearly.
</completeness_contract>

<verification_loop>
After each crate's edits:
  CARGO_TARGET_DIR=target-main cargo check -p <crate> --all-targets

At end:
  ./scripts/pre-landing.sh
  CARGO_TARGET_DIR=target-main cargo deny check bans
  CARGO_TARGET_DIR=target-main cargo tree --workspace --invert ring --target x86_64-unknown-linux-gnu

Don't run raw fmt/clippy/test mid-flight — pre-landing.sh
covers them at the end. Per-crate `cargo check` is the only
mid-flight cargo invocation.

If pre-landing fails:
1. Read the failure carefully.
2. Fix root cause; don't `#[allow]` over real lints.
3. Re-run.
</verification_loop>

<structured_output_contract>
At end of round 02, return:

1. **Summary** (2–3 sentences): the new mhs streaming shape;
   was the H3RequestBody named differently; was anything
   removed.
2. **Touched files**: grouped by crate.
3. **Public API changes** in mhs: full before/after of the
   `Http3Server::start` signature; new public types; removed
   constants/helpers.
4. **Test coverage added**: file paths + test names + body
   sizes used.
5. **Verification results**: per-crate cargo check PASS list,
   `pre-landing.sh` PASS / FAIL, `cargo deny check bans` PASS
   / FAIL, banned-dep tree inverts PASS / FAIL.
6. **Residual risks**: anything left as a TODO (e.g. trailers
   support not added, h3 API quirks worked around).
7. **Git state**: `./scripts/status.sh` output. NO commits.
8. **Outcome paragraph** for the prompt file's `## Outcome`.
</structured_output_contract>
</task>
