# Design violations found in deployment testing

**Date**: 2026-05-01
**Author**: Claude Code
**Severity**: Multiple design violations — each individually
blocking, collectively revealing shallow understanding of the
system's own architecture during implementation.

## Summary of mistakes

Every fix below was a case where the implementation violated
the system's documented design. In most cases the design doc
was correct and the code was wrong; in some cases the design
doc was also stale.

---

## 1. Scope middleware ran on `/v1/_meta/` paths

**What happened**: The branding endpoint (`/v1/_meta/branding`)
returned `unscoped_request` when the WebUI sent an
`X-Tenant-Id` header with the request (which it does for all
requests after login, including the branding fetch on reload).

**Design violation**: The auth middleware already skipped
`/v1/_meta/` paths (line 65, `META_PREFIX` check). The scope
middleware should have had the same check — meta endpoints are
public by design, not tenant-scoped.

**Root cause of the mistake**: The scope middleware was written
without considering that `_meta` paths need the same bypass as
auth. The two middleware layers were not treated as a unit.

**Fix**: Scope middleware now checks `starts_with(META_PREFIX)`
and assigns `RequestScope::Operator` without consulting the
resolver.

---

## 2. Executor posted to bare mechanics worker URL

**What happened**: `MechanicsWorkerExecutor` posted to
`http://127.0.0.1:3001/` but the mechanics worker listens on
`POST /api/v1/mechanics`. Result: 404.

**Design violation**: The executor was written without checking
the mechanics worker's actual route table. The mechanics
crate's `MechanicsServer` documentation explicitly says
"POST /api/v1/mechanics".

**Root cause of the mistake**: The executor was implemented
from the abstract `StepExecutor` trait surface without verifying
the concrete HTTP contract of the mechanics worker it wraps.

**Fix**: Executor appends `/api/v1/mechanics` to the configured
base URL.

---

## 3. Executor didn't send bearer token

**What happened**: The mechanics worker requires bearer token
auth on every request (`is_authorized` checks `tokens` set).
The executor didn't send any `Authorization` header.

**Design violation**: The mechanics worker's fail-closed auth
(empty `tokens = []` → 401 on all requests) is intentional and
documented. The executor's HTTP client should have sent the
configured token.

**Root cause of the mistake**: The executor was written as a
minimal HTTP client without considering the mechanics worker's
auth requirements. No `mechanics_worker_token` config field
existed.

**Fix**: Added `mechanics_worker_token` config field;
executor sends `Authorization: Bearer <token>`.

---

## 4. Connector router dispatched by Host header only

**What happened**: The lowerer built `HttpEndpoint` URLs
pointing at `http://127.0.0.1:3000/connector` (the embedded
connector router). The connector router dispatched by `Host`
header (`<realm>.connector.<domain_suffix>`). But the request
from the mechanics worker had `Host: 127.0.0.1:3000` — no
realm information in the host.

**Design violation**: The connector router was designed for
multi-host deployments with per-realm DNS. The embedded
single-binary deployment shape has no per-realm hostnames.
The lowerer can't make hostname assumptions — the URL is
configured, not derived.

**Root cause of the mistake**: The connector router was
designed for the separate-process deployment shape and never
adapted for the embedded deployment shape. The lowerer tried
to work around this by injecting a synthetic `Host` header,
which is fragile and makes hostname assumptions.

**Fix**: Connector router now supports path-based dispatch
(`/{realm}` route). The lowerer embeds the realm in the URL
path (`http://127.0.0.1:3000/connector/prod`). No hostname
assumptions needed. Host-based dispatch remains as a fallback
for deployments with per-realm DNS.

---

## 5. Connector router was inside auth middleware

**What happened**: The connector router was merged via
`extra_routes`, which sits inside the API server's
auth/scope/authz middleware stack. When the mechanics worker's
JS script called the connector router with a COSE_Sign1
connector token, the API's auth middleware tried to
authenticate it as an API token, failed, and returned 401
before the connector router ever ran.

**Design violation**: The connector router has its own auth
model (COSE_Sign1 verification by the connector service). It
must not go through the API's auth middleware. The two auth
models are architecturally separate — the API authenticates
tenant callers with `pht_` tokens; the connector authenticates
step-level calls with COSE_Sign1 tokens.

**Root cause of the mistake**: `extra_routes` was the only
mechanism for adding routes, and it merged inside the
middleware stack. When the connector router was added as
`extra_routes`, the middleware conflict was not considered.

**Fix**: Added `bypass_routes` builder method. Routes merged
via `bypass_routes` sit outside the auth/scope/authz
middleware stack. The connector router uses `bypass_routes`;
test endpoints that need auth continue to use `extra_routes`.

---

## 6. `TenantEndpointConfig` encrypted blob description

**What happened**: Design doc 11 (Security and Cryptography)
stated the encrypted blob includes "implementation name" and
described the lowerer as a "pure byte forwarding" encryption
translator.

**Design violation**: The `implementation` field is stored as
a separate plaintext content slot on the entity revision (not
inside the encrypted blob). The lowerer reads both
`implementation` (plaintext) and `encrypted_config`
(ciphertext), assembles a composite `{realm, impl, config}`
payload, and encrypts it to the realm's KEM key. It is not
byte-forwarding — it's assembly.

**Root cause of the mistake**: The design doc was written
before the `implementation` content slot was separated from
the encrypted blob. The doc was not updated when the entity
schema changed.

**Fix**: Updated design docs 10 and 11 to reflect the
separate `implementation` content slot and the lowerer's
assembly behavior.

---

## Pattern

Every mistake above is the same failure mode: implementing
one component without verifying how it connects to the
adjacent component's actual interface. The component's own
logic is correct in isolation; the integration point is wrong.

- Scope middleware: correct scope logic, wrong set of skipped
  paths (didn't match auth's skip set).
- Executor: correct HTTP client, wrong URL path (didn't check
  the worker's route table).
- Executor: correct request building, missing auth header
  (didn't check the worker's auth requirements).
- Lowerer: correct token/payload crypto, wrong transport
  assumptions (assumed hostname routing in an embedded
  deployment).
- Connector router: correct dispatch logic, wrong middleware
  layer (merged where API auth runs, not where it should
  bypass).
- Design doc: correct security model, stale description of
  what's encrypted vs. plaintext.
