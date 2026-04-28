# Sub-phase C — Claude code review

**Author:** Claude Code · **Audience:** Yuka ·
**Date:** 2026-04-28 (Tue) JST afternoon

Non-crypto sub-phase. No Gate-2 crypto review needed.

## Verdict

**PASSES.** Authorization middleware is clean and correctly
implements the doc 10 §"Authorization" spec.

## What landed

- **`src/middleware/authz.rs`** — real authorization:
  `RequiredPermission` extension declares per-route atoms,
  `RequestInstanceScope` extension for sub-phase D to plug
  in instance IDs. The middleware checks tenant-scope
  agreement, evaluates permissions (role-based for
  Principal via `evaluate_permission`, claim-list for
  Ephemeral), and enforces instance-scope.
- **`src/store.rs`** — `ApiStore` trait bundling
  `StoreExt + ContentStore` (needed because
  `evaluate_permission` loads content-addressed role
  permission documents). `ApiStoreHandle` delegation
  wrapper.
- **`src/lib.rs`** — builder now takes `Arc<dyn ApiStore>`,
  exports `RequiredPermission`, `AuthzState`,
  `RequestInstanceScope`, `authorize`. Middleware chain
  updated with `AuthzState` extension.
- **`src/error.rs`** — `ErrorCode::Forbidden` +
  `ApiError::Forbidden` → HTTP 403.
- **`authz_placeholder.rs`** removed.
- **11 authz integration tests** in
  `tests/authz_middleware.rs`.

## Security checklist

- **403 shape**: all forbidden responses use the generic
  envelope with `code = forbidden`. The
  `forbidden_response_has_structured_envelope_without_sensitive_details`
  test asserts no required atom or subject string leaks
  into the response body. ✅
- **Tenant-scope enforcement**: `tenant_scope_allows` checks
  `auth_context.tenant_id() == scope_tenant` for
  `Tenant` scope; `Operator` skips (correct — operator
  endpoints use separate permission atoms in sub-phase H).
  Test `tenant_scope_mismatch_returns_403`. ✅
- **Principal permission**: delegates to
  `evaluate_permission` (already tested in
  philharmonic-policy). Tests
  `principal_happy_path_allows_granted_permission` and
  `principal_permission_denied_returns_403` exercise it
  end-to-end via mock store with real role + membership
  + permission documents. ✅
- **Ephemeral permission**: `permissions.iter().any(|p| p
  == required.0)`. Test
  `ephemeral_happy_path_allows_claim_permission` and
  `ephemeral_permission_denied_returns_403`. ✅
- **Instance-scope**: `instance_scope_allows` checks if
  `RequestInstanceScope` extension matches the token's
  `instance_scope`. If no `RequestInstanceScope` attached
  (endpoint doesn't take an instance ID), the token's
  instance scope is irrelevant. Tests
  `ephemeral_instance_scope_mismatch_returns_403` and
  `ephemeral_instance_scope_is_irrelevant_without_request_instance_id`.
  ✅
- **Unauthenticated on protected endpoint** → 403 (not 401
  — this is correct: the request passed auth without error
  but has no auth context, which means something is
  misconfigured). Test
  `unauthenticated_caller_on_protected_endpoint_returns_403`.
  ✅
- **Public endpoints** (no `RequiredPermission` attached) →
  authz skips entirely, handler runs. Test
  `public_endpoint_without_required_permission_skips_authz`.
  ✅
- **No panics in src** — single `unwrap_or_else` on
  correlation_id fallback. ✅
- **No `unsafe`** ✅

## Notable design choices

- **`ApiStore = StoreExt + ContentStore`** is a good
  abstraction. The builder takes `Arc<dyn ApiStore>` instead
  of `Arc<dyn StoreExt>`, which ensures the store handle
  supports content-blob reads needed by
  `evaluate_permission`'s role-document loading. The
  `ApiStoreHandle` delegation wrapper implements all
  sub-traits by forwarding to the inner `Arc<dyn ApiStore>`.
- **`RequiredPermission` as an Extension** is the right
  pattern for route-level permission declaration. Sub-phases
  D–H attach it via `.layer(Extension(RequiredPermission(atom::*)))`
  on their route groups. No middleware modification needed
  when new endpoints are added.
- **`RequestInstanceScope` ready for D** — the authz
  middleware reads it if present, skips the check if absent.
  Sub-phase D just needs to extract the instance UUID from
  the URL and attach the extension.
