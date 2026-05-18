# HTTP/3 And Endpoint Lifecycle Fixes

**Date:** 2026-05-18
**Prompt:** Chat request with `/tmp/test-script.js` and `/tmp/test-context.json`, asking to fix the HTTP/3 client path rather than disabling H3.

## Finding

The first reproduced failure shape covered a TCP/TLS API listener with no HTTP/3 UDP listener. A cached `Alt-Svc` route or DNS HTTPS RR can still lead `mechanics-http-client` to try opportunistic H3 first. If that H3 probe is allowed to live under the large endpoint timeout, the endpoint call can fail as `request timed out` before the normal TCP/TLS connector route is attempted.

The later production-like symptom is stricter: the HTTP/3 API server is alive, accepting requests, and responding, but follow-up step executions still time out before the connector router sees a new request. That points at client-side lifecycle state, not server availability or slow connectors.

`mechanics-http-client::http3::Http3State` cached QUIC client endpoints inside the long-lived client. Earlier fixes made each request open a fresh H3 connection, but those connections were still built on a reused Quinn endpoint/UDP socket. If that endpoint state is poisoned by a previous cancelled, timed-out, or half-closed request, later "fresh" H3 attempts can inherit the broken lower layer and fail before observable connector-router traffic appears.

The original endpoint cache also relied on address-family-sensitive UDP behaviour. IPv4 targets now bind an IPv4 UDP socket and IPv6 targets bind an IPv6 UDP socket, but the final fix goes further: the QUIC endpoint itself is now disposable per H3 attempt rather than stored across mechanics steps.

The follow-up production symptom was stronger: after one soft-failed endpoint step, later workflow/chat instances produced no `lo` packets for `tcp port 443 or udp port 443`. Because production is not on the dev box, I treated that as a long-lived mechanics-worker state issue rather than a local packet-capture observation. The code paths with that persistence are the shared `mechanics_http_client::Client` held by `DefaultEndpointHttpClient` and the Boa native-async job executor that carries D17 tail promises.

The JS-visible error string, such as ``Error: endpoint `llm` request failed: request timed out``, means the `mechanics:endpoint` builtin is being invoked and returning an endpoint transport failure to JavaScript. It does not prove the connector router/provider received the underlying request. In the observed packet captures, that lower-level request path was the missing/stale part.

## Fix Landed

`mechanics-http-client/src/request.rs` now bounds the pre-request opportunistic H3 phases separately from the endpoint/request timeout. If connect/setup/open/upload cannot complete in the short local windows, the client negative-caches H3 for the origin, removes the stale `Alt-Svc` entry, and continues to the normal TCP/TLS request path. The H3 feature remains enabled; this prevents an unavailable H3 path from being terminal before any H3 server can consume the request.

`mechanics-http-client/src/http3.rs` now creates a fresh QUIC endpoint for each HTTP/3 attempt. The resolved target address is selected before the endpoint is created, IPv4 targets bind `0.0.0.0:0`, IPv6 targets bind `[::]:0`, and `H3ResponseBody` retains the fresh endpoint owner until the response stream is consumed or dropped.

`mechanics-http-client::Client::fresh_transport()` now rebuilds only the hyper TCP/TLS transport while preserving request defaults and shared H3 discovery/cache state (`Alt-Svc`, HTTPS RR, and negative cache). `mechanics-core`'s default endpoint transport uses that fresh TCP/TLS transport for every `mechanics:endpoint` execution, so a poisoned TCP connection pool cannot survive from one endpoint call into the next. This does not remove tail promise polling and does not disable H3.

`mechanics-core/src/internal/http/transport.rs` also removes the outer timeout wrapper around the whole endpoint operation. Endpoint `timeout_ms` is now represented as an absolute deadline: the request/header phase receives the current remaining budget, then the response-body read receives the remaining budget. That keeps timeout coverage for both phases without dropping the whole transport future from outside at an arbitrary await point.

`mechanics-http-client` now carries that absolute request deadline into the `Response` object. Body collection for both hyper/TCP and H3 waits only until that deadline. This fixes the lifecycle bug where an H3 response could produce headers and then leave body collection pending, keeping the mechanics worker in tail polling and preventing later step executions from reaching any `:443` socket activity. It does not add an arbitrary short timeout for connector bodies; the configured request budget remains authoritative.

`mechanics-core/src/internal/executor.rs` now uses one native-async polling loop for both "wait until the main promise settles" and "continue to tail quiescence". The early reply is emitted inside that same loop, so already-started sibling endpoint futures are not dropped at the main-promise boundary. `mechanics-core/src/internal/runtime.rs` now consumes the combined run result directly instead of running a second tail poll with a fresh in-flight future set.

`mechanics-http-client/src/http3.rs` also adds `http3_state_uses_fresh_ipv4_udp_endpoint_for_ipv4_targets`, which verifies both the IPv4 bind and that successive endpoint creations do not reuse the same UDP endpoint. `mechanics-http-client/CHANGELOG.md` records the fix under `Unreleased`.

## Direct Runtime Regression

After review of `/tmp/test-script.js` and `/tmp/test-context.json`, I added three request-path regressions in `mechanics-http-client/src/request.rs`.

- `h3_advertised_on_tcp_only_api_port_falls_back_to_tcp` starts a real local HTTPS server on the API port, primes the client with `Alt-Svc: h3` on that same port, leaves UDP/H3 unbound, sends an `llm`-shaped JSON POST with HTTP/3 still enabled, and asserts that the TCP/TLS server receives `POST /llm` before the endpoint-style timeout expires. This is the direct disabled-H3-listener case from the prompt.
- `stale_h3_advertisement_falls_back_to_tcp_before_endpoint_timeout` starts a real local HTTPS server, primes the client with a stale `Alt-Svc: h3` route to a UDP blackhole, sends an `llm`-shaped JSON POST with HTTP/3 still enabled, wraps the whole send/body-read in an outer endpoint-style timeout, and asserts that the TCP/TLS server receives `POST /llm` before the timeout expires.
- `h3_timeout_negative_caches_origin_for_followup_tcp_request` starts a real local H3 peer that accepts the POST and request body but never emits response headers while a TCP/TLS server is listening on the same host/port. The first request still returns the configured timeout because the request was already consumed by H3, but the timeout now negative-caches H3 and removes `Alt-Svc`; the follow-up request reaches the TCP/TLS server instead of repeating the stale H3 route.

`mechanics-http-client/src/http3.rs` also makes the HTTP/3 response-header phase observe the same absolute request deadline. Once an H3 peer has accepted the request, the client no longer applies an arbitrary short probe timeout or replays the consumed request over TCP/TLS.

The `/tmp/test-tshark2.txt` follow-up narrowed the newer symptom further. That capture included completed localhost connector-service calls, including the large LLM-shaped POST, but the user clarified that the next step execution again timed out before a fresh connector-router call was made. The additional H3 timeout negative-cache change addresses that specific lifecycle gap: a timed-out H3 attempt no longer leaves the same origin's H3 route eligible for immediate reuse by the next endpoint call from the same client.

The later no-packets symptom is addressed by transport isolation and by preserving in-flight native async jobs across the early-reply boundary. `mechanics-core/src/internal/http/transport.rs` refreshes only the TCP/TLS hyper pool before executing an endpoint request, and the existing D17 tail-promise polling behaviour remains intact.

The endpoint transport lifecycle cleanup keeps that isolation workaround but removes the earlier belt-and-braces outer timeout. Timeout enforcement is now phase-local and deadline-based, which avoids using cancellation of the entire endpoint operation as normal control flow.

The `/tmp/test-tshark.txt` follow-up showed one successful H3 step and one stale H3 request with no usable response. The corresponding fix is response-body deadline propagation: later steps had no `:443` trace because the worker could remain occupied by the stale prior response body during tail polling, not because connector endpoints were slow.

The tightened `mechanics-core` runtime regression now verifies that a fire-and-forget endpoint call receives the early reply promptly but does not allow tail polling to finish while the endpoint future is still blocked. This directly covers the prior lifecycle gap where the main-promise wait could drop an in-flight endpoint future before the tail-poll phase had a chance to drive it.

## Validation

- `./scripts/rust-test.sh mechanics-http-client` passed.
- `./scripts/rust-lint.sh mechanics-http-client` passed.
- `./scripts/rust-test.sh mechanics-core` passed.
- `./scripts/rust-lint.sh mechanics-core` passed.
- `./scripts/pre-landing.sh` passed. The run auto-detected
  `mechanics-http-client`; it printed the existing cargo-doc filename
  collision warning, the known sandbox-limited `rustup` temp-file
  warning, and existing duplicate-lock warnings, then ended with
  `pre-landing: all checks passed`.
- `./scripts/check-md-bloat.sh` reported `97685 total`.
- `./scripts/tokei.sh` reported `198888 total`.

## Notes

This change does not disable HTTP/3 and does not change the H3 discovery policy. It makes unavailable H3 fail open to TCP/TLS for replayable requests, removes the long-lived QUIC endpoint/socket cache from the H3 client path, and prevents stale TCP/TLS transport state from crossing mechanics endpoint executions.
