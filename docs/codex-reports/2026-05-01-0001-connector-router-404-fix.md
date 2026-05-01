# Connector router 404 fix

**Date:** 2026-05-01
**Prompt:** Direct chat request to fix 404s at `/connector/<realm>` and write a report.

## Summary

The observed 404s came from forwarding the public routing prefix to the
realm connector service. The embedded API route accepts
`/connector/<realm>`, but the connector service binary only registers
`POST /`. Before this fix, a request to `/connector/prod` selected the
`prod` realm and then forwarded the unchanged path downstream, so the
connector service saw `/connector/prod` and returned Axum's route-level
404.

The same shape existed in standalone path-based connector-router mode:
`/{realm}` selected the realm but forwarded `/{realm}` to the service.

## Changes

- `bins/philharmonic-api-server/src/main.rs` now rewrites
  `/connector/<realm>` to `/` before calling `dispatch_to_realm`.
- The same rewrite preserves any path below the realm prefix:
  `/connector/prod/health?trace=true` becomes `/health?trace=true`.
- `philharmonic-connector-router/src/dispatch.rs` now consumes
  `/{realm}` in path-based dispatch, so standalone router mode forwards
  `/prod?trace=true` to `/?trace=true`.
- Focused unit coverage was added for the API-server rewrite helper, and
  the connector-router path dispatch test now asserts the stripped
  upstream URI.

## Remaining notes

If `connector_dispatch` is empty or missing, the API server still falls
through to the WebUI fallback for `/connector/<realm>` instead of
returning an explicit connector-router configuration error. That was not
changed in this fix because it is a separate operator-experience issue
from the confirmed upstream 404.

