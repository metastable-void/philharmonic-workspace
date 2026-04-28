# Phase 8 sub-phase F — principal, role, and minting-authority CRUD

**Date:** 2026-04-28
**Slug:** `phase-8-sub-phase-f-identity-crud`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Sub-phases A–E landed skeleton, auth, authz, workflow
endpoints, and endpoint-config CRUD. **This dispatch adds
the identity-management endpoint families:** principals,
roles, role memberships, and minting authorities per doc 10
§"Principal and role management" + §"Minting authority
management".

Non-crypto sub-phase (token generation uses the existing
`generate_api_token` from `philharmonic-policy`, which is
already reviewed). No crypto-review gate.

## References

- [`docs/design/10-api-layer.md`](../design/10-api-layer.md)
  §"Principal and role management" (lines 309-336) +
  §"Minting authority management" (lines 338-353).
- `philharmonic-policy` — `generate_api_token() ->
  (Zeroizing<String>, TokenHash)`, `TokenHash`,
  `Principal`, `PrincipalKind`, `RoleDefinition`,
  `RoleMembership`, `MintingAuthority`, `PermissionDocument`,
  `atom::*`.
- `philharmonic-api/src/routes/workflows.rs` and
  `src/routes/endpoints.rs` — patterns for route modules.
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md) — §10.3, §11.

## Scope

### In scope

#### 1. Principals (`src/routes/principals.rs`)

- `POST /v1/principals` — create. Requires
  `tenant:principal_manage`. Body: `{display_name,
  kind}` (kind: "user" or "service"). Calls
  `generate_api_token()` → stores `TokenHash` as
  `credential_hash` content slot. Returns 201 +
  `{principal_id, token}` where `token` is the plaintext
  `pht_...` string. **Token returned once; never stored
  in plaintext.**
- `GET /v1/principals` — list. Requires
  `tenant:principal_manage`. Paginated. Returns
  `{principal_id, display_name, kind, is_retired}`.
- `POST /v1/principals/{id}/rotate` — rotate credential.
  Requires `tenant:principal_manage`. Generates new token,
  replaces `credential_hash` content slot. Returns the new
  token **once**.
- `POST /v1/principals/{id}/retire` — retire. Requires
  `tenant:principal_manage`.

**Security: the plaintext `pht_` token is ONLY in the
response body of create and rotate. It MUST NOT appear in
logs, error messages, or any other response. Use
`Zeroizing<String>` for the token in handler scope; drop
after serializing the response.**

#### 2. Roles (`src/routes/roles.rs`)

- `POST /v1/roles` — create. Requires `tenant:role_manage`.
  Body: `{display_name, permissions}` where `permissions` is
  a JSON array of permission atom strings. Validates atoms
  against `ALL_ATOMS`. Stores as `PermissionDocument`
  content. Returns 201 + role_id.
- `GET /v1/roles` — list. Requires `tenant:role_manage`.
  Paginated.
- `PATCH /v1/roles/{id}` — modify (new revision). Requires
  `tenant:role_manage`. Body: partial update of
  `{display_name?, permissions?}`.
- `POST /v1/roles/{id}/retire` — retire. Requires
  `tenant:role_manage`.

#### 3. Role memberships (`src/routes/memberships.rs`)

- `POST /v1/role-memberships` — assign role to principal.
  Requires `tenant:role_manage`. Body: `{principal_id,
  role_id}`. Validates both exist in the tenant.
  Returns 201 + membership_id.
- `DELETE /v1/role-memberships/{id}` — remove (retire).
  Requires `tenant:role_manage`.

#### 4. Minting authorities (`src/routes/authorities.rs`)

- `POST /v1/minting-authorities` — create. Requires
  `tenant:minting_manage`. Body: `{display_name,
  permission_envelope, max_lifetime_seconds}`. Calls
  `generate_api_token()` → stores credential hash. Returns
  201 + `{authority_id, token}`. **Token returned once.**
- `GET /v1/minting-authorities` — list. Requires
  `tenant:minting_manage`. Paginated.
- `POST /v1/minting-authorities/{id}/rotate` — rotate
  credential. Returns new token **once**.
- `POST /v1/minting-authorities/{id}/bump-epoch` — bump
  `epoch` scalar. Invalidates outstanding ephemeral tokens.
- `POST /v1/minting-authorities/{id}/retire` — retire.
- `PATCH /v1/minting-authorities/{id}` — modify permission
  envelope or minting constraints (new revision).

#### 5. Error handling

- Invalid permission atoms → 400.
- Principal/role/authority not found → 404.
- Wrong tenant → 404.
- Already retired → 400.

#### 6. Tests

Integration tests in `tests/identity_crud.rs`:

- Principal lifecycle: create → list → rotate → verify new
  token works → retire.
- Role lifecycle: create → list → modify → retire.
- Membership: assign → list (verify membership) → remove.
- Authority lifecycle: create → list → bump epoch → rotate →
  modify envelope → retire.
- Permission enforcement: create principal without permission
  → 403.
- Token returned only once: create response includes token;
  list response does NOT include token.
- Cross-tenant isolation: principal from tenant A not visible
  to tenant B.

### Out of scope

- **Token minting endpoint** (`POST /v1/tokens/mint`) —
  sub-phase G.
- **Audit + rate limit** — sub-phase H.
- **`cargo publish`** — sub-phase I.

## Workspace conventions

- **No panics in library `src/`** (§10.3).
- **No `unsafe`**.
- **Token plaintext MUST NOT be logged.**
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
- No new crypto — uses existing `generate_api_token`.

## Deliverables

1. `src/routes/principals.rs` — 4 handlers.
2. `src/routes/roles.rs` — 4 handlers.
3. `src/routes/memberships.rs` — 2 handlers.
4. `src/routes/authorities.rs` — 6 handlers.
5. `src/routes/mod.rs` — wire all four into router.
6. `tests/identity_crud.rs` — integration tests (7+).

Working tree: dirty. Do not commit.

---

## Outcome

**Status:** Landed clean 2026-04-28.
**Claude review:** PASSES. No security issues. Token
plaintext handled via `Zeroizing<String>` borrow into
response structs, dropped after handler returns. Zero
`tracing` calls in any identity route module — no risk
of logging token material. No panics on library paths.

Files: `src/routes/principals.rs` (300 lines, 4 handlers),
`src/routes/roles.rs` (303 lines, 4 handlers),
`src/routes/memberships.rs` (251 lines, 2 handlers),
`src/routes/authorities.rs` (490 lines, 6 handlers),
`src/routes/identity.rs` (298 lines, shared helpers),
`src/routes/mod.rs`, `src/lib.rs`,
`tests/identity_crud.rs` (683 lines, 7 tests).

66 tests green. Clippy clean. Non-crypto; no Gate-2 required.
