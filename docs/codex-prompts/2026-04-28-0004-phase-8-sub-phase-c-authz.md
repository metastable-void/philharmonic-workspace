# Phase 8 sub-phase C — authorization middleware in `philharmonic-api`

**Date:** 2026-04-28
**Slug:** `phase-8-sub-phase-c-authz`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Sub-phases A (skeleton) and B (auth: B0 primitives + B1
middleware) are done. `RequestContext.auth` is now populated
with a real `AuthContext` on every non-meta request. The
`authz_placeholder` middleware permits everything.

**This dispatch replaces the authz placeholder** with real
permission-atom evaluation, tenant-scope enforcement, and
ephemeral instance-scope enforcement.

Non-crypto sub-phase. No crypto-review gate.

## References (read end-to-end before coding)

- [`docs/design/10-api-layer.md`](../design/10-api-layer.md)
  §"Authorization" — permission enforcement, instance-scope,
  tenant-scope.
- [`docs/design/09-policy-and-tenancy.md`](../design/09-policy-and-tenancy.md)
  §"Permission atoms" §"Evaluation" — the atom list and
  evaluation model.
- [`ROADMAP.md` §Phase 8](../../ROADMAP.md) — sub-phase C
  scope.
- `philharmonic-policy` — `evaluate_permission(store,
  principal, tenant, required_atom)` for Principal contexts;
  `atom::*` constants; `PermissionDocument`.
- `philharmonic-api/src/middleware/authz_placeholder.rs` —
  the file C replaces.
- `philharmonic-api/src/auth.rs` — `AuthContext` enum with
  `tenant_id()`, `is_ephemeral()`, `is_principal()`.
- `philharmonic-api/src/scope.rs` — `RequestScope::Tenant(id)`
  / `RequestScope::Operator`.
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md) — §4, §10.3,
  §11.

## Scope

### In scope

#### 1. Authorization middleware (`src/middleware/authz.rs`)

Replace `authz_placeholder.rs` with `authz.rs` containing:

**Per-route permission declaration.** Each route (or route
group) declares which permission atom(s) it requires. For
sub-phase C, only the meta endpoints exist (which require
no permission — they're public). Sub-phases D–H will add
real endpoints and declare their atoms. The authz
middleware needs a mechanism so later sub-phases can
attach required permissions to routes without modifying
the middleware itself.

**Design: route-level `Extension<RequiredPermission>`.** Each
route (or route group) attaches a `RequiredPermission`
extension before the authz middleware runs. The middleware
reads it from the request extensions. If absent, the
route is treated as public (no permission required —
this covers `/v1/_meta/*`). If present, the middleware
enforces it.

```rust
#[derive(Clone, Debug)]
pub struct RequiredPermission(pub &'static str);
```

**Authz middleware flow:**

1. Extract `RequestContext` from extensions (must exist —
   auth middleware already set it). If missing → 500.
2. Extract `RequiredPermission` from extensions. If absent
   → public endpoint, skip authz, call `next.run(request)`.
3. If `RequestContext.auth` is `None` → 403 (unauthenticated
   caller hitting a protected endpoint; auth middleware only
   leaves `None` for meta paths, so this is a config error
   or a hand-crafted request).
4. **Tenant-scope enforcement.** If `RequestScope::Tenant(scope_tenant)`:
   check `auth_context.tenant_id() == scope_tenant`.
   Mismatch → 403 (cross-tenant). If `RequestScope::Operator`:
   skip tenant check (operator endpoints are outside tenant
   scope; sub-phase H enforces operator-specific permissions).
5. **Permission check:**
   - `AuthContext::Principal`: call
     `philharmonic_policy::evaluate_permission(store,
     principal_id, tenant_id, required_atom)`.
     Returns `false` → 403.
   - `AuthContext::Ephemeral`: check
     `permissions.contains(&required_atom.to_string())`.
     Not present → 403.
6. **Instance-scope enforcement** (ephemeral only). If
   `AuthContext::Ephemeral` has `instance_scope = Some(id)`:
   extract the instance ID from the request URL (if the
   endpoint takes one). If the URL's instance ID doesn't
   match the token's `instance_scope` → 403. If the
   endpoint doesn't take an instance ID, the instance-scope
   claim is irrelevant (the token can still access
   non-instance-scoped endpoints within its permission
   envelope).
   **For sub-phase C:** implement the instance-scope check
   infrastructure but defer URL-instance-ID extraction to
   sub-phase D (which adds the workflow endpoints that
   actually take instance IDs). The check should be
   structured so D can plug in the extraction without
   modifying the middleware.
7. Call `next.run(request)`.

**Error shape:** 403 responses use the structured error
envelope: `{"error":{"code":"forbidden","message":
"<context-appropriate>","correlation_id":"..."}}`. Add
`ErrorCode::Forbidden` to the enum.

#### 2. `AuthzState` for store access

The authz middleware needs the store for `evaluate_permission`
(Principal path). Wire it as `Extension<AuthzState>` or
reuse the existing `AuthState`'s store handle. Check what's
cleanest — if `AuthState` is already in the extensions and
carries the store, the authz middleware can extract it
directly. Otherwise add a separate `AuthzState`.

#### 3. Error updates (`src/error.rs`)

- Add `ErrorCode::Forbidden`.
- Add `ApiError::Forbidden` variant → HTTP 403.

#### 4. Middleware chain update (`src/lib.rs`)

Replace `authz_placeholder` with the real `authz` middleware
in the layer chain. The ordering stays:
```
correlation_id → request_logging → scope_resolver →
    auth → authz → handler
```

#### 5. Remove `src/middleware/authz_placeholder.rs`

Update `src/middleware/mod.rs`.

#### 6. Tests (`tests/authz_middleware.rs`)

Using mock stores (same `tests/common/` helpers from B1):

- **Principal happy path**: principal with a role granting
  the required permission → 200.
- **Principal permission denied**: principal without the
  required permission → 403.
- **Ephemeral happy path**: ephemeral token with the
  required permission in `permissions` → 200.
- **Ephemeral permission denied**: ephemeral token missing
  the required permission → 403.
- **Tenant-scope mismatch**: auth context tenant ≠ request
  scope tenant → 403.
- **Operator scope skips tenant check**: auth context
  tenant doesn't match (irrelevant), request scope is
  `Operator` → 200 (if permission is granted).
- **Unauthenticated caller on protected endpoint**: auth
  is `None` (meta-path somehow reaching a protected
  endpoint) → 403.
- **Public endpoint (no RequiredPermission)**: no
  permission declared → 200 regardless of auth state.
- **403 response has correct envelope shape**: assert
  `code = forbidden`, has correlation_id, no sensitive
  details.

### Out of scope

- **Instance-scope URL extraction** — sub-phase D plugs
  this in when workflow endpoints are added.
- **Operator-specific permission enforcement** — sub-phase H.
  For now, `Operator` scope skips tenant check; operator
  endpoints aren't implemented yet.
- **Real endpoint handlers** — sub-phases D-H.
- **`cargo publish`** — sub-phase I.

## Workspace conventions

- Edition 2024, MSRV ≥ 1.88.
- **No panics in library `src/`** (§10.3).
- **No `unsafe`** in `src/`.
- **Rustdoc on every `pub` item.**

## Pre-landing

```sh
./scripts/pre-landing.sh philharmonic-api
```

Must pass green.

## Git

Do NOT commit, push, branch, tag, or publish. Leave dirty.

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
- No destructive ops.

## Deliverables

1. `src/middleware/authz.rs` — real authz middleware.
2. `src/middleware/mod.rs` — updated exports.
3. `src/lib.rs` — middleware chain + re-export
   `RequiredPermission`.
4. `src/error.rs` — `Forbidden` variant.
5. `tests/authz_middleware.rs` — 9+ integration tests.
6. Removed: `src/middleware/authz_placeholder.rs`.

Working tree: dirty. Do not commit.

## Structured output contract

1. **Summary** (3-6 sentences).
2. **Files touched**.
3. **Verification results**.
4. **Residual risks / TODOs**.
5. **Git state**.

---

## Outcome

**Status:** Landed clean 2026-04-28.
**Claude review:** PASSES — see
[`docs/notes-to-humans/2026-04-28-0009-c-claude-code-review.md`](../notes-to-humans/2026-04-28-0009-c-claude-code-review.md).

Files: `src/middleware/authz.rs` (new), `src/store.rs` (new),
`src/lib.rs` (builder + exports), `src/error.rs` (Forbidden),
`tests/authz_middleware.rs` (11 tests), `tests/common/mod.rs`
(ContentStore impl on MockStore). Removed
`src/middleware/authz_placeholder.rs`.

42 tests green (9 unit + 15 auth + 11 authz + 2+2+3 existing).
Clippy clean. Non-crypto; no Gate-2 required.
