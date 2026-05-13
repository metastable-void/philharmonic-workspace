# D22 — `mechanics-http-client` HTTP/3 client side (initial dispatch)

**Date:** 2026-05-13
**Slug:** `d22-mechanics-http-client-http3-client`
**Round:** 01 (initial dispatch — D22 client half, ROADMAP §3.I,
single crate `mechanics-http-client`)
**Subagent:** `codex:codex-rescue`

## Motivation

D20 (landed 2026-05-13) introduced `mechanics-http-client` 0.1.0
— hyper-rustls + webpki-roots + aws-lc-rs, HTTP/1.1 and HTTP/2
via ALPN. D22 is the next transport-layer step: add HTTP/3 (QUIC)
on the **client** side, with HTTPS DNS RR lookup (RFC 9460 SVCB/
HTTPS) as the primary discovery mechanism — matching modern-
browser behaviour (Firefox 84+, Chrome 113+, curl 8.10+).

The user explicitly picked D22 next, overriding the D19 → D22
amortisation suggestion in the ROADMAP. D19 (DNS connector,
hickory-resolver introduction) will happen later; this dispatch
brings `hickory-resolver` into the workspace via mhc's new
`http3` feature.

**Server side is OUT OF SCOPE** for this dispatch. Even though
ROADMAP §3.I describes both halves of D22 together, the server
side touches three release bins + `mechanics`'s HTTPS code and
deserves its own prompt with deployment-validation context.
This dispatch lives entirely inside `mechanics-http-client`.

**Important deviation from ROADMAP §3.I**: per Yuka's explicit
direction during prompt-drafting (2026-05-13, after the ROADMAP
update committed in `bd6d769`), the `http3` Cargo feature ships
**default-on**, not opt-in as §3.I originally suggested. h3 is
a baseline workspace capability now, not an optional add-on.
Operationally this means every `https://` request triggers a
DNS HTTPS RR lookup on first contact for that origin; the
negative cache + per-origin caches keep the steady-state cost
near zero. The feature flag still exists so consumers who
explicitly need to compile h3 out (e.g. compile-size-sensitive
embeds) can do `default-features = false`, and
`ClientBuilder::http3(false)` still works as a runtime kill-
switch. Cascade impact on the four mhc consumers (mechanics-
core, http-forward, llm-openai-compat, api-server): their
default-feature compiles will pull `hickory-resolver` /
`quinn` / `h3` / `h3-quinn` automatically; cargo-tree
verifications need to confirm the ring-free posture survives
the additional deps.

If §3.I's wording contradicts the default-on direction below,
the direction below wins — Claude will rewrite §3.I post-
dispatch to match. Everything else in §3.I (discovery priority,
HTTP/1.1 invariants, TLS-side ALPN invariant, sequencing
notes) stands unchanged.

## References

- [`docs/ROADMAP.md` §3.I (D22)](../ROADMAP.md#i-http3-client--server-1-dispatch-future-session)
  — the authoritative scope, discovery-priority order, and
  hard constraints (including the HTTP/1.1-keeps-working
  invariants). **If anything below contradicts §3.I, the
  ROADMAP wins.**
- [`docs/archive/2026-05-13-roadmap-d20-done.md`](../archive/2026-05-13-roadmap-d20-done.md)
  — D20 outcome + the original D20 scope notes, useful as
  context for how the client got to its current shape.
- RFC 9460 (SVCB and HTTPS RRs) — discovery-record format. The
  `alpn`, `port`, `ipv4hint`, `ipv6hint` SvcParams matter for
  this dispatch; the others (`mandatory`, `no-default-alpn`,
  `ech`, …) can be ignored in v1 if you note it in residual
  risks.
- RFC 7838 (HTTP Alternative Services) — `Alt-Svc` header
  format and the `ma` directive.
- [`mechanics-http-client/src/lib.rs`](../../mechanics-http-client/src/lib.rs)
  + sibling modules — the current 0.1.0 shape. The
  discovery + h3 wiring slots in around
  `RequestBuilder::send` and `Client::build`.

## Context files pointed at

- [`mechanics-http-client/src/lib.rs`](../../mechanics-http-client/src/lib.rs)
  — crate root; re-exports + crate-level rustdoc. New module
  declarations land here gated on `#[cfg(feature = "http3")]`.
- [`mechanics-http-client/src/client.rs`](../../mechanics-http-client/src/client.rs)
  — `Client`, `ClientBuilder`, `ClientInner`. The h3 surface
  attaches here: builder learns optional h3 toggles, inner
  carries the HTTPS-RR / Alt-Svc caches and the QUIC endpoint.
- [`mechanics-http-client/src/request.rs`](../../mechanics-http-client/src/request.rs)
  — `RequestBuilder::send`. The discovery-priority dispatch
  lives here (or in a helper invoked from here): for
  `https://` URIs, try HTTPS RR → if h3, attempt QUIC → on
  failure fall back to the existing hyper path. For
  `http://` URIs the existing path is unchanged.
- [`mechanics-http-client/src/response.rs`](../../mechanics-http-client/src/response.rs)
  — `Response`. The Alt-Svc-cache update happens after first
  response on `https://`: parse `Alt-Svc` header from
  `self.parts.headers`, write into the per-origin cache.
- [`mechanics-http-client/src/tls.rs`](../../mechanics-http-client/src/tls.rs)
  — `webpki_roots_client_config()`. Reuse this for the QUIC-
  side rustls config; `quinn` accepts a `rustls::ClientConfig`
  via `quinn::crypto::rustls::QuicClientConfig`. The
  `aws-lc-rs` crypto provider stays.
- [`mechanics-http-client/src/error.rs`](../../mechanics-http-client/src/error.rs)
  — `Error` enum. Likely needs (at minimum) a `QuicHandshake`
  variant under the `http3` feature, plus possibly a `Dns`
  variant for HTTPS RR lookup failures. Your call exactly,
  document in residuals.
- [`mechanics-http-client/src/tests.rs`](../../mechanics-http-client/src/tests.rs)
  — existing wiremock-based integration tests. The new tests
  for HTTPS RR cache, Alt-Svc cache, and h3 fallback go
  alongside.
- [`mechanics-http-client/Cargo.toml`](../../mechanics-http-client/Cargo.toml)
  — version bump + new feature + new (gated) deps.
- [`mechanics-http-client/CHANGELOG.md`](../../mechanics-http-client/CHANGELOG.md)
  — `[0.2.0]` entry under `[Unreleased]`.

## Outcome

**Pending — will be updated after the Codex run.**

---

## Shape (locked decisions)

These follow ROADMAP §3.I and the workspace's existing TLS-
posture stance. Don't relitigate the design.

### Discovery priority

For `https://` origins, on first-contact for that origin (or
after a cache entry's TTL/max-age expired):

1. **HTTPS DNS RR lookup (primary).** Query the `HTTPS` RR
   (DNS TYPE 65) for the origin hostname via the configured
   resolver. Honour the `alpn`, `port`, `ipv4hint`, `ipv6hint`
   SvcParams. If `alpn` advertises `h3`, populate the per-
   origin HTTPS RR cache with `{port, addresses, has_h3,
   expires_at}` and attempt HTTP/3 over QUIC. Cache TTL =
   the resource record's TTL.
2. **Alt-Svc fallback.** When HTTPS RR returned NODATA /
   NXDOMAIN / no `h3` in `alpn`, the first request goes over
   HTTP/2 (existing hyper path). If the response's `Alt-Svc`
   header advertises `h3=":443"` (or any port), populate the
   per-origin Alt-Svc cache with `{port, expires_at}` derived
   from the `ma` directive. Subsequent requests within
   `max-age` upgrade to HTTP/3.
3. **HTTP/2-only fallback.** When neither discovery path
   produced h3, or the QUIC handshake failed, requests for
   that origin stay on HTTP/2 / HTTP/1.1 for some negative-
   cache window (e.g. 5 minutes — your call, document it).

For `http://` origins, **none** of the above runs. No DNS HTTPS
RR lookup, no QUIC, no Alt-Svc honouring. The existing hyper
HTTP/1.1+HTTP/2 cleartext path is byte-for-byte unchanged. This
is a hard constraint from ROADMAP §3.I.

### Non-h3 connections keep working (hard constraint, ROADMAP §3.I, reiterated by Yuka)

With h3 now default-on, the most load-bearing invariant of this
dispatch is: **any request that 0.1.0 would have completed
successfully must still complete successfully on 0.2.0.** h3 is
opportunistic; non-h3 paths are the always-works baseline.
Specifically:

All three HTTP/1.1 modes must keep working:

1. **Plain HTTP/1.1 over cleartext TCP** — `http://` URLs.
   No discovery logic touched, no h3 attempt, no Alt-Svc.
2. **HTTP/1.1 over TLS** — `https://` URLs where the server's
   TLS-side ALPN only advertises `http/1.1`. Negotiation
   lands on HTTP/1.1 on the TLS connection.
3. **HTTP/1.1 selected via ALPN** — when the server offers
   multiple TLS-side protocols (`h2, http/1.1`) and the
   negotiation lands on `http/1.1`.

HTTP/2 over TLS continues to be the normal fallback when h3 is
unavailable.

The TLS-side ALPN list on the client MUST stay `h2, http/1.1`
after this dispatch — adding `h3` is **additive**, on the
**QUIC-side** ALPN only (which is a separate channel; the
client offers `h3` to a QUIC server, but offers `h2, http/1.1`
to a TCP+TLS server). This invariant is load-bearing for the
HTTPS-without-h3 fallback to work correctly.

### Graceful degradation (hard constraint, ROADMAP §3.I + 2026-05-13)

Any failure inside the h3 path must degrade silently to the
existing hyper HTTP/2/1.1 path; it must **not** propagate as
a request error. Specifically:

- DNS HTTPS RR query error (resolver unreachable, SERVFAIL,
  timeout) → treat as "no HTTPS RR available", fall through
  to Alt-Svc / HTTP/2 path. Optional `tracing::debug!` line;
  no `warn!`, no `Error::Dns` surfaced to the caller for the
  request.
- HTTPS RR parses but contains no `h3` in `alpn` → treat as
  cache-miss-for-h3, fall through. Normal.
- QUIC handshake failure (connection refused, TLS handshake
  failed, version negotiation failed, idle timeout during
  handshake, anything else `quinn` reports) → fall through
  to HTTP/2 path. Populate negative cache. Optional
  `tracing::debug!` line; no `warn!`, no `Error::QuicHandshake`
  surfaced to the caller for the request.
- h3 stream error after a successful handshake (server sent
  GOAWAY, stream reset, etc.) → tricky case. For v1, **do
  not** silently retry on HTTP/2 (the request may have had
  side effects). Surface the error to the caller as
  `Error::Cancelled` or a new variant. Document the choice
  in residuals.
- All other h3-path errors (cache lock poisoning,
  internal-state inconsistencies, etc.) → fall through to
  HTTP/2 path; log internally; never error the caller.

`Error::Dns` and `Error::QuicHandshake` exist for *terminal*
failures (the request reached neither h3 nor h2/h1, e.g.
because Alt-Svc rerouted to a port the system can't reach
and there's no fallback). They MUST NOT be returned in cases
where the legacy hyper path would have worked.

The h3 path is **purely opportunistic** — its job is to be
faster when possible, never to be the only way to succeed.

### New `http3` feature

- **Default-on.** `default = ["http3"]` in `Cargo.toml`.
  Consumers who don't want h3 in their dep tree opt out with
  `default-features = false` (and they pay the lost-functionality
  cost — no h3 discovery, no QUIC, just hyper HTTP/1.1+HTTP/2).
- Gates: all new dep additions, all new module code, all new
  public surface (config knobs, error variants). Without the
  feature, `mechanics-http-client` compiles byte-identical to
  today's 0.1.0 (modulo the version bump) — the feature gate is
  load-bearing for the opt-out path.

### Dependencies (under `http3` feature only)

- `hickory-resolver` (latest 0.24 or 0.25, your choice based
  on tokio compatibility — workspace uses tokio 1.x). Pull
  with feature flags that disable any default DoH/DoT
  upstream-resolver bundling — system-config mode only
  (`/etc/resolv.conf`). Pure-Rust, async, no `unsafe` in our
  own code.
- `quinn` (latest 0.11.x or newer). Configure with
  `quinn::crypto::rustls::QuicClientConfig::try_from(rustls::ClientConfig)`
  using the existing `webpki_roots_client_config()`. Reuse
  the workspace's `aws-lc-rs` crypto provider; **do not**
  pull `ring`.
- `h3` (latest 0.0.x). HTTP/3 framing over QUIC.
- `h3-quinn` (latest matching `h3` version). Adapter between
  `h3` and `quinn` transport.

If any of these conflict with the workspace's `[patch.crates-io]`
block or with mhc's existing rustls/aws-lc-rs feature pinning,
flag in residual risks rather than working around silently.

### Caches

Per-`Client` state, `Arc`-wrapped (the client is cheap-to-clone
already; the caches live inside `ClientInner` alongside
`hyper`, `default_timeout`, `default_headers`).

- **HTTPS RR cache**: `HashMap<Origin, HttpsRrEntry>` behind
  a `RwLock` (or `parking_lot::RwLock` if mhc already pulls
  it transitively — check first, prefer std). `HttpsRrEntry`
  carries `{port, addresses, has_h3, expires_at: Instant}`.
- **Alt-Svc cache**: `HashMap<Origin, AltSvcEntry>` same
  shape. `AltSvcEntry` carries `{port, expires_at: Instant}`.
- **Negative cache**: same shape, marks "this origin has no
  h3 via either mechanism, don't re-probe until expires_at."

`Origin` = `(scheme, host, port)` tuple or whatever shape
fits cleanly with the http crate's `Uri` parts. Your call.

Expired entries are lazy-evicted on lookup; a background
sweep task is overkill for v1 (document if you disagree).

### QUIC connection lifecycle

- One `quinn::Endpoint` per `Client`. Created lazily on the
  first attempted h3 request. UDP socket bound to a random
  local port via `quinn::Endpoint::client(SocketAddr)`.
- Per-origin connections cached and reused across requests
  to the same origin (HTTP/3 multiplexes on a single QUIC
  connection — don't open a new one per request).
- QUIC handshake failure → return a structured error from
  the discovery path that triggers fallback. Don't propagate
  the raw `quinn::ConnectError` to the user; map it.
- 0-RTT on the client side: **not in v1**. Always do a full
  handshake. 0-RTT replay safety is a server-side concern
  for D22's server half; we don't need to enable it on the
  client side for this dispatch.

### Configuration knobs

Add to `ClientBuilder` (all under `#[cfg(feature = "http3")]`):

- `http3(self, enabled: bool) -> Self` — runtime kill-switch.
  **Default `true`** when the feature is compiled (which is
  default). Setting `false` skips all h3 discovery + QUIC at
  runtime, reverting that `Client` to the 0.1.0 hyper-only
  behaviour. The Cargo feature flag controls *compile-time
  availability*; this method controls *runtime use* — useful
  for operators in known-broken-QUIC environments (corp
  proxies, certain CDN egress paths) without needing a
  recompile.
- (Optional) `http3_negative_cache_duration(self, d: Duration) -> Self`
  — how long to remember "this origin doesn't support h3".
  Default ~5 minutes. Skip if it adds surface noise.

Do **not** expose the HTTPS RR / Alt-Svc caches publicly. They
are internal `Arc`-wrapped state on `ClientInner`. (Test code
needs to peek at them; see Tests section for the gated
test-only accessor.)

### Error model

`Error` enum may grow one or two new variants under
`#[cfg(feature = "http3")]`:

- `QuicHandshake(String)` — h3 handshake failed; the request
  will have been retried over HTTP/2 already, so this is only
  returned if the fallback also failed.
- `Dns(String)` — HTTPS RR lookup failure (resolver error,
  not "no record found" which is a normal cache-miss signal).

Mark `Error` `#[non_exhaustive]` (it already is). New variants
under the feature flag are non-breaking for consumers who
don't enable the feature.

### Version bump

`0.1.0 → 0.2.0` (minor, **not** patch). Justification: new
default-on feature flag, new dep tree (hickory-resolver, quinn,
h3, h3-quinn), new default behaviour for `https://` requests
(DNS HTTPS RR lookup, opportunistic h3 upgrade). The Cargo
metadata change alone (default features expanded, dep list
expanded) warrants the minor; the new runtime behaviour seals
it. Update `CHANGELOG.md` with a `[0.2.0] - 2026-05-13`
entry above `[0.1.0]`. Lead the entry with the operationally-
visible change ("h3 is now negotiated for `https://` requests
opportunistically — DNS HTTPS RR lookup first, Alt-Svc as
fallback"), not the dep-list-grew framing.

## Tests

Required (extend `mechanics-http-client/src/tests.rs` or a new
`tests/http3.rs` integration test file, your call):

1. **`http://` URL with h3 default-on** — default
   `ClientBuilder` (h3 enabled). `client.get("http://server
   /path").send().await` must NOT trigger any DNS HTTPS RR
   lookup, must NOT attempt QUIC, and must succeed via the
   existing wiremock-based HTTP/1.1 path. Assert: no
   resolver interaction (use a counter-wrapped fake
   resolver injected via the test-only accessor), no QUIC
   endpoint created.

2. **`https://` URL with h3 runtime-disabled** —
   `ClientBuilder::http3(false)` (feature is on; runtime
   kill-switch off). Existing 0.1.0 behaviour preserved
   byte-for-byte: no HTTPS RR lookup, no QUIC, hyper path
   only. The 5 wiremock-based tests from 0.1.0 still pass
   unchanged when configured with `http3(false)`.

2b. **Default-features compile (h3 enabled implicitly)** — the
    5 wiremock tests from 0.1.0 (which use `Client::new()`
    and `Client::builder()`) must still pass with the
    default-on feature, since the wiremock fixtures
    advertise neither HTTPS RR (the fake resolver returns
    NODATA) nor `Alt-Svc: h3` and the QUIC fallback path
    is never reached. This proves the "h3 default-on
    doesn't regress existing-server behaviour" property.

3. **HTTPS RR cache hit advertises h3 → QUIC succeeds** —
   inject (via the gated test-only accessor) an HTTPS RR
   cache entry for `https://example.test/` that advertises
   `alpn=h3` with the address pointing at a local `h3`
   server fixture. Assert: the request flows over HTTP/3.
   Counter on the (non-existent) HTTP/2 fallback path
   stays at 0.

4. **HTTPS RR cache hit advertises h3 → QUIC fails → falls
   back to HTTP/2** — same as above but the local h3
   fixture is offline. Assert: the request completes via
   the wiremock HTTP/2 fixture, an error counter on the
   QUIC attempt incremented exactly once, and the negative
   cache now has an entry for that origin.

5. **Alt-Svc cache populated from first response** — first
   `https://example.test/` request goes over HTTP/2 (no
   HTTPS RR populated). The wiremock fixture returns
   `Alt-Svc: h3=":<port>"; ma=3600` in the response.
   Assert: after the response, the Alt-Svc cache (via the
   gated test-only accessor) has an entry for that origin
   with `expires_at` ≈ now + 3600 s.

6. **Negative cache short-circuits re-probing** — populate
   the negative cache for `https://example.test/`. Make a
   request. Assert: no DNS HTTPS RR lookup happened (the
   counter on the fake resolver is 0), no QUIC attempt
   happened, request went straight to the HTTP/2 path.

7. **HTTP/1.1 over TLS (server only advertises `http/1.1`)**
   — wiremock fixture configured for HTTPS with TLS-side
   ALPN advertising only `http/1.1`. Request: succeeds over
   HTTP/1.1 over TLS. Assert: response version is
   `Version::HTTP_11`. (wiremock may or may not give you
   easy ALPN control — if it doesn't, mark this test
   `#[ignore]` with a TODO and write a unit test for the
   ALPN-list assertion at the rustls config level instead.)

8. **TLS-side ALPN list invariant** — unit test reading the
   client's TLS-side ALPN protocols list from the rustls
   `ClientConfig` (the one returned by
   `tls::webpki_roots_client_config()`). Assert: it
   contains exactly `[b"h2", b"http/1.1"]` in that order,
   no `b"h3"` present. This is the load-bearing
   invariant that lets HTTP/1.1-only-server case work.

9. **HTTPS RR / Alt-Svc parsing unit tests** — pure-input/
   pure-output tests for the SvcParam parser and the
   `Alt-Svc` header parser. Cover at least:
   - HTTPS RR `alpn=h3` (single ALPN)
   - HTTPS RR `alpn=h3,h2` (multi-ALPN)
   - HTTPS RR with `port` and `ipv4hint` SvcParams set
   - HTTPS RR with no `alpn` (treat as "no h3")
   - Alt-Svc `h3=":443"; ma=3600`
   - Alt-Svc `h3-29=":443"; ma=3600` (draft version — treat
     as "no h3" since we only support the RFC version)
   - Alt-Svc `clear` (cache eviction directive)
   - Malformed Alt-Svc header (ignore gracefully, don't
     error the response)

10. **Graceful degradation on DNS / QUIC failure** —
    `https://example.test/` request with the fake resolver
    returning SERVFAIL (or timing out). Assert: the request
    still completes successfully via the wiremock HTTP/2
    fixture; no `Error::Dns` returned; negative cache gets
    a "no h3 here" entry. Repeat with the resolver returning
    a valid HTTPS RR + `h3` but the QUIC fixture refusing
    connections: same assertion, request completes via
    HTTP/2, no `Error::QuicHandshake` returned, negative
    cache populated. This is the load-bearing "h3 is
    opportunistic, never the only way to succeed" check.

Existing `mechanics-http-client` tests (the 5 wiremock-based
ones from 0.1.0) must remain green byte-for-byte.

### Test-only accessor for cache injection

Add a `#[cfg(test)]` (or `#[doc(hidden)] pub`) method on
`Client` (or a free function in a test-only module) that lets
the test suite inject HTTPS RR / Alt-Svc / negative-cache
entries. Without it the tests can't deterministically exercise
the discovery paths. Document the accessor's existence in
residual risks (operators should never call it).

### h3 server fixture

For tests 3 and 4 you need a local h3 server. The `h3` crate
has a `server` example. A reasonable shape: spawn a tokio task
on test setup that binds a `quinn::Endpoint::server(...)` on
`127.0.0.1:0`, accepts one connection, and replies to one
request with a fixed body. Tear down on test teardown.
Use the workspace's existing wiremock for the HTTP/2 fallback
path in the same test.

If the h3 server fixture is too gnarly for v1, you may
`#[ignore]` tests 3 and 4 with explicit TODOs and lean on the
unit tests (test 9) for the parsing logic + a state-machine
test for the discovery dispatch that uses a fake transport.
Document the choice in residual risks.

## Verification flow

```sh
./scripts/pre-landing.sh
```

Runs cargo-deny bans + fmt + check + clippy (-D warnings) +
rustdoc + test (with D21's dep-aware narrowing — mhc + its
reverse-dep closure). Slow — minutes — run once before final
commit, not in a tight edit/run loop.

```sh
./scripts/check-api-breakage.sh mechanics-http-client 0.1.0
```

`cargo-semver-checks` against the 0.1.0 crates.io baseline. The
expected output is "minor bump justified" — new feature flag,
new conditionally-public surface. If it flags any breaking
change to the default-feature surface, **stop** and surface in
residuals; the dispatch's invariant is that default-feature
behaviour stays byte-identical to 0.1.0.

Also helpful (note: `http3` is in default features now, so the
plain `cargo tree -p mechanics-http-client -e normal` includes
quinn/hickory/h3 already):

```sh
cargo tree -p mechanics-http-client -e normal \
  | grep -E "reqwest|rustls-platform-verifier|rustls-native-certs|ring v"
```

Should print nothing. The TLS posture (aws-lc-rs + webpki-roots
only, no ring, no platform-verifier) MUST carry over to the
default-feature tree now that h3 is default-on. Run the same
grep on each of the four mhc consumers as a cascade check:

```sh
for crate in mechanics-core philharmonic-connector-impl-http-forward \
             philharmonic-connector-impl-llm-openai-compat \
             philharmonic-api-server; do
  echo "=== $crate ==="
  cargo tree -p $crate -e normal \
    | grep -E "reqwest|rustls-platform-verifier|rustls-native-certs|ring v" \
    || echo "(clean)"
done
```

Surface the outputs in the structured report.

Also confirm the `default-features = false` opt-out path:

```sh
cargo tree -p mechanics-http-client --no-default-features -e normal \
  | grep -E "quinn|h3|hickory"
```

Should print nothing — opting out compiles the 0.1.0 baseline.

Skip:

- No publish — Claude reviews and decides post-Codex.
- No edits outside `mechanics-http-client/`. If you find
  yourself wanting to touch `mechanics-core`, any connector
  impl, or the bins, **stop** and surface in residuals.

## Prompt (verbatim)

<task>
Ship D22 client side: add HTTP/3 (QUIC) support to
`mechanics-http-client` 0.1.0 → 0.2.0, with HTTPS DNS RR lookup
(RFC 9460 SVCB/HTTPS) as the primary discovery mechanism and
`Alt-Svc` header as the secondary fallback. The new behaviour
ships behind a non-default `http3` Cargo feature.

Server side is OUT OF SCOPE — that's a separate dispatch.

Single crate. No public-surface breakage outside the `http3`
feature. No crypto path touched. No edits anywhere outside
`mechanics-http-client/`.

Deliverables (in order):

1. **New `http3` Cargo feature**, **default-on**
   (`default = ["http3"]`). Gates every new dep, every new
   module, every new piece of public surface. Consumers who
   opt out with `default-features = false` get a compile
   byte-identical to 0.1.0 (modulo the version bump) — the
   feature gate is load-bearing for the opt-out path.

2. **Dependencies under the `http3` feature.** Add
   `hickory-resolver` (system-config mode only — no bundled
   DoH/DoT upstream resolver), `quinn`, `h3`, `h3-quinn`.
   All four must use `aws-lc-rs` as the crypto provider; do
   not pull `ring`. Verify with `cargo tree -p
   mechanics-http-client --features http3 -e normal | grep
   ring v` — must be empty.

3. **HTTPS DNS RR cache + lookup.** Under
   `mechanics-http-client/src/`, add a module (e.g.
   `https_rr.rs`) that:
   - Defines an `HttpsRrEntry { port: u16, addresses:
     Vec<IpAddr>, has_h3: bool, expires_at: Instant }`.
   - Queries the `HTTPS` resource record (DNS TYPE 65) via
     hickory-resolver in system-config mode.
   - Parses the `alpn`, `port`, `ipv4hint`, `ipv6hint`
     SvcParams. Other SvcParams may be ignored for v1
     (document in residuals).
   - Caches per-origin entries with TTL = the RR's TTL.

4. **Alt-Svc cache + parser.** Add a module (e.g.
   `alt_svc.rs`) that parses `Alt-Svc` response headers
   per RFC 7838 and caches per-origin entries with
   `expires_at` derived from `ma`. Recognise `h3=...`
   only; treat `h3-29=...` and other draft variants as
   "no h3". Recognise `clear` as a cache-eviction
   directive.

5. **Discovery priority dispatch in `RequestBuilder::send`.**
   For `https://` URIs only:
   - Check HTTPS RR cache. On hit advertising `h3`,
     attempt HTTP/3. On miss, query DNS.
   - Check Alt-Svc cache. On hit, attempt HTTP/3 to the
     advertised authority.
   - Check negative cache. On hit, skip both probes.
   - QUIC handshake failure → fall back to HTTP/2 via the
     existing hyper path; populate negative cache.
   - After HTTP/2 first response, parse `Alt-Svc` header
     from the response and update the Alt-Svc cache.
   For `http://` URIs none of the above runs.

6. **QUIC client.** One `quinn::Endpoint` per `Client`
   (lazy-created on first h3 attempt). Per-origin
   connection reuse. Use the existing
   `tls::webpki_roots_client_config()` adapted via
   `quinn::crypto::rustls::QuicClientConfig::try_from(...)`.
   QUIC-side ALPN: `[b"h3"]`. **TLS-side ALPN
   (used by the existing TCP+TLS path via hyper-rustls) MUST
   stay `[b"h2", b"http/1.1"]`** — adding `h3` to the QUIC-
   side ALPN is additive; the TLS-side list stays
   unchanged. This is a hard invariant.

7. **Caches on `ClientInner`.** Add HTTPS RR cache, Alt-Svc
   cache, negative cache. `Arc`-wrapped, `RwLock`-protected.
   Lazy eviction on lookup; no background sweep task.

8. **`ClientBuilder::http3(self, enabled: bool) -> Self`.**
   Runtime kill-switch. **Default `true`** when the feature
   is compiled (which is default). Setting `false` reverts
   the `Client` to the 0.1.0 hyper-only behaviour at
   runtime — useful for operators in known-broken-QUIC
   environments. Add `http3_negative_cache_duration(self,
   d: Duration) -> Self` too if it's not awkward (default
   ~5 min); skip if it adds surface noise.

9. **Error model.** Add `QuicHandshake(String)` and `Dns
   (String)` variants to `Error` under `#[cfg(feature =
   "http3")]`. `Error` stays `#[non_exhaustive]`.

10. **Test-only cache accessor.** Add a gated (`#[cfg(test)]`
    or `#[doc(hidden)] pub`) method on `Client` to inject
    HTTPS RR / Alt-Svc / negative-cache entries from the
    test suite. Without it the discovery tests can't be
    deterministic.

11. **Test suite.** Implement the 9 tests listed in the
    prompt's "Tests" section. Existing 5 wiremock-based
    tests from 0.1.0 must remain green byte-for-byte. The
    h3 server fixture for tests 3 + 4 may be substituted
    with `#[ignore]` + a transport-level state-machine
    test if the fixture is too gnarly; document the choice.

12. **HTTP/1.1 invariants verified.** Tests 1, 2, 7, 8
    enforce the three HTTP/1.1 modes + the TLS-side-ALPN
    invariant from ROADMAP §3.I. These MUST pass.

13. **Version + changelog.** Bump `Cargo.toml` from
    `0.1.0` to `0.2.0`. Add a `[0.2.0] - 2026-05-13`
    entry to `CHANGELOG.md` above `[0.1.0]` referencing
    ROADMAP D22 client side.

14. **Verification.** Run `./scripts/pre-landing.sh` and
    `./scripts/check-api-breakage.sh mechanics-http-client
    0.1.0`. Plus the `cargo tree --features http3` grep for
    forbidden deps. All outputs go in the structured-output
    report.

15. **No publish.** Claude reviews and decides post-Codex.

## Hard constraints

- **Non-h3 connections keep working.** With h3 now default-on,
  this is the single most load-bearing invariant. Any request
  that 0.1.0 would have completed successfully must still
  complete successfully on 0.2.0 with default features. h3 is
  opportunistic; HTTP/2 over TLS / HTTP/1.1 over TLS /
  cleartext HTTP/1.1 are the always-works baseline.
- **Graceful degradation on h3 failure.** DNS HTTPS RR query
  errors, QUIC handshake failures, and h3-stack runtime errors
  fall through to the existing hyper HTTP/2/1.1 path without
  surfacing as request errors. Populate the negative cache so
  the next request to that origin skips the h3 attempt.
  `Error::Dns` / `Error::QuicHandshake` exist only for
  *terminal* failures (the request reached no usable transport
  at all), not for "h3 didn't work, fell back to HTTP/2."
  See the prompt's "Graceful degradation" section for the
  exception around h3 stream errors after a successful
  handshake.
- **HTTP/1.1 keeps working in all three modes.** Plain
  cleartext TCP (`http://`); HTTP/1.1 over TLS (server only
  offers `http/1.1`); HTTP/1.1 selected via ALPN (server
  offers `h2, http/1.1` and ALPN lands on `http/1.1`). The
  client's TLS-side ALPN list stays `[b"h2", b"http/1.1"]`
  after this dispatch — adding `h3` is additive on the
  QUIC-side ALPN only.
- **`http://` URLs untouched.** No HTTPS RR lookup. No QUIC
  attempt. No Alt-Svc honouring. Same cleartext hyper path
  as 0.1.0.
- **TLS posture preserved across the new default tree.**
  aws-lc-rs + webpki-roots only. No `ring`, no
  `rustls-platform-verifier`, no `rustls-native-certs`. Same
  Mozilla CA bundle frozen at compile time. Verify with the
  default-feature `cargo tree -p mechanics-http-client -e
  normal | grep -E "...ring v..."` grep, **and** the same
  grep against the four mhc consumers (cascade check).
- **`--no-default-features` compile is the 0.1.0 baseline.**
  A consumer that opts out with `default-features = false`
  sees byte-identical public API to 0.1.0 modulo the
  version-number bump. No quinn / h3 / hickory in the dep
  tree under that compile.
- **0-RTT off on the client.** Always do a full QUIC
  handshake. 0-RTT replay safety is the server's concern;
  the server side is a separate dispatch and v1 of this
  dispatch ships safe defaults.
- **No `unsafe` blocks** beyond what quinn / h3 / hickory-
  resolver require internally.
- **No panics in lib `src/`.** No `.unwrap()` / `.expect()`
  on `Result`/`Option`, no `panic!` / `unreachable!` /
  `todo!` / `unimplemented!` on reachable paths. Tests
  exempt. (See CONTRIBUTING.md §10.3.)
- **Library boundary stays clean.** No file I/O, no env-var
  lookup, no config-file parsing in `mechanics-http-client`'s
  library code beyond what hickory-resolver does internally
  for `/etc/resolv.conf`.

<structured_output_contract>
**Critical: emit this report before `task_complete`.**

Six sections, in this order:

1. **Summary** — one paragraph: what landed, scope decisions
   (whether tests 3 + 4 used a real h3 fixture or were
   `#[ignore]`'d with a state-machine substitute), dep
   versions picked (hickory-resolver / quinn / h3 / h3-quinn),
   version bump applied, semver-checks outcome, ring-free
   cargo-tree check result.
   Include the verbatim string `RUN STATUS: COMPLETE` or
   `RUN STATUS: PARTIAL — <reason>` for grep.

2. **Touched files** — exhaustive list with
   `(new|edited|deleted) <path> — <one-line note>`.

3. **Verification results** — exact commands + outcomes:
   - `./scripts/pre-landing.sh` — pass/fail/exit code.
   - `./scripts/check-api-breakage.sh mechanics-http-client
     0.1.0` — pass/fail/excerpt. Default-feature surface
     change (new public methods on `ClientBuilder`) should
     be reported as minor-justified, not breaking.
   - `cargo tree -p mechanics-http-client -e normal |
     grep -E "reqwest|rustls-platform-verifier|
     rustls-native-certs|ring v"` — must be empty; report
     the exact command + (empty) output. (h3 is default-on
     now, so no `--features` flag needed.)
   - Same grep run against `mechanics-core`,
     `philharmonic-connector-impl-http-forward`,
     `philharmonic-connector-impl-llm-openai-compat`,
     `philharmonic-api-server` (cascade verification — the
     four mhc consumers must each have a clean tree under
     the new default deps).
   - `cargo tree -p mechanics-http-client --no-default-
     features -e normal | grep -E "quinn|h3|hickory"` —
     must be empty (opt-out path compiles the 0.1.0
     baseline).

4. **Residual risks / known issues** — including:
   - h3 fixture choice (real local server vs.
     `#[ignore]` + state-machine substitute) and why.
   - HTTPS RR SvcParam handling: which SvcParams you
     honour (`alpn`, `port`, `ipv4hint`, `ipv6hint`
     expected) and which you ignore (`mandatory`,
     `no-default-alpn`, `ech`, …).
   - Negative-cache duration default; whether it's a
     `ClientBuilder` knob or hardcoded.
   - QUIC endpoint lifecycle: one per `Client`,
     lazy-created, never explicitly closed (relies on
     `Drop`). Surface if you chose differently.
   - hickory-resolver version + feature flags picked; any
     surprises during `cargo tree` analysis (extra
     transitive deps it pulls).
   - Whether `parking_lot::RwLock` or `std::sync::RwLock`
     was used for the caches.
   - Test-only accessor shape (`#[cfg(test)]` vs.
     `#[doc(hidden)] pub`).

5. **Git state** — current `HEAD` SHA in the parent
   workspace repo and in the `mechanics-http-client`
   submodule. Confirm no commits made.

6. **Open questions** — questions for Yuka or Claude:
   - Whether the negative-cache default duration is
     correct or should be tuned based on operational
     experience.
   - Whether the test-only cache accessor should be a
     permanent `#[doc(hidden)] pub` surface (useful for
     consumers' integration tests) or stay `#[cfg(test)]`
     only.
   - Any sub-shapes you punted on because they felt out
     of scope but might be worth a follow-up.
</structured_output_contract>

<default_follow_through_policy>
- Implement in the order listed in Deliverables.
- Run `cargo test -p mechanics-http-client --features http3`
  directly for fast iteration before invoking pre-landing.
- Don't add `tracing-subscriber`, `env_logger`, or any other
  log-consumer. Producer-side `tracing::debug!` /
  `tracing::warn!` is fine if you want to add observation
  hooks; `tracing` is not a dep yet, add it under the
  `http3` feature if you use it. Don't add it under default
  features.
- If `cargo build` seems stuck for minutes, run
  `./scripts/build-status.sh` (or watch it) before
  declaring a hang. The QUIC stack's first build adds
  meaningful compile time on top of the existing
  hyper-rustls + aws-lc-rs cost.
- No edits outside `mechanics-http-client/`. If you find
  yourself wanting to touch `mechanics-core`, any connector
  impl, any of the bins, or the workspace `Cargo.toml`'s
  `[patch.crates-io]` block, **stop** and surface in
  residuals.
- If the negative-cache duration knob (item 8) feels like
  premature configuration surface, skip it — a hardcoded
  default is fine.
</default_follow_through_policy>

<completeness_contract>
"Done" means:

- `http3` Cargo feature added, **default-on**
  (`default = ["http3"]`).
- hickory-resolver + quinn + h3 + h3-quinn added under the
  feature.
- HTTPS RR lookup + cache implemented.
- Alt-Svc parser + cache implemented.
- Discovery dispatch wired into `RequestBuilder::send`.
- QUIC client lifecycle (endpoint + connection reuse)
  implemented.
- Caches live on `ClientInner` behind `Arc<RwLock<...>>`.
- `ClientBuilder::http3()` toggle added.
- `Error` enum has new `QuicHandshake` + `Dns` variants
  under the feature.
- All 9 tests pass (or tests 3 + 4 are `#[ignore]`'d with
  state-machine substitutes and that's documented).
- Existing 5 0.1.0 wiremock tests still green
  byte-for-byte.
- `cargo-semver-checks` against 0.1.0 reports only
  additive surface changes (new methods on `ClientBuilder`,
  new `Error` variants under the gated feature) — no
  breaking removals.
- Default-feature `cargo tree -p mechanics-http-client`
  has no ring / no platform-verifier / no native-certs.
- Cascade `cargo tree` on the four mhc consumers also has
  no ring / no platform-verifier / no native-certs.
- `--no-default-features` `cargo tree` has no quinn / h3 /
  hickory (proves the opt-out path compiles the 0.1.0
  baseline).
- TLS-side ALPN list still `[b"h2", b"http/1.1"]`
  (test 8).
- `Cargo.toml` bumped 0.1.0 → 0.2.0.
- `CHANGELOG.md` entry added.
- `./scripts/pre-landing.sh` clean.
- `./scripts/check-api-breakage.sh mechanics-http-client
  0.1.0` run and surfaced in residuals.
- Six-section structured-output report emitted before
  `task_complete`.

Partial completion is acceptable only if you hit a token
limit or a genuine blocker — say so explicitly with
`RUN STATUS: PARTIAL — <reason>`. A half-shipped state
machine where discovery sometimes runs over `http://` URLs
or breaks `http://` behaviour is worse than no h3 support;
if you can't finish, leave the discovery dispatch wired
behind an internal `if false` so the `http3` feature
compiles but reverts to the 0.1.0 path at runtime, and
document it loudly.

A run without the structured-output report is
**incomplete**, even if the code landed.
</completeness_contract>

<verification_loop>
1. Implement deps + default-on feature flag + caches +
   discovery + QUIC client + tests in order.
2. `cargo test -p mechanics-http-client` (default features
   — h3 enabled) — confirms all 5 0.1.0 wiremock tests +
   the 9+1 new tests pass.
3. `cargo test -p mechanics-http-client
   --no-default-features` — confirms the opt-out compile
   passes the original 5 tests too (when applicable; some
   tests may be gated on `http3` and that's fine).
4. `CARGO_TARGET_DIR=target-main cargo check --workspace`
   — catches any downstream coupling. The four mhc
   consumers will now pick up h3 transitively; verify they
   still compile.
5. Default-features `cargo tree` greps for ring / platform-
   verifier / native-certs on mhc + four consumers.
6. `cargo tree --no-default-features` grep for quinn / h3 /
   hickory must be empty.
7. Run `./scripts/pre-landing.sh` once.
8. Run `./scripts/check-api-breakage.sh
   mechanics-http-client 0.1.0`.
9. Emit structured-output report.
10. `task_complete`.
</verification_loop>

<missing_context_gating>
If you find yourself needing information not in this prompt
or the cited authoritative sources (ROADMAP §3.I, the D20
archive, RFC 9460, RFC 7838, the existing mhc 0.1.0 source),
**stop** and report what's missing in the structured
output's "Open questions" section.

Specifically: do **not**:

- Touch any crate other than `mechanics-http-client`.
- Add a new public surface to `mechanics-http-client`
  outside the `http3` feature gate.
- Add `tracing-subscriber`, `env_logger`, or any other
  log-consumer crate.
- Modify the TLS-side ALPN list (`h2, http/1.1`) for the
  hyper-rustls path. h3 only goes in QUIC-side ALPN.
- Bundle a DoH or DoT resolver. hickory-resolver in
  system-config mode reads `/etc/resolv.conf`; that's
  the entire DNS upstream story for v1.
- Implement 0-RTT on the client.
- Add server-side QUIC listening. That's the next D22
  dispatch.
- Touch any `.claude/`, `docs/`, `docs-jp/`, `HUMANS.md`,
  `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md`, or
  `scripts/` content. (You may read these as reference;
  no edits.)
- Publish to crates.io. No `cargo publish` even
  `--dry-run`. Claude reviews and decides post-Codex.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `mechanics-http-client/src/lib.rs` (edited — new module
  declarations gated on `#[cfg(feature = "http3")]`; small
  rustdoc tweak noting the new feature).
- `mechanics-http-client/src/client.rs` (edited — caches
  on `ClientInner`, new `ClientBuilder::http3()` method,
  QUIC endpoint plumbing).
- `mechanics-http-client/src/request.rs` (edited —
  discovery dispatch in `send`).
- `mechanics-http-client/src/response.rs` (edited —
  Alt-Svc header parsing after first response).
- `mechanics-http-client/src/error.rs` (edited — new
  `QuicHandshake` + `Dns` variants under the feature).
- `mechanics-http-client/src/tls.rs` (edited if needed —
  exposing the `rustls::ClientConfig` to QUIC; the
  `aws-lc-rs` provider stays).
- `mechanics-http-client/src/https_rr.rs` (new — HTTPS RR
  query + parse + cache types).
- `mechanics-http-client/src/alt_svc.rs` (new — Alt-Svc
  parser + cache types).
- `mechanics-http-client/src/http3.rs` (new — QUIC client
  wrapper, h3 dispatch).
- `mechanics-http-client/src/tests.rs` or
  `mechanics-http-client/tests/http3.rs` (new test
  module).
- `mechanics-http-client/Cargo.toml` (edited — version
  bump, new feature, new gated deps).
- `mechanics-http-client/CHANGELOG.md` (edited —
  `[0.2.0]` entry).
- `Cargo.lock` (regenerates when cargo runs).

Files NOT to touch (flag if you find a reason to):

- Any file under `mechanics-core/`, `mechanics/`,
  `mechanics-config/`, `philharmonic*/`, `inline-blob/`,
  any other workspace member.
- The workspace `Cargo.toml` `[patch.crates-io]` block.
- `.claude/`, `docs/`, `docs-jp/`, `HUMANS.md`,
  `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md`,
  `scripts/`, `deny.toml`.

Git rules: signed-off + signed commits via
`scripts/commit-all.sh` only. **Codex never runs
`commit-all.sh`** — Claude commits after reviewing your
work. No `cargo publish`. No raw `git commit` /
`git push` / `git add` / `git reset` / `git rebase` /
`git revert`. Read-only `git log` / `git diff` /
`git show` is fine.
</action_safety>
</task>
