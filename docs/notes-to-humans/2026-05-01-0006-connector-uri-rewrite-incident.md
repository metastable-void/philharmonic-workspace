# Connector URI rewrite incident — Claude missed, Codex fixed

**Date**: 2026-05-01
**Author**: Claude Code
**Severity**: Critical bug — connector routing broken in production

## What happened

The connector router's path-based dispatch (`/connector/{realm}`)
forwarded the full request URI to the upstream connector service.
A request to `/connector/prod` was forwarded with URI path
`/connector/prod` — the upstream connector service has no route
for that path and returned 404.

The correct behavior: strip the `/connector/{realm}` prefix from
the URI before forwarding, so the upstream sees `/` (or whatever
sub-path follows the realm segment).

## Claude's failure

Claude made three successive attempts to fix the connector
routing:

1. **Host header injection** — injected a synthetic `Host:
   prod.connector.localhost` header so the connector router's
   host-based dispatch would match. Fragile hostname assumption.
   User rejected: "no hostname assumptions can be made."

2. **Path-based dispatch** — added `/{realm}` route to the
   connector router and embedded realm in the lowerer's URL.
   But didn't strip the realm from the URI before forwarding.
   The upstream got `/prod` (or `/connector/prod`) instead of
   `/`. Still 404.

3. **Bypass axum routing** — moved connector dispatch out of
   axum's router/nest/oneshot chain into direct function calls.
   Fixed the axum state-propagation issue but still didn't strip
   the realm prefix from the forwarded URI. Still 404.

In all three attempts, Claude focused on the routing machinery
(how to get the request to the right handler) and never
considered what the upstream connector service actually receives
as the request URI. The URI rewrite was the actual bug.

## Codex's fix

Codex added `strip_path_realm` to the connector router crate and
`rewrite_connector_uri` to the API server binary. Both strip the
routing prefix (`/{realm}` or `/connector/{realm}`) from the
request URI before forwarding, leaving only the path suffix and
query string for the upstream.

The fix is the same pattern that the host-based dispatch already
did via `rewrite_uri` — replacing scheme + authority to point at
the upstream. The path-based dispatch additionally needs to
remove the realm segment from the path. Claude's implementation
of path-based dispatch missed this because `rewrite_uri` already
existed and seemed sufficient.

## Pattern

Same failure mode as the other design violations found today
(notes-to-humans 0005): implementing one component without
verifying what the adjacent component receives. Claude verified
that the connector router matched the request, verified that the
upstream was selected correctly, but never verified that the
forwarded request URI was correct for the upstream.

The specific blind spot: axum's `.nest()` strips prefixes
automatically, so when Claude was using `.nest("/connector",
inner)`, the prefix stripping happened implicitly. When Claude
moved to direct dispatch (bypassing `.nest()`), the implicit
prefix stripping was lost — and Claude didn't notice because it
was never explicitly modeled.
