# Sub-phase B1 — Claude code review (pre-Gate-2)

**Author:** Claude Code · **Audience:** Yuka ·
**Date:** 2026-04-28 (Tue) JST afternoon

Codex round `task-moi5ay26-k0fwf2` (~16 min). Code across
three submodules: `philharmonic-store`,
`philharmonic-store-sqlx-mysql`, `philharmonic-api`.

## Verdict

**PASSES** Claude-level code review. No security issues
found. Implementation faithfully follows the B1 handoff
contract from the B0 proposal. Ready for human review.

B1 is crypto-touching (calls `verify_ephemeral_api_token`),
so Yuka's code-review gate fires before merge. The review
scope is narrow: correct call-site usage of the already-
reviewed B0 primitive, authority-tenant binding, error
collapsing, no key material in logs.

## Security checklist

### Generic 401 for all verify failures ✅

`AuthFailure` is a private enum (line 293) with 16 variants
covering every rejection path. It implements `Debug` (for
`tracing::warn!(?failure, ...)`) but is **never externally
visible**. All code paths that encounter an `AuthFailure`
call `unauthenticated_response(correlation_id)` (line 269)
which emits a fixed HTTP 401 + `{"error":{"code":
"unauthenticated","message":"invalid token",...}}`.

The `assert_unauthenticated` test helper (line 261-275)
verifies every 401 response: correct status, correct code,
correct message, `details` is null, body does NOT contain
the kid string, "signature", "expiry", or "epoch".
Every negative test calls this helper. ✅

### Authority-tenant binding ✅

Line 182: `if authority_tenant != claims.tenant { return
Err(AuthFailure::AuthorityTenantMismatch) }`. Placed
immediately after authority lookup and before epoch check
— matches the B1 handoff contract exactly.

Test `ephemeral_authority_tenant_mismatch_returns_generic_401`
(line 453): authority on tenant B, token claims tenant A,
request scope = tenant A, valid signature → asserts 401. ✅

### Authority-epoch enforcement ✅

Line 187-190: loads `epoch` as `i64` scalar, converts via
`u64::try_from(stored_epoch)` (negative → fail-closed as
`NegativeAuthorityEpoch` → 401), checks equality against
`claims.authority_epoch`.

Test `ephemeral_authority_epoch_mismatch_returns_generic_401`
(line 482): stored epoch = 8, claims epoch = 7 → 401.
Test `ephemeral_authority_negative_epoch_returns_generic_401`
(line 496): stored epoch = -1 → 401. ✅

### Bearer routing ✅

Line 92: `if token.starts_with(TOKEN_PREFIX)` routes to
long-lived path; otherwise to ephemeral path. Ephemeral
tokens are base64url-decoded (line 162) before passing to
`verify_ephemeral_api_token`.

### Meta endpoint auth skip ✅

Line 60: `if request.uri().path().starts_with(META_PREFIX)
{ return next.run(request).await }` — skips auth, leaves
`RequestContext.auth = None`. Test
`meta_endpoint_without_auth_still_succeeds` confirms 200
on `/v1/_meta/health` without any bearer. ✅

### No panics in library src ✅

Only one `unwrap_or_else` at line 290 (fallback to fresh
uuid for correlation_id when context is missing) — not a
panic path. No `.unwrap()` / `.expect()` / `panic!` /
`todo!` on reachable library paths.

### No key material in logs ✅

`AuthFailure::Debug` (line 320-349) prints variant names
and structural details (store errors, etc.) but never
prints the bearer token bytes, token hash, or signing key
material. The `tracing::warn!(?failure, ...)` at lines 68
and 76 logs the `Debug` output — safe because `AuthFailure`
doesn't carry sensitive data as fields (the token string is
owned by the caller, not stored in the error).

### Credential lookup via `find_by_content` ✅

Line 121: `ContentValue::new(token_hash.0.to_vec()).digest()`
computes the content address of the token hash bytes. This
is then passed to `find_by_content_typed::<Principal>` and
(on miss) `find_by_content_typed::<MintingAuthority>`.

The `find_by_content` trait method was added to
`EntityStore` with a default impl that returns a fatal
error (line 167-176 in store/entity.rs) — existing store
implementations that don't override it will fail closed.
MySQL impl at store-sqlx-mysql/entity.rs:364-407 uses
the correct SQL pattern with an index. ✅

### CanonicalJson → serde_json::Value conversion ✅

`AuthContext::from_ephemeral_claims` (auth.rs:60-74) uses
`serde_json::from_slice(claims.claims.as_bytes())` to
convert the canonical JSON bytes to `serde_json::Value`
for the runtime `injected_claims` field. Errors map to
`AuthFailure::InjectedClaimsJson` → 401. ✅

## Test coverage

15 integration tests in `tests/auth_middleware.rs`:
- 2 happy paths (long-lived + ephemeral)
- 1 missing Authorization header → 401
- 1 malformed bearer → 401
- 1 invalid pht_ format → 401
- 1 pht_ not found → 401
- 1 pht_ principal retired → 401
- 1 pht_ tenant suspended → 401
- 1 ephemeral bad signature → 401
- 1 ephemeral authority not found → 401
- 1 ephemeral authority-tenant mismatch → 401
- 1 ephemeral authority retired → 401
- 1 ephemeral epoch mismatch → 401
- 1 ephemeral negative epoch → 401
- 1 meta endpoint without auth → 200

Plus the pre-existing 7 tests from sub-phase A (middleware
chain, error envelope, correlation ID) still pass.

All 22 `philharmonic-api` tests + 22 `philharmonic-store`
tests green. Clippy clean.

## Store crate changes (review scope)

Minimal footprint:
- `philharmonic-store/src/entity.rs` — `find_by_content`
  trait method with default fatal-error impl.
- `philharmonic-store/src/ext.rs` — no changes to `StoreExt`
  (it's a supertrait of `EntityStore`, so `find_by_content`
  is automatically available).
- `philharmonic-store-sqlx-mysql/src/entity.rs` — MySQL
  impl: SQL mirrors `find_by_scalar` but joins
  `attribute_content` on `content_hash`.
- `philharmonic-store-sqlx-mysql/src/schema.rs` — index
  `ix_attr_content_hash (attribute_name, content_hash)` added
  inline to the `CREATE TABLE attribute_content` statement.

## Middleware chain ordering

The builder (lib.rs:156-171) layers are applied in reverse
(axum processes outermost layer first):
```
correlation_id → request_logging → scope_resolver →
    auth (with Extension<AuthState>) → authz_placeholder → handler
```
This matches the sub-phase A spec. Auth runs after scope
resolution and before authz, which is correct. ✅

## Noted observations (not defects)

- **`AuthState` is `Extension`-based, not `State`-based.**
  The auth middleware receives its dependencies via
  `Extension<AuthState>` rather than axum's typed `State`.
  This works because `Extension` propagates through layers.
  A typed-`State` approach would require the router's state
  type to include `AuthState`, which adds generic-parameter
  complexity. The `Extension` pattern is pragmatic for
  middleware that carries its own dependency bag.

- **`exactly_one` helper (line 258-267)** rejects ambiguous
  credentials (>1 entity with the same credential hash).
  This is a data-integrity guard — in a healthy substrate,
  credential hashes are unique per entity. The helper fails
  closed rather than picking the first match.

- **Existing sub-phase A tests updated.** The `middleware_chain`,
  `correlation_id`, and `error_envelope` tests were refactored
  to use the shared `common::basic_builder()` helper so they
  work with the new builder that requires store + registry.
  The test behavior is unchanged.

## What Yuka should focus on

1. **The `AuthFailure` → 401 collapsing.** Every path that
   returns `Err(AuthFailure::*)` eventually hits the
   `unauthenticated_response` function. Confirm no path
   leaks variant-specific details to the HTTP response.
2. **`ContentValue::new(token_hash.0.to_vec()).digest()`**
   — confirm this is the correct way to compute the content
   address of a `TokenHash`'s 32 bytes, matching how
   credential_hash content slots are populated during
   principal creation.
3. **The `find_by_content` SQL** in
   `philharmonic-store-sqlx-mysql` — confirm the
   `MAX(revision_seq)` subquery correctly filters to the
   latest revision.
4. **Base64url decoding of ephemeral tokens** (line 162) —
   confirm `URL_SAFE_NO_PAD` is the correct alphabet for
   COSE_Sign1 bytes that might arrive as HTTP Bearer values.
