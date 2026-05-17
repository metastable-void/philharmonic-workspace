# API H3 Listener Stop Investigation

**Date:** 2026-05-17
**Prompt:** Chat request to investigate `/tmp/production-api.log` after api-bin stopped serving and production `ss -ln` no longer showed `:443`.

## Scope

This report uses the supplied `/tmp/production-api.log` copy and the local source checkout. I am intentionally not using the current workspace host's socket state as evidence, because the production machine is not this machine.

## Finding

The log does not show a startup bind failure. It shows a successful API startup on May 15 at 08:35:10: schema migration ran, then `philharmonic-api listening on [::]:443 (https+h3, h3 [::]:443)`, then one API verifying key and one connector realm loaded.

The likely cause of `:443` disappearing is the final HTTP/3 line:

```text
May 16 15:39:16 chat philharmonic-api[78825]: philharmonic-api HTTP/3 listener stopped: internal: incoming connection rejected: the cryptographic handshake failed: error 40: peer is incompatible: NoSignatureSchemesInCommon
```

That line maps directly to a code path where one rejected QUIC/H3 handshake is treated as a fatal listener error. In `mechanics-http-server/src/server.rs:273-279`, `endpoint.accept()` yields an incoming QUIC connection and `incoming.accept()` is called. If `incoming.accept()` rejects the handshake, the accept loop returns `Error::Internal("incoming connection rejected: ...")` instead of logging and continuing.

`philharmonic/src/server/https.rs:59-100` then explains why that H3 error can also remove the TCP HTTPS listener. The TCP+TLS accept loop and the H3 handle run inside one spawned task. The task uses `tokio::select!`; if the H3 handle completes first, it logs `philharmonic-api HTTP/3 listener stopped: ...`, exits the select, calls `h3_handle.shutdown()`, and then the spawned task ends. Ending that task drops the `TcpListener` created at `philharmonic/src/server/https.rs:26`, so production `ss -ln` would no longer show `:443`.

The hundreds of earlier TLS handshake failures are probably background internet traffic or incompatible clients. They are handled differently: TCP TLS handshake failures are logged at `philharmonic/src/server/https.rs:75` inside a per-connection task and then that task returns; HTTPS connection failures are similarly logged at `philharmonic/src/server/https.rs:85`. Those errors do not stop the listener. The final H3 error is unique because it resolves the shared H3 listener handle and tears down the listener task.

## Log Shape

The copied log has 948 records by `awk` line count; `wc -l` reports 947 because the file lacks a trailing newline. The relevant counts are:

- `running schema migration`: 1
- `listening on`: 1
- `TLS handshake failed`: 920
- `HTTPS connection failed`: 24
- `HTTP/3 listener stopped`: 1

Top recurring TLS messages were `BadCertificate` (524), `Connection reset by peer` (135), `NoCipherSuitesInCommon` (101), and `InvalidContentType` (61). None of those recurring TCP/TLS messages were the terminal listener-stop log.

## Process-Exit Caveat

The code path above explains why the listener disappeared. It does not, by itself, prove that the whole `philharmonic-api` process exited. `bins/philharmonic-api-server/src/main.rs:359-367` starts the server, logs that it is listening, and then waits in the reload loop. `start_tls_axum_server` returns after spawning the listener task, so a dead listener task is not propagated back to `main`.

If production truly showed the process exiting, that part needs host-side evidence not present in `/tmp/production-api.log`: `journalctl -u philharmonic-api` around May 16 15:39:16, `systemctl status philharmonic-api`, or supervisor logs. The generated systemd unit uses `Type=simple` and `Restart=on-failure` (`philharmonic/src/server/install.rs:152-155`), so systemd would not restart a still-running process whose detached listener task died.

## Recommended Fixes

The immediate production mitigation is to disable `bind_h3` and restart the API service, leaving TCP HTTPS active without advertising or binding HTTP/3 until the code is fixed.

The code fix should be two-sided:

1. In `mechanics-http-server`, treat `incoming.accept()` handshake rejection as a per-connection warning and continue the accept loop. A malformed or incompatible QUIC client should not be fatal to the H3 listener.
2. In `philharmonic::server::https::start_tls_axum_server`, do not let H3 listener completion terminate the TCP HTTPS accept loop. If H3 dies, either log and keep HTTPS alive, or supervise/restart only the H3 side.

A regression test should assert that a synthetic or induced H3 accept failure does not finish the HTTP/3 handle and does not stop the TCP HTTPS listener.
