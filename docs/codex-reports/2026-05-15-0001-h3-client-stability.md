# HTTP/3 Client Stability Follow-Up

**Date:** 2026-05-15
**Prompt:** Chat investigation: mechanics-worker HTTPS requests to api-bin connector router hang when `bind_h3` is enabled.

## Summary

The goal is to keep HTTP/3 usable when api-bin advertises it, without letting H3 make support-chat endpoint calls less reliable than the H1.1/H2 path. The observed failure pattern is that a greeting and first chat turn can succeed, then a later `endpoint("llm")` call times out after the mechanics endpoint window. In one deployment, `tcpdump -i lo 'port 3002'` on the api-server host showed no connector-service traffic, which means the request was blocked before the connector router opened its local upstream connection.

The latest pass fixed five remaining HTTP/H3 hazards without adding operator-facing knobs, disabling H3, or adding durable per-request state:

1. The fallback DNS lookup used by the H3 Alt-Svc path was unbounded.
2. H3 request setup and upload phases after stream open could wait too long before the request was fully handed to api-server.
3. The H3 client buffered the entire response body inside `RequestBuilder::send`, which mixed transport setup with whole-response completion and made response streaming impossible.
4. The connector-router next-hop connection path needed a short transport setup deadline while still allowing the full endpoint request window for legitimate connector work.
5. HTTP/3 server request tasks were only reaped when another stream arrived or the connection closed.

## Changes

`mechanics-http-client` now applies a short built-in TCP connect timeout through the hyper connector. This bounds next-hop connection establishment for H1.1/H2 without shortening the response body window. The connector router benefits from this directly because its upstream forwarder uses `mechanics-http-client` and already disables idle pooling for local connector-service hops.

The H3 Alt-Svc fallback DNS path is now bounded by a short timeout. If DNS resolution for the advertised H3 endpoint stalls, the client treats that as an H3 handshake/probe failure and falls back to the normal HTTPS transport path instead of consuming the endpoint's full timeout before any request can be sent.

The H3 stream open path remains bounded, and the H3 request-body send and finish phases now have their own short upload-phase bounds. Those phases happen before api-server has a complete request to dispatch, so they should fail quickly when the QUIC/H3 path is wedged. Once the request has started, the client still avoids blind replay on another transport.

The H3 response path now returns a streaming response body instead of buffering all DATA frames before constructing `Response`. `Response::into_body` and `bytes_with_cap` both work with the new H3 body. This keeps H3 aligned with the H1.1/H2 response model: `send()` obtains response headers, and later body consumption is handled by the caller.

Because H3 responses are now streamed, `mechanics-core` wraps the full endpoint HTTP operation in the endpoint timeout: request send, response headers, and response body read. The timeout is still the endpoint's configured full request window, not a new short transport timeout. This preserves long-running connector calls while preventing body reads from escaping the intended endpoint deadline.

`mechanics-http-server` now reaps completed per-stream H3 request tasks while the connection is idle and waiting for the next accepted stream. Completed request tasks no longer sit inside a connection-local `JoinSet` until another stream arrives or the QUIC connection closes.

## Intended Behavior

When `bind_h3` is enabled and H3 is reachable, mechanics-worker can keep using H3 for replayable HTTPS endpoint calls to api-bin's connector router. Short deadlines apply to connection acquisition, H3 DNS probing, stream opening, and request upload completion. The actual connector request can still consume the endpoint's configured timeout window, including response body reading.

For the api-server connector-router hop, the important behavior is that local next-hop connection failures should surface quickly, but a slow legitimate connector response should not be killed by a new router-wide blanket timeout. H1.1/H2 remain available through the same mechanics HTTP client, and H3 failures before a request starts still fall back to TCP HTTPS.

The invariant remains: enabling H3 should add a transport option, not make non-first support-chat requests hang before the connector service is even contacted.

## Validation

Focused lint/doc validation passed for:

- `./scripts/rust-lint.sh mechanics-http-client`
- `./scripts/rust-lint.sh mechanics-core`
- `./scripts/rust-lint.sh mechanics-http-server`

Full `./scripts/pre-landing.sh` also passed after the final batch of fixes, including the ignored H3 client/server fixtures for `mechanics-http-client` and `mechanics-http-server`.
