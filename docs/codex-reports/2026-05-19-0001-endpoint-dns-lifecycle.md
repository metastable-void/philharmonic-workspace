# Endpoint DNS Lifecycle Timeout

**Date:** 2026-05-19
**Prompt:** Chat follow-up reporting `Error: endpoint \`llm\` request failed: request timed out` with no `tcp port 443`, `udp port 443`, or `tcp port 3002` packets.

## Finding

The follow-up symptom rules out connector slowness: the JavaScript `endpoint("llm")` call is made and catches `request timed out`, but packet capture shows no target-origin traffic. That means the transport can spend the endpoint deadline before opening a connector-router or connector-service socket.

One remaining pre-socket path was DNS. `mechanics-http-client` stored a `mechanics_dns::Resolver` inside the long-lived client and reused it for TCP/TLS resolution, HTTPS RR lookup, and H3 fallback address lookup. If that resolver state wedged after an earlier request, later endpoint executions could wait inside name resolution until the endpoint deadline expired, producing the JS-visible timeout without any `:443` or `:3002` trace.

## Fix

`mechanics-http-client` no longer stores shared DNS resolver runtime state in `ClientInner`. TCP/TLS resolution, HTTPS RR lookup, and H3 fallback address lookup now construct a resolver for the individual lookup. The client still preserves request defaults and the HTTP/3 discovery caches (`Alt-Svc`, HTTPS RR, and negative cache), but DNS runtime state no longer crosses endpoint executions.

The hyper DNS adapter also wraps TCP/TLS DNS lookup in a short independent timeout. A stuck resolver path now returns a DNS timeout promptly instead of consuming a multi-minute endpoint deadline before any socket is opened.

## Validation

- `./scripts/rust-test.sh mechanics-http-client` passed.
- `./scripts/rust-lint.sh mechanics-http-client` passed.
- `./scripts/pre-landing.sh` passed for the auto-detected
  `mechanics-http-client` change set and affected crates.
