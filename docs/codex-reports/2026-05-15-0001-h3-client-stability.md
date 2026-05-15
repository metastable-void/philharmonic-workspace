# HTTP/3 Client Stability Follow-Up

**Date:** 2026-05-15
**Prompt:** Chat investigation: mechanics-worker HTTPS requests to api-bin connector router hang when `bind_h3` is enabled.

## Summary

The investigation focused on preserving HTTP/3 as the preferred transport when api-bin advertises it, while ensuring that H3 cannot stall or break ordinary mechanics job HTTP calls. Enabling `bind_h3` makes api-bin advertise `Alt-Svc: h3=...` on HTTPS responses. After the worker observes that header, subsequent `https://.../connector/...` requests can be routed through the mechanics HTTP client's opportunistic HTTP/3 path.

Four client-side hazards were identified:

1. A stale cached H3 sender could surface as `Error::Cancelled` when reused after the underlying QUIC/H3 connection had closed.
2. The cached per-origin H3 `SendRequest` mutex was held for the full request/response lifetime, accidentally serializing all H3 requests to the same origin. One long connector-router call could therefore block later H3 calls even though QUIC and H3 are meant to multiplex streams.
3. The client consumed response DATA frames but did not read the trailing headers/end-of-stream phase before returning the buffered response. On a reused H3 connection this can leave the previous response stream unfinished, matching the observed pattern where the first request succeeds but later connector-router calls wait until the mechanics 300 second timeout.
4. The connector router's mhc-based forwarder buffered the entire inbound request body before opening the upstream connector-service connection. With api-server receiving the mechanics call over the new mechanics HTTP server path, this meant a stall in inbound body completion could occur before any TCP connection to `127.0.0.1:3002`, explaining a quiet `tcpdump -i lo 'port 3002'` even though mechanics had reached the public connector-router endpoint.

## Changes

The H3 client path now treats pooled H3 connections as disposable. If opening a request stream fails before request bytes are sent, the client evicts the cached H3 sender, retries once on a fresh H3 connection, then falls back to the normal HTTPS path if that also fails. If failure happens after the request stream has started, the client evicts the H3 sender but does not replay the request, avoiding duplicate non-idempotent connector calls.

The H3 stream-opening lock in `mechanics-http-client/src/http3.rs` was narrowed so it is held only while calling `send_request`. Request body upload, response header receive, and response body read now happen after the lock is released, allowing concurrent H3 streams to the same origin.

The H3 response path now drains trailers after the DATA frame loop before returning a buffered `Response`. The current caller does not expose response trailers, so they are intentionally discarded, but reading them completes the response stream and keeps the persistent H3 connection in a reusable state for the next mechanics job request.

`mechanics-http-client` now accepts streaming request bodies in addition to empty and replayable byte bodies. The connector router uses that streaming body path, so it no longer has to collect the full request before dialing the connector service. Empty and byte-backed mhc requests remain eligible for opportunistic H3 retry/fallback; non-replayable streaming bodies use the negotiated TCP transport path so the client does not have to replay partially consumed bodies after an H3 failure.

The QUIC/H3 setup path also gained a short connect/setup timeout. This prevents an advertised but unreachable UDP H3 path from consuming the full mechanics HTTP timeout before falling back to HTTPS. Separately, client and server QUIC transport policy was adjusted to use H3 keepalive and a longer idle timeout, reducing avoidable idle disconnects without depending on keepalive for correctness.

## Intended Behavior

When `bind_h3` is enabled and the H3 listener is reachable, mechanics-worker should use H3 successfully for api-bin connector-router calls. Completed H3 responses should be fully drained before the connection is reused. The embedded connector router should open its upstream connector-service connection without waiting for the entire inbound request body to buffer first. When H3 is stale, unreachable, or fails before the request starts, the client should recover quickly and continue over HTTPS. Once a request has begun on H3, the client must not blindly replay it on another transport.

The important invariant is that enabling H3 should improve transport options, not make connector-router requests less reliable than HTTPS-only operation.

## Validation

Focused validation passed for `mechanics-http-client`, and full `./scripts/pre-landing.sh` passed after the H3 client changes. The pre-landing run included the ignored H3 client fixtures. The follow-up trailer-drain and streaming-forwarder changes were also covered by `cargo fmt` with `CARGO_TARGET_DIR=target-main`, focused `./scripts/rust-lint.sh` and `./scripts/rust-test.sh` runs for the affected crates, and another successful `./scripts/pre-landing.sh`.
