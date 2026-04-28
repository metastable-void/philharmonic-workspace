# Phase 8 sub-phase G token mint audit

**Date:** 2026-04-28
**Prompt:** docs/codex-prompts/2026-04-28-0008-phase-8-sub-phase-g-token-mint.md

## Scope

I reviewed the prompt, the current `philharmonic-api` implementation it
describes, and the adjacent token primitive in `philharmonic-policy`.
The main files reviewed were:

- `philharmonic-api/src/routes/mint.rs`
- `philharmonic-api/src/lib.rs`
- `philharmonic-api/src/middleware/auth.rs`
- `philharmonic-api/src/middleware/authz.rs`
- `philharmonic-api/tests/token_mint.rs`
- `philharmonic-api/tests/common/mod.rs`
- `philharmonic-policy/src/api_token.rs`
- `docs/design/10-api-layer.md`
- `docs/design/crypto-proposals/2026-04-28-phase-8-ephemeral-api-token-primitives.md`

The implementation is crypto-touching because
`philharmonic-api/src/routes/mint.rs` calls
`mint_ephemeral_api_token`. This should stay behind the
Yuka crypto call-site review gate.

## Summary

I did not find a direct minting authorization bypass. The handler requires a
tenant-scoped `AuthContext::Principal`, confirms that the authenticated row is a
`MintingAuthority`, rejects ordinary `Principal` rows, rejects retired
authorities, checks the authority tenant, requires the authority envelope to
contain `mint:ephemeral_token`, clips requested permissions to the authority
envelope, validates instance scope, enforces the 4 KiB injected-claims cap, and
passes `iat` as the `now` argument to `mint_ephemeral_api_token`.

The main issues are configuration correctness and hardening gaps rather than a
straight privilege escalation:

1. The API builder accepts an arbitrary mint issuer string and signing key
   without checking that they match the verifying-key registry that will later
   authenticate the minted tokens.
2. The endpoint can mint tokens that the verifier will reject as too large,
   because `subject` and the effective permission list are not bounded or
   deduplicated and mint-side code does not check `MAX_TOKEN_BYTES`.
3. `requested_permissions` accepts arbitrary strings and logs stripped values
   verbatim at `info`, even though the design language talks about stripped
   permission atoms.
4. The referenced design says injected claims should reject
   Philharmonic-reserved names, but the implementation only checks size and
   canonical JSON. The reserved-name set is not obvious in the local docs, so
   this needs either implementation or explicit deferral.

## Findings

### 1. Builder does not bind minting issuer/signing key to the verifier registry

**Severity:** Medium correctness / availability

`PhilharmonicApiBuilder::build` extracts `api_verifying_key_registry`,
`api_signing_key`, and `issuer` as separate dependencies, then constructs
`AuthState` from the registry and `MintState` from the signing key plus issuer
without checking that the minting configuration is actually verifiable by the
same API instance (`philharmonic-api/src/lib.rs:213-230`,
`philharmonic-api/src/lib.rs:250-251`). The mint route then copies that
builder-supplied issuer into `EphemeralApiTokenClaims.iss` and the signing
key's `kid` into `claims.kid` (`philharmonic-api/src/routes/mint.rs:91-103`).

The verifier is stricter: it looks up the protected-header `kid` in
`ApiVerifyingKeyRegistry`, checks the registry entry's acceptance window, and
rejects if `claims.iss != key_entry.issuer`
(`philharmonic-policy/src/api_token.rs:580-625`). A deployment can therefore
build a router that successfully returns freshly minted tokens that the same
router will later reject.

The prompt explicitly called out that `iss` should mirror how the verifying-key
registry entry's `issuer` field is set. The current builder makes that a
deployment convention rather than a fail-fast invariant.

**Recommended fix:** make `build()` validate the minting key configuration before
constructing the router:

- require `registry.lookup(signing_key.kid())` to exist;
- require the registry entry's `issuer` to equal the builder issuer;
- ideally also compare the registry verifying key to the verifying key derived
  from `ApiSigningKey`, which likely needs a small `philharmonic-policy` helper
  because `ApiSigningKey` currently exposes only `kid()`.

If multi-process deployments intentionally want to mint with a key absent from
the local verification registry, that exception should be a named builder mode
with explicit docs. The default safe behavior should be fail-fast consistency.

### 2. Mint endpoint can return tokens larger than the verifier accepts

**Severity:** Medium correctness / hardening

`verify_ephemeral_api_token` rejects serialized COSE tokens larger than
`MAX_TOKEN_BYTES` (16 KiB) (`philharmonic-policy/src/api_token.rs:14-17`,
`philharmonic-policy/src/api_token.rs:558-563`). The mint endpoint enforces the
4 KiB cap for `injected_claims`, but does not bound the `subject` string, does
not deduplicate or cap the number of effective permissions, and does not check
the final serialized token length before returning it
(`philharmonic-api/src/routes/mint.rs:273-297`,
`philharmonic-api/src/routes/mint.rs:299-315`,
`philharmonic-api/src/routes/mint.rs:407-414`).

Because `clip_permissions` pushes every requested permission that is present in
the envelope, duplicate requested atoms survive. A caller can request many
copies of an allowed atom or provide a very large `subject`; the endpoint may
return HTTP 200 with a token that later fails authentication due to the verifier
size limit.

This is not a privilege escalation, but it breaks the mint endpoint's contract:
a successful mint response should produce a usable token unless the token
expires or authority state changes afterward.

**Recommended fix:** enforce mint-side token viability before returning:

- deduplicate effective permissions while preserving a deterministic order, or
  reject duplicate requested atoms;
- add bounded lengths for `subject` and `requested_permissions`;
- after `token.to_bytes()`, reject if `token_bytes.len() > MAX_TOKEN_BYTES`
  before base64url encoding, mapping this to a 400-class invalid request if the
  cause is caller-controlled.

The last check is the minimum defense-in-depth guard. The input limits make the
API behavior easier to explain and test.

### 3. Arbitrary `requested_permissions` strings are logged as stripped permissions

**Severity:** Low to medium log-hygiene / audit-quality issue

The authority's stored `permission_envelope` parses as `PermissionDocument`,
which validates atoms against `ALL_ATOMS`
(`philharmonic-policy/src/permission.rs:85-105`). The incoming
`requested_permissions` vector, however, is just `Vec<String>` and is never
validated against `ALL_ATOMS` before clipping (`philharmonic-api/src/routes/mint.rs:273-297`,
`philharmonic-api/src/routes/mint.rs:407-414`).

Every requested string outside the envelope is copied into `stripped` and logged
with `tracing::info!(stripped_permissions = ?stripped, ...)`
(`philharmonic-api/src/routes/mint.rs:288-293`). The prompt asked to log stripped
atoms, not arbitrary caller-supplied strings. Today a caller can put long,
misleading, or sensitive values in `requested_permissions` and have them written
to the audit log path.

**Recommended fix:** decide the desired wire behavior for unknown permission
strings:

- strict option: reject unknown permission atoms with 400 before clipping, using
  the same `ALL_ATOMS` validation as identity-management routes;
- lenient option: silently strip unknown strings but log only a count, and log
  names only for known atoms denied by the envelope.

The strict option is simpler and keeps "stripped permissions" semantically equal
to "known permission atoms that the authority cannot delegate."

### 4. Injected-claim reserved-name validation from doc 10 is not implemented

**Severity:** Low spec gap, possibly medium depending on downstream merge rules

`docs/design/10-api-layer.md` says the minting endpoint validates
`injected_claims` size and shape, including "no Philharmonic-reserved claim
names" (`docs/design/10-api-layer.md:400-401`). The prompt's step list only
required size/canonicalization, and the implementation follows the prompt:
`canonical_claims` serializes, checks size, canonicalizes, and checks size again
(`philharmonic-api/src/routes/mint.rs:299-315`). It does not enforce object
shape or reserved names.

The immediate token format nests injected claims under the top-level `claims`
field, so injected JSON cannot override signed top-level fields like `iss`,
`tenant`, `authority`, or `kid` inside the COSE claim structure. However,
workflow execution later passes `injected_claims` through as
`SubjectContext.claims` (`philharmonic-api/src/routes/workflows.rs:679-699`).
If any downstream consumer merges subject claims with framework-managed fields,
reserved-name ambiguity can reappear there.

**Recommended fix:** either:

- define the reserved injected-claim names and enforce them in
  `canonical_claims`, with tests; or
- update `docs/design/10-api-layer.md` / the roadmap to mark this validation as
  deferred and explain why nested `claims` is sufficient for v1.

I did not find an authoritative reserved-name list in the reviewed docs, so this
needs a design decision before implementation.

## Positive Checks

- `POST /v1/tokens/mint` is wired into the route table
  (`philharmonic-api/src/routes/mod.rs:36-42`).
- The route rejects non-principal auth contexts and ordinary principals, then
  confirms the authenticated row kind is `MintingAuthority`
  (`philharmonic-api/src/routes/mint.rs:137-174`).
- Retired authority, tenant mismatch, missing authority revision, negative
  stored epoch, lifetime zero/exceeded, oversized canonical claims, and missing
  instance scope fail closed or return structured errors in the reviewed code
  paths (`philharmonic-api/src/routes/mint.rs:73-90`,
  `philharmonic-api/src/routes/mint.rs:183-224`,
  `philharmonic-api/src/routes/mint.rs:251-271`,
  `philharmonic-api/src/routes/mint.rs:318-349`).
- The signing call passes the same `iat` used in the claims as the primitive's
  `now` argument (`philharmonic-api/src/routes/mint.rs:84-106`), which matches
  the B0/G handoff contract.
- The minting-event log includes subject, authority, tenant, and instance but
  does not log injected claims or token bytes (`philharmonic-api/src/routes/mint.rs:116-123`).
- The auth middleware's ephemeral-token path enforces authority lookup,
  authority tenant binding, retired status, epoch equality, active tenant, and
  generic external 401s for verification/substrate failures
  (`philharmonic-api/src/middleware/auth.rs:172-214`,
  `philharmonic-api/src/middleware/auth.rs:287-294`,
  `philharmonic-api/src/middleware/auth.rs:338-367`).

## Test Coverage Notes

`philharmonic-api/tests/token_mint.rs` covers the requested happy path,
permission clipping, authority max lifetime, oversized injected claims,
instance-scoped minting, missing instance scope, ordinary principal rejection,
retired authority rejection, and cross-tenant authority rejection.

Coverage gaps worth adding:

- authority envelope lacks `mint:ephemeral_token` -> 403;
- lifetime of zero -> 400;
- lifetime above the 24-hour system maximum while still below authority max ->
  400;
- unknown or duplicate `requested_permissions` behavior once decided;
- builder issuer/kid/registry mismatch fails fast, if finding 1 is fixed;
- successful mint response cannot exceed `MAX_TOKEN_BYTES`, if finding 2 is
  fixed;
- injected-claim reserved names or explicit non-enforcement, if finding 4 is
  resolved.

## Verification

I ran:

```sh
./scripts/rust-test.sh philharmonic-api
```

Result: pass. The run reported 15 unit tests, 60 integration tests across the
crate's test binaries, and 1 doctest passing. `tests/token_mint.rs` ran 9 tests
and all passed.

No Rust files were modified by this audit. This report is the only intended
workspace change.
