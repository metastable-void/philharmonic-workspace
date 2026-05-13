# D22 server — `mechanics-http-server` library (initial dispatch)

**Date:** 2026-05-13
**Slug:** `d22-mechanics-http-server-lib`
**Round:** 01 (initial dispatch — D22 server library half,
ROADMAP §3.I, single crate `mechanics-http-server`)
**Subagent:** `codex:codex-rescue`

## Motivation

D22 client (`c7efa37`) landed earlier this turn — mhc 0.2.0
adds opportunistic HTTP/3 discovery (HTTPS DNS RR + Alt-Svc)
and QUIC client. Server side is the natural counterpart: the
workspace's three release bins (mechanics-worker via
`mechanics`, philharmonic-api-server via `philharmonic-api`,
philharmonic-connector-bin via `philharmonic-connector-service`)
need to be able to *serve* HTTP/3 alongside the existing
HTTP/1.1 + HTTP/2 over TCP+TLS path, and emit `Alt-Svc` so
HTTPS-RR-blind clients can still upgrade after their first
HTTP/2 exchange.

`mechanics-http-server` is the shared library that provides
the server-side HTTP/3 building blocks. This dispatch ships
the library at 0.0.0 → 0.1.0 substantive content. **Consumer
integration into the three serving library crates** —
`mechanics`, `philharmonic-api`, `philharmonic-connector-service`
— **and the three bins' config plumbing** is OUT OF SCOPE
for this dispatch; that's a follow-up Codex round once this
library lands clean.

The reserved 0.0.0 placeholder (parent `b427779`, submodule
v0.0.0 on crates.io) is on the workspace's
`[patch.crates-io]` already. Local dev resolves to the
checked-out path; publication of 0.1.0 to crates.io will
happen post-landing, following the D20 / D22-client publish
pattern.

## References

- [`docs/ROADMAP.md` §3.I (D22)](../ROADMAP.md#i-http3-client--server-1-dispatch-future-session)
  — authoritative scope, discovery-priority order, hard
  constraints (HTTP/1.1 invariants, TLS-side ALPN
  invariant). **If anything below contradicts §3.I, the
  ROADMAP wins.**
- [`HUMANS.md` §"HTTP/3 support notes"](../../HUMANS.md) (read-only
  for agents; this is Yuka's note-to-self). Two locked-in
  constraints from there:
  1. **Activation knob**: "HTTP/3 is enabled for a server
     whenever it is configured with the HTTP/3 UDP bind port
     (the convention would be the top-level `bind_h3:
     Option<SockAddr>`)." The consumer integration will plumb
     this into each bin's config schema; this library's job
     is to **accept a `SocketAddr` and run** when given one.
  2. **State persistence boundary**: "Forgetting HTTP/3
     support statuses across statelessness boundaries is
     fine, but static LazyLock/Mutex states can be kept by
     the lib crate." Process-static state is fine for
     per-server-instance bookkeeping (endpoint, accept-loop
     task handle).
- [D22 client archived prompt](2026-05-13-0001-d22-mechanics-http-client-http3-client-01.md)
  — the client-side counterpart. Useful context for the
  matching TLS-posture, the aws-lc-rs + webpki-roots
  decision, and the dep-version choices we already made
  (quinn 0.11.9, h3 0.0.8, h3-quinn 0.0.10,
  hickory-resolver 0.25.2). **Match these exact versions
  in mhs** so the workspace pulls one copy of each.
- [`mechanics/Cargo.toml`](../../mechanics/Cargo.toml),
  [`philharmonic-api/Cargo.toml`](../../philharmonic-api/Cargo.toml),
  [`philharmonic-connector-service/Cargo.toml`](../../philharmonic-connector-service/Cargo.toml)
  — read-only for this dispatch. The consumer integration
  dispatch will modify these. They tell you what the
  existing TCP+TLS server setup looks like (axum, hyper-
  rustls or similar), so mhs's API can fit naturally.

## Context files pointed at

- [`mechanics-http-server/Cargo.toml`](../../mechanics-http-server/Cargo.toml)
  — currently the 0.0.0 placeholder. Version bump + deps.
- [`mechanics-http-server/src/lib.rs`](../../mechanics-http-server/src/lib.rs)
  — currently the placeholder docstring. Replace with the
  real crate-level rustdoc + module declarations.
- New module files Codex creates under `mechanics-http-server/src/`.
- [`mechanics-http-server/CHANGELOG.md`](../../mechanics-http-server/CHANGELOG.md)
  — does not exist yet. Codex creates it with a `[0.1.0]`
  entry.
- [`mechanics-http-client/src/`](../../mechanics-http-client/src/)
  for reference — the client uses quinn + h3 too; symmetry
  in how the QUIC config is built (aws-lc-rs provider, ALPN
  list shape) is important.
- The bins' existing HTTPS startup code: read-only context.
  `mechanics/src/server.rs` (or similar — locate via
  `git grep`) for how axum + rustls is set up today.

## Outcome

**Pending — will be updated after the Codex run.**

---

## Shape (locked decisions)

These follow ROADMAP §3.I + HUMANS.md + the D22-client
matching constraints. Don't relitigate the design.

### Public surface (locked)

A small, focused API consumed by `mechanics`,
`philharmonic-api`, and `philharmonic-connector-service`:

```rust
// mechanics-http-server/src/lib.rs (sketch)

/// Optional HTTP/3 listener that runs alongside an existing
/// TCP+TLS server (hyper-rustls + h2 + http/1.1).
pub struct Http3Server { /* ... */ }

/// Configuration knobs for the HTTP/3 listener.
#[derive(Clone, Debug)]
pub struct Http3ServerConfig {
    /// UDP bind address. The `Option<SocketAddr>` matches
    /// HUMANS.md's "HTTP/3 is enabled for a server whenever it
    /// is configured with the HTTP/3 UDP bind port" — when
    /// `None`, `Http3Server::start` is a no-op and returns
    /// without opening anything.
    pub bind_h3: Option<SocketAddr>,
    /// Alt-Svc `ma` value (max-age, seconds). Default 86400 (24h).
    pub alt_svc_max_age_secs: u64,
    /// 0-RTT replay safety policy: methods allowed to use 0-RTT.
    /// Default `[Method::GET, Method::HEAD]` (idempotent methods
    /// only).
    pub zero_rtt_idempotent_methods: Vec<http::Method>,
}

impl Http3Server {
    /// Build but don't start.
    pub fn new(config: Http3ServerConfig) -> Self;

    /// Start the UDP listener and route incoming h3 requests
    /// into the supplied tower::Service (typically an axum
    /// Router). Returns a handle that can be awaited for the
    /// accept-loop's exit, or dropped to trigger shutdown.
    ///
    /// If `bind_h3.is_none()`, returns an inert handle that
    /// completes immediately on first poll — no UDP socket
    /// opened, no resources held.
    ///
    /// The `tls_cert_chain` and `tls_private_key` are the
    /// operator-supplied server cert + key. mhs builds its
    /// own `rustls::ServerConfig` for the QUIC side; the
    /// caller's `rustls::ServerConfig` for the TCP+TLS side
    /// is NOT shared (different ALPN lists — see invariants).
    pub fn start<S>(
        self,
        service: S,
        tls_cert_chain: Vec<rustls::pki_types::CertificateDer<'static>>,
        tls_private_key: rustls::pki_types::PrivateKeyDer<'static>,
    ) -> Result<Http3Handle, Error>
    where
        S: tower::Service<http::Request<...>, Response = http::Response<...>>
            + Clone + Send + 'static,
        // ...trait bounds Codex's choice based on what axum's
        // Router actually implements + what h3's request loop
        // needs.
    ;
}

/// Handle returned by `Http3Server::start`. Implements
/// `Future<Output = Result<(), Error>>` for the accept-loop's
/// lifetime. Drop to trigger graceful shutdown.
pub struct Http3Handle { /* ... */ }

/// Tower layer that adds `Alt-Svc: h3=":<port>"; ma=<max-age>"`
/// to every response. Apply to the TCP+TLS path's axum Router
/// when h3 is enabled, so HTTPS-RR-blind clients can upgrade
/// after their first HTTP/2 exchange.
pub fn alt_svc_layer(h3_port: u16, max_age_secs: u64) -> AltSvcLayer;

pub struct AltSvcLayer { /* ... */ }

/// Structured error model.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid TLS material: {0}")]
    InvalidTls(String),
    #[error("QUIC endpoint bind failed: {0}")]
    BindFailed(String),
    #[error("internal: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

Exact trait bounds, return types, and the `Http3Handle`
shape are Codex's call within the locked design. Document
the choices in residual risks.

### TLS posture (load-bearing, mirrors mhc)

- **Crypto provider**: `aws-lc-rs`. No `ring`.
- **rustls version**: `0.23` with `default-features = false,
  features = ["std", "tls12", "aws_lc_rs"]` — matching mhc.
- **QUIC stack**: `quinn = "0.11.9"` with
  `default-features = false, features = ["runtime-tokio",
  "rustls-aws-lc-rs"]`. Same exact version as mhc.
- **h3 stack**: `h3 = "0.0.8"`, `h3-quinn = "0.0.10"`. Same
  exact versions as mhc.
- **QUIC-side ALPN list**: `[b"h3"]` only.
- **Server cert/key**: operator-supplied (passed in at
  `Http3Server::start`). mhs does NOT manage cert
  acquisition / rotation; it just configures rustls to use
  what the caller gave it.
- **No webpki-roots** in mhs's deps. That's a CLIENT trust
  store; the server doesn't need it.

### Invariants (hard constraints)

- **TLS-side ALPN list is NOT mhs's concern.** The TCP+TLS
  listener lives in the consumer crate (e.g. `mechanics`'s
  existing HTTPS setup); its ALPN list stays `[h2, http/1.1]`
  as today. mhs handles the QUIC side ONLY. **`b"h3"` MUST NOT
  appear in any rustls `ServerConfig` mhs builds for TCP+TLS**
  (mhs shouldn't be building TCP+TLS configs in the first
  place — flag if you find yourself doing so).
- **Cleartext HTTP/1.1 still works.** When a consumer bin is
  built without its `https` feature, the bin's existing
  cleartext HTTP/1.1 listener stays untouched. mhs is
  inert in that case — operator's `bind_h3 = None` means
  `Http3Server::start` opens nothing.
- **HTTP/1.1 over TLS still works.** Same logic: mhs doesn't
  touch the TCP+TLS listener. The existing ALPN-negotiation-
  to-`http/1.1` path is preserved by construction.
- **HTTP/2 over TLS still works.** Same logic.
- **0-RTT replay safety.** Only allow 0-RTT on idempotent
  methods (default `GET` and `HEAD`). The default policy
  refuses 0-RTT for everything else. Future per-route opt-in
  is a config knob; v1 ships the conservative default only.
- **No `unsafe`** beyond what quinn / h3 require internally.
- **No panics in lib `src/`.** Tests exempt.
- **Library boundary stays clean.** No file I/O, no env-var
  lookup, no config-file parsing. TLS materials are passed
  in as bytes (`rustls::pki_types::CertificateDer` +
  `rustls::pki_types::PrivateKeyDer`).
- **Mechanics-Philharmonic independence.** mhs MUST NOT
  depend on any `philharmonic-*` crate.

### Process-static state (HUMANS.md hint)

Yuka's HUMANS.md note says: *"Forgetting HTTP/3 support
statuses across statelessness boundaries is fine, but static
LazyLock/Mutex states can be kept by the lib crate."*

Interpretation for the server side: process-static state for
per-Server-instance bookkeeping (one `Http3Server` per bin
process is typical) is permitted. A `static LazyLock<Mutex<...>>`
holding the active endpoint reference is fine if Codex
finds a use for it. **Do not** make `Http3Server` itself a
singleton — the public type stays instantiable; the static
is implementation-side state if needed.

If the implementation doesn't need any static state (each
`Http3Server` carries its own `quinn::Endpoint` field), skip
this — Codex's call. Document the decision.

### Version bump + CHANGELOG

`0.0.0 → 0.1.0`. The 0.0.0 was a name reservation; 0.1.0 is
the substantive first release. Create
`mechanics-http-server/CHANGELOG.md` with the standard
header + a `[0.1.0] - 2026-05-13` entry naming D22 server
library and pointing at ROADMAP §3.I.

## Tests

The library's test surface is constrained by the difficulty
of standing up real h3 fixtures inside a small crate (same
problem D22 client hit; Codex chose state-machine
substitutes there). Required tests:

1. **`Http3Server::start` with `bind_h3 = None` is inert.**
   Construct an `Http3ServerConfig` with `bind_h3: None`,
   call `start(...)` with a dummy axum Router + dummy
   cert/key, assert: no UDP socket opened (verify via
   listing the process's sockets, or just by inspecting
   that the returned handle is the inert variant), no
   QUIC endpoint constructed, returned handle completes
   immediately when polled.

2. **`Http3Server::start` with `bind_h3 = Some(addr)` opens
   the listener.** Use a random ephemeral port (`127.0.0.1:0`).
   Construct with a self-signed cert (the workspace doesn't
   have a test-cert generator yet — use `rcgen` as a
   `[dev-dependencies]` add). Start the server, assert: a
   UDP socket is bound at the reported address, the
   accept-loop task is alive. Tear down by dropping the
   handle.

3. **End-to-end h3 request via mhc** (`#[ignore]`-able if
   the cross-crate dev-dep is awkward). With the server
   from test 2 running, use `mechanics-http-client` (added
   as a dev-dep with `default-features = false, features =
   ["http3"]` — wait, mhc's `http3` IS the default, so just
   path-dep on it) configured to inject an HTTPS RR cache
   entry pointing at the test server's port and announcing
   `h3` in `alpn`, send a `GET /test` request, assert:
   the request flows over HTTP/3, server's axum Router
   sees it, response body matches. If this proves too
   gnarly (cross-crate test setup + self-signed cert in
   the client), `#[ignore]` with a clear TODO referencing
   "D22 server round 02 fixture work."

4. **Alt-Svc layer adds the header.** Unit test for
   `alt_svc_layer(443, 86400)` wrapped around a trivial
   tower service. Send a request, assert: the response has
   `Alt-Svc: h3=":443"; ma=86400`. Exercise multiple
   responses to confirm the layer isn't one-shot.

5. **Alt-Svc layer respects custom port + max-age.** Same
   shape as test 4 but with a non-default port (e.g.
   8443) and max-age (e.g. 3600). Assert: the header
   value reflects the configured port and ma.

6. **0-RTT policy default rejects non-idempotent methods.**
   Unit test for whatever predicate / lookup function mhs
   uses internally to decide if 0-RTT is allowed. Assert:
   `Method::GET` → allowed; `Method::HEAD` → allowed;
   `Method::POST` → rejected; `Method::PUT` → rejected;
   `Method::PATCH` → rejected; `Method::DELETE` → rejected;
   `Method::OPTIONS` → rejected (operator can extend the
   list via config but the default excludes it).

7. **0-RTT policy honours config override.** Custom
   `zero_rtt_idempotent_methods = vec![GET, HEAD, OPTIONS]`,
   assert: OPTIONS is now allowed.

Tests 1-2 + 4-7 should be straightforward. Test 3 is the
real end-to-end and is the most valuable; do not over-skip
it. If you `#[ignore]` it, leave a state-machine substitute
that at least verifies the request-routing-into-axum path
in isolation.

## Verification flow

```sh
./scripts/pre-landing.sh
```

Runs cargo-deny bans + fmt + check + clippy (-D warnings) +
rustdoc + test. With D21's dep-aware narrowing — mhs is
dirty, no consumer pulls it yet (consumer integration is the
next dispatch), so the test phase will narrow to mhs alone
in this round.

```sh
cargo tree -p mechanics-http-server -e normal \
  | grep -E "reqwest|rustls-platform-verifier|rustls-native-certs|ring v|webpki-roots"
```

Should print nothing. mhs is server-side — webpki-roots is
specifically a client-side trust store and MUST NOT be in
the server crate's tree. The other forbidden deps apply
workspace-wide.

```sh
./scripts/check-api-breakage.sh mechanics-http-server 0.0.0
```

Likely will fail similarly to the mhc 0.2.0 check (the
0.0.0 placeholder was just published ~minutes ago, well
within the workspace's 3-day menhera-cooldown threshold).
Surface the result in residuals; the 0.0.0 → 0.1.0 jump
is unambiguously additive (placeholder → first substantive
release), no breaking surface to check against.

Skip:

- No publish — Claude reviews and decides post-Codex.
- No edits outside `mechanics-http-server/`. If you find
  yourself wanting to touch `mechanics`, `philharmonic-api`,
  `philharmonic-connector-service`, any of the bins, or any
  other workspace crate, **stop** and surface in residuals.
  Those edits are the **next** Codex dispatch's scope.

## Prompt (verbatim)

<task>
Ship D22 server library: take `mechanics-http-server` from its
0.0.0 name-reservation placeholder to a substantive 0.1.0
release. The library provides an opportunistic HTTP/3 (QUIC)
listener with `Alt-Svc` middleware, designed to run alongside
an existing TCP+TLS HTTP/1.1+HTTP/2 server inside the
workspace's three release bins.

**Consumer integration into `mechanics`, `philharmonic-api`,
and `philharmonic-connector-service`, plus the three bins'
config plumbing, is OUT OF SCOPE.** That's a follow-up
Codex dispatch. This round is purely inside
`mechanics-http-server/`.

Single crate. No edits outside `mechanics-http-server/`
except `Cargo.lock` (regenerates).

Deliverables (in order):

1. **Cargo.toml**: bump 0.0.0 → 0.1.0. Add deps:
   `quinn = "0.11.9"` (`default-features = false`, features
   `["runtime-tokio", "rustls-aws-lc-rs"]`),
   `h3 = "0.0.8"`,
   `h3-quinn = "0.0.10"`,
   `rustls = "0.23"` (`default-features = false`, features
   `["std", "tls12", "aws_lc_rs"]`),
   `tokio = "1"` (features `["rt", "time", "net", "sync"]`),
   `http = "1"`,
   `tower = "0.5"` (with feature `["util"]` if Codex needs
   ServiceBuilder helpers),
   `tower-layer = "0.3"` (if not transitively available),
   `tracing = "0.1"`,
   `thiserror = "2"`.
   Dev-deps: `tokio = "1"` with full features,
   `rcgen = "0.13"` (self-signed cert generation for tests),
   `mechanics-http-client = { path = "../mechanics-http-client" }`
   for the end-to-end test (Path 3 above — only if you keep
   it un-`#[ignore]`d).
   Match the exact mhc deps versions so the workspace pulls
   one copy of each.
   **No `webpki-roots`** in mhs.

2. **CHANGELOG.md** (new file). Standard header (Keep a
   Changelog + SemVer). `[0.1.0] - 2026-05-13` entry: name
   the new crate's purpose, ROADMAP D22 server library
   reference.

3. **`src/lib.rs`** (replace placeholder): real crate-level
   rustdoc explaining the role (opt-in HTTP/3 listener +
   Alt-Svc middleware), the activation model (`bind_h3:
   Option<SocketAddr>` config, inert when `None`), the TLS
   posture (aws-lc-rs + caller-supplied cert/key, NO
   webpki-roots, NO ring), HTTP/1.1+HTTP/2 are NOT this
   crate's concern (they live in the consumer bin's
   existing TCP+TLS server). Module declarations. Public
   re-exports.

4. **`src/server.rs`** (new): `Http3Server`,
   `Http3ServerConfig`, `Http3Handle`. The
   `Http3Server::start` method:
   - Validates the config + TLS material.
   - When `bind_h3 = None`: returns an inert handle that
     completes immediately on poll. No UDP socket, no
     `quinn::Endpoint`.
   - When `bind_h3 = Some(addr)`: builds a
     `rustls::ServerConfig` with `aws_lc_rs::default_provider()`,
     wraps the operator-supplied cert+key, sets the
     QUIC-side ALPN to `[b"h3"]`. Wraps that into
     `quinn::crypto::rustls::QuicServerConfig::try_from(...)`,
     binds a `quinn::Endpoint::server(...)` on the
     supplied addr. Spawns a tokio task running the
     accept-loop: for each incoming `quinn::Connecting`,
     spawn a per-connection task that runs
     `h3::server::Connection::new(h3_quinn::Connection)`,
     accepts h3 requests, routes them through the
     supplied tower::Service. Returns the
     `Http3Handle` referencing that task.
   - `Http3Handle` impls `Future<Output = Result<(), Error>>`
     and triggers graceful shutdown when dropped (cancel
     the accept-loop, drain in-flight connections).

5. **`src/alt_svc.rs`** (new): `AltSvcLayer` + the
   `alt_svc_layer(h3_port, max_age_secs)` factory.
   Implements `tower::Layer`. Wraps a service; on every
   response, inserts (or appends — your call, document)
   the `Alt-Svc: h3=":<port>"; ma=<max-age>"` header.
   Unit-test that the layer is idempotent (applying twice
   shouldn't duplicate the header — or if it does, document
   why).

6. **`src/zero_rtt.rs`** (new — or fold into `server.rs`):
   the 0-RTT replay-safety policy. A small predicate
   `is_zero_rtt_safe(method: &http::Method, allowed:
   &[http::Method]) -> bool`. Hooked into the per-
   connection accept path so that 0-RTT data is only
   replayed for methods in the allowed list.

7. **`src/error.rs`** (new): `Error` enum + `Result<T>`
   alias. Variants: `InvalidTls(String)`, `BindFailed
   (String)`, `Internal(String)`. `#[non_exhaustive]`.
   Implement `Debug` + `thiserror::Error`.

8. **Tests** (in `src/tests.rs` or `tests/` integration —
   your call). Cover the 7 tests listed in the prompt's
   "Tests" section. Test 3 (end-to-end via mhc) may be
   `#[ignore]`'d if cross-crate test setup proves
   intractable; if so, leave a state-machine substitute
   verifying request-routing-into-axum in isolation, and
   surface in residuals.

9. **Verification.** Run `./scripts/pre-landing.sh`. Run
   the `cargo tree` grep (must be empty including for
   `webpki-roots`). Try `./scripts/check-api-breakage.sh
   mechanics-http-server 0.0.0` — likely fails on the
   3-day cooldown threshold, surface result in residuals.

10. **No publish.** Claude reviews and decides post-Codex.

## Hard constraints

- **HTTP/1.1, HTTP/2 over TCP+TLS, and cleartext HTTP/1.1
  all keep working.** mhs is purely additive — it provides
  ONLY the QUIC-side server surface. The consumer bin's
  existing TCP+TLS listener (with `[h2, http/1.1]` ALPN) is
  not mhs's concern; this dispatch must not change anything
  about it.
- **`bind_h3 = None` is fully inert.** No UDP socket, no
  endpoint, no resources held. The handle completes
  immediately. This is the load-bearing "operators who
  don't want h3 server-side don't pay for it" guarantee
  matching HUMANS.md's activation model.
- **0-RTT replay safety on by default.** Only idempotent
  methods (`GET`, `HEAD`) accept 0-RTT data. Future
  per-route opt-in is config surface for v2; v1 ships
  conservative defaults.
- **TLS posture.** aws-lc-rs crypto, caller-supplied
  server cert + key, no webpki-roots in mhs's deps, no
  ring anywhere in the runtime tree. Verify via
  `cargo tree` grep.
- **No `unsafe`** beyond what quinn / h3 / rustls require
  internally.
- **No panics in lib `src/`.** No `.unwrap()` / `.expect()`
  on `Result`/`Option`, no `panic!` / `unreachable!` /
  `todo!` / `unimplemented!` on reachable paths. Tests
  exempt.
- **Library boundary stays clean.** TLS material is passed
  in as `rustls::pki_types::CertificateDer` +
  `rustls::pki_types::PrivateKeyDer`. No file I/O for
  cert/key loading inside mhs; that's the bin's job.
- **No `philharmonic-*` dep.** Mechanics-family
  independence stays.
- **QUIC-side ALPN is `[b"h3"]` only.** Not `["h3", "h2"]`
  (h2 over QUIC isn't a thing); not multi-version (the
  draft `h3-29` etc. are out of scope).

<structured_output_contract>
**Critical: emit this report before `task_complete`.**

Six sections, in this order:

1. **Summary** — one paragraph: what landed, structural
   choices (whether test 3 stayed live or was `#[ignore]`'d,
   whether Codex used `static LazyLock<Mutex<...>>` state
   anywhere, the `Http3Handle` future shape), version
   applied, semver-checks outcome (likely cooldown-failed),
   `cargo tree` grep results.
   Include the verbatim string `RUN STATUS: COMPLETE` or
   `RUN STATUS: PARTIAL — <reason>` for grep.

2. **Touched files** — exhaustive list with
   `(new|edited|deleted) <path> — <one-line note>`.

3. **Verification results** — exact commands + outcomes:
   - `./scripts/pre-landing.sh` — pass/fail/exit code.
   - `./scripts/check-api-breakage.sh mechanics-http-server
     0.0.0` — pass/fail/excerpt (cooldown failure is OK,
     report the message verbatim).
   - `cargo tree -p mechanics-http-server -e normal |
     grep -E "reqwest|rustls-platform-verifier|rustls-
     native-certs|ring v|webpki-roots"` — must be empty;
     report the exact command + (empty) output.

4. **Residual risks / known issues** — including:
   - Test 3 (end-to-end via mhc) status: live or
     `#[ignore]`'d, and why.
   - Whether `static LazyLock<...>` state was used and what
     it holds.
   - Alt-Svc layer: insert vs. append semantics for the
     header; idempotency under repeated layer-stacking.
   - `Http3Handle` future shape: how graceful shutdown is
     wired (cancel-on-drop vs. explicit `.shutdown()`).
   - rustls 0.23 + aws-lc-rs `ServerConfig` builder pattern
     quirks: pki_types / sign provider details that don't
     fit cleanly.
   - Any rcgen API quirks for the self-signed test cert.

5. **Git state** — current `HEAD` SHA in the parent
   workspace repo and in the `mechanics-http-server`
   submodule. Confirm no commits made.

6. **Open questions** — questions for Yuka or Claude:
   - Should `Alt-Svc` semantics be insert-only, append-only,
     or insert-or-replace?
   - Should `Http3Handle` impl `IntoFuture` instead of
     `Future` directly (or both)?
   - Should the 0-RTT method list be `Vec<Method>` or a
     `BTreeSet<Method>` to canonicalise duplicates?
   - Anything you punted on as out-of-scope but worth a
     follow-up.
</structured_output_contract>

<default_follow_through_policy>
- Implement in the order listed in Deliverables.
- Run `cargo test -p mechanics-http-server` directly for
  fast iteration before invoking pre-landing.
- Match the mhc dep versions exactly (quinn 0.11.9, h3
  0.0.8, h3-quinn 0.0.10, rustls 0.23). The workspace
  pulls one copy of each that way.
- The `Http3Server::start` method's trait bounds on the
  service `S` will end up looking complex (axum's `Router`
  is generic over body types). Lift bounds from
  `hyper-util`'s service-fn pattern or axum's
  `serve_with_graceful_shutdown` for inspiration; document
  the final bound shape in residuals.
- For the per-connection h3 loop, the canonical shape is:
  `let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(quic_conn)).await?;
   while let Some(resolver) = h3_conn.accept().await? { ... }`
  — accept request, spawn a per-stream task that calls the
  tower::Service and writes the response back via the h3
  stream.
- 0-RTT enforcement happens INSIDE the per-stream task —
  inspect the request's method, check against the policy,
  on rejection refuse to serve from 0-RTT data (h3 / quinn
  provide hooks for distinguishing 0-RTT data).
- If `cargo build` looks stuck for minutes, run
  `./scripts/build-status.sh` rather than declaring a hang.
  The QUIC stack adds significant first-build time.
</default_follow_through_policy>

<completeness_contract>
"Done" means:

- Cargo.toml bumped 0.0.0 → 0.1.0 with the listed deps.
- CHANGELOG.md created with the `[0.1.0]` entry.
- `Http3Server` + `Http3ServerConfig` + `Http3Handle`
  implemented per the locked surface.
- `AltSvcLayer` + factory function implemented.
- 0-RTT replay-safety policy implemented with the default
  GET/HEAD allow-list + config override.
- `Error` enum + `Result` alias.
- 7 tests pass (or test 3 is `#[ignore]`'d with a state-
  machine substitute and that's documented).
- `./scripts/pre-landing.sh` clean.
- `cargo tree -p mechanics-http-server | grep ring|reqwest|
  platform-verifier|native-certs|webpki-roots` empty.
- Six-section structured-output report emitted before
  `task_complete`.

Partial completion is acceptable only if you hit a token
limit or a genuine blocker — say so explicitly with
`RUN STATUS: PARTIAL — <reason>`. A half-shipped state where
`Http3Server::start` exists but doesn't actually accept
connections is worse than no library; if you can't finish
the accept loop, leave it stubbed with a clear `todo!()`-
adjacent `Error::Internal("not yet implemented")` return
and document loudly.

A run without the structured-output report is
**incomplete**, even if the code landed.
</completeness_contract>

<verification_loop>
1. Implement Cargo.toml + lib.rs skeleton + error.rs.
2. Implement server.rs (the core surface).
3. Implement alt_svc.rs + zero_rtt.rs.
4. Implement tests.
5. `cargo test -p mechanics-http-server` — green.
6. `CARGO_TARGET_DIR=target-main cargo check --workspace`
   — catches any downstream coupling. No other crate pulls
   mhs yet, so this is mostly a sanity check.
7. `cargo tree -p mechanics-http-server | grep -E "..."`
   empty.
8. Run `./scripts/pre-landing.sh` once.
9. Run `./scripts/check-api-breakage.sh
   mechanics-http-server 0.0.0` — likely cooldown-fails;
   capture verbatim.
10. Emit structured-output report.
11. `task_complete`.
</verification_loop>

<missing_context_gating>
If you need information not in this prompt or the cited
authoritative sources (ROADMAP §3.I, HUMANS.md HTTP/3
notes, the D22 client archived prompt + landed code,
RFC 9114 HTTP/3, RFC 9000 QUIC, RFC 7838 Alt-Svc),
**stop** and report what's missing in the structured
output's "Open questions" section.

Specifically: do **not**:

- Touch any crate other than `mechanics-http-server`.
- Add `webpki-roots` to mhs (it's a client trust store).
- Implement a TCP+TLS listener inside mhs (that lives in
  the consumer bin, not here).
- Bundle a cert generator inside mhs (operators supply
  certs; mhs takes bytes).
- Implement HTTP-spec-side request/response decoding —
  use `http` crate types and let `h3` handle framing.
- Touch any `.claude/`, `docs/`, `docs-jp/`, `HUMANS.md`,
  `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md`, or
  `scripts/` content. (May read for reference; no edits.)
- Publish to crates.io. No `cargo publish` even `--dry-run`.
  Claude reviews and decides post-Codex.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `mechanics-http-server/Cargo.toml` (edited — version bump,
  deps).
- `mechanics-http-server/CHANGELOG.md` (new — `[0.1.0]`
  entry).
- `mechanics-http-server/src/lib.rs` (edited — replace
  placeholder, add module declarations + public re-exports).
- `mechanics-http-server/src/server.rs` (new — `Http3Server`,
  `Http3ServerConfig`, `Http3Handle`).
- `mechanics-http-server/src/alt_svc.rs` (new — Alt-Svc
  tower layer).
- `mechanics-http-server/src/zero_rtt.rs` (new — 0-RTT
  policy predicate; or fold into server.rs).
- `mechanics-http-server/src/error.rs` (new — Error +
  Result).
- `mechanics-http-server/src/tests.rs` (new — or
  `mechanics-http-server/tests/...` integration).
- `Cargo.lock` (regenerates).

Files NOT to touch (flag if you find a reason to):

- Any file under `mechanics/`, `mechanics-core/`,
  `mechanics-config/`, `mechanics-http-client/`,
  `philharmonic*/`, `inline-blob/`, any other workspace
  member.
- The workspace `Cargo.toml` `[patch.crates-io]` block.
- `.claude/`, `docs/`, `docs-jp/`, `HUMANS.md`,
  `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md`,
  `scripts/`, `deny.toml`.

Git rules: signed-off + signed commits via
`scripts/commit-all.sh` only. **Codex never runs
`commit-all.sh`** — Claude commits after reviewing.
No `cargo publish`. No raw `git commit` / `git push` /
`git add` / `git reset` / `git rebase` / `git revert`.
Read-only `git log` / `git diff` / `git show` is fine.
</action_safety>
</task>
