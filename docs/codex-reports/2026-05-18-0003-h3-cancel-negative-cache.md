# HTTP/3 Response Owner and Cancel Negative Cache

**Date:** 2026-05-18
**Prompt:** Chat request about mechanics-worker endpoint requests timing out against api-bin's connector router after a forced long-running request, with `H3_NO_ERROR` / `Connection closed by client` when API `bind_h3` is disabled.

## Finding

`mechanics-core::DefaultEndpointHttpClient` applies the endpoint timeout around the full endpoint operation, including the response-body read. It also passes the same timeout into `mechanics-http-client::RequestBuilder`, where the H3 attempt uses that timeout to insert an HTTP/3 negative-cache entry on timeout.

Those two timers race. If the outer endpoint timeout fires first, it drops the in-flight H3 request future before `mechanics-http-client` can return `Error::Timeout` and negative-cache the origin. The caller still sees the expected endpoint timeout, but the shared client can keep trying the same cached Alt-Svc / H3 path on later requests.

The later operator correction changed the replay decision: for this client, H3 attempts are only made for empty bodies or replayable byte bodies, so a terminal H3 stream failure can fail open to the TCP/TLS path after negative-caching the origin.

Follow-up correction from the operator: the outer mechanics jobs were not timing out; the requests themselves were broken after H3 response headers. That pointed at the response-streaming H3 client path rather than only the cancellation path.

The h3 crate closes a connection with `H3_NO_ERROR` and reason `"Connection closed by client"` when the last `h3::client::SendRequest` is dropped. `mechanics-http-client` returned an H3 streaming response body after `recv_response()`, but then dropped the only `SendRequest` as `Http3State::request()` returned. Body reads then continued on a connection the client had just locally closed.

## Fix Landed

`mechanics-http-client/src/request.rs` now wraps each H3 attempt in an internal cancellation guard. If the H3 attempt future is cancelled or dropped before completion, the guard inserts the usual origin-level H3 negative-cache entry. If the attempt completes normally, the guard is disarmed and existing success / handshake / stream-error handling remains in charge.

Negative-cache insertion also removes any cached `Alt-Svc` entry for the origin. This matters when api-bin previously advertised H3 and is later restarted with `bind_h3` disabled: after one H3 failure, the client should not retry the old `Alt-Svc` route every time the shorter negative-cache window expires.

This means a forced or outer-timeout-cancelled long-running H3 request still fails for that request, but subsequent requests from the same client skip H3 instead of repeatedly entering the same stale route.

`mechanics-http-client/src/http3.rs` now stores the `SendRequest` owner inside `H3ResponseBody`. The owner stays alive until the caller consumes or drops the response body, so POST-over-H3 remains enabled and streamed H3 bodies are not cut off by a local client close immediately after headers.

`mechanics-http-client/CHANGELOG.md` records the fix under `Unreleased`.

Follow-up correction: the API-side H3 listener can be disabled while the client still auto-discovers HTTPS RR records that advertise H3. The H3 probe must therefore fail open to the TCP/TLS stack. `mechanics-http-client/src/request.rs` now treats any terminal H3 stream failure as `Ok(None)` from the H3 probe after inserting the origin negative-cache entry, so the existing H2/H1.1 path runs for the same replayable request. The H3 connect, setup, stream-open, and upload phases remain independently bounded before fallback; the response-header wait is not capped separately, so a successful long-running H3 endpoint remains governed by the caller's normal request timeout.

## Validation

- `./scripts/rust-lint.sh mechanics-http-client` passed after the response-owner fix.
- `./scripts/rust-test.sh mechanics-http-client` passed after the response-owner fix.
- The added in-process H3 regression pair in `mechanics-http-client/src/http3.rs`
  pins the response-owner bug directly: one test drops `SendRequest`
  before reading the delayed body and observes the client-close /
  `H3_NO_ERROR` error; the fixed path keeps `SendRequest` inside
  `H3ResponseBody` and reads the same delayed body successfully.
- `./scripts/pre-landing.sh mechanics-http-client` passed after the response-owner fix. The wrapper ran workspace lint, workspace tests, and the ignored `mechanics-http-client` H3 fixture placeholders.
- Follow-up H3-to-TCP fallback patch: `./scripts/rust-lint.sh mechanics-http-client`, `./scripts/rust-test.sh mechanics-http-client`, and `./scripts/pre-landing.sh` passed. The final pre-landing run auto-detected `mechanics-http-client` and completed workspace lint/tests plus the ignored `mechanics-http-client` H3 fixture placeholders.
- `mechanics-http-client/src/request.rs` now includes `h3_missing_udp_listener_falls_back_before_request_timeout`, which binds a silent local UDP socket and verifies the advertised-H3/no-H3-service path returns `Ok(None)` and negative-caches the origin before a large endpoint timeout can be consumed.
- `git diff --check` and `git -C mechanics-http-client diff --check` passed.
- `./scripts/check-md-bloat.sh` reported `97715 total`.
- `./scripts/tokei.sh` reported `197961 total`.

## Notes

This change does not touch SCK, COSE, ML-KEM/X25519/HKDF/AES-GCM, payload-hash binding, or `pht_` token generation.

The pre-landing run printed a sandbox-limited `rustup check` warning because `rustup` tried to create a temp file under read-only `/home/ubuntu/.rustup/tmp`. The script treats that check as non-fatal and all repository checks passed.
