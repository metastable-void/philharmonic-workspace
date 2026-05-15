# HTTP/3 Client Stability Follow-Up

**Date:** 2026-05-15
**Prompt:** Chat investigation: mechanics-worker HTTPS requests to api-bin connector router hang when `bind_h3` is enabled.

## Summary

The investigation focused on preserving HTTP/3 as the preferred transport when api-bin advertises it, while ensuring that H3 cannot stall or break ordinary mechanics job HTTP calls. Enabling `bind_h3` makes api-bin advertise `Alt-Svc: h3=...` on HTTPS responses. After the worker observes that header, subsequent `https://.../connector/...` requests can be routed through the mechanics HTTP client's opportunistic HTTP/3 path.

Thirteen HTTP-path hazards were identified:

1. A stale cached H3 sender could surface as `Error::Cancelled` when reused after the underlying QUIC/H3 connection had closed.
2. The cached per-origin H3 `SendRequest` mutex was held for the full request/response lifetime, accidentally serializing all H3 requests to the same origin. One long connector-router call could therefore block later H3 calls even though QUIC and H3 are meant to multiplex streams.
3. The client consumed response DATA frames but did not read the trailing headers/end-of-stream phase before returning the buffered response. On a reused H3 connection this can leave the previous response stream unfinished, matching the observed pattern where the first request succeeds but later connector-router calls wait until the mechanics 300 second timeout.
4. The connector router's mhc-based forwarder buffered the entire inbound request body before opening the upstream connector-service connection. With api-server receiving the mechanics call over the new mechanics HTTP server path, this meant a stall in inbound body completion could occur before any TCP connection to `127.0.0.1:3002`, explaining a quiet `tcpdump -i lo 'port 3002'` even though mechanics had reached the public connector-router endpoint.
5. The connector router still buffered the entire upstream response body before returning response headers to mechanics-worker. Long-running or streaming connector responses could therefore make the public connector-router request appear idle until the mechanics 300 second timeout, even after the upstream connector service had accepted the request.
6. The api-server dynamic connector route held `connector_dispatch.read().await` across the whole awaited forwarding path. Tokio's `RwLock` is fair/write-preferring, so a config reload waiting for the write lock can make later connector requests wait behind the reload while an earlier connector request is still in flight. In that state the later request has reached api-server but has not reached the connector forwarder, which matches the observed quiet `tcpdump -i lo 'port 3002'`.
7. A cached H3 sender whose underlying QUIC/H3 connection had gone stale could hang while opening a new bidirectional request stream. That hang happens before request headers or body bytes are sent, so it can consume the full mechanics HTTP timeout without any api-server connector-router trace.
8. Cached `Alt-Svc` was checked only after the client probed HTTPS DNS records. That made non-first requests vulnerable to a slow HTTPS-RR lookup even though the first response had already advertised a usable H3 alternative.
9. Concurrent endpoint calls in the same mechanics job can share a cached H3 sender. If one task is already opening a stream, later tasks can wait on the sender mutex before their own stream-open timeout starts.
10. The embedded connector router's upstream mhc client reused idle TCP connections to connector services. For local connector-service calls this stale-pool risk is not worth the small reuse win, and it can make non-first upstream forwards fail before a new loopback connection is visible.
11. Connector implementations that call external HTTP services built mhc clients with default idle pooling. The OpenAI-compatible LLM path can therefore reuse a stale provider keep-alive connection on the second or later LLM request in a chat workflow.
12. The H3 request and response adapters waited for optional trailers after DATA EOF even though the connector paths do not use trailers. If the h3 stack does not resolve the "no trailers" phase promptly, a complete POST body or response body can be held open indefinitely, which matches `endpoint("llm")` reaching the H3 path and then timing out without needing any durable replay or operator-tuned policy explanation.
13. Reusing a cached H3 connection still leaves a non-first request vulnerable to failures after `send_request` succeeds but before api-server has received enough request bytes to enter connector dispatch. In that state a public connector-router request can consume the mechanics endpoint timeout while `tcpdump -i lo 'port 3002'` stays quiet, because the router never reaches the upstream connector-service dial.

## Changes

The H3 client path now treats H3 connections as per-request disposable transport state. The client still reuses its local QUIC endpoint and still prefers advertised H3 alternatives, but it does not share a cached `SendRequest` across separate mechanics endpoint calls. This removes the stale non-first H3 connection class without disabling H3, adding operator-facing knobs, or introducing durable request-deduplication state.

Opening an H3 request stream is now bounded by a short timeout. This makes the stale-sender path produce the same retryable pre-request error as an immediate stream-open failure, instead of waiting for the outer mechanics endpoint timeout. The stream-open and H3 connect/setup budgets are intentionally sub-second because the support-chat path must fall back quickly when H3 is stale or unreachable.

Cached `Alt-Svc` entries are now tried before HTTPS DNS records, so the normal second-and-later request path avoids DNS probing. HTTPS-RR lookup is also bounded by a short timeout before falling through to TCP HTTPS.

The H3 stream-open timeout now covers only the fresh `send_request` call. Concurrent chat endpoint calls therefore cannot queue behind a stale per-origin sender lock, because that lock no longer exists.

The older cached-sender lock is gone from `mechanics-http-client/src/http3.rs`. Concurrent endpoint calls now open independent H3 connections instead of queueing behind or sharing one per-origin `SendRequest`.

The H3 response path now treats DATA EOF as response-body completion and does not wait for optional trailers that are not exposed to callers. This avoids turning a complete response body into a stuck mechanics endpoint future.

The HTTP/3 server request-body adapter similarly treats DATA EOF as request-body completion. The connector router and connector services do not consume request trailers, so this keeps POST bodies flowing through axum and into the upstream connector service instead of waiting forever for a trailer phase that may never arrive.

`mechanics-http-client` now accepts streaming request bodies in addition to empty and replayable byte bodies. The connector router uses that streaming body path, so it no longer has to collect the full request before dialing the connector service. Empty and byte-backed mhc requests remain eligible for opportunistic H3 retry/fallback; non-replayable streaming bodies use the negotiated TCP transport path so the client does not have to replay partially consumed bodies after an H3 failure.

`mechanics-http-client::Response` now exposes a raw streaming body path for forwarders. The connector router uses it to return upstream response headers immediately and stream response DATA frames back to the mechanics caller instead of waiting for upstream EOF. Because this path preserves the wire representation rather than using mhc's decompression helpers, the router now preserves representation headers such as `Content-Encoding` while continuing to drop hop-by-hop framing headers like `Content-Length` and `Transfer-Encoding`.

The api-server connector dispatch path now selects the realm upstream while holding the dispatch config read lock, drops that guard, and only then awaits the connector forwarder. `philharmonic-connector-router` exposes `dispatch_to_upstream` for this already-selected URI case, so api-server does not have to borrow `DispatchConfig` across network I/O.

The connector router's production forwarder now disables mhc idle connection reuse for upstream connector-service calls. The mechanics worker already does this for job endpoint traffic; applying the same policy inside the router avoids stale keep-alive reuse on the local connector-service hop.

The OpenAI-compatible LLM connector and generic HTTP-forward connector now also disable mhc idle connection reuse for their upstream provider/client hops. This preserves request correctness for bursty chat workloads where a provider-side stale keep-alive is more harmful than reconnect cost.

Mechanics endpoint transport errors now include the endpoint name in the JavaScript-visible error string, for example ``endpoint `llm` request failed: request timed out``. This is diagnostic rather than a transport fix, but it makes future chat-script failures attributable to `embed`, `vector_search`, `llm`, `db`, or another configured endpoint without guessing from timing.

The QUIC/H3 setup path also gained a short connect/setup timeout. This prevents an advertised but unreachable UDP H3 path from consuming the full mechanics HTTP timeout before falling back to HTTPS. Separately, client and server QUIC transport policy was adjusted to use H3 keepalive and a longer idle timeout, reducing avoidable idle disconnects without depending on keepalive for correctness.

## Intended Behavior

When `bind_h3` is enabled and the H3 listener is reachable, mechanics-worker should be able to use H3 for connector-router calls, including POST endpoints such as `llm`, without requiring operator-facing knobs or durable deduplication state. H3 should not depend on cross-request connection reuse for correctness; non-first connector-router calls should be fresh at the H3 connection layer so a stale previous QUIC/H3 connection cannot block the router before the `127.0.0.1:3002` upstream dial. Completed request and response DATA streams should be enough to complete the body when trailers are unused. The embedded connector router should open its upstream connector-service connection without waiting for the entire inbound request body to buffer first, and it should return upstream response headers without waiting for the whole upstream body to finish. Config reloads must not make unrelated connector requests queue behind long-running connector forwarding. When H3 is stale, unreachable, or fails before a request starts, the client should recover quickly and continue over HTTPS. Once a request has begun on H3, the client must not blindly replay it on another transport.

The important invariant is that enabling H3 should improve transport options, not make connector-router requests less reliable than HTTPS-only operation.

## Validation

Focused validation passed for `mechanics-http-client`, and full `./scripts/pre-landing.sh` passed after the H3 client changes. The pre-landing run included the ignored H3 client fixtures. The follow-up request/response trailer-wait removal, disposable H3 connection change, request-streaming forwarder, response-streaming forwarder, and api-server dispatch-lock changes were also covered by focused `./scripts/rust-lint.sh` and `./scripts/rust-test.sh` runs for the affected crates, plus another successful `./scripts/pre-landing.sh`.
