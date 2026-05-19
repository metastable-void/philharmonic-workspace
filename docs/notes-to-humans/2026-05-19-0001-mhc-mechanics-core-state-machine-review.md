# State-machine review: `mechanics-http-client` + `mechanics-core`

**Date:** 2026-05-19 (Tue) JST
**Audience:** Yuka
**Scope:** Full state-machine review of the two crates, per Yuka's
2026-05-19 request. Read in order with the `§10.0.1 structural
correctness` lens from CONTRIBUTING.md — name each state machine,
its states, its transitions, its invariants, and surface the parts
I cannot construct cleanly rather than wave hands.

**Submodule heads at review time:**
- `mechanics-http-client` @ `7a710ca` (today's per-call DNS +
  3s TCP/TLS DNS lookup timeout change)
- `mechanics-core` @ `a7354bf` (D7+D19 audit sweep / latest main)

**Coverage caveat (deficit surfaced up front):** Boa engine
internals are not directly modelled here. I cover the parts of
those state machines that are visible at the mechanics-core
boundary (`Context`, `Module`, `JsPromise::state`,
`runtime_limits`, `JobExecutor` hook). Internal Boa
sub-state-machines — module-evaluation graph, microtask draining
semantics inside `module.load_link_evaluate`, GC mark/sweep,
`clear_kept_objects` semantics, promise rejection tracker
interactions with `NativeFunction::from_async_fn` — are treated
as opaque dependencies whose behaviour I can describe at the
contract level only. Where this matters for an invariant, the
report says so.

---

## Part 1: `mechanics-http-client` (mhc)

### 1.1 `Client` lifecycle state machine

**States:**
- *Constructed* — `ClientBuilder::build()` succeeded; an
  `Arc<ClientInner>` is live.
- *Cloned* — `Client::clone()` is a cheap `Arc::clone`. All
  clones share the same `ClientInner`.
- *Refreshed* — `Client::fresh_transport()` allocates a **new**
  `ClientInner` with a freshly-built `hyper` legacy client (new
  TCP/TLS connection pool, new `HttpsConnector`,
  per-call-fresh-resolver) but with the four H3 discovery
  caches (`https_rr_cache`, `alt_svc_cache`, `negative_cache`,
  `http3: Arc<Http3State>`) **shared** via `Arc::clone` with
  the original. Two distinct `ClientInner`s post-refresh, both
  ultimately point at the same H3 discovery state.

**Invariants the type system already enforces:**
- `Client` is `Clone`; `ClientInner` is not. Cheap clone via
  `Arc`; deep clone via `fresh_transport()`.
- `fresh_transport()` carries forward exactly the **request
  defaults** and **H3 discovery state**, not the TCP/TLS pool.
  Verified by inspection of [client.rs:151-177](mechanics-http-client/src/client.rs#L151-L177).

**Invariant the code enforces but the type system doesn't:**
- `aws-lc-rs` is installed as the rustls default
  `CryptoProvider` lazily on first `build()`. There is no
  per-`Client` provider — it is process-global. Re-`build()`s
  re-install the same provider. Effectively a once-per-process
  side effect; the comment in lib.rs spells this out.

**State machine of the `hyper` pool inside `ClientInner.hyper`:**
- I treat this as **opaque hyper-util state**. The visible
  contract: `pool_max_idle_per_host` and `pool_idle_timeout`
  control idle reuse. In the current `MechanicsPool` wiring
  ([pool/api.rs:121](mechanics-core/src/internal/pool/api.rs#L121))
  `pool_max_idle_per_host(0)` disables idle reuse entirely; and
  `DefaultEndpointHttpClient::execute` calls
  `fresh_transport()` on every single request anyway. So in the
  production path, the hyper pool's idle-reuse state machine is
  **dead state** — every endpoint call is a fresh `ClientInner`
  whose pool will never serve a second request.

  This makes the "pool" abstraction in `ClientInner.hyper`
  effectively a single-shot transport. That's an intentional
  isolation choice (per the comment), but it means the
  state-machine cost of reasoning about pool reuse is being
  paid for nothing in this workspace. Flag for future
  consideration: either commit to the no-reuse model and stop
  wiring pool config through, or commit to reuse and stop
  freshening per request.

### 1.2 Per-request pipeline state machine
([request.rs:173-245](mechanics-http-client/src/request.rs#L173-L245))

**States** of `RequestBuilder::send()`:

```
[chain] → [validated] → ?H3-eligible → ?H3-result → [TCP/TLS path] → [response]
                              │            │
                              │            ├─ Ok(Some(response))         ──→ return
                              │            ├─ Ok(None) (fall through)    ──→ TCP/TLS
                              │            └─ Err(_)                     ──→ propagate
                              │
                              └─ skip (H3 disabled / non-HTTPS / streaming body)
```

**Transitions:**
- `[chain] → [validated]`: deferred-error short-circuits;
  deferred URI parse fail surfaces; `uri = Result<Uri>?`
  resolves; deadline computed from `timeout.or(default_timeout)`.
- `?H3-eligible`: triggered iff
  `http3_enabled && Origin::from_uri(uri).is_some() &&
  body.replayable_for_h3().is_some()`. `Origin::from_uri` only
  returns Some for `https://` schemes — H3 path is HTTPS-only by
  construction. The `replayable_for_h3` returns None for
  `RequestPayload::Streaming(_)`, so streaming bodies skip H3.
- `[H3-result]`: see §1.3 for the sub-state-machine.
- `[TCP/TLS path]`: `client.inner.hyper.request(req)` →
  optionally bounded by `tokio::time::timeout(remaining(deadline))`.
  Errors mapped through `map_legacy_error` (substring heuristic
  — see §1.10 deficit).
- After TCP/TLS response: `maybe_update_alt_svc` parses the
  `Alt-Svc` header and updates the per-origin cache.

**Invariants:**
- The H3 path's `Ok(None)` fall-through and the TCP/TLS path
  build separate `Request` objects. The H3 one uses an empty `()`
  body (`request_for_http3`); the TCP/TLS one uses the original
  body. Headers are computed once and reused.
- `maybe_update_alt_svc` only fires on the TCP/TLS path — H3
  responses don't update Alt-Svc state. See §1.10 deficit.

### 1.3 H3 attempt-retry state machine
([request.rs:288-402](mechanics-http-client/src/request.rs#L288-L402))

**`try_http3` outer state:**
1. **negative-cache check** ([request.rs:297-299](mechanics-http-client/src/request.rs#L297-L299)):
   if hit, return `Ok(None)` immediately. Fall through to TCP/TLS.
2. **Alt-Svc lookup** ([request.rs:301-320](mechanics-http-client/src/request.rs#L301-L320)):
   if fresh, attempt H3 against the advertised authority/port
   with `addresses=&[]` (no IP hints). On `Ok(Some(response))`
   return. On `Ok(None)` proceed to HTTPS RR path.
3. **HTTPS RR lookup** ([request.rs:322-339](mechanics-http-client/src/request.rs#L322-L339)):
   bounded by `H3_HTTPS_RR_LOOKUP_TIMEOUT` (150 ms) at the
   call site. Cache-then-lookup with eviction-on-stale. If
   `has_h3`, attempt H3 against discovered port + ipv4hint /
   ipv6hint addresses. Same Ok/None handling.
4. If neither succeeded: return `Ok(None)` → TCP/TLS fallback.

**`request_http3_with_stale_retry` inner attempt loop**
([request.rs:354-402](mechanics-http-client/src/request.rs#L354-L402)):
- Loop bound: 2 attempts (`for attempt in 0..2`).
- Each iteration:
  - Arm `H3AttemptCancellationGuard` (RAII; on `Drop` while
    armed → negative-cache + Alt-Svc evict).
  - Call `client.inner.http3.request(...)` — see §1.4.
  - Disarm guard.
  - Match result:
    - `Ok(response)` → return `Some(response)`.
    - `Err(Handshake(_))` → negative-cache, return `None`
      (fall to TCP/TLS). Note: Handshake does NOT trigger the
      Alt-Svc evict via `insert_negative` — the function
      `insert_negative` itself clears Alt-Svc.
    - `Err(Timeout)` → negative-cache, return `Err(Timeout)`
      (propagate as request-level timeout; the caller knows
      this origin is unhealthy for H3).
    - `Err(Stream { retry_without_h3: true })` at `attempt==0`
      → `continue` (retry fresh H3 attempt).
    - `Err(Stream { retry_without_h3: true })` at `attempt==1`
      → negative-cache, return `None`.
    - `Err(Stream { retry_without_h3: false })` → negative-
      cache, return `None`. (No retry; already past the
      "request started" point.)
- After loop falls off (both attempts produced
  `retry_without_h3: true` failures): negative-cache, return
  `None`.

**`retry_without_h3` semantics** (from http3.rs):
- `true` is set only when the failure happened **before the
  request was visibly observable to the upstream** —
  specifically the `send_request` send (line 84-89 of
  [http3.rs](mechanics-http-client/src/http3.rs#L79-L89)).
- `false` is set when the failure happened after request bytes
  were committed — `send_data`, `finish`, response header
  receive, etc. Per
  [http3.rs:329-333](mechanics-http-client/src/http3.rs#L329-L333):
  `stream_error_after_request_started()`.

**Cancellation-guard invariant:**
- If the H3 future is cancelled by an outer deadline/await,
  the guard's `Drop` fires `insert_negative(...)` with
  `Instant::now()`. This catches the "JS endpoint timed out
  with no observable target traffic" class of failure —
  the next request through this client will skip H3 instead of
  re-burning the deadline on the same wedged origin.
- Verified by [request.rs:585-624](mechanics-http-client/src/request.rs#L585-L624)
  unit tests.

### 1.4 `Http3State::request` attempt internal state machine
([http3.rs:67-103](mechanics-http-client/src/http3.rs#L67-L103))

Phases, in order, each with its own bound:

| Phase                 | Bound                                  | Failure → `Http3AttemptError`                |
|-----------------------|----------------------------------------|-----------------------------------------------|
| DNS / address pick    | 150 ms (`H3_DNS_LOOKUP_TIMEOUT`)       | `Handshake(string)`                           |
| QUIC connect          | 500 ms (`H3_CONNECT_TIMEOUT`)          | `Handshake(string)`                           |
| h3 setup              | 500 ms (`H3_CONNECT_TIMEOUT`)          | `Handshake(string)`                           |
| `send_request` headers| 150 ms (`H3_STREAM_OPEN_TIMEOUT`)      | `Stream { retry_without_h3: true }`           |
| `send_data` body      | 500 ms (`H3_STREAM_UPLOAD_TIMEOUT`)    | `Stream { retry_without_h3: false }`          |
| `finish` close-write  | 500 ms (`H3_STREAM_UPLOAD_TIMEOUT`)    | `Stream { retry_without_h3: false }`          |
| `recv_response`       | **absolute request deadline** (or unbounded if none) | timeout → `Timeout`; other → `Stream { retry_without_h3: false }` |
| Wrap response         | n/a                                    | n/a                                           |

**Invariants:**
- A **fresh** Quinn `Endpoint` is built per attempt
  ([http3.rs:136-149](mechanics-http-client/src/http3.rs#L136-L149)).
  Bind family matches the resolved remote IP (`0.0.0.0:0` for
  IPv4, `[::]:0` for IPv6). Verified by
  [test http3_state_uses_fresh_ipv4_udp_endpoint_for_ipv4_targets](mechanics-http-client/src/http3.rs#L359-L377).
- After `connection()` returns, a tokio task is spawned to
  `driver.wait_idle().await` — drives the h3 connection until
  it goes idle. The spawned task is "fire and forget"; nothing
  joins it. **It is implicitly killed** when the `quinn::Endpoint`
  is dropped (the endpoint's `_endpoint` is held by `H3ResponseBody`;
  see §1.5).
- Pre-response phases (DNS, connect, setup, stream-open,
  upload) have tight phase timeouts; once the request has been
  accepted (response headers + body waits), the loop falls back
  to the caller's absolute deadline. This is the
  "no-arbitrary-short-probe-after-request-accepted" pattern
  Yuka spelled out in [client.rs:148-150](mechanics-http-client/src/client.rs#L148-L150).
- `H3_KEEP_ALIVE_INTERVAL = 15s`,
  `H3_MAX_IDLE_TIMEOUT = 120s`. Per-attempt transport config.

### 1.5 `H3ResponseBody` lifecycle state machine
([http3.rs:178-271](mechanics-http-client/src/http3.rs#L178-L271))

**States** (`H3ResponseBodyState`):
- `Ready(Some(stream))` — initial; between frames.
- `Ready(None)` — appears in the enum but I don't see a code
  path that reaches it. After `take()` we always set
  `Reading(future)` immediately, and after `Reading` returns a
  data frame we set `Ready(Some(stream))`. After `Reading`
  returns `Ok(None)` (end of body) we set `Done`, not
  `Ready(None)`. Treat as **dead state** worth deleting (see
  §1.10 deficit).
- `Reading(future)` — recv_data future in-flight.
- `Done` — terminal; all subsequent polls return `Poll::Ready(None)`.

**Transitions** (`poll_frame`):
- `Ready(Some(stream))` → `Reading(future)` (start recv).
- `Reading(future)` → on `Ok(Some(bytes))` → `Ready(Some(stream))`,
  yield `Frame::data(bytes)`.
- `Reading(future)` → on `Ok(None)` → `Done`, yield `None`.
- `Reading(future)` → on `Err(e)` → `Done`, yield `Some(Err(e))`.
- `Done` → no transition; yields `None` forever.

**Drop behaviour** ([http3.rs:207-213](mechanics-http-client/src/http3.rs#L207-L213)):
- `Ready(Some(stream))` → explicit `cancel_request_stream(stream)`
  → `stop_sending(H3_REQUEST_CANCELLED)` + `stop_stream(H3_REQUEST_CANCELLED)`.
- `Reading(future)` → no explicit cancel. The future owns the
  stream by value; dropping the future drops the stream. h3-quinn
  is expected to handle in-flight stream cancel cleanly on drop,
  but the asymmetry with the explicit cancel above is worth
  noting (see §1.10 deficit).
- `Done` → no-op.

**Ownership invariant** (the one Yuka cares about for "fresh
QUIC endpoint per attempt"):
- `H3ResponseBody` holds **two** retained owners as `_sender`
  and `_endpoint` (`H3SendRequest`, `quinn::Endpoint`). Both
  are kept alive until the body is dropped. Verified by
  [test dropping_h3_send_request_before_body_reproduces_client_close](mechanics-http-client/src/http3.rs#L409-L440)
  which demonstrates that dropping them prematurely breaks the
  body. This is the load-bearing invariant for "the QUIC
  endpoint lives exactly as long as it's needed for one
  request".

### 1.6 Cache state machines

Three independent per-`Origin` caches, all behind
`Arc<RwLock<HashMap<Origin, _>>>`. All three share a common
"check, evict-if-stale, optionally insert" idiom.

#### 1.6.1 Negative cache
([request.rs:496-520](mechanics-http-client/src/request.rs#L496-L520))

- **Insert**: `expires_at = now + http3_negative_cache_duration`
  (default 5 min). On insert, also **remove the Alt-Svc entry
  for the same origin**.
- **Read** (`negative_cache_hit`): if `expires_at > now` → hit;
  else evict + miss.
- **Eviction triggers**:
  - H3 attempt error (Handshake, Timeout, Stream non-retryable,
    or after retry budget exhausted).
  - `H3AttemptCancellationGuard::Drop` if armed (outer
    cancellation).
- **Implicit semantics**: a 5-min cooldown is a flat rate — no
  exponential backoff. After 5 min, the next H3 attempt to a
  persistently-broken origin will re-burn the H3 phase budget
  before falling through. See §1.10 deficit.

#### 1.6.2 Alt-Svc cache
([alt_svc.rs](mechanics-http-client/src/alt_svc.rs) + [request.rs:482-542](mechanics-http-client/src/request.rs#L482-L542))

- **Insert source**: `maybe_update_alt_svc` parses the
  `Alt-Svc` response header on **TCP/TLS responses only** (the
  `request.rs:243` site fires only after the hyper request path
  returns). H3 responses do not trigger this — see §1.10.
- **Update kinds** (`AltSvcUpdate`):
  - `Clear` — `Alt-Svc: clear` header value → cache.remove.
  - `Entry(AltSvcEntry)` — valid `h3="...:port"; ma=N` parse →
    cache.insert with `expires_at = now + ma`.
  - `None` — header missing, parse failed, or no `h3` variant
    (e.g. `h3-29` draft) → no-op.
- **Eviction**: lazy on read; also forced by `insert_negative`
  on the same origin.
- **Parser** discards draft `h3-XX` variants and accepts only
  the final `h3` token. Verified by
  [alt_svc.rs test ignores_draft_h3_variants](mechanics-http-client/src/alt_svc.rs#L166-L169).

#### 1.6.3 HTTPS RR cache
([https_rr.rs](mechanics-http-client/src/https_rr.rs) + [request.rs:452-479](mechanics-http-client/src/request.rs#L452-L479))

- **Insert source**: `https_rr::lookup` via the per-call
  `mechanics_dns::Resolver` (today's per-call change applies
  here too). Bounded by `H3_HTTPS_RR_LOOKUP_TIMEOUT` (150 ms) at
  the call site.
- **Pick-best heuristic** (`https_rr::lookup`): scan returned
  records; if any has `alpn=h3`, return that one immediately;
  else return the first record observed. TTL from RR.
- **Read** (`https_rr_entry`): take write-lock, check
  freshness, evict-if-stale, **drop lock**, do network lookup,
  **reacquire write-lock** to insert. Two-phase locking around
  the await point — see §1.10 deficit.

### 1.7 Response body collection state machine
([response.rs](mechanics-http-client/src/response.rs))

- `Response::body` is `Option<ResponseBody>` where
  `ResponseBody` is `Hyper(hyper::body::Incoming)` or
  `H3(Box<H3ResponseBody>)`. The `Option` becomes `None` on
  body consumption (`into_body` or `bytes_with_cap`).
- `bytes_with_cap(max)` state:
  1. Take body out of the Option.
  2. Loop `next_frame` with deadline-bounded `tokio::time::timeout`.
  3. For each frame: try `.into_data()` → if data, check
     `new_total = buf.len() + data.len() > max_bytes`, error
     `BodyTooLarge` on overflow; else extend buf.
  4. End-of-body → freeze, return.
  5. After return: read `Content-Encoding`, decompress
     (`identity` / `gzip` / `x-gzip` / `deflate` / `br`).
- The cap applies to **wire bytes**; decompressed payload may
  exceed the cap. Documented invariant in lib.rs.

**Invariant**: the deadline propagates through the body
collection. Even if response headers arrived before the
deadline, body-frame waits still observe it. Verified by
[test stalled_response_body_exits_at_request_deadline](mechanics-http-client/src/response.rs#L263-L274).

### 1.8 DNS adapter state machine (post-today)
([dns.rs](mechanics-http-client/src/dns.rs))

`HyperDnsResolver` is now a zero-size unit-like struct after
today's commit. Per-call behaviour:
1. `Resolver::new()` → builds a fresh hickory resolver (load
   `/etc/resolv.conf` or fall back to Cloudflare set on ENOENT).
   Failure → `io::Error::other(...)`.
2. `tokio::time::timeout(3s, resolver.lookup_socket_addrs(host, 0))`.
   Timeout → `io::Error::new(TimedOut, "DNS lookup for `<host>` timed out")`.
3. `addrs.is_empty()` → `io::Error::new(NotFound, ...)`.
4. Otherwise → `Ok(addrs.into_iter())`.

**Invariant** (post-today's change): no DNS resolver state
crosses request boundaries. A wedged resolver path returns
within 3 s instead of holding the multi-minute endpoint
deadline open (the production-state hypothesis Yuka is testing).

### 1.9 Error model state machine
([error.rs](mechanics-http-client/src/error.rs))

Variants are flat — no error chain. `is_timeout()` and
`is_unreachable()` are the only public discriminators.
`map_legacy_error` ([request.rs:544-558](mechanics-http-client/src/request.rs#L544-L558))
classifies hyper-util errors by **substring match on the message
string**: `connection|connect|dns` → `Unreachable`,
`certificate|tls|TLS` → `Tls`, else → `Internal`. See §1.10
deficit.

### 1.10 mhc deficits / structural concerns I cannot fully model

In §10.0.1 spirit — listing things where the state machine is
either fragile, asymmetric, or where I don't have enough
evidence to be sure the model is correct.

**A. `map_legacy_error` substring heuristic.** Fragile —
hyper-util doesn't expose a kind enum, so we classify by error
message text. If hyper-util changes wording in a point release,
classification silently drifts. Comment at request.rs:546-548
acknowledges this. Suggested fix: open a hyper-util issue for a
typed-error-kind accessor, or replace with `error.source()`
chain walking until we hit a typed error we recognise. Either
direction has a clear state machine; current one doesn't.

**B. Negative cache is a flat 5-min cooldown.** No exponential
backoff, no success-reset. Production-symptom interaction: if
the underlying H3 wedge is a recurring resource-poisoning issue,
the cache expires every 5 min and the next request burns the
H3 phase budget against the still-wedged origin. The state
machine is "wait 5 min, retry once, if still bad cool down 5
min again". A bounded ramp (1m → 5m → 25m, reset on success)
would mark the origin "definitively H3-incapable" faster
without losing the recovery path. Not load-bearing for the
2026-05-19 production hypothesis but worth filing.

**C. Alt-Svc only updates on TCP/TLS responses.**
`maybe_update_alt_svc` fires after the hyper request path
returns. H3 responses don't carry an Alt-Svc update back into
the cache. Consequence: a server that flips `Alt-Svc: clear` on
the H3 path (e.g. mid-deployment, instructing clients to drop
H3) will not be observed by mhc until the negative cache or
existing Alt-Svc entry expires and a TCP/TLS request runs.
Defensible (you're already on H3) but the state machine is
asymmetric.

**D. HTTPS RR write-lock release across the network lookup.**
[request.rs:457-478](mechanics-http-client/src/request.rs#L457-L478)
takes the write lock, checks freshness, evicts stale, drops the
lock, does the bounded DNS lookup (~150 ms), then reacquires
the write lock to insert. Between drop and reacquire, racing
callers may also do the lookup and all insert — last-writer-wins.
Wasteful (multiple in-flight lookups for one origin under
concurrent first-touch) but not incorrect. Either hold the lock
across the await (probably bad — blocks other origins' reads)
or add a per-origin in-flight set. Current design is the
simpler one; flagging.

**E. `H3ResponseBodyState::Ready(None)` is dead state.** No
code path reaches it. The state machine has four states but
only three are inhabited. Either delete the variant or surface
the path that should reach it (e.g. an explicit "consumed and
empty" terminal distinct from `Done`). Cleanup, not a bug.

**F. Drop asymmetry between `Ready(Some)` and `Reading`.**
Dropping `H3ResponseBody` in `Ready(Some(stream))` explicitly
`stop_sending` + `stop_stream`s the stream with
`H3_REQUEST_CANCELLED`. Dropping in `Reading(future)` relies on
h3-quinn's own teardown when the future is dropped. h3-quinn is
trusted to send the appropriate stream resets, but the
asymmetry means we can't prove from this code alone that
`Reading`-drop produces the same wire effect as `Ready(Some)`-drop.
Worth a test asserting peer observes equivalent reset frames
under both drop paths.

**G. Hyper TCP/TLS pool is effectively dead.** §1.1 detail.
The pool config is wired through but `pool_max_idle_per_host=0`
+ `fresh_transport()` per request → pool never holds an idle
connection. Either commit to the no-reuse model and stop
plumbing pool config, or commit to reuse (which contradicts the
stability rationale). Current state is a deferred decision.

**H. `aws-lc-rs` install side effect is process-global.**
`tls::webpki_roots_client_config()` lazily installs `aws_lc_rs`
as the rustls default `CryptoProvider`. A test that builds a
`Client` after some other test installed a *different* provider
will misbehave. Currently safe because nothing else installs a
provider in this workspace, but the global-install state is not
captured in any local state machine.

**I. The DNS resolver per-call cost.** Today's fix builds a
fresh hickory resolver per lookup. Hickory's `Resolver::new()`
parses `/etc/resolv.conf` (or applies the Cloudflare fallback)
every call. For a tight loop of endpoint requests, this is
non-trivial allocation. Trade-off explicitly accepted in the
2026-05-19 commit message; flagging as observed cost.

---

## Part 2: `mechanics-core`

### 2.1 `MechanicsPool` lifecycle state machine
([pool/api.rs](mechanics-core/src/internal/pool/api.rs) + [pool/drop_impl.rs](mechanics-core/src/internal/pool/drop_impl.rs))

**States** of the pool as a whole:
- *Constructed*: `MechanicsPool::new(config)` validates config,
  spawns `worker_count` workers (each successfully runtime-
  initialised), spawns one supervisor thread, returns the pool.
  `closed=false`, `restart_blocked=false`.
- *Running*: handling `run` / `run_nonblocking_enqueue`
  invocations; supervisor reconciles missing workers.
- *Restart-blocked*: at least one missing worker, restart guard
  refused to allow a fresh spawn. `restart_blocked=true`.
  `run` returns `worker_unavailable` iff `live_workers == 0` AND
  `restart_blocked == true`. (`restart_blocked` alone does not
  block `run` — a partially-degraded pool keeps serving.)
- *Closed*: `closed=true`. Subsequent `run` calls return
  `pool_closed`.
- *Drained*: drop completed. Queue drained, all workers joined,
  supervisor joined.

**Transitions**:
- *Constructed → Running*: implicit on return.
- *Running → Restart-blocked*: supervisor's `reconcile_workers`
  loop calls `record_restart_attempt(now)` → false → set
  `restart_blocked=true`.
- *Restart-blocked → Running*: a successful spawn in the next
  reconcile cycle clears `restart_blocked=false` (line
  `set_restart_blocked(false)` inside `spawn_worker`).
- *Running → Closed*: `Drop::drop` sets `closed=true` first,
  then drains.

**Drop sequence invariant** ([pool/drop_impl.rs](mechanics-core/src/internal/pool/drop_impl.rs)):
1. `mark_closed` — subsequent `run` calls fail fast.
2. Drain pending queue: each `PoolMessage::Run(job)` gets a
   `canceled("pool dropped before job execution")` reply.
3. For each worker handle, send `()` on its
   `shutdown_tx` (worker's `select!` will fire on the next
   recv).
4. Send to supervisor shutdown_tx, join supervisor.
5. Drain workers map, join each.

**The drop sequence cannot interrupt an in-flight job.** A
worker that has already entered `runtime.run_source_with_early_reply`
will run that job to completion (or to runtime-limit-hit
abort), then loop, then exit on the next select-recv from
shutdown. So pool drop blocks until the longest in-flight job
finishes. This is correct but worth knowing.

### 2.2 Worker thread state machine
([pool/shared.rs:167-307](mechanics-core/src/internal/pool/shared.rs#L167-L307))

**States** of one worker thread, in order:
1. *Spawning*: `thread::Builder::new().spawn(...)` returns; the
   thread's body begins inside a `catch_unwind`.
2. *Initialising*: build `RuntimeInternal::new_with_endpoint_http_client`.
   - Success → send `Ok(())` on `ready_tx`; transition to
     Running.
   - Failure → send `Err(...)` on `ready_tx`; transition to
     Exited (without entering the loop).
3. *Running*: `select! { recv(shutdown_rx), recv(rx) }` loop.
   - `recv(shutdown_rx)` → break → Exited.
   - `recv(rx) = Ok(PoolMessage::Run(pool_job))`:
     - If `pool_job.is_canceled()` (already timed out at pool
       side) → send `canceled` reply, `continue` the loop.
     - Else: run job inside `catch_unwind`:
       - `Ok(Ok(()))` → no action (early_reply was called).
       - `Ok(Err(err))` → if not already replied, send err.
       - `Err(panic)` → if not already replied, send
         `worker_panic` reply, **break** → Exited.
   - `recv(rx) = Err(_)` (channel disconnected) → break → Exited.
4. *Exited*: after loop ends or after init failure, the
   `catch_unwind` outer block sends `WorkerExit::new(worker_id)`
   on `exit_tx` (or, on outer-panic-during-startup, sends both
   `ready_tx Err` and `exit_tx`). Thread ends.

**Reply discipline invariant** ([pool/shared.rs:230-261](mechanics-core/src/internal/pool/shared.rs#L230-L261)):
- `replied: Arc<AtomicBool>` is shared between the worker's
  error-handling branches and the `early_reply` closure.
- `early_reply` sets it to `true` after sending.
- Error branches use `swap(true, AcqRel) == false` to decide
  whether to send their own error.
- This prevents double-replies via the reply channel.
- **Contract**: `run_source_with_early_reply` must call
  `early_reply` at most once. If it returns `Ok(())` without
  calling `early_reply`, the pool side waits and times out (the
  worker is healthy; the bug is recoverable but obscure). See
  §2.5 invariant analysis.

### 2.3 Supervisor thread state machine
([pool/api.rs:148-181](mechanics-core/src/internal/pool/api.rs#L148-L181))

**Single loop**, `select!` over three sources:
1. `supervisor_shutdown_rx` → break.
2. `worker_exit_receiver` → on `Ok(event)`: remove the
   worker_id's handle, join it. On `Err(_)`: break.
3. `reconcile_tick` (period =
   `reconcile_interval(restart_window) = clamp(window/4, 50ms, 500ms)`)
   → fall through to reconcile.

After every successful select arm: check `is_closed()` → break.
Else `reconcile_workers(&shared)`:
- Compute `missing = desired - live_workers`.
- If `missing == 0` → clear `restart_blocked`, return.
- For each missing slot:
  - `record_restart_attempt(now)` → if false → set
    `restart_blocked=true`, return.
  - Try `spawn_worker` → if Err → set `restart_blocked=true`,
    return.

**Invariant**: at most `max_restarts_in_window` worker spawns
in any `restart_window` interval (sliding window via
`RestartGuard`). Crash-loop protection.

**Edge condition I want to call out**: if `reconcile_workers`
fires inside `spawn_worker` *while another spawn is in
progress*, the workers map is locked behind `RwLock` only for
short critical sections (insert at line 282-285). The supervisor
is single-threaded so there's no internal race within one
reconcile cycle. But if the user calls `MechanicsPool::stats()`
from another thread while reconcile is in progress, they may
observe partial state (a worker handle visible before the
worker is fully initialised). `stats()` documents this as a
non-blocking snapshot, so the API contract holds.

### 2.4 `PoolJob` lifecycle and cancellation
([pool/worker.rs](mechanics-core/src/internal/pool/worker.rs) + [pool/api.rs:208-275](mechanics-core/src/internal/pool/api.rs#L208-L275))

**States**:
- *Built*: `PoolJob::new(job, reply_tx, canceled_arc)` —
  `canceled=false`.
- *Enqueued*: `tx.send_timeout(message, enqueue_wait)` returned
  Ok. The PoolJob is now in the bounded crossbeam channel,
  visible to workers via `rx`.
- *Picked up*: worker received the message, runs `is_canceled()`
  check.
- *Running*: worker called `runtime.run_source_with_early_reply`.
- *Completed*: reply was sent (Ok or Err).
- *Cancelled-before-run*: pool side stored `canceled = true`
  before worker picked up the message. Worker observes via
  `is_canceled()` and sends `canceled` reply.

**Cancellation triggers**:
- Pool-side `run`/`run_nonblocking_enqueue` reply timeout: sets
  `canceled = true` AFTER waiting `remaining_for_reply` on
  `reply_rx.recv_timeout`. If the worker has already started
  the job by then, the canceled flag has no effect — the script
  runs to its `max_execution_time` (deadline-bounded). Pool
  returns `run_timeout`; worker eventually emits its own reply
  to the disconnected channel (best-effort send drops the
  result).
- Pool-side `run` enqueue-timeout limited-by-run-timeout: marks
  the job canceled and sends `run_timeout` reply BEFORE the
  worker could pick it up. But because the job is still queued
  in `rx`, a worker may still pop it; `is_canceled()` check
  catches that and sends `canceled` reply (which goes to the
  same `reply_tx`).

**Race between the two replies**: both pool side and worker
side may attempt to send on the same `reply_tx`. The reply
channel is `bounded(1)`. Whoever sends first wins; the second
send drops. This is acceptable because both replies represent
"job did not produce a result" — neither one is a successful
script output that would be lost.

### 2.5 `run_source_with_early_reply` state machine
([runtime.rs:256-417](mechanics-core/src/internal/runtime.rs#L256-L417))

The most state-rich part of mechanics-core. Phases:

**Phase A — setup** (lines 265-298):
- Extract `(source, arg, config)` from job.
- `self.hooks.clear()` — reset the unhandled-rejection counter.
- For each endpoint in config: call `endpoint.prepare_runtime()`
  → fail-fast on any prepare error.
- Build `MechanicsState` (per-job).
- Compute `deadline = now + max_execution_time`.
- `ctx.create_realm()` → fresh realm. `ctx.enter_realm(new)` →
  returns previous realm. Per-job realm isolation.
- `bundle_builtin_modules(&self.loader, ctx)` — re-register the
  built-in module set in the loader (idempotent overwrite).
- Set runtime limits (loop_iteration, recursion, stack_size).
- `self.queue.set_deadline(Some(deadline))`.
- `ctx.insert_data(state)` — make MechanicsState accessible from
  builtins (notably `mechanics:endpoint`).

**Phase B — module + main** (lines 303-394, inside closure):
- `Module::parse(source, None, ctx)` — syntax check + AST.
- `module.load_link_evaluate(ctx)` → returns a promise for
  module evaluation.
- `ctx.run_jobs()` → drives microtasks once to settle the
  evaluation. This is the synchronous `JobExecutor::run_jobs`
  path which calls `run_jobs_async_until_then_to_quiescence`
  with no stop predicate.
- Inspect `module_eval.state()`:
  - `Fulfilled(_)` → continue.
  - `Pending` → error `"Module evaluation promise did not settle"`.
  - `Rejected(e)` → propagate as JS error.
- **Strict check**: `if self.hooks.has_unhandled_rejections() →
  error "Unhandled promise rejection"`. The module-eval phase
  treats top-level unhandled rejections as fatal.
- Convert `arg` JSON → `JsValue::from_json`.
- Get `module.default` → assert it's a function.
- Call `main.call(null, [arg], ctx)` → `res: JsValue`.
- Wrap to promise: `res.as_promise().unwrap_or(JsPromise::resolve(res, ctx))`.
- Enter the tail-drive loop (see §2.6).

**Phase C — tail-drive** (run_jobs_until_then_to_quiescence,
line 331):
- See §2.6 for the inner state machine.
- `should_stop = || !matches!(res.state(), Pending)`.
- `on_stop` (called once when main first settles):
  - Match `res.state()`:
    - `Fulfilled(v)` → `js_value_to_json(ctx, v)`.
    - `Pending` → error `main_pending_error()` (defensive;
      shouldn't reach here because `should_stop` guards).
    - `Rejected(e)` → JS error.
  - Set `main_replied = true`.
  - Call `early_reply(main_result)` once.
- After tail-drive returns, check `!main_replied && res
  is Pending` → return `main_pending_error()`. This is a
  belt-and-braces safety in case the loop returned Complete
  without ever firing on_stop.
- Match `tail_exit`:
  - `Complete` → nothing.
  - `DeadlineExceeded(snapshot)` → log
    `tail_poll_aborted` (warning-level).

**Phase D — teardown** (lines 400-404, always runs):
- `ctx.remove_data::<MechanicsState>()`.
- `self.queue.set_deadline(None)`.
- `self.queue.clear_all()` — drains the four sub-queues. (The
  `FutureGroup` of in-flight async jobs has already been
  dropped because it's local to the tail-drive function.)
- `self.hooks.clear()`.
- `ctx.enter_realm(previous_realm)` — restore.

**Phase E — result classification** (lines 409-416):
- `Ok(())` → return `Ok(())`. Worker treats as "early_reply
  already fired".
- `Err(e)` with `main_replied=true` → swallow `e` (literally
  `let _ = e`), return `Ok(())`. **The tail-side error is
  discarded.** See §2.10 deficit.
- `Err(e)` with `main_replied=false` → return `Err(...)`. The
  worker then forwards via the error-reply branch.

**Subtle but load-bearing semantic** (the long comment at
lines 335-366): the `has_unhandled_rejections()` check is
applied **strictly at module-load time** but **deliberately
NOT applied** at main-settled time. The rationale is
`NativeFunction::from_async_fn`'s spec-tracker behaviour
producing false-positive unhandled rejections when an inner
promise rejects but the outer wrapper catches it. Yuka and the
maintainers explicitly chose Node-style "warning not kill" for
the main-side. This is a deliberate state-machine semantic
decision worth preserving when refactoring; do not
unilaterally re-add the strict check.

### 2.6 Executor `Queue` + tail-drive loop
([executor.rs](mechanics-core/src/internal/executor.rs))

**Queue structure** (per-worker, lives the worker's lifetime):
- Four sub-queues:
  - `async_jobs: VecDeque<NativeAsyncJob>`
  - `promise_jobs: VecDeque<PromiseJob>` (microtasks)
  - `timeout_jobs: BTreeMap<JsInstant, Vec<TimeoutJob>>` (sorted)
  - `generic_jobs: VecDeque<GenericJob>` (macrotasks)
- `deadline: RefCell<Option<JsInstant>>` (set per-job by
  `RuntimeInternal`).
- Owned tokio current-thread runtime + LocalSet.

**`enqueue_job`** (Boa's hook, line 334):
- `Job::PromiseJob` → push to promise_jobs.
- `Job::AsyncJob` → push to async_jobs.
- `Job::TimeoutJob` → compute fire_at = now + delay, insert
  into BTreeMap. **Overflow guard**: `instant_checked_add`
  returns None on overflow → fall back to `JsInstant(u64::MAX,
  0)` ("never fires"). See §2.10 deficit.
- `Job::GenericJob` → push to generic_jobs.
- Unknown variant → wrap as GenericJob that throws TypeError.

**`run_jobs_async_until_then_to_quiescence` loop** (lines 207-329):

```
loop {
    1. if !stopped && should_stop() { on_stop(ctx); stopped = true; }
    2. if deadline_exceeded → return DeadlineExceeded(snapshot)
    3. drain async_jobs into FutureGroup
    4. if all queues empty AND FutureGroup empty → return Complete
    5. wait/step:
       - if FutureGroup empty AND only timeouts remain:
           tokio::time::sleep(min(next_timeout_at - now, deadline - now))
       - else:
           - if any sync-ready (promise/generic/due-timeout) → no wait
           - else if wait_budget > 0 → tokio::time::timeout(budget, group.next())
           - else → group.next().await
           - match next_result:
               Some(Ok(_)) → in_flight -= 1
               Some(Err(err)) → return Err
               None → no-op
    6. recheck deadline
    7. drain_jobs(ctx):
       - drain all due TimeoutJobs (sorted by fire_at)
       - pop ONE GenericJob and call
       - take ALL PromiseJobs and call each
       - clear_kept_objects
    8. task::yield_now().await
}
```

**Microtask-vs-macrotask ordering**: per turn,
`drain_timeout_jobs` runs **all due timers** → ONE generic job
→ **all microtasks (PromiseJobs)**. The "1 generic per turn"
rule is asymmetric with timeouts/microtasks and worth knowing.
In practice generic jobs are rare in builtin code.

**Quiescence condition** (line 243-249): empty FutureGroup +
all four sub-queues empty → `Complete`. Note: "empty
FutureGroup" excludes async jobs that haven't been pulled out
of `async_jobs` yet, because step 3 always drains async_jobs
into the group before step 4.

**`should_stop` → `on_stop` one-shot**: `on_stop` is wrapped in
`Option<...>` and `.take()`-d. After on_stop fires once,
`stopped=true` and the predicate is never re-checked. The
loop continues running tail jobs until quiescence or deadline.

**Deadline cancellation cleans up via Drop**: when the loop
returns `DeadlineExceeded`, the `FutureGroup` (local variable)
goes out of scope and drops. In-flight async jobs (tokio
futures held by the FutureGroup) are cooperatively cancelled.
Any captured `&mut Context` reference in those futures was
mediated through a `RefCell` borrow, which is released on the
future's drop.

### 2.7 Endpoint execute state machine
([http/endpoint/execute.rs](mechanics-core/src/internal/http/endpoint/execute.rs))

**Per-call pipeline**:
1. `build_url_for_options` — validate URL template +
   urlParams + queries → fully-resolved URL or error.
2. Resolve `timeout_ms`, `response_max_bytes` via
   `per-endpoint || pool-default`.
3. Resolve `default_content_type`: only if method supports body
   AND body is not Absent.
4. `build_headers_prepared` — precedence: auto-defaults →
   endpoint configured → allowlisted JS overrides. Override is
   rejected if not in `prepared.allowed_overrides`.
5. Retry loop `for attempt in 1..=max_attempts`:
   - Build a **fresh** `EndpointHttpRequest` per attempt (no
     shared mutable body).
   - `client.execute(req).await` (this is the pluggable
     trait).
   - On `Ok(res)`:
     - If `attempt < max_attempts && retry_policy.should_retry_status(status)`
       → `sleep retry_delay_for_status(...)`, continue.
       (`retry_delay_for_status` respects `Retry-After`.)
     - Else → set final_response, break.
   - On `Err(err)`:
     - If retryable transport error AND attempts remain →
       `sleep retry_delay_for_transport(attempt)`, continue.
     - Else → return Err.
6. Check `final_response`. Status filter: if
   `!allow_non_2xx_status && status not 2xx` → "HTTP status N" error.
7. Extract exposed response headers (allowlist-filtered via
   `prepared.exposed_response_allowlist()`).
8. Content-length-cap fast-fail.
9. Copy body bytes into local Vec with `extend_body_with_limit`.
10. Decode per `response_body_type` (Json | Utf8 | Bytes).

**Invariants**:
- Retries do not mutate the request body — every attempt builds
  fresh from `options.body` clone.
- `tokio::time::sleep` participates in the executor loop (the
  await yields back, lets timeouts/microtasks run).
- Per-call timeout passes through to the transport (`req.timeout(...)`
  inside `DefaultEndpointHttpClient::execute`); body collection
  is separately timeout-bounded inside the transport.

### 2.8 `DefaultEndpointHttpClient::execute` state machine
([http/transport.rs:189-279](mechanics-core/src/internal/http/transport.rs#L189-L279))

For each call:
1. **`client.fresh_transport()`** — new `ClientInner` with fresh
   hyper, fresh DNS resolver (per call), shared H3 caches.
2. `EndpointRequestDeadline::new(timeout_ms)` → `Some(deadline)` or None.
3. Build request via mhc's `RequestBuilder`.
4. Send: `req.send().await` mapped to `io::Error::TimedOut` on
   mhc-`is_timeout`, else `into_io_error`.
5. Extract status, content_length, headers.
6. **Pre-body content-length cap** check.
7. Read body bounded by `tokio::time::timeout(deadline.remaining()?, ...)`
   if deadline set.

**Per-call cost**: §1.10.G already noted — every endpoint call
builds a new hyper TCP/TLS pool. Today's DNS-per-call change
adds another per-call allocation on top. Acceptable but measurable.

### 2.9 `MechanicsState` lifetime
([runtime.rs:62-124](mechanics-core/src/internal/runtime.rs#L62-L124))

- Inserted into `ctx.data` at job start, removed at job end.
- Carries `Arc<BoaMechanicsConfig>` (immutable),
  `Arc<dyn EndpointHttpClient>` (transport),
  scalar defaults, and **per-job** `prepared_endpoints` HashMap.
- Implements `JsData + Finalize + Trace` with
  `#[unsafe_ignore_trace]` on every field — none of the fields
  hold GC-managed values.
- Per-job lifetime → no cross-job leakage of prepared endpoints
  or config.

### 2.10 mechanics-core deficits / structural concerns

**A. Tail-side errors are silently swallowed when main already
replied.** [runtime.rs:411-413](mechanics-core/src/internal/runtime.rs#L411-L413):
`Err(e) if main_replied → let _ = e; Ok(())`. A tail promise
that throws after main resolved (`Promise.resolve().then(throw)`)
produces neither an error reply to the caller nor an observable
log line beyond `tail_poll_aborted` (which only fires on
deadline exceedance, not on tail-side errors that complete
synchronously). Suggested fix: tracing::warn! at least the
error message + job_id when discarding, so operators see
"tail promise threw post-resolve" in logs.

**B. `Boa::Context` is per-worker, lifetime is worker's lifetime.**
Per-job realm isolation prevents `globalThis` cross-talk, but
the Context's heap, module loader's registered modules, GC
roots, and any other Context-scoped state accumulate across
all jobs the worker runs. There is no "reset Context" path
short of killing the worker. If Boa has a slow leak in any
Context-scoped data structure, long-running workers grow
unboundedly until restart. **I do not have evidence either way**
that this is a real leak; flagging as "the state machine
admits this possibility" rather than "this is a known bug".

**C. Tokio runtime is per-worker, lifetime is worker's lifetime.**
The `Queue` owns one `tokio::runtime::Runtime` for the
worker's lifetime. Every job uses the same runtime. If a tail
task somehow corrupts tokio-internal state (a panicked future
inside the LocalSet, a never-completing IO driver wakeup),
the corruption persists across jobs. There is no
"reset tokio runtime" path short of killing the worker.
Self-healing depends on the panic catching in worker.rs causing
the worker to exit; the supervisor then spawns a replacement
with a fresh `RuntimeInternal` (and therefore fresh tokio
runtime). The cycle works, but only if the corruption manifests
as a panic — silent corruption stays.

**D. `run_source_with_early_reply` Ok(()) contract is
informal.** The function is documented to call `early_reply`
on the main path. If a future maintainer changes the body and
returns `Ok(())` without firing `early_reply`, the pool side
times out via `reply_rx.recv_timeout` (returns
`run_timeout`); the worker is healthy and continues. Recoverable
but easy to mis-author. Could be enforced with a debug_assert
or by returning `Result<MainResult, _>` instead of `Result<(),_>`
and centralising the early_reply send.

**E. Tail-promise quiescence holds the worker for the entire
`max_execution_time` even after main resolved.** This is
intentional ("fire-and-forget async work completes") but means
worker throughput is bounded by `max_execution_time`, not by
main-path latency. A workflow whose main returns in 50 ms but
launches a setTimeout(noop, 10_000) will block the worker for
10 s. Operators should know this when sizing
`max_execution_time` vs. `worker_count` vs. expected QPS.

**F. `drain_timeout_jobs` runs all due timers synchronously.**
A misbehaving script firing many `setTimeout(fn, 0)` in a loop
creates many TimeoutJobs at near-identical instants. On the
next macrotask turn, `drain_timeout_jobs` runs all of them in
one synchronous call before any `task::yield_now()`. The
`max_loop_iterations` and `max_execution_time` runtime limits
catch this eventually, but the failure mode is "burst-then-bound"
rather than gentle backpressure. Not a bug per se.

**G. Overflow on `instant_checked_add` yields `u64::MAX`
timer that never fires.** [executor.rs:340-342](mechanics-core/src/internal/executor.rs#L340-L342).
A `setTimeout(fn, Number.MAX_SAFE_INTEGER)` from JS schedules a
TimeoutJob that will never run. Silent. The `fn` reference is
held by the BTreeMap entry indefinitely (until `clear_all()` at
job teardown). Not a leak (job-scoped), but the semantic is
"silent dead timer". Could throw a runtime error at
enqueue_job time instead.

**H. `pool_max_idle_per_host(0)` + `fresh_transport()` per
call** — already covered in §1.10.G; mhc and mechanics-core
combined effectively rebuild the entire transport for every
endpoint call. If Yuka decides to switch to keep-alive reuse
later, the right place is here (single edit: drop the
`fresh_transport()` call), not in mhc.

**I. Reply channel race when both pool and worker reply.**
§2.4 already covered: both sides may try to send on the same
`bounded(1)` reply channel. Whichever wins, the other's send
drops silently. This is correct (both are "no result" replies)
but worth knowing — if either side starts producing a
*successful* result race partner, that result could be lost.
Current code does not have that race.

**J. Supervisor restart on init failure leaks workers map
slot.** When `spawn_worker` fails AFTER inserting into the
workers map but BEFORE `ready_rx.recv() Ok(Ok(()))`, the
`remove_worker_handle(worker_id)` + `handle.join()` path runs
([pool/shared.rs:292-296](mechanics-core/src/internal/pool/shared.rs#L292-L296)).
The worker thread sends `WorkerExit` on its way out, the
supervisor receives it, tries to remove the worker_id from the
map (already removed → no-op), then joins (already joined →
no-op). Idempotent join is fine because `WorkerHandle::join`
consumes self. Looks correct on close inspection but the
multi-path cleanup is intricate; worth a test asserting the
slot stays consistent.

---

## Part 3: Cross-crate concerns I cannot resolve from one read

These are not deficits in any single state machine; they are
modelling gaps where the right answer requires evidence I don't
have:

1. **Whether the 2026-05-19 per-call DNS fix resolves the
   production symptom.** I can model the state machine: per-call
   DNS prevents inter-request resolver state from poisoning
   later lookups, and the 3 s timeout prevents the multi-minute
   endpoint deadline from being consumed by a single wedged
   lookup. The hypothesis is internally coherent. But the dev
   box cannot reproduce the symptom and I cannot observe
   production. Per CLAUDE.md "Production is not this machine" —
   no resolution possible from this side.

2. **Whether the H3 negative-cache + Alt-Svc eviction
   coordination is sufficient under server-side flips.** The
   model: server stops advertising H3 → mhc's existing Alt-Svc
   entry stays cached until `ma=` expires; in the meantime
   every H3 attempt fails (Handshake) → negative-cache (5 min)
   + Alt-Svc evict. Once Alt-Svc is gone and negative cache
   expires, fall to HTTPS RR lookup. If HTTPS RR is similarly
   stale, the cycle repeats. Worst-case observed time-to-
   convergence: max(5min, RR-TTL). For server-side rapid flips
   this could be sluggish but not pathological. Worth a
   simulation test if Yuka cares about deployment-time
   convergence.

3. **Boa GC and `clear_kept_objects` semantics across many
   jobs on one Context.** I treated this as opaque (top-of-doc
   caveat). The right move if this becomes a concern is a
   long-running test that drives N synthetic jobs through a
   single worker and measures resident memory + Context heap
   stats. Boa upstream may have tooling.

4. **`h3::client::SendRequest` reuse semantics.** The
   `H3ResponseBody` retains `_sender: H3SendRequest`. The h3
   crate docs claim a single `SendRequest` can spawn multiple
   `RequestStream`s, but mhc only creates one per
   `connection()` and never reuses the `SendRequest` for a
   second request. Whether holding the `SendRequest` after the
   request stream is consumed correctly drives the connection
   to idle (so the spawned `wait_idle` task exits cleanly) is a
   detail I'm taking on h3-quinn's contract. Confidence-low
   inference; high-confidence answer needs reading h3-quinn
   source or an h3-quinn maintainer.

---

## Suggested follow-ups (none of these are urgent)

In rough priority order. None are load-bearing for the 2026-05-19
production hypothesis; they are state-machine cleanups surfaced
by this review.

1. **mhc §1.10.A**: replace `map_legacy_error` substring
   heuristic with `error.source()` chain walking. Pre-req:
   probably waits on a hyper-util typed-error accessor.
2. **mhc §1.10.E**: delete `H3ResponseBodyState::Ready(None)`
   dead variant; or surface the missing transition.
3. **mhc §1.10.B**: add exponential backoff to the H3 negative
   cache. Reset on success. Low-cost; meaningfully reduces
   "wedged-origin retry storm" surface.
4. **mechanics-core §2.10.A**: at minimum, `tracing::warn!` the
   discarded tail-side error before swallowing. One-line fix.
5. **mechanics-core §2.10.G**: throw a runtime error at
   `enqueue_job` time when a `setTimeout` delay overflows,
   instead of silently scheduling a never-firing timer.
6. **mhc §1.10.G**: decide whether to commit to per-request
   `fresh_transport()` (then drop pool config plumbing) or to
   keep-alive reuse (then drop `pool_max_idle_per_host(0)`).
   Either is a coherent design; the current half-and-half
   wastes config surface.
7. **mhc §1.10.F**: assertion test that drop in
   `H3ResponseBodyState::Reading(future)` produces the same
   peer-observable stream-reset as drop in
   `Ready(Some(stream))`. Confidence test, not a bug.

---

## Closing

The two crates are structurally well-formed for the work they
do. The mhc H3-attempt state machine has the most surface area
and the most carefully thought-through cancellation paths; the
mechanics-core executor loop is dense but coherent. The known
weak spots — substring error classification in mhc, swallowed
tail-side errors in mechanics-core, and the
fresh_transport-per-request /pool-config tension — are all
state-machine simplifications that improve auditability rather
than functional bugs. The 2026-05-19 per-call DNS change adds
the right kind of isolation for the symptom Yuka is chasing;
whether it actually fixes the production symptom remains
open by construction.

Per §10.0.1: every deficit listed here surfaces a place where
*I* could not construct the state machine cleanly from the
code as-is, OR could construct it but observed an asymmetry /
silent-failure / dead-state that the workspace should know
about. I have not invented fixes for any of them; that's
Yuka's call.
