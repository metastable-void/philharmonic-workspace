# HTTP/3 Disabled-Listener Fallback

**Date:** 2026-05-18
**Prompt:** Chat request with `/tmp/test-script.js` and `/tmp/test-context.json`, asking to fix the HTTP/3 client path rather than disabling H3.

## Finding

The runtime failure shape from the prompt is a TCP/TLS API listener with no HTTP/3 UDP listener. A cached `Alt-Svc` route or DNS HTTPS RR can still lead `mechanics-http-client` to try opportunistic H3 first. If that H3 probe is allowed to live under the large endpoint timeout, the endpoint call can fail as `request timed out` before the normal TCP/TLS connector route is attempted.

`mechanics-http-client::http3::Http3State` cached one QUIC client endpoint and always created that endpoint with an IPv6 unspecified UDP bind address. Later HTTP/3 attempts reused the same endpoint for every alternative service target, including IPv4-only addresses from DNS HTTPS records, Alt-Svc resolution, or normal address lookup.

That relies on platform-specific dual-stack UDP behaviour. On hosts where an IPv6 UDP socket cannot send to IPv4 targets, or where the server is re-enabled only on an IPv4 bind, the client can fail the HTTP/3 path even though the H3 server is healthy. The existing disabled-H3 fallback tests did not catch this because they only asserted fallback before a large timeout, not that an enabled IPv4 H3 target would use an IPv4 client socket.

## Fix Landed

`mechanics-http-client/src/request.rs` now bounds the whole opportunistic H3 probe separately from the endpoint/request timeout. If the H3 probe does not complete inside that small fail-open window, the client negative-caches H3 for the origin, removes the stale `Alt-Svc` entry, and continues to the normal TCP/TLS request path. The H3 feature remains enabled; this only prevents an unavailable H3 path from being terminal for replayable endpoint requests.

`mechanics-http-client/src/http3.rs` now keeps separate cached QUIC endpoints for IPv4 and IPv6 targets. The resolved target address is selected before the endpoint is fetched, and IPv4 targets bind `0.0.0.0:0` while IPv6 targets bind `[::]:0`.

`mechanics-http-client/src/http3.rs` also adds `http3_state_uses_ipv4_udp_endpoint_for_ipv4_targets`, which verifies the IPv4 bind directly under Tokio. `mechanics-http-client/CHANGELOG.md` records the fix under `Unreleased`.

## Direct Runtime Regression

After review of `/tmp/test-script.js` and `/tmp/test-context.json`, I added two request-path regressions in `mechanics-http-client/src/request.rs`.

- `h3_advertised_on_tcp_only_api_port_falls_back_to_tcp` starts a real local HTTPS server on the API port, primes the client with `Alt-Svc: h3` on that same port, leaves UDP/H3 unbound, sends an `llm`-shaped JSON POST with HTTP/3 still enabled, and asserts that the TCP/TLS server receives `POST /llm` before the endpoint-style timeout expires. This is the direct disabled-H3-listener case from the prompt.
- `stale_h3_advertisement_falls_back_to_tcp_before_endpoint_timeout` starts a real local HTTPS server, primes the client with a stale `Alt-Svc: h3` route to a UDP blackhole, sends an `llm`-shaped JSON POST with HTTP/3 still enabled, wraps the whole send/body-read in an outer endpoint-style timeout, and asserts that the TCP/TLS server receives `POST /llm` before the timeout expires.
- `stalled_h3_response_headers_fall_back_to_tcp_before_endpoint_timeout` starts a real local H3 peer that accepts the POST and request body but never emits response headers, while the same origin has a working TCP/TLS server. This reproduces the client-side shape that can otherwise consume the full mechanics endpoint timeout without the connector-router TCP path seeing the request.

`mechanics-http-client/src/http3.rs` also bounds the HTTP/3 response-header phase separately. That is not the disabled-listener case, but it closes the neighbouring fail-open gap for replayable endpoint requests once an H3 peer has accepted the QUIC connection.

## Validation

- `./scripts/rust-test.sh mechanics-http-client` passed.
- `./scripts/rust-lint.sh mechanics-http-client` passed.
- `./scripts/pre-landing.sh` passed. The run auto-detected
  `mechanics-http-client`; it printed the existing cargo-doc filename
  collision warning, the known sandbox-limited `rustup` temp-file
  warning, and existing duplicate-lock warnings, then ended with
  `pre-landing: all checks passed`.
- `./scripts/check-md-bloat.sh` reported `97871 total`.
- `./scripts/tokei.sh` reported `198735 total`.

## Notes

This change does not disable HTTP/3 and does not change the H3 discovery policy. It makes unavailable H3 fail open to TCP/TLS for replayable requests, and it makes the cached QUIC client endpoint match the remote address family so H3 remains usable when the API-side H3 listener is re-enabled.
