# MHC And Mechanics Core State-Machine Review

**Date:** 2026-05-19
**Prompt:** Chat request to do a full state-machine review of
`mechanics-http-client` and `mechanics-core`.

## Scope

This review covered the current working tree for:

- `mechanics-http-client/src/{client,dns,request,http3,response,alt_svc,https_rr,error}.rs`
- `mechanics-core/src/internal/http/{transport.rs,endpoint/execute.rs,endpoint/request.rs}`
- `mechanics-core/src/internal/runtime.rs`
- `mechanics-core/src/internal/executor.rs`
- `mechanics-core/src/internal/pool/{api.rs,shared.rs,worker.rs,drop_impl.rs,config.rs}`
- Relevant tests under `mechanics-core/src/internal/pool/tests/` and
  `mechanics-http-client/src/tests.rs`

I did not run production observation. Per workspace rules, production state
cannot be inferred from this sandbox. This is a code-state review.

## Executive Summary

The current MHC endpoint path no longer contains the old long-lived DNS resolver
state in the client. TCP/TLS DNS resolution, HTTPS RR lookup, and H3 fallback
address lookup are per-lookup. `mechanics-core` also calls
`Client::fresh_transport()` per endpoint request, so hyper TCP/TLS connection
pool state is not reused across endpoint calls.

For the exact symptom "JS catches `endpoint \`llm\` request failed: request
timed out`, connector router is never called, and no `tcp/udp :443` or
`tcp :3002` packets appear", the current code has no obvious remaining
multi-minute pre-socket wait in MHC itself. The bounded pre-socket phases I
found are: HTTPS RR lookup at 150 ms, H3 DNS lookup at 150 ms, H3 connect/setup
at 500 ms each, and TCP/TLS DNS lookup at 3 s. A sustained 300 s no-packet wait
therefore points to either older code in the process under test, an endpoint
async job not being polled, a blocking custom endpoint client, a retry/sleep
state, or an observation filter that misses the actual pre-target work such as
DNS. The report below lists the remaining state-machine hazards.

## State Machines

### MHC Client State

Stable client state:

- `ClientInner.hyper`: hyper TCP/TLS client.
- request defaults: timeout, headers, pool knobs.
- H3 discovery caches: HTTPS RR cache, Alt-Svc cache, negative cache.
- `Http3State`: effectively stateless in production; creates QUIC endpoints per
  attempt.

Fresh endpoint invocation state:

1. `DefaultEndpointHttpClient::execute` clones the configured client.
2. It calls `Client::fresh_transport()`.
3. It builds the request, applies the remaining endpoint deadline as the MHC
   request timeout, then awaits `req.send()`.
4. It reads the response body under the same endpoint deadline.

The only MHC state intentionally shared across `fresh_transport()` clones is H3
discovery cache state. TCP/TLS transport state is new per endpoint invocation.

### MHC Request State

`RequestBuilder::send` transitions:

1. Deferred builder errors.
2. Parse URL and compute request deadline.
3. Build header map.
4. If H3 is enabled, URL is HTTPS, and the body is replayable:
   - check negative cache;
   - try Alt-Svc target if fresh;
   - otherwise try HTTPS RR target if fresh/available;
   - return H3 response, fall back to TCP/TLS, or return timeout.
5. Build hyper request body.
6. Await hyper TCP/TLS request under the remaining deadline.
7. Wrap response with the same deadline for body reads.
8. Update Alt-Svc from TCP/TLS response headers.

Important H3 path decisions:

- H3 handshake failure inserts negative cache and falls back to TCP/TLS.
- H3 stream failure before request start retries once on fresh H3, then falls
  back to TCP/TLS.
- H3 stream failure after request start falls back to TCP/TLS.
- H3 response-header timeout is terminal `Error::Timeout`, because the request
  was already sent.
- Cancellation while an H3 attempt is in flight inserts negative cache through
  `H3AttemptCancellationGuard`.

### MHC H3 Body State

`H3ResponseBodyState` transitions:

- `Ready(Some(stream))`: owns the stream and can start a read.
- `Reading(future)`: `recv_data()` is in flight.
- `Ready(Some(stream))`: a data frame completed and the next read can start.
- `Done`: EOF or error.

The body owns the H3 sender and QUIC endpoint so that the connection remains
alive while response data is read. Dropping the body while it is still `Ready`
cancels the request stream.

### Mechanics-Core Endpoint State

JS endpoint call transitions:

1. Boa native async function parses endpoint name and options.
2. It clones `MechanicsState` from the current Boa context.
3. It looks up the endpoint and prepared endpoint runtime cache for this job.
4. It calls `execute_endpoint`.
5. Transport or decode errors are wrapped as
   `endpoint \`name\` request failed: ...` and returned to JS as a catchable
   error.

`execute_endpoint` transitions:

1. Resolve URL slots, queries, headers, body type, endpoint timeout, and
   response-body cap.
2. For each retry attempt:
   - build `EndpointHttpRequest`;
   - call `EndpointHttpClient::execute`;
   - retry selected statuses or transport errors after policy delay;
   - otherwise retain terminal response or return terminal error.
3. Enforce non-2xx policy.
4. Expose allowlisted response headers.
5. Enforce response-body cap again at the decoded endpoint layer.
6. Decode JSON / UTF-8 / bytes into `EndpointResponse`.

### Mechanics-Core Runtime State

Per worker:

- One long-lived `RuntimeInternal`.
- One Boa context and one custom module loader.
- One `Queue` with a current-thread Tokio runtime and LocalSet.
- A shared `Arc<dyn EndpointHttpClient>`.

Per job:

1. Clear host hook counters.
2. Prepare endpoint runtime cache from that job's config.
3. Create a new Boa realm.
4. Set Boa execution limits and Queue deadline.
5. Insert `MechanicsState` into the context.
6. Parse/load/evaluate module.
7. Call default export.
8. Poll jobs until default export settles.
9. Send early reply with the main result.
10. Continue tail-promise polling to quiescence or execution deadline.
11. Remove `MechanicsState`, clear queue state, clear hooks, and restore realm.

This preserves the tail-promise polling feature. Main result delivery and tail
quiescence are separate states.

### Mechanics-Core Pool State

Pool lifecycle:

- `new`: validate config, create endpoint client, create channels, spawn target
  workers, spawn supervisor.
- `run`: bounded enqueue wait, then bounded reply wait.
- `run_nonblocking_enqueue`: immediate enqueue or `QueueFull`, then bounded
  reply wait.
- worker loop: receive job, skip canceled jobs, run runtime with early reply,
  catch panics, report exit.
- supervisor loop: reap worker exits, reconcile missing workers unless closed.
- `drop`: mark closed, cancel queued jobs, request worker shutdown, stop
  supervisor, join workers.

Caller `run_timeout` and Boa execution deadline are separate. If `run_timeout`
fires after a worker has started a job, the caller stops waiting, but the worker
continues until Boa/runtime limits stop it. The code comments document this.

## Findings

### 1. High: H3 cache expiry arithmetic can panic on reachable inputs

Two expiry calculations use unchecked `Instant + Duration`:

- `mechanics-http-client/src/alt_svc.rs:77-93`
- `mechanics-http-client/src/request.rs:510-515`

`Alt-Svc` `ma` is an untrusted response-header value. A peer can advertise an
extreme `ma`, and `now + max_age` can panic if the resulting `Instant` is out of
range. `http3_negative_cache_duration` is local configuration, but it is public
builder input and can also overflow the `Instant` addition.

Impact:

- In MHC as a library, this violates the no reachable panics rule.
- In mechanics-core, a panic during an endpoint call can be caught as a worker
  panic, killing/restarting that worker rather than returning a normal endpoint
  error.

Recommended fix:

- Use `checked_add` for both expiry calculations.
- Clamp to a conservative maximum expiry or reject oversized local negative
  cache durations at build time.
- Add tests for `Alt-Svc: h3=":443"; ma=<huge>` and
  `http3_negative_cache_duration(Duration::MAX)`.

### 2. Medium: H3 pre-response phases are not consistently capped by request deadline

`RequestBuilder::send` computes a request deadline before H3 discovery and
passes it into `try_http3` (`mechanics-http-client/src/request.rs:178-217`).
But `Http3State::request` only applies that deadline to response-header receive:

- stream open uses `H3_STREAM_OPEN_TIMEOUT` (`http3.rs:79-89`);
- request body upload and finish use `H3_STREAM_UPLOAD_TIMEOUT`
  (`http3.rs:91-96`, `http3.rs:296-307`);
- QUIC connect and H3 setup use `H3_CONNECT_TIMEOUT`
  (`http3.rs:116-125`);
- H3 DNS fallback uses `H3_DNS_LOOKUP_TIMEOUT` (`http3.rs:282-289`);
- only `recv_response_with_deadline` uses the caller deadline
  (`http3.rs:309-326`).

Impact:

- A very short caller timeout can be exceeded by H3 discovery/connect/open/upload
  before MHC returns.
- Mechanics-core endpoint deadlines are still eventually enforced by the outer
  endpoint future, but H3 can overrun the exact per-request timeout contract.

Recommended fix:

- Thread the request deadline through every H3 phase.
- Compute each phase's budget as `min(phase_timeout, remaining(deadline))`.
- Distinguish "phase timeout before request start" from "caller deadline
  expired"; the former can fall back to TCP/TLS, the latter should return
  `Error::Timeout`.
- Add a test with a sub-`H3_CONNECT_TIMEOUT` request timeout and stale Alt-Svc.

### 3. Medium: Endpoint timeout is per retry attempt, not aggregate

`execute_endpoint` computes `timeout_ms` once, then builds a fresh
`EndpointHttpRequest` for each retry attempt (`mechanics-core/src/internal/http/endpoint/execute.rs:59-140`).
`DefaultEndpointHttpClient::execute` creates a new `EndpointRequestDeadline` for
each transport request (`mechanics-core/src/internal/http/transport.rs:196-204`).
Retry sleeps are also outside that per-attempt deadline
(`execute.rs:145-163`).

Impact:

- With `max_attempts > 1`, one endpoint call can run for roughly
  `max_attempts * timeout_ms + retry delays`.
- This may be intentional, but it is not explicit in the state machine.
- In mechanics-core, Boa `max_execution_time` is the real aggregate cap. If the
  endpoint call is awaited and the Boa deadline fires first, JS sees a runtime
  execution timeout/pending-main style failure rather than a clean endpoint
  timeout.

Recommended fix:

- Decide and document whether endpoint timeout means "per attempt" or "entire
  endpoint call".
- If aggregate semantics are intended, compute an endpoint-call deadline once in
  `execute_endpoint` and pass remaining time into each attempt and retry sleep.
- If per-attempt semantics are intended, add explicit docs and tests.

### 4. Medium: Retry policy treats deterministic local errors as retryable I/O

`EndpointRetryPolicy::should_retry_transport_error` retries every non-timeout
`std::io::Error` when `retry_on_io_errors` is true. The default is true. But
`DefaultEndpointHttpClient` maps several deterministic local conditions to
`InvalidData` or generic I/O errors:

- content-length over response cap (`transport.rs:231-240`);
- streamed body over response cap (`transport.rs:245-253`);
- invalid request/header/URL errors after MHC maps them into `io::Error`;
- response decode/body errors if they surface inside transport.

`execute_endpoint` applies retry policy to all `client.execute(req)` errors
(`execute.rs:157-165`).

Impact:

- Configured retries can repeat known-bad local errors that cannot succeed on a
  later attempt.
- This wastes endpoint budget and can obscure the first cause.

Recommended fix:

- Introduce an endpoint transport error classification rather than using only
  `std::io::ErrorKind`.
- Retry only network-ish errors and selected timeouts.
- Do not retry `InvalidInput`, `InvalidData`, body-cap errors, decode errors, or
  request construction failures.

### 5. Low/Medium: MHC legacy error classification is string-based

`map_legacy_error` classifies hyper legacy errors by substring
(`mechanics-http-client/src/request.rs:544-555`).

Impact:

- DNS lookup timeout from `HyperDnsResolver` carries `io::ErrorKind::TimedOut`
  (`mechanics-http-client/src/dns.rs:43-55`), but after hyper wraps it, MHC may
  classify it as `Unreachable` because the message contains `dns`, not as
  `Timeout`.
- Mechanics-core then sees an ordinary I/O error, not `ErrorKind::TimedOut`,
  which changes retry policy behavior (`retry_on_timeout` vs
  `retry_on_io_errors`) and JS-visible error text.

Recommended fix:

- Prefer structured source/downcast inspection if hyper-util exposes the
  underlying connector error.
- At minimum, make timeout phrase/kind detection explicit before DNS/connect
  string detection, and add tests for resolver timeout classification.

### 6. Low: HTTPS RR lookup failures are not negative-cached

`https_rr_entry` caches successful `Some(entry)` results, including non-H3 HTTPS
RR entries, but lookup timeout/failure and `Ok(None)` are not cached
(`mechanics-http-client/src/request.rs:452-478`).

Impact:

- An origin with failing/no HTTPS RR lookup pays the 150 ms discovery timeout on
  every request until another cache path applies.
- This is bounded and does not explain a 300 s no-packet timeout in current
  code, but it is avoidable churn in a hot path.

Recommended fix:

- Consider a short "no HTTPS RR / lookup failed" cache entry distinct from the
  H3 negative cache.
- Keep it short enough that newly deployed HTTPS RR records are discovered.

### 7. Low: Partial pool construction failure relies on channel drop, not explicit worker join

`MechanicsPool::new` spawns workers before spawning the supervisor
(`mechanics-core/src/internal/pool/api.rs:113-143`). If a later worker spawn
fails after earlier workers succeeded, `new` returns before constructing a
`MechanicsPool`, so normal `Drop` cleanup does not run. The already-started
workers should exit when the shared channels are dropped, but their handles are
dropped with the partially constructed `MechanicsPoolShared` rather than
explicitly joined.

Impact:

- This is a rare startup-failure edge, not the observed endpoint timeout.
- It is still a lifecycle cleanliness gap.

Recommended fix:

- Make pool construction use a guard that requests shutdown and joins any
  already-started workers if any later construction step fails.

## Sound Invariants Observed

- Mechanics-core worker runtime state is cleaned after every job:
  `MechanicsState` is removed, queue deadline is cleared, queued jobs are
  cleared, hooks are cleared, and the prior realm is restored
  (`mechanics-core/src/internal/runtime.rs:400-404`).
- Tail-promise polling is an explicit state, not an accidental leak. The runtime
  sends early main reply, then continues polling until quiescence or deadline
  (`runtime.rs:331-393`).
- The executor waits only up to the Boa deadline while async jobs are in flight
  (`mechanics-core/src/internal/executor.rs:294-327`).
- H3 response bodies own the sender and endpoint, which prevents the connection
  owner from being dropped before body reads complete (`mechanics-http-client/src/http3.rs:197-203`).
- H3 RwLock cache operations are short and do not hold locks across `.await`.
- `DefaultEndpointHttpClient` bounds response body reads with the remaining
  endpoint deadline (`mechanics-core/src/internal/http/transport.rs:263-267`).
- Endpoint errors include the endpoint name at the JS boundary, which matches
  the observed JS-caught error shape (`mechanics-core/src/internal/runtime/builtins/endpoint.rs`).

## Production-Symptom Implications

In the current code, if an awaited endpoint call reaches
`DefaultEndpointHttpClient::execute`, the longest target-origin pre-socket path
inside MHC should be seconds, not minutes. For a 300 s no-packet timeout:

- If production is not running this exact code, the old shared resolver/runtime
  state remains plausible.
- If production is running this code, inspect whether the Boa async endpoint job
  is actually being polled after the first successful step. The packet capture
  symptom can happen before transport if the `NativeAsyncJob` future never gets
  polled or the single-threaded worker runtime is blocked by a custom endpoint
  client.
- Inspect retry state. Retry sleeps do not emit packets and are outside the
  per-attempt endpoint timeout.
- Add trace points at: JS endpoint entry, `execute_endpoint` attempt start,
  `DefaultEndpointHttpClient::execute` before `fresh_transport`, before
  `req.send`, MHC H3 cache decision, MHC TCP fallback start, DNS lookup start,
  and hyper request start. Those transitions would identify the exact
  no-packet state without relying on inference from `tshark`.

## Suggested Follow-Up Order

1. Fix unchecked H3 expiry arithmetic.
2. Make H3 phase timers respect the caller request deadline.
3. Decide and encode aggregate-vs-per-attempt endpoint timeout semantics.
4. Split retryable network errors from deterministic local errors.
5. Replace or harden string-based MHC error classification.
6. Add transition tracing around endpoint attempt state.
7. Add partial pool-construction cleanup guard.

## Validation

This was a review-only task. I did not change Rust code and did not run
pre-landing. The report itself was added under `docs/codex-reports/`.
