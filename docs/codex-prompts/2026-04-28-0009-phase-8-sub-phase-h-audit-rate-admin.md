# Phase 8 sub-phase H — audit, rate limit, tenant admin, operator endpoints

**Date:** 2026-04-28
**Slug:** `phase-8-sub-phase-h-audit-rate-admin`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Sub-phases A–G landed skeleton through token minting. **This
dispatch adds the remaining endpoint families:** tenant
administration, audit log access, rate limiting, and operator
endpoints per doc 10 §"Tenant administration" + §"Audit log
access" + §"Rate limiting".

Non-crypto sub-phase. No crypto-review gate.

## References

- [`docs/design/10-api-layer.md`](../design/10-api-layer.md)
  §"Tenant administration" (lines 418-428) + §"Audit log
  access" (lines 430-434) + §"Rate limiting" (lines 436-449).
- `philharmonic-policy` — `AuditEvent` entity, `atom::*`
  (TENANT_SETTINGS_READ, TENANT_SETTINGS_MANAGE, AUDIT_READ,
  DEPLOYMENT_TENANT_MANAGE).
- `philharmonic-api/src/routes/` — existing route modules for
  pattern reference.
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md) — §10.3, §11.

## Scope

### In scope

#### 1. Tenant admin routes (`src/routes/tenant.rs`)

- `GET /v1/tenant` — read tenant settings. Requires
  `tenant:settings_read`. Returns tenant metadata (display
  name, status, created_at).
- `PATCH /v1/tenant` — update settings (new revision).
  Requires `tenant:settings_manage`. Body: `{display_name?}`.

Both routes are tenant-scoped (`RequestScope::Tenant`).

#### 2. Audit log routes (`src/routes/audit.rs`)

- `GET /v1/audit` — list audit events in the tenant.
  Requires `audit:read`. Paginated. Filterable via query
  params: `event_type`, `since` (UnixMillis), `until`
  (UnixMillis), `principal_id` (optional).

For sub-phase H, audit events are `AuditEvent` entities
in the store. Sub-phase H adds a helper to write audit
events from handlers (e.g., the token-minting audit record
in G currently logs to `tracing::info!` — H can optionally
upgrade that to an `AuditEvent` entity write, or leave it
for a follow-up).

#### 3. Rate limiting middleware (`src/middleware/rate_limit.rs`)

Per-tenant token-bucket rate limiting per doc 10 §"Rate
limiting". v1 implementation: single-node in-memory token
buckets.

- Use the `governor` crate (or implement a simple
  token-bucket; check cooldown first). If `governor` is
  available and cooldown-clear, prefer it. Otherwise a
  minimal hand-rolled bucket is fine.
- Bucket keyed by `(tenant_id, endpoint_family)` where
  endpoint families are: `workflow`, `credential`,
  `minting`, `audit`, `admin`.
- Per-minting-authority rate limit on the mint endpoint
  (keyed by `authority_id` in addition to tenant).
- Exceeded → 429 with `Retry-After` header + structured
  error envelope with `ErrorCode::RateLimited`.
- Default bucket configuration: builder accepts a
  `RateLimitConfig` with per-family rates. Sensible
  defaults (e.g., 100 req/s per tenant for workflow, 10
  req/s for minting).

Add `ErrorCode::RateLimited` to the enum. Add
`ApiError::RateLimited` → HTTP 429.

Wire the rate-limit middleware into the chain between
auth and authz:
```
correlation_id → request_logging → scope_resolver →
    auth → rate_limit → authz → handler
```

#### 4. Operator endpoints (`src/routes/operator.rs`)

Deployment-operator surface. These require
`RequestScope::Operator` and `Principal` auth with
deployment-level permissions.

v1 minimum per the ROADMAP:
- `POST /v1/operator/tenants` — create tenant. Requires
  `deployment:tenant_manage`. Body: `{subdomain_name,
  display_name}`. Validates subdomain against
  `validate_subdomain_name`. Returns 201 + tenant_id.
- `POST /v1/operator/tenants/{id}/suspend` — suspend
  tenant. Requires `deployment:tenant_manage`.
- `POST /v1/operator/tenants/{id}/unsuspend` — unsuspend.
  Requires `deployment:tenant_manage`.

#### 5. Tests

Integration tests in `tests/audit_rate_admin.rs`:

- Tenant settings read → 200.
- Tenant settings update → 200 + verify change.
- Audit list → paginated (seed some audit events first).
- Rate limit: burst N+1 requests → last one gets 429 with
  `Retry-After` header.
- Operator create tenant → 201.
- Operator suspend/unsuspend → 200.
- Operator endpoint from tenant scope → 403.
- Tenant endpoint from operator scope → 403.
- All error envelopes have correct codes.

### Out of scope

- **Distributed rate limiting** (Redis-backed) — post-v1.
- **Audit event writes from existing handlers** (upgrading
  G's `tracing::info!` to entity writes) — follow-up.
- **`cargo publish`** — sub-phase I.

## Workspace conventions

- **No panics in library `src/`** (§10.3).
- **No `unsafe`**.
- **Rustdoc on every `pub` item.**

## Pre-landing

```sh
./scripts/pre-landing.sh philharmonic-api
```

## Git

Do NOT commit, push, branch, tag, or publish.

## Verification loop

```sh
./scripts/pre-landing.sh philharmonic-api
cargo test -p philharmonic-api --all-targets
cargo doc -p philharmonic-api --no-deps
git -C philharmonic-api status --short
git -C . status --short
```

## Action safety

- Edits only in `philharmonic-api/` + `Cargo.lock`.
- No new crypto.

## Deliverables

1. `src/routes/tenant.rs` — 2 tenant-admin handlers.
2. `src/routes/audit.rs` — audit-list handler.
3. `src/routes/operator.rs` — 3 operator handlers.
4. `src/middleware/rate_limit.rs` — rate-limiting middleware.
5. `src/routes/mod.rs` — wire all new routes.
6. `src/lib.rs` — rate-limit config + middleware chain.
7. `src/error.rs` — `RateLimited` variant.
8. `Cargo.toml` — `governor` dep (if used).
9. `tests/audit_rate_admin.rs` — integration tests (9+).

Working tree: dirty. Do not commit.

---

## Outcome

**Status:** Landed clean 2026-04-28.
**Claude review:** PASSES. Hand-rolled token bucket (no
governor dep), `tokio::sync::Mutex` keyed by
`(tenant_id, family)`. Operator endpoints require
`RequestScope::Operator`. Audit events filterable by type,
time range, principal. No panics on library paths. No
sensitive data in logs.

Files: `src/routes/tenant.rs` (168), `src/routes/audit.rs`
(237), `src/routes/operator.rs` (191),
`src/middleware/rate_limit.rs` (277), `src/error.rs`,
`src/lib.rs`, `tests/audit_rate_admin.rs` (542, 9 tests).

86 tests green. Clippy clean. Non-crypto; no Gate-2 required.
