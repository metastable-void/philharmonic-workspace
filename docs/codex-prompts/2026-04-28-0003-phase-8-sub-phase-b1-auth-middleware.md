# Phase 8 sub-phase B1 — auth middleware in `philharmonic-api`

**Date:** 2026-04-28
**Slug:** `phase-8-sub-phase-b1-auth-middleware`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Sub-phase A landed the `philharmonic-api` skeleton with a
placeholder auth middleware that leaves `RequestContext.auth =
None`. Sub-phase B0 landed the ephemeral-token primitives in
`philharmonic-policy 0.2.0` (Gate-1 + Gate-2 both passed,
commit `ea43e24`).

**This dispatch implements sub-phase B1: the real
authentication middleware** that replaces `auth_placeholder`
with long-lived `pht_` token lookup and ephemeral COSE_Sign1
verification. It populates `RequestContext.auth` with the
authenticated `AuthContext`.

B1 is crypto-touching (it calls `verify_ephemeral_api_token`)
but introduces no new crypto primitives — it's a consumer of
already-reviewed code. The crypto code-review gate fires on
B1 before merge.

## References (read end-to-end before coding)

- [`docs/design/10-api-layer.md`](../design/10-api-layer.md)
  §"Authentication" — the long-lived + ephemeral verification
  flows (steps 1-6 for pht_, steps 1-10 for ephemeral).
  §"Distinguishing authentication contexts" — `AuthContext`
  enum shape.
- [`docs/design/11-security-and-cryptography.md`](../design/11-security-and-cryptography.md)
  §"Long-lived API tokens" §"Ephemeral API tokens" — token
  formats, verification flows, revocation.
- [`docs/design/crypto-proposals/2026-04-28-phase-8-ephemeral-api-token-primitives.md`](../design/crypto-proposals/2026-04-28-phase-8-ephemeral-api-token-primitives.md)
  §"Out of scope (sub-phase B0)" §"B1 handoff contract" —
  **normative requirements** on B1. Every item in the B1
  handoff contract must be implemented with tests.
- [`docs/notes-to-humans/2026-04-28-0006-b0-landed-whats-next.md`](../notes-to-humans/2026-04-28-0006-b0-landed-whats-next.md)
  — B1 scope overview.
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md) — §4, §5, §10.3,
  §10.4, §10.9, §11.
- `philharmonic-policy` 0.2.0 — the B0 primitives:
  `parse_api_token`, `TokenHash`, `verify_ephemeral_api_token`,
  `ApiVerifyingKeyRegistry`, `EphemeralApiTokenClaims`,
  `ApiTokenVerifyError`.
- `philharmonic-store` — `EntityStore`, `EntityStoreExt`,
  `ContentStore`, `ContentStoreExt`, `StoreExt`.
- `philharmonic-api/src/middleware/auth_placeholder.rs` — the
  file B1 replaces.
- `philharmonic-api/src/auth.rs` — the `AuthContext` enum
  B1 populates.

## Three-crate scope

B1 touches **three crates** (wider than A or B0):

1. **`philharmonic-store`** — adds `find_by_content` to
   `EntityStore` trait + `EntityStoreExt::find_by_content_typed`
   + mock implementation.
2. **`philharmonic-store-sqlx-mysql`** — MySQL implementation
   of `find_by_content` + index migration.
3. **`philharmonic-api`** — the auth middleware itself +
   builder dependency additions + tests.

The store extension is needed because `credential_hash` is a
**content slot** on `Principal` and `MintingAuthority` (see
`philharmonic-policy/src/entity.rs`). The existing
`find_by_scalar` only works on scalar attributes (`Bool`,
`I64`). To look up an entity by credential hash, B1 needs a
`find_by_content` query. The SQL is analogous to
`find_by_scalar` but joins `attribute_content` on
`content_hash`.

## Decisions fixed upstream

1. **Generic HTTP 401 for all verify failures.** Per the B1
   handoff contract: all verify failures (B0 typed errors AND
   B1 substrate checks) collapse to HTTP 401 with
   `{"error":{"code":"unauthenticated","message":"invalid
   token","correlation_id":"..."}}`. No leaking of kid /
   window / signature / expiry details externally. Internal
   `tracing::warn!` may log typed variants.

2. **Authority-tenant binding.** After looking up the minting
   authority by `claims.authority`, check
   `authority.tenant == claims.tenant`. Reject if mismatch.
   Place immediately after authority lookup, before epoch
   acceptance.

3. **Authority-epoch enforcement.** The authority's current
   `epoch` scalar (stored as `I64` in substrate) must equal
   `claims.authority_epoch`. Conversion rule:
   `u64::try_from(stored_i64)` — negative or out-of-range
   fails closed.

4. **Bearer token routing.** Parse `Authorization: Bearer
   <token>` header. Route on `pht_` prefix:
   - `pht_`-prefixed → long-lived path (see §"Long-lived
     lookup" below).
   - Otherwise → ephemeral path (see §"Ephemeral
     verification" below).
   - Missing/malformed header → generic 401 (same shape).

5. **No auth required on meta endpoints.** The `/v1/_meta/*`
   routes already work without auth (sub-phase A). B1's
   middleware should skip auth for paths matching
   `/v1/_meta/*` and leave `RequestContext.auth = None` for
   those routes. All other routes get the auth check.

6. **`RequestScopeResolver` runs before auth.** Scope
   resolution happens in the existing scope middleware (from
   sub-phase A). Auth middleware runs after scope and reads
   `RequestContext.scope` to cross-check tenant. But
   **tenant-scope enforcement** (reject if `claims.tenant !=
   request scope tenant`) is sub-phase C's job — B1 only
   populates `auth`, it doesn't enforce scope agreement.

## Scope

### In scope

#### 1. `philharmonic-store` additions

- **`EntityStore::find_by_content`** — new trait method:
  ```rust
  async fn find_by_content(
      &self,
      kind: Uuid,
      attribute_name: &str,
      content_hash: Sha256,
  ) -> Result<Vec<EntityRow>, StoreError>;
  ```
  Finds entities of the given kind whose latest revision's
  content attribute `attribute_name` has content hash equal
  to `content_hash`. Analogous to `find_by_scalar`.

- **`EntityStoreExt::find_by_content_typed`** — typed wrapper
  (blanket-impl, same pattern as `find_by_scalar_typed`).

- **Mock `EntityStore` in `entity.rs`** — extend the mock
  with a `set_find_by_content_response` setter and the impl.

#### 2. `philharmonic-store-sqlx-mysql` additions

- **`EntityStore::find_by_content` impl** — SQL:
  ```sql
  SELECT e.id, i.public, e.kind, e.created_at
  FROM entity e
  JOIN identity i ON i.internal = e.id
  JOIN attribute_content a ON a.entity_id = e.id
  WHERE e.kind = ?
    AND a.attribute_name = ?
    AND a.content_hash = ?
    AND a.revision_seq = (
        SELECT MAX(r.revision_seq)
        FROM entity_revision r
        WHERE r.entity_id = e.id
    )
  ```

- **Index migration** — add a key on `attribute_content` for
  the lookup:
  ```sql
  CREATE INDEX ix_attr_content_hash
      ON attribute_content (attribute_name, content_hash);
  ```
  Add this to the schema initialization function alongside
  the existing `CREATE TABLE` statements. If the schema uses
  `IF NOT EXISTS` for tables, use `CREATE INDEX IF NOT EXISTS`
  (MySQL 8+ supports this).

#### 3. `philharmonic-api` auth middleware

- **Replace `src/middleware/auth_placeholder.rs`** with
  `src/middleware/auth.rs` containing the real auth
  middleware function.

- **Auth middleware flow:**
  1. Extract `Authorization` header from request.
  2. If path starts with `/v1/_meta/`, skip auth — leave
     `RequestContext.auth = None`, call `next.run(request)`.
  3. If header is missing or not `Bearer <token>` → 401.
  4. If token starts with `pht_` → long-lived path:
     a. `parse_api_token(token) → TokenHash`.
     b. Compute the content address of `TokenHash`: store
        the 32 bytes as a `ContentValue`, get its `Sha256`
        content hash. (Use `philharmonic_types::Sha256::digest`
        or equivalent.)
     c. `find_by_content_typed::<Principal>("credential_hash",
        content_hash)` — find principal.
     d. If not found, try
        `find_by_content_typed::<MintingAuthority>(
        "credential_hash", content_hash)` — minting
        authorities authenticate as themselves.
     e. If still not found → 401.
     f. Load latest revision of the found entity.
     g. Check `is_retired` scalar == false; check the
        entity's tenant is not suspended (load the
        `Tenant` entity, check `TenantStatus`).
     h. Build `AuthContext::Principal { principal_id,
        tenant_id }` (or, for a minting authority, still
        `Principal` — minting authorities are persistent
        principals for auth purposes; the distinction
        matters at the minting endpoint, sub-phase G).
  5. Otherwise → ephemeral path:
     a. `verify_ephemeral_api_token(token_bytes, registry,
        now)` — calls the B0 primitive. On error → 401.
     b. Load `MintingAuthority` entity by
        `claims.authority` UUID (look up by entity ID, NOT
        by credential hash). If not found → 401.
     c. Check `authority.tenant == claims.tenant` (the
        authority-tenant binding from the B1 handoff
        contract). Mismatch → 401.
     d. Check authority is not retired (`is_retired` scalar).
        Retired → 401.
     e. Load the authority's `epoch` scalar
        (`ScalarValue::I64`). Convert to `u64` via
        `u64::try_from(stored_i64)`. Negative/out-of-range
        → 401 (fail closed). Check
        `epoch == claims.authority_epoch`. Mismatch → 401.
     f. Check tenant not suspended.
     g. Build `AuthContext::Ephemeral { subject, tenant_id,
        authority_id, permissions, injected_claims,
        instance_scope }`.
  6. Attach `RequestContext.auth = Some(auth_context)` to
     the request extensions.
  7. Call `next.run(request)`.

- **Builder updates** (`src/lib.rs`):
  - `PhilharmonicApiBuilder` gains two new required
    dependencies:
    - `store: Arc<dyn StoreExt>` (combined entity + content
      + identity store; verify this is the right combined
      trait or use separate `Arc<dyn EntityStoreExt>` +
      `Arc<dyn ContentStore>` — check what `StoreExt` bundles).
    - `api_verifying_key_registry: ApiVerifyingKeyRegistry`.
  - `build()` returns `BuilderError::MissingDependency` if
    either is absent.
  - The auth middleware is wired into the middleware chain
    replacing `auth_placeholder`:
    ```
    correlation_id → request_logging → scope_resolver →
        auth → authz_placeholder → handler
    ```

- **Error handling** (`src/error.rs`):
  - Add `ErrorCode::Unauthenticated` to the enum.
  - Add `ApiError::Unauthenticated` variant.
  - Maps to HTTP 401 + generic error envelope. The body
    is `{"error":{"code":"unauthenticated","message":"invalid
    token","details":null,"correlation_id":"..."}}`.

- **`AuthContext` updates** (`src/auth.rs`):
  - Add convenience methods for downstream handlers:
    `tenant_id() -> EntityId<Tenant>` (works for both
    variants), `is_ephemeral() -> bool`,
    `is_principal() -> bool`.
  - Conversion from `EphemeralApiTokenClaims` to
    `AuthContext::Ephemeral`: the `claims: CanonicalJson`
    field converts to `serde_json::Value` for
    `AuthContext::Ephemeral::injected_claims` (parse the
    canonical JSON text as `serde_json::Value`). This is
    the boundary where `CanonicalJson` (wire form) meets
    `serde_json::Value` (runtime form).

- **Remove `src/middleware/auth_placeholder.rs`** and update
  `src/middleware/mod.rs` to export the new `auth` module.

#### 4. Tests

- **`philharmonic-store` unit test** — `find_by_content` mock
  returns correct rows; typed variant checks kind.

- **`philharmonic-api` integration tests** in
  `tests/auth_middleware.rs`:
  - **Long-lived happy path**: mint a `pht_` token via
    `generate_api_token`, store the principal with the
    credential hash in a mock store, send a request with
    `Authorization: Bearer pht_...`, assert 200 + correct
    `AuthContext::Principal` in the response (use an
    inspection handler that serializes the auth context).
  - **Ephemeral happy path**: mint an ephemeral token via
    `mint_ephemeral_api_token`, set up a mock store with a
    matching `MintingAuthority` (correct tenant, correct
    epoch, not retired, tenant not suspended), send request,
    assert 200 + correct `AuthContext::Ephemeral`.
  - **Missing Authorization header** → 401.
  - **Malformed bearer** → 401.
  - **Invalid pht_ token** (wrong length) → 401.
  - **pht_ token not found in store** → 401.
  - **pht_ token found but principal retired** → 401.
  - **pht_ token found but tenant suspended** → 401.
  - **Ephemeral token with bad signature** → 401.
  - **Ephemeral token valid but authority not found** → 401.
  - **Ephemeral authority-tenant mismatch** → 401
    (the normative negative test from the B1 handoff
    contract: authority belongs to tenant B, claims.tenant =
    A, request scope = A, signature valid → reject).
  - **Ephemeral authority retired** → 401.
  - **Ephemeral authority epoch mismatch** → 401.
  - **Ephemeral authority epoch negative in substrate** → 401
    (the i64→u64 conversion negative test from the B1
    handoff contract).
  - **Meta endpoint without auth** → 200 (confirms
    `/v1/_meta/health` doesn't require auth).
  - All 401 responses assert the generic error envelope shape
    (code = `unauthenticated`, no kid/expiry/details leaked).

  Use mock stores (not testcontainers) for all B1 tests.
  Testcontainers integration comes in later sub-phases.

### Out of scope

- **Authz (permission enforcement)** — sub-phase C.
- **Tenant-scope enforcement** (claims.tenant vs
  RequestScope) — sub-phase C.
- **Instance-scope enforcement** — sub-phase C/D.
- **Workflow/endpoint/CRUD handlers** — sub-phases D-H.
- **Token minting endpoint** — sub-phase G.
- **Rate limiting, audit** — sub-phase H.
- **`cargo publish`** — sub-phase I.
- **Testcontainers integration tests** — later sub-phases.
- **Workspace-root `Cargo.toml` edits** (beyond `Cargo.lock`
  regeneration).

## Workspace conventions (recap)

- Edition 2024, MSRV ≥ 1.88.
- License `Apache-2.0 OR MPL-2.0`.
- `thiserror` for library errors; no `anyhow` in non-test.
- **No panics in library `src/`** (§10.3).
- **Library takes bytes, not file paths** (§10.4).
- **No `unsafe`** in `src/`.
- **Rustdoc on every `pub` item.**
- HTTP client split (§10.9): `philharmonic-api` is runtime →
  axum (hyper + tokio + rustls). No `ureq`.

## Pre-landing

```sh
./scripts/pre-landing.sh philharmonic-api
./scripts/pre-landing.sh philharmonic-store
```

Both must pass green.

## Git

You do NOT commit, push, branch, tag, or publish. Leave
the working tree dirty. Claude commits via
`./scripts/commit-all.sh` post-review.

Read-only git is fine.

## Verification loop

```sh
# Build + test all three crates
./scripts/pre-landing.sh philharmonic-store
./scripts/pre-landing.sh philharmonic-api
cargo test -p philharmonic-api --all-targets
cargo test -p philharmonic-store --all-targets
cargo doc -p philharmonic-api --no-deps

# Status
git -C philharmonic-store status --short
git -C philharmonic-store-sqlx-mysql status --short
git -C philharmonic-api status --short
git -C . status --short
```

## Missing-context gating

- If `StoreExt` doesn't bundle entity + content stores as
  expected, check what trait combination the evaluation code
  in `philharmonic-policy/src/evaluation.rs` uses
  (`S: EntityStoreExt + ContentStore`). Use the same pattern.
- If `philharmonic-types::Sha256` doesn't have a `digest`
  constructor from raw bytes, look at how content-hashing
  works in `philharmonic-store/src/content.rs` and adapt.
- If mock stores in `philharmonic-store` don't support
  the combined query pattern B1 needs, extend the mocks.
- If MySQL 8's `CREATE INDEX IF NOT EXISTS` syntax differs
  from what the schema file uses, adapt.
- If any architecturally-significant surprise: STOP and flag.

## Action safety

- No `cargo publish`, no `git push`, no branch creation.
- Edits allowed in: `philharmonic-api/`,
  `philharmonic-store/`, `philharmonic-store-sqlx-mysql/`,
  and `Cargo.lock` in the workspace root.
- No destructive ops.
- No new crypto. B1 calls existing primitives only.

## Deliverables

1. `philharmonic-store/src/entity.rs` — `find_by_content`
   trait method + ext + mock.
2. `philharmonic-store-sqlx-mysql/src/entity.rs` — MySQL impl.
3. `philharmonic-store-sqlx-mysql/src/schema.rs` — index.
4. `philharmonic-api/src/middleware/auth.rs` — real auth.
5. `philharmonic-api/src/middleware/mod.rs` — updated exports.
6. `philharmonic-api/src/lib.rs` — builder gains store +
   registry deps; middleware chain updated.
7. `philharmonic-api/src/error.rs` — `Unauthenticated`
   variant.
8. `philharmonic-api/src/auth.rs` — convenience methods +
   conversion from claims.
9. `philharmonic-api/Cargo.toml` — new deps
   (`philharmonic-store`, `philharmonic-policy` ≥ 0.2.0).
10. `philharmonic-api/tests/auth_middleware.rs` — integration
    tests (14+ tests per §"Tests").
11. Removed: `philharmonic-api/src/middleware/auth_placeholder.rs`.

Working tree: dirty across three submodules + parent. Do not
commit.

## Structured output contract

1. **Summary** (3-6 sentences).
2. **Files touched** — every file added / modified / removed.
3. **Verification results** — pre-landing output for both
   crates, test counts, `cargo doc` clean.
4. **Residual risks / TODOs**.
5. **Git state** per submodule + parent.
6. **Dep versions**.

## Completeness contract

- Auth middleware replaces `auth_placeholder` completely (no
  placeholder code remains).
- All 14+ integration tests in §"Tests" exist and run green.
- Generic 401 envelope verified: no kid/expiry/details in
  any 401 response body.
- Authority-tenant binding test exists and asserts rejection.
- Negative-epoch test exists and asserts rejection.
- `find_by_content` exists on `EntityStore` with mock + MySQL
  impl.
- Builder enforces store + registry as required dependencies.
- Crate stays at 0.0.0 (not publishable yet).

---

## Outcome

Pending — will be updated after Codex run.
