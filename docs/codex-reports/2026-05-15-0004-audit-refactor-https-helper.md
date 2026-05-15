# Audit refactor: shared HTTPS helper

**Date:** 2026-05-15
**Prompt:** HUMANS.md §Priority: Audit & refactor; follow-up: feature-gate philharmonic meta-crate additions.

This slice removed the duplicated HTTPS + HTTP/3 Axum accept-loop from `bins/philharmonic-api-server/src/main.rs` and `bins/philharmonic-connector/src/main.rs`. The shared implementation now lives in `philharmonic/src/server/https.rs` as `philharmonic::server::https::start_tls_axum_server`, with `validate_tls_server_files` preserving the existing reload-time certificate/key parse check.

The meta-crate feature split is intentional:

- `server` gates the existing deployment-bin infrastructure: shared CLI args, TOML config loading, install support, key generation, and SIGHUP reload handling.
- `server-key-material` gates the key-file helpers and their `hex`/`zeroize` dependency surface.
- `server-https` gates the new HTTPS/HTTP3 helper and its `axum`, `hyper-util`, `mechanics-http-server`, `tokio`, and `tokio-rustls` dependency surface.

I kept `server-https` separate from the existing `https` feature because `https` means the mechanics TLS re-export path (`mechanics/https`). The API and connector binaries need deployment-server TLS/H3 glue, not the mechanics runtime re-export tree. Their bin-local `https` feature now maps to `philharmonic/server-https`.

The default `philharmonic` feature set includes `server`, `server-key-material`, and `server-https` so default workspace checks still compile these shipped helper paths. Consumers that use `default-features = false` now get none of the deployment helper modules unless they opt in explicitly.

