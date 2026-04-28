# Phase 8 sub-phase G — token minting endpoint (crypto-touching)

**Date:** 2026-04-28
**Slug:** `phase-8-sub-phase-g-token-mint`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Sub-phases A–F landed skeleton, auth, authz, workflow
endpoints, endpoint-config CRUD, and identity-management
CRUD. **This dispatch adds the token-minting endpoint** per
doc 10 §"Minting endpoint": `POST /v1/tokens/mint`.

Crypto-touching: calls `mint_ephemeral_api_token` from
`philharmonic-policy`. Code-review gate fires before merge.

## References

- [`docs/design/10-api-layer.md`](../design/10-api-layer.md)
  §"Minting endpoint" (lines 355-417) — full spec.
- [`docs/design/crypto-proposals/2026-04-28-phase-8-ephemeral-api-token-primitives.md`](../design/crypto-proposals/2026-04-28-phase-8-ephemeral-api-token-primitives.md)
  §"Out of scope" §"G handoff contract" — normative
  requirements on the minting endpoint.
- `philharmonic-policy` — `mint_ephemeral_api_token(
  signing_key, claims, now)`, `ApiSigningKey`,
  `EphemeralApiTokenClaims`, `CanonicalJson`,
  `MAX_INJECTED_CLAIMS_BYTES`, `MintingAuthority`,
  `PermissionDocument`, `atom::MINT_EPHEMERAL_TOKEN`.
- `philharmonic-api/src/routes/authorities.rs` — existing
  authority route module (sub-phase F).
- `philharmonic-api/src/middleware/auth.rs` — auth flow that
  already looks up minting authorities.
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md) — §10.3, §10.4,
  §11.

## Scope

### In scope

#### 1. Minting route (`src/routes/mint.rs`)

`POST /v1/tokens/mint`

- Requires `Principal` authentication where the authenticated
  entity is a `MintingAuthority` (the authority authenticates
  using its own long-lived `pht_` credential, same as any
  principal). Requires `mint:ephemeral_token` permission.
- **Note on AuthContext shape**: sub-phase B1's auth
  middleware wraps minting authorities as
  `AuthContext::Principal { principal_id, tenant_id }` where
  `principal_id` is actually a `MintingAuthority` UUID (known
  design compromise from B1, finding 4 of B1 audit). The mint
  handler needs to look up the minting authority by this UUID
  to read its `permission_envelope`, `epoch`, and
  `minting_constraints` (max_lifetime_seconds).

**Processing flow (per doc 10 §355-409):**

1. Extract `AuthContext::Principal { principal_id, tenant_id }`
   from `RequestContext`. If not Principal → 403.
2. Look up the entity by `principal_id` UUID. If its kind is
   `MintingAuthority::KIND` → proceed as minting authority.
   If kind is `Principal::KIND` → 403 ("only minting
   authorities can mint tokens"). If not found → 500.
3. Load the authority's latest revision. Read:
   - `permission_envelope` content slot → parse as
     `PermissionDocument`.
   - `epoch` scalar → `i64`, convert via `u64::try_from`.
   - `minting_constraints` content slot → parse as JSON
     with `max_lifetime_seconds` field.
   - `is_retired` scalar → must be false.
   - `tenant` entity ref → must match `tenant_id`.
4. Validate `lifetime_seconds` from the request: must be > 0,
   must be ≤ authority's `max_lifetime_seconds`, must be ≤
   system maximum (24h = 86400).
5. **Permission clipping**: intersect
   `requested_permissions` with the authority's
   `permission_envelope`. Permissions not in the envelope are
   silently stripped (log the stripped atoms at
   `tracing::info!` level for audit; **do not log the full
   injected claims**).
6. Validate `injected_claims`:
   - Serialize to `CanonicalJson`.
   - Check `canonical.as_bytes().len() <=
     MAX_INJECTED_CLAIMS_BYTES` (4 KiB). Reject if exceeded.
7. If `instance_id` provided: validate it exists as a
   `WorkflowInstance` within the tenant.
8. Construct `EphemeralApiTokenClaims`:
   - `iss`: the deployment's issuer string (from the
     `ApiSigningKey`'s configured kid prefix, or a builder-
     supplied issuer string — check how the verifying-key
     registry entry's `issuer` field is set and mirror it).
   - `iat`: `UnixMillis::now()`.
   - `exp`: `iat + lifetime_seconds * 1000`.
   - `sub`: `request.subject`.
   - `tenant`: `tenant_id.internal().as_uuid()`.
   - `authority`: authority entity's internal UUID.
   - `authority_epoch`: the loaded `epoch` value.
   - `instance`: `request.instance_id` (optional).
   - `permissions`: the clipped permission list.
   - `claims`: the `CanonicalJson` from step 6.
   - `kid`: the `ApiSigningKey`'s kid.
9. Call `mint_ephemeral_api_token(signing_key, claims, now)`.
10. Encode the resulting COSE_Sign1 bytes as base64url
    (URL_SAFE_NO_PAD).
11. Return the response.

**The `ApiSigningKey` is a new builder dependency.** The
builder gains:
```rust
pub fn api_signing_key(mut self, key: ApiSigningKey) -> Self
pub fn issuer(mut self, issuer: String) -> Self
```

#### 2. Audit record

Doc 10 says to record a `TokenMintingEvent` audit record
with "subject identifier and authority ID only; not the full
injected claims." For sub-phase G, log this via
`tracing::info!` with structured fields. A formal
`AuditEvent` entity write is sub-phase H's scope.

#### 3. Error handling

- Not a minting authority → 403.
- Authority retired → 403.
- Authority tenant mismatch → 403.
- `lifetime_seconds` exceeds maximum → 400.
- `injected_claims` exceeds 4 KiB → 400.
- `instance_id` not found in tenant → 400.
- Signing failure → 500 (generic, no crypto details).
- All 403/400/500 use the structured error envelope.

#### 4. Tests (`tests/token_mint.rs`)

Using mock stores + real `ApiSigningKey` + registry:

- **Happy path**: authority mints a token, verify it decodes
  to the expected claims.
- **Permission clipping**: request permissions beyond
  envelope → response token carries only envelope-
  intersected permissions.
- **Lifetime exceeded**: request > authority max → 400.
- **Injected claims too large**: > 4 KiB → 400.
- **Instance-scoped mint**: request with `instance_id`,
  verify token carries instance.
- **Non-authority principal tries to mint** → 403.
- **Retired authority** → 403.
- **Cross-tenant authority** → 403.
- All error responses use structured envelope.

### Out of scope

- **`AuditEvent` entity writes** — sub-phase H.
- **Rate limiting** — sub-phase H.
- **Operator endpoints** — sub-phase H.
- **`cargo publish`** — sub-phase I.

## Workspace conventions

- **No panics in library `src/`** (§10.3).
- **No `unsafe`**.
- **Do not log injected claims or token bytes.**
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
- No new crypto — calls existing `mint_ephemeral_api_token`.

## Deliverables

1. `src/routes/mint.rs` — minting endpoint handler.
2. `src/routes/mod.rs` — wire mint route.
3. `src/lib.rs` — builder gains `api_signing_key` + `issuer`.
4. `tests/token_mint.rs` — integration tests (8+).

Working tree: dirty. Do not commit.

---

## Outcome

Pending — will be updated after Codex run.
