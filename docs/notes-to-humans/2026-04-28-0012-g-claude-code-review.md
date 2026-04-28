# Sub-phase G — Claude code review + Codex audit resolution

**Author:** Claude Code · **Audience:** Yuka ·
**Date:** 2026-04-28 (Tue) JST late afternoon

Crypto-touching sub-phase (calls `mint_ephemeral_api_token`).
Code-review gate fires.

## Verdict

**PASSES** after Codex audit fixes. Three findings fixed,
one deferred.

## What the endpoint does

`POST /v1/tokens/mint` — the minting authority authenticates
with its long-lived `pht_` credential, the handler validates
the request, clips permissions, constructs claims, signs a
COSE_Sign1 token, and returns it as base64url.

## Implementation review

### Crypto call site (`mint.rs:106`)

`mint_ephemeral_api_token(&state.signing_key, &token_claims,
iat)` — signing key from builder, claims constructed locally,
`iat` = `UnixMillis::now()` (same value used in the claims'
`iat` field). Error → generic `"token signing failed"` 500.
No crypto error details leaked. ✅

### Permission clipping (`mint.rs:284-309`)

Intersects `requested_permissions` with authority's
`permission_envelope`. Stripped atoms logged at
`tracing::info!` for audit. Duplicates removed via
`HashSet`. ✅

### 4 KiB claims cap (`mint.rs:311-327`)

Checks both raw JSON and canonical JSON bytes against
`MAX_INJECTED_CLAIMS_BYTES`. ✅

### Lifetime validation (`mint.rs:263-283`)

Validates > 0, ≤ authority max, ≤ 24h system max
(`SYSTEM_MAX_LIFETIME_SECONDS = 86_400`). ✅

### Authority checks (`mint.rs:137-174, 198-236`)

- Confirms authenticated entity is `MintingAuthority::KIND`
  (rejects ordinary `Principal::KIND` → 403).
- Loads authority revision: checks tenant binding, checks
  `is_retired`, checks `permission_envelope` contains
  `mint:ephemeral_token`, reads `epoch` (via
  `u64::try_from(i64)`), reads `minting_constraints`.
- ✅

### Audit logging (`mint.rs:117-123`)

`tracing::info!` with subject + authority_id + tenant_id +
instance_id. **No injected claims. No token bytes.** ✅

### Token encoding (`mint.rs:114`)

`URL_SAFE_NO_PAD.encode(token_bytes)` — base64url without
padding. ✅

### No logging of sensitive material

Zero `tracing` calls in the module that include token bytes,
injected claims, or signing key material. The two
`tracing::warn!` calls log only the error type for signing/
serialization failures. The `tracing::info!` for stripped
permissions logs only known permission atom strings (after
the validation fix). ✅

### No panics on library paths

No `.unwrap()` / `.expect()` / `panic!` on reachable
library code paths. ✅

## Codex audit findings + resolutions

Report at
[`docs/codex-reports/2026-04-28-0005-phase-8-g-token-mint-audit.md`](../codex-reports/2026-04-28-0005-phase-8-g-token-mint-audit.md).

### Finding 1 (Medium): Builder doesn't bind signing key to verifier registry

**Fixed.** `build()` now validates:
- `registry.lookup(signing_key.kid())` exists.
- `entry.issuer == builder_issuer`.

Mismatch → `BuilderError::ConfigurationMismatch`. This
catches the most common misconfiguration (wrong kid or
wrong issuer at startup) before any request is served.

New `BuilderError::ConfigurationMismatch(&'static str)`
variant added.

Five test files updated to use
`common::test_api_verifying_key_registry()` instead of
`ApiVerifyingKeyRegistry::new()` — the validation caught
stale empty-registry fixtures.

### Finding 2 (Medium): Mint can return tokens exceeding `MAX_TOKEN_BYTES`

**Fixed.** After `token.to_bytes()`, added:
```rust
if token_bytes.len() > MAX_TOKEN_BYTES {
    return Err(ApiError::InvalidRequest(...));
}
```

Also: `clip_permissions` now deduplicates via `HashSet` —
duplicate requested atoms are collapsed before encoding.

### Finding 3 (Low-Medium): Arbitrary strings logged as stripped permissions

**Fixed.** New `validate_permission_atoms` function runs
before clipping: every `requested_permissions` entry is
checked against `ALL_ATOMS`. Unknown atoms → 400. Only
known permission strings reach the log path.

### Finding 4 (Low): Reserved injected-claim names not enforced

**Deferred.** The `claims` field is nested inside the
COSE_Sign1 payload under a top-level `claims` key — it
cannot override signed fields like `iss`, `tenant`,
`authority`, `kid`. No authoritative reserved-name list
exists in the design docs. The risk is limited to
downstream workflow scripts that might merge subject claims
with framework fields — a design decision best made when
that merge surface is implemented (Phase 9).

## What Yuka should focus on for Gate-2

1. **`mint_ephemeral_api_token` call site** — confirm
   signing key + claims + iat are correctly threaded.
2. **Builder validation** — confirm the kid/issuer
   consistency check in `build()` catches the intended
   misconfiguration.
3. **`MAX_TOKEN_BYTES` post-serialization check** — confirm
   the 400 response is returned before base64 encoding.
4. **`validate_permission_atoms`** — confirm unknown atoms
   are rejected before any logging or clipping.
5. **Audit log fields** — confirm no injected claims or
   token bytes in any `tracing` call.

## Test coverage

9 integration tests in `tests/token_mint.rs`:
- Happy path (mint + verify round-trip)
- Permission clipping (out-of-envelope stripped)
- Authority max lifetime exceeded → 400
- Oversized injected claims → 400
- Instance-scoped minting
- Missing instance → 400
- Non-authority principal → 403
- Retired authority → 403
- Cross-tenant authority → 403

77 total tests green. Clippy clean.
