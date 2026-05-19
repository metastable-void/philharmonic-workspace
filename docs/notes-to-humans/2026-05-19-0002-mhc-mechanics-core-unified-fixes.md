# Unified fix proposal: `mhc` + `mechanics-core` state-machine review

**Date:** 2026-05-19 (Tue) JST
**Audience:** Yuka
**Source inputs:**
1. [Claude's state-machine review](2026-05-19-0001-mhc-mechanics-core-state-machine-review.md)
   (notes-to-humans 0001) — exhaustive state-machine enumeration
   with deficits per §10.0.1.
2. [Codex's state-machine review](../codex-reports/2026-05-19-0002-mhc-mechanics-core-state-machine-review.md)
   (codex-reports 0002) — production-symptom-oriented review with
   numbered findings.

**Purpose:** consolidate both reports into a single actionable
fix proposal. Each fix below is framed as a structural correction
(§10.0.1) — not a workaround for the surface symptom. Where the
two reports disagree or only one caught the issue, that is
called out explicitly.

**No code changes in this round** — this is a proposal for Yuka
to approve, redirect, or sequence. Dispatch suggestions (Claude
vs Codex vs Yuka-decides) are advisory.

---

## Report overlap analysis

What each report uniquely caught, what they agreed on:

| Issue | Claude 0001 | Codex 0002 |
|---|---|---|
| H3 cache expiry `Instant + Duration` reachable panic | **missed** | **caught** (Finding #1) |
| Retry policy treats deterministic local errors as retryable I/O | **missed** | **caught** (Finding #4) |
| Endpoint timeout aggregate-vs-per-attempt semantics | **missed** | **caught** (Finding #3) |
| H3 phase timers don't respect caller request deadline | noted as design choice (§1.4 table) | flagged as bug (Finding #2) |
| HTTPS RR lookup failures not negative-cached | **missed** | **caught** (Finding #6) |
| `map_legacy_error` substring heuristic | **caught** (§1.10.A) | **caught** (Finding #5) |
| Partial pool construction cleanup | **caught** (§2.10.J) | **caught** (Finding #7) |
| Tail-side errors silently swallowed when main replied | **caught** (§2.10.A) | missed |
| `H3ResponseBodyState::Ready(None)` dead state | **caught** (§1.10.E) | missed |
| Drop asymmetry in `H3ResponseBody` (Ready(Some) vs Reading) | **caught** (§1.10.F) | missed |
| Hyper pool config effectively dead | **caught** (§1.10.G) | missed |
| H3 negative cache flat 5-min (no exponential backoff) | **caught** (§1.10.B) | missed |
| Alt-Svc updates only on TCP/TLS responses | **caught** (§1.10.C) | missed |
| HTTPS RR write-lock release across await | **caught** (§1.10.D) | missed |
| aws-lc-rs install side effect process-global | **caught** (§1.10.H) | missed |
| `setTimeout` overflow yields silent dead timer | **caught** (§2.10.G) | missed |
| `drain_timeout_jobs` runs all due timers synchronously | **caught** (§2.10.F) | missed |
| Tail-promise quiescence holds worker for full `max_execution_time` | **caught** (§2.10.E) | missed |
| Worker-lifetime Boa Context + tokio runtime as silent-corruption surface | **caught** (§2.10.B/C) | missed |
| `run_source_with_early_reply` informal contract | **caught** (§2.10.D) | missed |
| Reply channel race between pool side and worker side | **caught** (§2.10.I) | missed |
| Trace points around endpoint attempt state | missed | **caught** (Production-Symptom Implications §) |

**Net:** Codex caught three bug-class issues I missed
(reachable panics, retry-policy mis-classification,
endpoint-timeout aggregate ambiguity) and one observability
suggestion. I caught the larger inventory of state-machine
asymmetries and cleanup items. The reports are complementary
and neither alone would have produced this fix list.

---

## Category 1 — Correctness fixes (must)

These are bug-class. Each can produce observable failures.

### 1.1 H3 cache expiry uses unchecked `Instant + Duration`
**(Codex Finding #1.)** Reachable panic on hostile or
accidentally-huge input.

**State-machine gap:** Two sites compute `now + duration` where
`duration` is untrusted. Rust's `std::ops::Add<Duration> for
Instant` calls `checked_add` internally and **panics** on
overflow. This violates CONTRIBUTING.md §10.3 ("no panics in
library `src/`"). The two sites:
- [mhc/src/alt_svc.rs:92](mechanics-http-client/src/alt_svc.rs#L92):
  `expires_at: now + max_age` where `max_age` is parsed from the
  upstream's `Alt-Svc: h3=":port"; ma=N` header. A malicious or
  buggy server can send `ma=18446744073709551615`.
- [mhc/src/request.rs:512-515](mechanics-http-client/src/request.rs#L512-L515):
  `now + client.inner.http3_negative_cache_duration`. The
  duration is local config, but it is a public-builder input
  (`ClientBuilder::http3_negative_cache_duration`) so an
  operator can pass `Duration::MAX`.

**State-machine framing:** the Alt-Svc and negative caches are
key-value maps `Origin → Instant`. The transition "insert with
TTL" must produce a value of type `Instant` that is valid.
Today the transition is partial — for some inputs there is no
valid output. The transition function should be total.

**Structural fix:**
- **Alt-Svc parse**: clamp `ma` at parse time to a sane
  operator-level maximum (24 h = 86 400 s is the conventional
  cap; RFC 7838 doesn't mandate but Chromium uses 24 h). Reject
  values that don't parse or exceed the clamp via
  `AltSvcUpdate::None` (already a documented variant for
  "header was syntactically wrong / unsupported").
- **Negative-cache insert**: `now.checked_add(duration)
  .unwrap_or(<sentinel-far-future>)`. The sentinel should be
  `Instant::now() + Duration::from_secs(60 * 60)` (1 h) — any
  failure to add a reasonable duration to `now` means the
  durations are pathological and a 1 h cooldown is the failsafe.
- **`ClientBuilder::http3_negative_cache_duration` validation**:
  reject `Duration` ≥ a hard cap (1 h is the natural ceiling
  given the use case) at build time.

**Tests required:**
- `Alt-Svc: h3=":443"; ma=18446744073709551615` → entry
  expires_at is clamped, no panic.
- `Alt-Svc: h3=":443"; ma=999999999999` → same.
- `ClientBuilder::http3_negative_cache_duration(Duration::MAX)
  .build()` → returns an error.

**Risk / complexity:** Low. Two narrow edits + one validator +
two tests.

**Dispatch:** **Codex** (touches mhc src/; non-trivial enough
to warrant the gate). Single-round.

---

### 1.2 Endpoint retry policy treats deterministic local errors as retryable I/O
**(Codex Finding #4.)** Retries that cannot possibly succeed.

**State-machine gap:** `execute_endpoint`'s retry loop applies
`retry_policy.should_retry_transport_error(&err)` to every
`Err(io::Error)` from `client.execute(req)`
([execute.rs:158](mechanics-core/src/internal/http/endpoint/execute.rs#L158)).
The default policy retries every non-timeout `io::Error`. But
`DefaultEndpointHttpClient::execute` ([transport.rs](mechanics-core/src/internal/http/transport.rs))
maps several **deterministic local** conditions to
`io::Error`:
- `ErrorKind::InvalidData` for body-cap violations
  (content-length-too-large at line 235, streamed-body-too-large
  at line 247).
- `ErrorKind::InvalidInput` for malformed request construction.
- Generic `Error::other(...)` for decode/serialise issues.

A workflow that sends a request whose response exceeds the cap
will retry `max_attempts` times — every attempt rebuilds the
same request, hits the same cap on the same upstream, and fails
the same way. Wasted budget, masked first cause.

**State-machine framing:** the retry policy distinguishes
*retryable transport conditions* (transient network state) from
*terminal conditions* (request/response shape is wrong). Today
the discrimination uses `io::ErrorKind`, but that enum is too
coarse — `InvalidData` is the same kind whether it came from a
TCP-level corruption (rare, theoretically retryable) or from a
content-length cap violation (always terminal). The state
machine treats them as the same transition.

**Structural fix:** introduce a typed transport-error layer at
the `EndpointHttpClient` boundary.

**Option A (preferred, larger):** change
`EndpointHttpClient::execute` to return
`Result<EndpointHttpResponse, EndpointTransportError>` where
`EndpointTransportError` is a new enum:

```rust
pub enum EndpointTransportError {
    Network(String),       // retryable: TCP/DNS/connect failures
    Timeout,               // retryable per retry_on_timeout policy
    BodyTooLarge { ... },  // terminal: cap violation
    InvalidRequest(String),// terminal: header/URL/serialise
    Decode(String),        // terminal: response decode
    Other(String),         // unknown — conservative: not retryable
}
```

`retry_policy.should_retry_transport_error(&err)` then
discriminates on the enum, not on `io::ErrorKind`. This is the
right state-machine model: the error type *encodes* its
retryability class.

**Option B (smaller surgery):** keep `io::Result` boundary but
narrow `should_retry_transport_error` to a positive-allowlist
of `ErrorKind`s: `ConnectionReset`, `ConnectionRefused`,
`ConnectionAborted`, `NetworkUnreachable`, `HostUnreachable`,
plus `TimedOut` gated by `retry_on_timeout`. Reject everything
else explicitly. This is fewer touched lines but leaves the
discrimination implicit.

I lean **Option A** because it makes the contract auditable.
Option B is a workaround that the next maintainer will have to
re-derive from the kind list.

**Tests required:**
- Body-cap violation does not retry.
- TCP connection-refused retries (existing behaviour preserved).
- Decode error does not retry.
- Header construction error does not retry.

**Risk / complexity:** Medium for Option A (touches the
`EndpointHttpClient` trait — public API change in
mechanics-core). Low for Option B.

**Dispatch:** **Codex**. If Option A, two rounds (define the
enum and migrate types in round 1; update retry policy + tests
in round 2). If Option B, one round.

---

### 1.3 Tail-side errors silently swallowed when main already replied
**(Claude §2.10.A.)** Observability bug — lost error signals.

**State-machine gap:** at the result-classification stage of
`run_source_with_early_reply`
([runtime.rs:411-413](mechanics-core/src/internal/runtime.rs#L411-L413)):

```rust
Err(e) if main_replied => {
    let _ = e;     // discard
    Ok(())
}
```

If a tail promise throws after main resolved
(`Promise.resolve().then(() => { throw 'x' })`), the JS-side
"unhandled rejection" produces a non-`DeadlineExceeded` `Err`
out of the tail-drive loop. The runtime correctly observes
`main_replied=true` and decides not to return Err to the worker
(main already produced its result), but **discards the error
without logging**. The only existing observability for tail
issues is `tail_poll_aborted`, which fires only on deadline
exceedance.

**State-machine framing:** the runtime's result-emission state
machine has two channels — main result (via `early_reply`) and
tail outcome (via the function's return). Today only the main
channel produces observable output. The tail channel is
silenced on errors when main was successful. That's correct for
caller semantics (main result wins) but wrong for observability
(operators want to see when tail promises misbehave).

**Structural fix:** one-line addition:

```rust
Err(e) if main_replied => {
    tracing::warn!(
        job_id = %job_id,
        error = %e,
        "tail promise produced an error after main resolved"
    );
    Ok(())
}
```

The warn produces a structured log entry; the actual reply
behaviour is unchanged.

**Tests required:** existing tail-promise tests can be extended
to assert the warn fires via `tracing-subscriber`'s test
subscriber facility.

**Risk / complexity:** Trivial.

**Dispatch:** **Claude** (housekeeping).

---

## Category 2 — Semantic gaps (should fix to lock the state machine)

These are not bugs — current behaviour is internally consistent.
But the state machine admits asymmetries or implicit decisions
that should be made explicit.

### 2.1 H3 phase timers don't respect caller request deadline
**(Codex Finding #2.)** Phase budget overrun.

**State-machine gap:** the per-request deadline (caller's
`timeout`) is plumbed into `try_http3` but applied only to
`recv_response`. Each pre-response phase uses its own constant:
- DNS lookup: 150 ms (`H3_DNS_LOOKUP_TIMEOUT`)
- QUIC connect: 500 ms (`H3_CONNECT_TIMEOUT`)
- h3 setup: 500 ms (`H3_CONNECT_TIMEOUT`)
- send_request: 150 ms (`H3_STREAM_OPEN_TIMEOUT`)
- send_data / finish: 500 ms each (`H3_STREAM_UPLOAD_TIMEOUT`)

Worst-case pre-response budget: 150 + 500 + 500 + 150 + 500 +
500 = **2300 ms**. A caller with `timeout(1500ms)` sees mhc
return after ≈ 2.3 s.

**State-machine framing:** the request-deadline state machine
should have one invariant: **at no point does mhc spend more
than `remaining(deadline)` blocking on a single call**. The
existing phase timeouts encode "this phase is unhealthy beyond
N" — useful for fail-fast even when there's no caller deadline.
But when there is a deadline, the effective per-phase budget
must be `min(phase_default, remaining(deadline))`.

**Structural fix:**
- Helper:
  ```rust
  fn phase_budget(phase_default: Duration, deadline: Option<Instant>) -> Duration {
      match deadline {
          None => phase_default,
          Some(d) => {
              let remaining = d.checked_duration_since(Instant::now())
                  .unwrap_or(Duration::ZERO);
              phase_default.min(remaining)
          }
      }
  }
  ```
- Apply at every `tokio::time::timeout(...)` site in http3.rs
  by replacing the constant with `phase_budget(constant,
  context.response_deadline)`.
- Preserve the distinction between phase failure (which can
  fall back to TCP/TLS via `Stream { retry_without_h3: true }`)
  and caller deadline exhaustion (which is terminal
  `Timeout`):
  - If the budget computed to ZERO because deadline already
    expired → return `Http3AttemptError::Timeout`.
  - If the phase fired its own bound (budget == phase_default)
    → return the existing phase error.
  - If the phase fired the deadline-derived budget → return
    `Timeout` (deadline exhausted mid-attempt).

**Tests required:**
- Sub-`H3_CONNECT_TIMEOUT` caller timeout (e.g. 100 ms) against
  a stale Alt-Svc → returns `Error::Timeout` within ≈ 100 ms.
- Caller timeout > sum-of-phases → existing behaviour
  preserved.

**Risk / complexity:** Medium. Touches all phase timeouts in
http3.rs (≈ 6 sites). Existing error variants are sufficient;
just plumbing.

**Dispatch:** **Codex**. One round.

---

### 2.2 Endpoint timeout aggregate-vs-per-attempt semantics undecided
**(Codex Finding #3.)** Implicit policy choice.

**State-machine gap:** `execute_endpoint` resolves `timeout_ms`
once at the top
([execute.rs:60](mechanics-core/src/internal/http/endpoint/execute.rs#L60)),
then passes it into each attempt's `EndpointHttpRequest` fresh.
With `max_attempts=3` and `timeout_ms=30000`, the worst-case
wall-clock is `3 × 30000 + retry_delays ≈ 90+ s`.

Two possible state-machine semantics:
1. **Per-attempt** (today's behaviour, undocumented): each
   attempt has its own budget; the endpoint call ends when all
   attempts are exhausted or one succeeds.
2. **Aggregate**: `timeout_ms` is the total wall-clock for the
   endpoint call, including retry sleeps. Each attempt's budget
   = `remaining(deadline)`.

The JS-side endpoint-call shape suggests aggregate ("call this
endpoint and tell me within Tms whether it worked") — the
script author isn't reasoning about retry topology. Per-attempt
semantics also interact awkwardly with Boa's
`max_execution_time`: if the endpoint future is awaited and the
Boa deadline fires first, JS sees a runtime-execution-timeout
rather than a clean endpoint error.

**Structural fix (aggregate):**
- Compute deadline once at top of `execute_endpoint`:
  `let deadline = timeout_ms.and_then(|ms| Instant::now().checked_add(...))`.
- Each attempt's `timeout_ms` field = `remaining(deadline)`
  converted to ms.
- Retry sleeps are bounded by `remaining(deadline)`; if
  remaining ≤ retry_delay, skip the sleep (or terminate).
- If `remaining(deadline) ≤ 0` before an attempt → return
  `io::Error(TimedOut, "endpoint call timed out across N attempts")`.

**Tests required:**
- `timeout_ms=1000, max_attempts=3, every attempt times out at
  400ms` → endpoint call returns after ≈ 1000 ms with timeout
  error (not after 1200+ ms).
- `timeout_ms=5000, attempt 1 succeeds in 100ms` → returns
  immediately with success.
- `timeout_ms=2000, attempt 1 returns 503, retry-after: 5` →
  returns after 2000 ms (skip the sleep that would exceed
  budget).

**Risk / complexity:** Medium. Touches `execute_endpoint` retry
loop + `DefaultEndpointHttpClient` deadline computation. The
retry-after interaction with the deadline is a small
state-machine — needs care.

**Dispatch:** **Yuka decides the semantic** (per-attempt vs
aggregate); then **Codex** implements one round. I lean
aggregate.

---

### 2.3 H3 negative cache has no exponential backoff
**(Claude §1.10.B.)** Wedged-origin retry storm surface.

**State-machine gap:** the negative cache key is just
`Origin → Instant`. Insert sets `expires_at = now +
http3_negative_cache_duration` (flat 5 min default). Every 5
min the next request burns the full H3 phase budget against the
still-wedged origin.

**State-machine framing:** the cache models "this origin
recently failed H3". It does not model "this origin has been
failing H3 for a long time". Both should produce different
retry cadences.

**Structural fix:** add a per-origin failure counter.

```rust
struct NegativeCacheEntry {
    expires_at: Instant,
    consecutive_failures: u32,
}
```

On insert:
- Lookup existing entry; copy `consecutive_failures + 1` (clamped).
- TTL = `base * 2^min(consecutive_failures, MAX_BACKOFF_POWER)`,
  clamped to e.g. 1 h.
- `expires_at = now + ttl` (with checked_add per §1.1).

On success (i.e. an H3 response succeeds): remove the origin
from the cache. The next failure starts fresh at the base TTL.

Optional: also surface success to a `success_at_or_after`
field, so an *unconditional* cache eviction after long success
streaks can happen.

**Tests required:**
- First failure → 5 min TTL.
- Three consecutive failures within a refresh window → 20 min
  TTL.
- Success after backoff → next failure starts fresh.

**Risk / complexity:** Medium. Touches negative-cache reads
and writes. Backward-compatible from the user's perspective.

**Dispatch:** **Codex** (after §1.1 lands; depends on the
checked-add fix).

---

### 2.4 HTTPS RR lookup failures are not cached
**(Codex Finding #6.)** Repeated 150 ms penalty per request.

**State-machine gap:**
[request.rs:452-479](mechanics-http-client/src/request.rs#L452-L479).
`https_rr_entry` caches successful `Some(entry)` results. Lookup
timeout (the 150 ms cap), `Ok(None)` (no records), and `Err(_)`
(DNS failure) all return None **without** caching the negative
result. The next request to the same origin pays the 150 ms
cost again.

**State-machine framing:** the cache should model both
positive ("this origin has an H3 RR") and negative ("this
origin has no H3 RR / lookup failed recently") outcomes. Today
it models only the positive side.

**Structural fix:** extend the cache value type:

```rust
enum HttpsRrCacheValue {
    Found(HttpsRrEntry),
    Negative { expires_at: Instant },
}
```

On lookup timeout or `Ok(None) | Err(_)`, insert
`Negative { expires_at: now + 30s }` (short TTL — newly-deployed
HTTPS RR records should be discovered quickly). On read, a
`Negative` hit returns None to the caller without performing
another lookup.

**Tests required:**
- Lookup that times out → cached as Negative; second request
  within 30 s does not re-attempt the lookup.
- Lookup that returns `Ok(None)` → same.
- After 30 s, lookup is re-attempted.

**Risk / complexity:** Low-Medium. Type change in https_rr.rs;
read/write sites in request.rs.

**Dispatch:** **Codex**. One round.

---

### 2.5 Alt-Svc cache only updates on TCP/TLS responses
**(Claude §1.10.C.)** Missed update channel.

**State-machine gap:** `maybe_update_alt_svc`
([request.rs:523-542](mechanics-http-client/src/request.rs#L523-L542))
fires only after the TCP/TLS path in `send()`. H3 responses
don't trigger any Alt-Svc update — so if the server flips
`Alt-Svc: clear` on the H3 path (e.g. mid-deployment), the
client doesn't observe it until either the negative-cache fires
(which won't happen for healthy H3 requests) or the existing
Alt-Svc entry's `ma=` expires.

**State-machine framing:** Alt-Svc is an HTTP response
extension — both TCP/TLS and H3 responses carry it. The cache
update should fire on both transports.

**Structural fix:** call `maybe_update_alt_svc` from
`Http3State::request` after constructing the `Response`. The
response's `parts.headers` is already in scope at
[http3.rs:97-102](mechanics-http-client/src/http3.rs#L97-L102).

Plumbing concern: `Http3State::request` doesn't currently take
the `Client` as a parameter (only `Http3Request`). Either pass
`&Client` (or the three relevant `Arc`s) into the request, or
return the Alt-Svc parse result from `Http3State::request` and
let the caller in `request_http3_with_stale_retry` apply it.
The second option is cleaner — it preserves `Http3State`'s lack
of dependency on the cache types.

**Tests required:**
- H3 response with `Alt-Svc: clear` evicts cached Alt-Svc.
- H3 response with `Alt-Svc: h3=":port"; ma=N` updates the
  cache.

**Risk / complexity:** Low.

**Dispatch:** **Codex** or **Claude** (small enough for
housekeeping; borderline). I lean Codex since it touches mhc
src/ and is part of a coordinated state-machine cleanup.

---

### 2.6 `setTimeout` overflow yields silent dead timer
**(Claude §2.10.G.)** Silent failure.

**State-machine gap:** in `Queue::enqueue_job` for
`TimeoutJob`
([executor.rs:340-342](mechanics-core/src/internal/executor.rs#L340-L342)):

```rust
let at = Self::instant_checked_add(now, t.timeout().into()).unwrap_or_else(|| {
    Self::js_instant_from_millis(u64::MAX).unwrap_or(JsInstant::new(u64::MAX, 0))
});
self.timeout_jobs.borrow_mut().entry(at).or_default().push(t);
```

If `setTimeout(fn, Number.MAX_SAFE_INTEGER)` overflows the
JsInstant arithmetic, the timer is scheduled at the sentinel
`u64::MAX` and will never fire. `fn` is held by the BTreeMap
entry until job teardown.

**State-machine framing:** the timer enqueue transition should
be total — every legal input maps to a valid scheduled state OR
to a runtime error. Today the failure path silently puts the
timer in an unreachable state.

**Structural fix:** on overflow, route to the existing
"unsupported job" error path:

```rust
let Some(at) = Self::instant_checked_add(now, t.timeout().into()) else {
    let realm = context.realm().clone();
    let err = GenericJob::new(
        |_| Err(JsError::from_native(
            JsNativeError::range().with_message(
                "setTimeout delay is too large for the current platform clock"
            ),
        )),
        realm,
    );
    self.generic_jobs.borrow_mut().push_back(err);
    return;
};
```

The script then sees a catchable `RangeError` instead of a
silently-dropped timer.

**Tests required:**
- `setTimeout(fn, Number.MAX_VALUE)` from JS rejects with
  RangeError.

**Risk / complexity:** Trivial.

**Dispatch:** **Claude** (housekeeping).

---

### 2.7 `map_legacy_error` substring heuristic
**(Both reports: Claude §1.10.A, Codex Finding #5.)** Fragile
classification.

**State-machine gap:** `map_legacy_error`
([request.rs:544-558](mechanics-http-client/src/request.rs#L544-L558))
classifies hyper-util errors by string substring matching.
Future hyper-util point releases that change error wording will
silently misclassify. Codex also notes a specific bad
interaction: DNS resolver timeout from `HyperDnsResolver`
returns `io::ErrorKind::TimedOut`, but after hyper wraps it,
mhc reads the message and classifies as `Unreachable` because
"dns" is in the message — losing the timeout kind.

**State-machine framing:** error classification is a function
from the error type to a closed set of mhc error variants. The
function should be total and decidable from the error
structure, not from its `Display` output.

**Structural fix:** walk `err.source()` chain looking for typed
errors:

```rust
fn map_legacy_error(err: hyper_util::client::legacy::Error) -> Error {
    let mut source: &(dyn std::error::Error + 'static) = &err;
    loop {
        if let Some(io) = source.downcast_ref::<std::io::Error>() {
            return match io.kind() {
                std::io::ErrorKind::TimedOut => Error::Timeout,
                std::io::ErrorKind::ConnectionRefused
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::NetworkUnreachable
                | std::io::ErrorKind::HostUnreachable
                | std::io::ErrorKind::NotFound /* DNS NXDOMAIN */ => {
                    Error::Unreachable(err.to_string())
                }
                _ => Error::Internal(err.to_string()),
            };
        }
        if let Some(hyper_err) = source.downcast_ref::<hyper::Error>() {
            if hyper_err.is_timeout() { return Error::Timeout; }
            if hyper_err.is_canceled() { return Error::Cancelled(err.to_string()); }
            // ... other hyper::Error predicates
        }
        match source.source() {
            Some(s) => source = s,
            None => return Error::Internal(err.to_string()),
        }
    }
}
```

This requires the `hyper::Error` type to actually appear in the
source chain — which it does in hyper-util's legacy::Error
implementation (worth verifying by reading hyper-util source).

**Tests required:**
- `HyperDnsResolver::call` returns `TimedOut` → mhc classifies
  as `Error::Timeout`.
- Connection refused → `Error::Unreachable`.
- TLS handshake failure (rustls error in source) → `Error::Tls`.

**Risk / complexity:** Medium. Reading hyper-util source to
confirm the chain.

**Dispatch:** **Codex** with a careful prompt to read
hyper-util first.

---

## Category 3 — Architecture decisions (Yuka calls)

These are not fixes — they're choices the workspace has been
deferring. Each needs a decision before any code lands.

### 3.1 Hyper pool config is effectively dead
**(Claude §1.10.G.)**

**Current state:** `pool_max_idle_per_host(0)` is set in
`MechanicsPool::new` ([api.rs:121](mechanics-core/src/internal/pool/api.rs#L121));
`DefaultEndpointHttpClient::execute` calls `fresh_transport()`
per request ([transport.rs:196](mechanics-core/src/internal/http/transport.rs#L196)).
Net effect: no idle reuse, ever. But the pool config knobs are
still wired through `ClientBuilder` and `ClientInner` and
`fresh_transport`.

**The decision:** commit to one of:
- **(A) No reuse, ever.** Drop `pool_max_idle_per_host` /
  `pool_idle_timeout` from `ClientBuilder` public API; remove
  the corresponding fields from `ClientInner`; document that
  mhc is a per-request transport. Pros: removes dead config
  surface; simplifies the state machine. Cons: backward-
  incompatible API change.
- **(B) Re-enable reuse.** Drop the `pool_max_idle_per_host(0)`
  override in `MechanicsPool::new`; drop the `fresh_transport()`
  per request in `DefaultEndpointHttpClient::execute`. Pros:
  recovers connection-reuse perf. Cons: re-introduces the
  pool-poisoning class that motivated today's per-call DNS fix.

**My lean:** (A) given the production-symptom history. But
this is a deployment-shape decision, not a code-correctness
one.

**Dispatch:** **Yuka decides** → **Codex** (one round either
direction).

---

### 3.2 `run_source_with_early_reply` informal Ok(()) contract
**(Claude §2.10.D.)**

**Current state:** the worker treats `Ok(())` from
`run_source_with_early_reply` as "main reply was already sent
via the closure". If the runtime is ever refactored to return
`Ok(())` without firing `early_reply`, the pool side times out
silently.

**Structural fix candidate:** change the return type to
`Result<RunOutcome, MechanicsError>` where:

```rust
enum RunOutcome {
    MainReplied,  // early_reply was invoked; tail may or may not have errored
    NoMainReply,  // shouldn't happen on healthy paths; signals contract bug
}
```

Or even stronger: take ownership of the reply sender inside
`run_source_with_early_reply` (no closure passed in) and have
the function return `Result<Value, MechanicsError>`. The worker
then unambiguously knows what to send.

**Risk / complexity:** Medium (touches the runtime contract).

**Dispatch:** **Claude** drafts a design note; **Codex**
implements.

---

### 3.3 Boa Context + tokio runtime per-worker lifetime
**(Claude §2.10.B/C.)** Silent-corruption surface.

**Current state:** each worker thread builds one
`RuntimeInternal` at spawn, which owns one Boa `Context` and
one `tokio::runtime::Runtime`. Both live for the worker's
entire lifetime. If either accumulates corruption (Boa heap
leak, tokio dangling task), all subsequent jobs on that worker
inherit it. No "reset" path short of killing the worker.

**The decisions:**
1. Worker self-recycle after N jobs? Adds throughput cost from
   re-init; bounds long-tail corruption.
2. Worker self-recycle on detected slow path (e.g. tail-poll-
   aborted)? Treats observable warning signs as triggers for
   recycle.
3. Status quo + monitoring? Add a memory-stat gauge to
   `MechanicsPoolStats`; let the operator decide.

**My lean:** (3) plus a future (2) once we have telemetry. But
this is speculative; production has not yet pointed at this
class.

**Dispatch:** **Yuka decides** when she sees telemetry. No
action this round.

---

### 3.4 Tail-promise quiescence holds worker for full timeout
**(Claude §2.10.E.)**

**Current state:** a workflow whose main returns in 50 ms but
launches `setTimeout(noop, 10_000)` blocks the worker for 10 s.
This is intended behaviour (fire-and-forget async) but couples
worker throughput to `max_execution_time`.

**The decision:** introduce a separate "tail timeout" shorter
than `max_execution_time`? Or accept the current shape and
document?

**My lean:** accept and document. Adding a second timeout
multiplies the state-machine surface for marginal benefit.

**Dispatch:** **Yuka decides**. If accept-and-document, that's
a CLAUDE.md or design-doc entry — Claude housekeeping.

---

## Category 4 — Cleanup (low-risk hygiene)

These are state-machine simplifications. No behaviour change.

### 4.1 Delete `H3ResponseBodyState::Ready(None)` dead variant
**(Claude §1.10.E.)**

No reachable transition produces `Ready(None)`. Either delete
the variant or document a path that should reach it.

**Dispatch:** **Claude** (one-line edit). Pre-req: confirm via
tests that nothing depends on the variant.

---

### 4.2 Drop asymmetry in `H3ResponseBody`
**(Claude §1.10.F.)**

`Drop` in `Ready(Some(stream))` explicitly cancels the stream;
in `Reading(future)` it relies on h3-quinn teardown. Add a test
confirming peer-observed reset is equivalent under both paths.

**Dispatch:** **Codex** (test-only; needs h3 fixture).

---

### 4.3 HTTPS RR write-lock release across await
**(Claude §1.10.D.)**

Read → drop lock → DNS lookup → reacquire lock. Racing callers
may all do the lookup; last-writer-wins. Not incorrect; just
wasteful.

**Structural fix:** add a per-origin in-flight set to dedupe
concurrent first-touch lookups. A `Mutex<HashSet<Origin>>` plus
a "wait for existing in-flight" via `Notify` or `oneshot`. Or
accept the waste and document.

**My lean:** accept and document. The dedupe complexity is
larger than the savings.

**Dispatch:** **Claude** (docs only) if accepting.

---

### 4.4 Partial pool construction cleanup
**(Both reports: Codex Finding #7, Claude §2.10.J.)**

If a later worker spawn fails after earlier workers succeeded,
`MechanicsPool::new` returns Err and `Drop` doesn't run. Already-
spawned workers exit on channel drop; their handles drop
without explicit join.

**Structural fix:** introduce a `PoolConstructor` RAII guard
that requests shutdown and joins any already-started workers on
drop. The guard releases ownership on successful construction;
on Err return, drop fires and cleans up.

**Tests required:** force a worker spawn failure mid-pool
construction (test-only hook exists in
`force_worker_runtime_init_failure`); assert no leaked worker
thread.

**Risk / complexity:** Low.

**Dispatch:** **Codex** (one round).

---

## Category 5 — Observability (Codex's production-symptom suggestions)

### 5.1 Add transition tracing around endpoint attempt state
**(Codex Production-Symptom Implications §.)**

Without explicit tracing, the production symptom ("JS catches
timeout, no target-origin packets") cannot be pinpointed to a
specific state. Add `tracing::debug!` (or info, with low
cardinality) at:

- JS endpoint entry (the Boa native function).
- `execute_endpoint` attempt start (with attempt number).
- `DefaultEndpointHttpClient::execute` before `fresh_transport()`.
- `DefaultEndpointHttpClient::execute` before `req.send()`.
- mhc H3 cache decision (negative-hit / Alt-Svc-hit / RR-hit /
  none).
- mhc TCP/TLS fallback start.
- DNS lookup start (`HyperDnsResolver::call`, `mechanics-dns`
  internal trace point).
- hyper `request()` start.

Each transition gets a structured log entry with the origin and
attempt number, so a production tcpdump can be correlated with
the mhc state at each instant. This is the right diagnostic
tooling to have BEFORE the next time the symptom appears, not
after.

**Risk / complexity:** Low to medium. Touches many sites but
each is one line.

**Dispatch:** **Codex** (one round, focused).

---

## Suggested sequencing

Ordered by dependency + priority:

**Wave 1 — bug fixes (must-do):**
1. §1.1 H3 cache expiry overflow → Codex (single round).
2. §1.3 Tail-side error tracing → Claude (one-line).
3. §1.2 Retry policy classification → Codex (Option A two rounds, or Option B one round).

**Wave 2 — semantic locking:**
4. §2.1 H3 phase timer deadline coordination → Codex.
5. §2.2 Endpoint timeout aggregate semantics → Yuka decides → Codex.
6. §2.4 HTTPS RR negative cache → Codex.
7. §2.5 Alt-Svc updates on H3 responses → Codex.
8. §2.6 setTimeout overflow → Claude.
9. §2.7 map_legacy_error substring → Codex (depends on hyper-util source review).

**Wave 3 — backoff (depends on Wave 1):**
10. §2.3 H3 negative cache exponential backoff → Codex.

**Wave 4 — cleanup:**
11. §4.1 Delete Ready(None) → Claude.
12. §4.2 Drop asymmetry test → Codex.
13. §4.4 Partial pool construction guard → Codex.

**Wave 5 — observability:**
14. §5.1 Endpoint-attempt tracing → Codex.

**Architecture decisions (parallel to all waves):**
- §3.1 Pool config dead — Yuka.
- §3.2 run_source_with_early_reply contract — Claude design + Codex impl.
- §3.3 Per-worker lifetime — Yuka, no action this round.
- §3.4 Tail quiescence — Yuka, accept-and-document if so.
- §4.3 HTTPS RR write-lock — Claude doc if accepting.

Total estimated work: **6–9 Codex dispatches** plus a handful
of Claude housekeeping commits.

---

## Things I am NOT proposing

For the record, things I deliberately did not propose:

1. **Workarounds for production symptom.** The 2026-05-19
   per-call DNS + 3s timeout fix already shipped; whether it
   resolves the production wedge can only be observed on
   production. Adding more speculative defensive code without
   production confirmation would compound complexity. Wave 1 is
   bug-class regardless; Wave 5 (tracing) is what positions the
   workspace to diagnose if the symptom recurs.
2. **Reverting today's per-call DNS change.** Codex's report
   notes the pre-socket pre-target bounds are now seconds, not
   minutes. The change is structurally sound; whether it fixes
   production is a separate question that doesn't affect
   correctness here.
3. **Changing the Boa async-rejection semantic.** The long
   comment at runtime.rs:335-366 documents a deliberate
   "warning not kill" choice for main-side unhandled
   rejections. Both reports leave this alone; I do too.
4. **Replacing crossbeam channels or tokio current-thread
   runtime.** The pool/runtime stack is coherent; no proposal
   touches it.
5. **Adding lock-free or RwLock-tuned cache implementations.**
   The three H3 caches use simple `RwLock<HashMap>`. Codex's
   "RwLock operations are short and do not hold locks across
   `.await`" observation confirms the current shape is fine.

---

## Closing

The two reports together produce a clear punch list. The
correctness fixes (Category 1) are the only items that produce
observable wrong behaviour today; everything else is
state-machine simplification or operator-comfort. Codex caught
the high-priority correctness issues I missed
(reachable-panic-on-overflow + retry-classification +
endpoint-timeout-aggregate). I caught the asymmetry inventory
and the observability gap. Neither alone would have produced
this list.

Per §10.0.1: each fix above identifies the state-machine model
the change should produce, not just the surface symptom. The
"structurally correct" framing is what makes the fix list
review-able — Yuka can confirm or redirect any individual fix
on its merits without needing to re-derive the underlying state
machine.

Awaiting your call on Wave 1 sequencing and any redirects on
the architecture decisions in Category 3.
