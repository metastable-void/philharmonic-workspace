# D22 server-integration — wire mechanics-http-server into the
# three release bins (round 01)

**Date:** 2026-05-14 (JST)
**Slug:** `d22-server-integration-h3`
**Round:** 01 — initial dispatch.
**Subagent:** `codex:codex-rescue`

## Motivation

D22 client (`mechanics-http-client 0.2.0`'s opportunistic HTTP/3)
and D22 server-lib (`mechanics-http-server 0.1.0`) both landed
2026-05-13. D22 server-integration is the remaining piece: wire
`mechanics-http-server`'s `Http3Server` + `AltSvcLayer` into the
three release bins behind a new `bind_h3: Option<SocketAddr>`
config field on each. After this dispatch, deploying any of the
three bins with `bind_h3 = "0.0.0.0:443"` (or similar) gives
clients opportunistic HTTP/3 transport via Alt-Svc advertised on
the TCP+TLS HTTP/2 path.

This is the next §3.G/§3.J-adjacent dispatch per Yuka's
2026-05-14 ordering directive: D22 server-integration first,
then the full D18 mechanics-core surgery (module redesign +
mechanics:* feature gating).

## References (read in this order)

1. `docs/ROADMAP.md` — current state preamble mentions D22
   server-integration as the next-natural dispatch after the
   §3.J production-security cleanup arc closed.
2. `docs/design/06-execution-substrate.md` (relevant only as
   reference; no JS-realm changes in this dispatch).
3. `CONTRIBUTING.md`:
   - **§3.1** `[profile.release]` (don't touch — bins inherit).
   - **§4** Git workflow.
   - **§5** Script wrappers + `CARGO_TARGET_DIR`.
   - **§6** POSIX shell.
   - **§10.3** No panics in library code; bins are exempt but
     should still surface errors cleanly via `eprintln!` /
     `Result`.
   - **§10.9** HTTP-client stack split (irrelevant for this
     server-side dispatch).
   - **§11** Pre-landing — **Codex CAN run `pre-landing.sh`
     after the 2026-05-14 zramswap bump.** /tmp can now hold a
     full workspace test cycle.
4. `mechanics-http-server/src/lib.rs`,
   `mechanics-http-server/src/server.rs`,
   `mechanics-http-server/src/alt_svc.rs` — the public API
   surface to integrate against.
5. Existing bin entry points:
   - `bins/mechanics-worker/src/main.rs` (`run` / `run_tls`
     via `mechanics::MechanicsPool::run_tls`)
   - `bins/philharmonic-api-server/src/main.rs`
     (`start_tls_server` + `axum::serve` paths around
     line 834-852)
   - `bins/philharmonic-connector/src/main.rs`
     (`start_tls_server` + `axum::serve` paths around
     line 483-491)
6. `mechanics/src/lib.rs` `run` / `run_tls` (around lines
   247 / 290) — `mechanics-worker` calls into these; the
   library likely needs an HTTP/3-aware variant.

## Current state of the three bins (snapshot)

- All three bins have a `bind: SocketAddr` config field +
  optional TLS variant (`[tls]` block).
- `mechanics-worker` uses `mechanics::MechanicsPool::run` /
  `run_tls`; the bin doesn't directly touch axum/hyper.
- `philharmonic-api-server` and `philharmonic-connector` both
  call `axum::serve(TcpListener, app)` for the plain HTTP path
  and `start_tls_server` (a bin-local helper using
  `tokio_rustls::TlsAcceptor` + `hyper::server::conn`) for the
  HTTPS path.

## Current state of `mechanics-http-server` (snapshot)

`mechanics-http-server 0.1.1` public surface (from
`src/lib.rs`):

```rust
pub use alt_svc::{AltSvcLayer, AltSvcService, alt_svc_layer};
pub use error::{Error, Result};
pub use server::{Http3Handle, Http3Server, Http3ServerConfig};
pub use zero_rtt::{default_zero_rtt_methods, is_zero_rtt_safe};
```

Key API shapes:

- `Http3ServerConfig` has `bind_h3: Option<SocketAddr>`
  (Codex can re-use this directly), `alt_svc_max_age_secs:
  u64`, and `zero_rtt_idempotent_methods`.
- `Http3Server::new(config) -> Http3Server`.
- `Http3Server::start(self, service: S, tls_cert_chain,
  tls_private_key) -> Result<Http3Handle>` where
  `S: Service<Request<()>, Response = Response<Bytes>> +
  Clone + Send + 'static`.
- `Http3Handle::is_inert()` / `local_addr()` / `shutdown()`,
  and `Http3Handle` impls `Future<Output = Result<()>>` so
  await-ing it waits for the accept loop.
- `AltSvcLayer` is a `tower::Layer` that wraps a service and
  injects the `alt-svc` header on responses.

**Service-shape mismatch:** mhs's `Http3Server::start` expects a
service with `Request<()>` body and `Response<Bytes>` body —
that's the HTTP/3 framing shape, where bodies are separately
streamed. axum's `Router` is
`Service<Request<axum::body::Body>, Response =
Response<axum::body::Body>>`. Codex will need either:
- An adapter shim that converts axum's Router into mhs's
  expected shape (read body into Bytes before invoking the
  Router, collect Router's response body to Bytes before
  returning), OR
- A helper exposed from `mechanics-http-server` (if it has
  one — check; if not, **add it**: e.g.
  `mechanics_http_server::axum::router_into_h3_service(router)
  -> impl Service<Request<()>, Response = Response<Bytes>>`),
  OR
- A direct streaming adapter using h3's body APIs.

Pick the cleanest option that doesn't buffer arbitrarily large
bodies in memory. If a streaming adapter is reasonable
(preferred), document the framing assumption inline.
**Adapter code, if substantial, should live in
`mechanics-http-server`** (not in each bin) so the three bins
share it; bump `mechanics-http-server` to `0.1.2` (still
unpublished per the no-publish-mid-work rule) when adding
public surface.

## Goal

For each of the three release bins:

1. Add a `bind_h3: Option<SocketAddr>` field to the bin's
   config struct (TOML).
2. Add a CLI arg mirror (`--bind-h3 <addr>`) that overrides
   the TOML value if both are present (existing pattern from
   `bind`).
3. When `bind_h3` is `Some(addr)` AND TLS is configured
   (HTTP/3 requires TLS): construct a
   `mechanics_http_server::Http3ServerConfig { bind_h3:
   Some(addr), alt_svc_max_age_secs: <reasonable default,
   e.g. 86400>, zero_rtt_idempotent_methods:
   default_zero_rtt_methods() }` and pass to
   `Http3Server::new(...).start(...)` alongside the same axum
   Router used by the TCP+TLS path.
4. When `bind_h3` is `Some(addr)` but TLS is NOT configured:
   error out at startup with a clear message ("HTTP/3 requires
   TLS; configure `[tls]` in the config file").
5. When `bind_h3` is `None`: behave exactly as today (no
   HTTP/3 listener; no Alt-Svc header).
6. When `bind_h3` is `Some(_)`: wrap the TLS-served axum Router
   with `AltSvcLayer` so HTTP/1.1 + HTTP/2 responses advertise
   the alternative QUIC endpoint via the `Alt-Svc` header.
7. Graceful shutdown: on SIGTERM / Ctrl-C, both the TCP+TLS
   axum listener AND the `Http3Handle` shut down cleanly. The
   existing bin has shutdown handling for the TCP path; the
   `Http3Handle` needs to be `await`-ed in `select!` (or
   equivalent) alongside the TCP `axum::serve` future, with
   `handle.shutdown()` called on the cancellation path.

For `mechanics-worker` specifically, the bin doesn't touch
axum directly — it calls `mechanics::MechanicsPool::run_tls`.
So `mechanics::MechanicsPool` likely needs a new method or an
extended `run_tls` to accept an optional
`Http3ServerConfig`. Naming options:
- `MechanicsPool::run_tls_with_h3(bind, tls_config, h3_config)`
- Extending `TlsConfig` to carry an `Option<H3Config>` (struct
  embed; cleaner)
- A new `MechanicsRunConfig` struct that subsumes
  `(bind, tls_config, h3_config)` (most flexible; recommended
  if existing run/run_tls callers can migrate cleanly)

Codex picks the option that integrates cleanly without
breaking other consumers (mechanics is at 0.5.1, unpublished;
breaking-API changes acceptable for in-workspace consumers,
but the public surface of mechanics should remain coherent
for downstream crates.io consumers).

## Per-crate version-bump policy

This dispatch lands code, not a fresh release. Per Yuka's
2026-05-14 "no publish crates mid-work unless necessary; bumps
not always strictly necessary" rule:

- **`mechanics-http-server`**: if Codex adds public surface
  (e.g. an axum adapter helper module), bump `0.1.1 → 0.1.2`
  and CHANGELOG-entry it. If Codex achieves the wiring with
  zero new public surface in mhs, no bump needed.
- **`mechanics`**: bump `0.5.1 → 0.5.2` if the public
  `MechanicsPool::run_tls` signature changes (it likely does
  — adding h3 config). CHANGELOG-entry. If the change is
  purely additive (new method `run_tls_with_h3`), still bump
  since downstream crates.io consumers see the new surface.
- **bins (`mechanics-worker`, `philharmonic-api-server`,
  `philharmonic-connector`)**: `publish = false`, no bumps.
- **`philharmonic-api`**: if its public API changes (unlikely
  for this dispatch — D22 is bin-level config plumbing, not
  philharmonic-api surface), bump and CHANGELOG. Otherwise
  leave as `0.1.10`.

## Tests

At minimum:

- **Per-bin smoke test (or one shared integration test)** that
  asserts:
  - When `bind_h3` is unset, the bin starts with the existing
    TCP+TLS behaviour. Existing tests should continue to pass.
  - When `bind_h3` is set, the bin starts both the TCP+TLS
    listener and the QUIC listener. A simple assertion: the
    bin's `eprintln!` log line includes "HTTP/3" or the QUIC
    bind address.
  - Bonus: an h3 client connects to the QUIC port and gets a
    200 from a representative endpoint. Use
    `mechanics-http-client`'s opportunistic HTTP/3 path (or
    raw `h3` + `h3-quinn` via the now-published
    `mechanics-h3-quinn`) as the client.

  Integration tests like these are typically `#[ignore]`-gated
  in this workspace (require port binding + cert generation;
  similar to docker-based tests in dockerlet). The `--ignored`
  phase of `pre-landing.sh` runs them.

- **mhs adapter tests** (if Codex adds the axum adapter
  helper): unit tests against a stub Router that returns
  fixed responses; verify the request body is correctly fed
  to the Router and the response is correctly returned to
  the h3 path.

- **Existing `mechanics-http-server` tests** stay green.

## Non-goals (explicit)

- **No `mechanics-http-client` changes.** Its opportunistic
  HTTP/3 path is unchanged; this dispatch is server-side only.
- **No webui changes.** The browser-side WebUI does not need
  any updates for HTTP/3 to work; that's transparent at the
  HTTP-client level.
- **No connector-router HTTP/3.** The connector-router crate is
  an intra-realm router; HTTP/3 there is out of scope. (It can
  be added later if needed.)
- **No new TLS cert handling.** Same cert chain + private key
  used for TCP+TLS is reused for HTTP/3 (mhs's
  `Http3Server::start` takes cert + key by argument). No
  separate `[tls_h3]` config block.
- **No publishing.** Claude publishes any bumped crates at
  the right moment (post-Codex review); Codex never invokes
  `publish-crate.sh`.
- **No design-doc rewrites.** Updates to ROADMAP / README /
  design docs land separately after Codex's wiring lands.
- **No HTTP/3-only mode.** `bind_h3` is opportunistic — TCP
  path always runs alongside.

## Commit discipline (binding)

Same as the D24 round 03 + h3-quinn vendor rounds:

- **Codex does NOT commit.** No `./scripts/commit-all.sh`,
  no `git commit` / `git add` / `git push` / `git stash`.
  Read-only `git status` / `git diff` / `git log` are fine.
- **Codex does NOT publish.** No `./scripts/publish-crate.sh`,
  no `cargo publish`. Claude publishes if needed.
- **Codex CAN run `pre-landing.sh`** now that
  /tmp tmpfs has been bumped to support the full workspace test
  cycle. Run it at the end of the dispatch (both default and
  `--xtask` if xtask was touched).
- Every cargo invocation needs `CARGO_TARGET_DIR=target-main`
  (or `target-xtask` for `xtask/`).
- Per-crate `CARGO_TARGET_DIR=target-main cargo check -p
  <crate> --all-targets` after each crate's edits.

## Outcome

Pending — will be updated after Codex round 01 run.

---

<task>
Wire `mechanics-http-server`'s `Http3Server` + `AltSvcLayer` into
the three release bins so HTTP/3 (QUIC) is enabled when each bin's
config sets `bind_h3: Option<SocketAddr>`. Same TLS cert is reused
across TCP+TLS and QUIC. When `bind_h3` is set, the TLS-served
axum Router gets wrapped with `AltSvcLayer` so HTTP/1.1+HTTP/2
responses advertise the alternative HTTP/3 endpoint via Alt-Svc.

**Authoritative references (read first; the prompt above
elaborates):**

1. `mechanics-http-server/src/{lib.rs,server.rs,alt_svc.rs}` —
   the public API surface to integrate against.
   `Http3Server::new(Http3ServerConfig).start(service,
   tls_cert_chain, tls_private_key) -> Result<Http3Handle>`.
2. `mechanics/src/lib.rs` `MechanicsPool::run` / `run_tls`
   (around lines 247 / 290) — `mechanics-worker` calls these.
3. `bins/mechanics-worker/src/main.rs`,
   `bins/philharmonic-api-server/src/main.rs`,
   `bins/philharmonic-connector/src/main.rs` — the three
   release bins.
4. `CONTRIBUTING.md` §§3.1, 4, 5, 10.3, 11.
5. `docs/ROADMAP.md` — current state preamble notes D22
   server-integration as next-natural.

**Concrete tasks:**

1. **Service-shape adapter.** mhs's `Http3Server::start`
   expects `Service<Request<()>, Response = Response<Bytes>>`.
   axum gives `Service<Request<axum::body::Body>, Response =
   Response<axum::body::Body>>`. Bridge cleanly — preferred
   approach: add a public helper at
   `mechanics_http_server::axum_compat::router_into_h3_service`
   (module name negotiable; the function is the load-bearing
   part) that accepts an `axum::Router` and returns the
   `Service` shape mhs needs. Streaming preferred over buffer-
   all-bodies. If the streaming version is non-trivial, a
   buffer-then-stream implementation with documented body-size
   cap is acceptable for round 01; flag in residual risks.

   Result: `mechanics-http-server 0.1.1 → 0.1.2`. CHANGELOG
   entry.

2. **`mechanics::MechanicsPool` h3 integration.** Add an h3-
   aware variant of `run_tls`. Recommended shape:
   ```rust
   pub fn run_tls_with_h3(
       &self,
       bind_addr: SocketAddr,
       tls_config: TlsConfig,
       h3_config: Option<mechanics_http_server::Http3ServerConfig>,
   ) -> std::io::Result<()>
   ```
   Or: extend `TlsConfig` to embed `Option<H3Config>`. Codex
   picks. Behaviour:
   - When `h3_config` is `None`: identical to existing
     `run_tls`. No Alt-Svc header.
   - When `h3_config` is `Some(_)`: start mhs `Http3Server`
     alongside the TCP+TLS server (same cert/key), wrap the
     axum Router with `AltSvcLayer`, await both futures via
     `tokio::select!` (or join-all), call `handle.shutdown()`
     on the cancellation path.

   Result: `mechanics 0.5.1 → 0.5.2`. CHANGELOG entry. Keep
   existing `run_tls` working unchanged for non-h3 callers.

3. **`bins/mechanics-worker/src/main.rs`:**
   - Add `bind_h3: Option<SocketAddr>` to the bin's config
     struct.
   - Add `--bind-h3 <addr>` CLI arg.
   - When `bind_h3` is `Some(_)` but TLS is None, error at
     startup with "HTTP/3 requires TLS; configure `[tls]`".
   - When `bind_h3` is `Some(_)` and TLS is configured, call
     `MechanicsPool::run_tls_with_h3` (or whichever shape
     lands in step 2).
   - `eprintln!` startup log line includes h3 bind address
     when running.

4. **`bins/philharmonic-api-server/src/main.rs`:**
   Same shape as (3) but with the bin's existing
   `start_tls_server` logic. The bin doesn't go through
   `mechanics::run_tls`; it has its own TCP+TLS code. So
   step 2's mechanics-level integration doesn't apply here —
   the bin spawns mhs's `Http3Server` directly. The Alt-Svc
   layer wraps the axum Router for the TLS path.

5. **`bins/philharmonic-connector/src/main.rs`:**
   Same shape as (4). Bin-local TCP+TLS code + direct
   `Http3Server` spawn + AltSvcLayer wrap.

6. **Integration tests** per bin (or one shared in
   `bins/*/tests/` or `philharmonic-api/tests/`):
   - `bind_h3 = None`: existing behaviour unchanged.
   - `bind_h3 = Some(_)` + TLS: bin starts, h3 client connects
     and gets 200.
   - `bind_h3 = Some(_)` + no TLS: bin errors at startup with
     clear message.
   - Mark long-running / network tests `#[ignore]` per the
     workspace's convention.

7. **`mechanics-http-server` tests** stay green.

**Per-crate version-bump policy:**

- `mechanics-http-server`: 0.1.1 → 0.1.2 if public surface
  added (the axum-compat module). CHANGELOG entry.
- `mechanics`: 0.5.1 → 0.5.2 (public surface change).
  CHANGELOG entry.
- `bins/*`: `publish = false`, no bumps.
- `philharmonic-api`: no bumps unless its public API changes
  (unlikely).

**Non-goals:**

- No mhc changes.
- No webui changes.
- No connector-router h3.
- No new TLS cert handling.
- No publishing (Claude does that post-review).
- No design-doc rewrites (separate Claude commit later).
- No `[features]` redesign in mechanics-core.

<action_safety>
- **Codex does NOT commit.** No `./scripts/commit-all.sh`, no
  `git commit`, no `git add`, no `git push`, no `git stash`.
  Read-only `git status` / `git diff` / `git log` are fine.
  Leave everything in the dirty working tree; Claude commits.
- **Codex does NOT publish.** No `./scripts/publish-crate.sh`,
  no `cargo publish`.
- **Codex CAN run `pre-landing.sh`** after the 2026-05-14
  zramswap bump — /tmp tmpfs now holds a full workspace test
  cycle. Run `./scripts/pre-landing.sh` at the end; if
  `xtask/` was touched (unlikely), also run `--xtask`.
- Every cargo invocation needs `CARGO_TARGET_DIR=target-main`
  (or `target-xtask` for `xtask/`).
- POSIX-ish host. No bash-only constructs in shell.
- Run `./scripts/xtask.sh calendar-jp` at session start and
  again before returning. If JST is outside regular hours
  (10:00–19:00, ext 21:00), add a one-line "(JST now HH:MM
  <day> — outside regular hours; proceeding.)" note in the
  final reply.
- POSIX sh for any shell wrapper. xtask bins are Rust.
- The workspace's runtime HTTP-TLS posture: rustls +
  aws-lc-rs + webpki-roots. mhs already enforces this in its
  Cargo.toml. Don't introduce ring, native-tls,
  rustls-platform-verifier, or rustls-native-certs anywhere.
</action_safety>

<missing_context_gating>
Before starting, run `./scripts/status.sh` — it should print
the parent-clean state. If the workspace has mid-flight dirty
work that isn't part of this dispatch's mandate, STOP and
report the divergence.

Read mhs's actual public API (`mechanics-http-server/src/lib.rs`
+ the modules it re-exports from). The prompt above sketches
the API shape; if the actual code differs, the code wins.

Read the three bins' main.rs files in full before editing.
Each has its own TLS handling pattern; don't assume they share
identical code.

The `mechanics-h3-quinn 0.0.10` crate (published to crates.io
2026-05-14 12:37 JST) is the workspace's vendored fork of
h3-quinn that drops the rustls-ring default; it's already wired
into mhc + mhs via the cargo `package` rename. No changes
needed there for this dispatch.
</missing_context_gating>

<default_follow_through_policy>
Land all three bins' wiring + the mhs adapter (if added) + the
mechanics::run_tls_with_h3 (or equivalent) + tests in this
single round. Don't stop after one bin and report "the other
two pending". The wiring is similar across the three bins.

If a hard blocker surfaces (e.g. mhs's API turns out to need
non-trivial extension beyond just an axum adapter; or the
service-shape mismatch can't be bridged cleanly), stop, document
the blocker, and report INCOMPLETE clearly. Don't paper over
with a half-wired bin.
</default_follow_through_policy>

<completeness_contract>
"Complete" means all of:

1. `mechanics-http-server` has the axum adapter (or
   equivalent shim); its tests pass; if public surface
   changed, version bumped to 0.1.2 with CHANGELOG entry.
2. `mechanics::MechanicsPool` has h3-aware variant; existing
   `run_tls` callers still work; version bumped to 0.5.2 with
   CHANGELOG entry.
3. All three bins have `bind_h3: Option<SocketAddr>` config
   field + CLI arg + correct behaviour (None / Some+TLS /
   Some+no-TLS error).
4. At least one integration test demonstrates h3 client
   connecting and getting 200 on a representative endpoint
   for at least one of the three bins (preferably all three;
   bare minimum is one).
5. `./scripts/pre-landing.sh` clean.
6. `./scripts/pre-landing.sh --xtask` clean (only required if
   xtask was touched; unlikely for this dispatch).
7. Banned-dep tree-invert checks remain clean:
   - `cargo tree --workspace --invert ring --target
     x86_64-unknown-linux-gnu`: empty.
   - `cargo deny check bans`: PASS.
8. `./scripts/status.sh` shows the dirty tree (Codex doesn't
   commit).
9. `## Outcome` section of this prompt file updated with the
   round 01 result.

If any of 1–7 is incomplete, report INCOMPLETE clearly with
what's done and what's left.
</completeness_contract>

<verification_loop>
After each crate's edits:
  CARGO_TARGET_DIR=target-main cargo check -p <crate> --all-targets

After all crates edited:
  ./scripts/pre-landing.sh
  ./scripts/pre-landing.sh --xtask  (only if xtask was touched)
  CARGO_TARGET_DIR=target-main cargo deny check bans
  CARGO_TARGET_DIR=target-main cargo tree --workspace --invert ring --target x86_64-unknown-linux-gnu

Do not run raw `cargo fmt/clippy/test` when `pre-landing.sh`
covers them. Per-crate `cargo check` is the only acceptable
mid-flight cargo invocation.

If `pre-landing.sh` fails:
1. Read the failure. If a specific bin's clippy / test
   caused it, the bin's wiring or the mhs adapter is wrong.
2. Fix the underlying issue. Don't paper over with `#[allow]`.
3. Re-run pre-landing.sh.
</verification_loop>

<structured_output_contract>
At end of round 01, return:

1. **Summary** (2–3 sentences): what landed; which bins gained
   h3; was an mhs adapter added.
2. **Touched files**: grouped by crate.
3. **Public API changes**:
   - `mechanics-http-server` — new public items added (if
     any), version bump.
   - `mechanics` — new public items (`MechanicsPool::run_tls_with_h3`
     or extended `TlsConfig` shape), version bump.
4. **Bin behaviour**:
   - `mechanics-worker`: `bind_h3 = None` / `Some+TLS` /
     `Some+no-TLS` cases verified.
   - `philharmonic-api-server`: same.
   - `philharmonic-connector`: same.
5. **Test coverage added**: which integration tests landed
   (file paths + test names).
6. **Verification results**:
   - `pre-landing.sh`: PASS / FAIL with tail of output.
   - `pre-landing.sh --xtask` (if run): PASS / FAIL.
   - `cargo deny check bans`: PASS / FAIL.
   - `cargo tree --invert ring`: empty / non-empty.
7. **Residual risks**:
   - Streaming-vs-buffer trade-off in the axum adapter.
   - Any feature held back from this round (e.g. h3 for
     connector-router deliberately deferred).
   - Anything that could regress when upstream mhs / quinn
     bumps.
8. **Git state**: `./scripts/status.sh` output showing dirty
   tree. **NO commits made.**
9. **Outcome paragraph** suitable for dropping into the prompt
   file's `## Outcome` section.
</structured_output_contract>
</task>
