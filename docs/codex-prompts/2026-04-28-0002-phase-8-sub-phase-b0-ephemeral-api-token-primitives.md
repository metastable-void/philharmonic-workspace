# Phase 8 sub-phase B0 — ephemeral API token primitives in `philharmonic-policy`

**Date:** 2026-04-28
**Slug:** `phase-8-sub-phase-b0-ephemeral-api-token-primitives`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Phase 8 sub-phase A landed the `philharmonic-api` skeleton
(commit `51de953`). Sub-phase B (auth middleware) needs
ephemeral-API-token COSE_Sign1 mint + verify primitives.
These don't exist anywhere in the workspace — verified by
audit and recorded in
[`docs/notes-to-humans/2026-04-28-0003-ephemeral-token-primitives-gap.md`](../notes-to-humans/2026-04-28-0003-ephemeral-token-primitives-gap.md).

Yuka approved Gate-1 on the r2 proposal at commit `255eb71`.
Approval record:
[`docs/design/crypto-approvals/2026-04-28-phase-8-ephemeral-api-token-primitives.md`](../design/crypto-approvals/2026-04-28-phase-8-ephemeral-api-token-primitives.md).

**This dispatch implements sub-phase B0: the primitives
only, in `philharmonic-policy`.** Sub-phase B1 (auth
middleware in `philharmonic-api` that calls these primitives)
is a separate Codex round dispatched after B0 lands and
passes Gate-2 code review.

## References (read end-to-end before coding)

- [`docs/design/crypto-proposals/2026-04-28-phase-8-ephemeral-api-token-primitives.md`](../design/crypto-proposals/2026-04-28-phase-8-ephemeral-api-token-primitives.md)
  — **the authoritative Gate-1-approved proposal (r2).**
  Every type, function, constant, error variant, verify step,
  test vector, and naming convention is pinned there. **If
  anything in THIS prompt contradicts the proposal, the
  proposal wins. Flag contradictions and stop.**
- [`docs/design/09-policy-and-tenancy.md`](../design/09-policy-and-tenancy.md)
  §"Ephemeral token claims" §"Instance-scoped ephemeral
  tokens" §"Minting authorities" — the claim-field spec.
- [`docs/design/11-security-and-cryptography.md`](../design/11-security-and-cryptography.md)
  §"Ephemeral API tokens" §"Ephemeral API token signing key
  rotation" — lifetime, rotation, signing-key relationship.
- [`docs/design/10-api-layer.md`](../design/10-api-layer.md)
  §"Authentication" §"Ephemeral API tokens" — the 14-step
  verification order (proposal r2's expansion of 10's list).
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md):
  - §4 git workflow (signed-off, signed, scripts only).
  - §5 + §5.1 + §5.3 cooldown rule.
  - §10.3 no panics in library `src/`.
  - §10.4 libraries take bytes, not file paths.
  - §11 pre-landing checks.
- The existing Wave A implementation for pattern reference
  (same Ed25519 + COSE_Sign1 primitives, different claim
  shape):
  - `philharmonic-connector-client/src/signing.rs` — minting
    pattern (`LowererSigningKey::mint_token`).
  - `philharmonic-connector-service/src/verify.rs` — verify
    pattern (`verify_token_internal`, 11-step order).
- `philharmonic-types` — `EntityId<E>`, `Uuid`, `UnixMillis`,
  `CanonicalJson`, `CanonError`.
- `philharmonic-policy` current surface — `src/lib.rs`,
  `src/token.rs` (pht_ tokens), `src/sck.rs` (SCK
  encrypt/decrypt), `src/entity.rs` (entity kinds),
  `src/evaluation.rs` (permission evaluation).

## Crate state (starting point)

- `philharmonic-policy` is a published submodule at 0.1.0.
- `Cargo.toml` has `sha2`, `zeroize`, `base64`, `rand`,
  `serde`, `serde_json`, `philharmonic-types` already.
- `src/lib.rs` re-exports from `entity`, `error`,
  `evaluation`, `permission`, `sck`, `token`.
- B0 adds a new `api_token` module and bumps version to
  0.2.0.

Target after this round: working mint + verify primitives
with 2 positive known-answer tests + 19 negative-path tests
+ round-trip/property tests. `pre-landing.sh` green. Working
tree dirty (no commit, no push, no publish).

## Decisions fixed upstream — from the Gate-1 r2 proposal

These are NOT open questions. Do not deviate.

1. **Ed25519 + COSE_Sign1 via `ed25519-dalek 2.x` + `coset
   0.4.x` + `ciborium 0.2.x`.** Same stack as Wave A.
2. **`CanonicalJson` for the injected-claims field.** From
   `philharmonic_types::CanonicalJson` (RFC 8785 / JCS).
3. **`iat: UnixMillis` added to claims.** B0 enforces
   lifetime invariants at both mint and verify.
4. **`ApiSigningKey` does NOT derive `Debug`.** Manual
   redacted impl only (prints `kid` + `<redacted>`).
5. **Verify order is 14 steps.** Proposal r2 §"Verification
   order". Each step has a named `ApiTokenVerifyError`
   variant.
6. **`issuer: String` in `ApiVerifyingKeyEntry`.** Verify
   step 11 checks `claims.iss == entry.issuer`.
7. **Strict COSE header profile.** Protected = only `alg` +
   `kid`. Unprotected = empty. Any `crit` or unknown
   protected label → reject.
8. **`kid` profile:** `[A-Za-z0-9._:-]`, length 1..=128.
   Validated at mint, verify, and registry insert.
9. **Constants pinned:**
   - `MAX_TOKEN_BYTES = 16 * 1024`
   - `MAX_INJECTED_CLAIMS_BYTES = 4 * 1024`
   - `MAX_TOKEN_LIFETIME_MILLIS = 24 * 60 * 60 * 1000`
   - `ALLOWED_CLOCK_SKEW_MILLIS = 60_000`
   - `KID_MIN_LEN = 1`, `KID_MAX_LEN = 128`
10. **`verify_ephemeral_api_token_with_limits`** variant
    accepts caller-supplied limits, clamped to
    `min(default, caller)`.
11. **`mint_ephemeral_api_token` takes `now: UnixMillis`.**
    Validates lifetime + claims-size + kid-profile at mint
    as defense-in-depth.
12. **`proptest` as a dev-dep** for property/fuzz testing.
13. **No `subtle` for kid equality.** Plain `==` on
    signature-validated claim bytes.
14. **Publish deferred** to Phase 8 close. Crate stays at
    0.2.0 locally; `[patch.crates-io]` bridges until then.

## Scope

### In scope

1. **`Cargo.toml`** — bump `version = "0.2.0"`. Add deps:
   - `coset = "0.4"` (cooldown-clear, latest 0.4.2)
   - `ciborium = "0.2"` (cooldown-clear, latest 0.2.2)
   - `ed25519-dalek = "2"` (cooldown-clear, latest 2.2.0)
   - Dev-deps: `proptest = "1"` (cooldown-clear, latest
     1.11.0)
   - Existing deps: `philharmonic-types` (already present;
     provides `CanonicalJson`, `Uuid`, `UnixMillis`),
     `zeroize` (already present), `serde` / `serde_json`
     (already present).
   - **Cooldown rule**: verify each dep version via
     `./scripts/xtask.sh crates-io-versions -- <crate>`.
     Workspace-internal deps exempt per §5.3.

2. **`src/api_token.rs`** — new module. Full surface per the
   Gate-1 proposal r2 §"What lands":
   - `EphemeralApiTokenClaims` struct (iss, iat, exp, sub,
     tenant, authority, authority_epoch, instance, permissions,
     claims: CanonicalJson, kid).
   - `ApiSigningKey` (seed: Zeroizing<[u8;32]>, kid: String;
     manual redacted Debug; `from_seed`, `kid`).
   - `ApiSignedToken` newtype wrapping `coset::CoseSign1`
     (with `new`, `as_cose_sign1`, `into_cose_sign1`,
     `to_bytes`, `from_bytes`).
   - `ApiVerifyingKeyEntry` (vk, issuer, not_before,
     not_after).
   - `ApiVerifyingKeyRegistry` (new, insert → Result,
     lookup).
   - `RegistryInsertError` (KidProfileViolation,
     DuplicateKid).
   - `VerifyLimits` (4 fields, all clamped).
   - `mint_ephemeral_api_token(signing_key, claims, now) →
     Result<ApiSignedToken, ApiTokenMintError>`.
   - `verify_ephemeral_api_token(cose_bytes, registry, now)
     → Result<EphemeralApiTokenClaims, ApiTokenVerifyError>`.
   - `verify_ephemeral_api_token_with_limits(cose_bytes,
     registry, now, limits) → Result<…, …>`.
   - `ApiTokenMintError` (6 variants per proposal).
   - `ApiTokenVerifyError` (14 variants per proposal).
   - Constants (6 per proposal).
   - Internal helper: `validate_kid_profile(kid: &str) →
     Result<(), ...>` shared by mint/verify/registry-insert.
   - Internal helper: `verify_internal(cose_bytes, registry,
     now, limits) → Result<…, …>` — the 14-step core.

3. **`src/lib.rs`** — add `mod api_token;` and re-export
   the public surface from the new module.

4. **`CHANGELOG.md`** — add `[Unreleased]` entry describing
   the addition (ephemeral API token mint + verify + key
   registry + 14-step verification + CanonicalJson claims).

5. **Test vectors** at
   `tests/vectors/api_token/*.hex` / `*.json`:
   - Ed25519 keypair: RFC 8032 §7.1 TEST 1 (seed + public
     key hex).
   - Two positive claim sets (with/without instance) per
     proposal §"Test-vector plan" §"Claim set".
   - Expected CBOR claim bytes (hex) for each.
   - Expected COSE_Sign1 bytes (hex) for each.
   - 19 negative-path vectors per proposal §"Negative-path
     vectors" — each with its expected
     `ApiTokenVerifyError` variant.

6. **`tests/api_token_vectors.rs`** — integration test file:
   - Two positive known-answer tests: mint with known seed +
     claims → assert exact COSE_Sign1 hex matches committed
     vector; verify → assert claims round-trip.
   - 19 negative-path tests: each feeds the corresponding
     negative vector to `verify_ephemeral_api_token` and
     asserts the specific error variant.
   - Round-trip test: mint → serialize → verify → assert
     claims equality.
   - `proptest` fuzz: random `EphemeralApiTokenClaims` →
     mint → verify → assert round-trip; random non-canonical
     JSON → assert `ClaimsNotCanonical` rejection on
     the verify path.

7. **Unit tests colocated in `src/api_token.rs`**:
   - `validate_kid_profile` — valid/invalid examples.
   - `ApiSigningKey::fmt::Debug` — assert output contains
     `<redacted>` and does NOT contain the seed hex.
   - `VerifyLimits` clamping — assert limits > defaults get
     clamped down to defaults.
   - `ApiVerifyingKeyRegistry::insert` — duplicate-kid
     rejection, profile-violation rejection.
   - `EphemeralApiTokenClaims` serde round-trip
     (CBOR → claims → CBOR byte equality).

### Out of scope (flag; do NOT implement)

- **Auth middleware in `philharmonic-api`** — sub-phase B1.
- **Token minting endpoint** — sub-phase G.
- **Permission clipping, 4 KiB enforcement at the endpoint
  level** — sub-phase G (B0 does enforce 4 KiB at the
  primitive level as defense-in-depth).
- **Authority lookup, epoch check, tenant binding,
  instance-scope-vs-URL** — B1 substrate-state checks.
- **External error collapsing** — B1 HTTP-layer concern.
- **Signing-key file I/O / KMS** — deployment binary.
- **`cargo publish`, `git tag`, commit, push.** Claude
  handles those after Gate-2 review. Working tree stays
  dirty.
- **Workspace-root `Cargo.toml` edits.**
- **Edits to any other crate** besides `philharmonic-policy/`
  and `Cargo.lock` regeneration in the workspace root.
- **LowererSigningKey Debug redaction** — separate follow-up
  (Q8 from the proposal).

## Workspace conventions (recap)

- Edition 2024, MSRV ≥ 1.88.
- License `Apache-2.0 OR MPL-2.0`.
- `thiserror` for library errors; no `anyhow` in non-test
  paths.
- **No panics in library `src/`** (§10.3): no `.unwrap()` /
  `.expect()` / `panic!` / `unreachable!` / `todo!` /
  `unimplemented!` on reachable paths. Tests exempt.
- **Library takes bytes, not file paths** (§10.4).
- **No `unsafe`** in `src/`.
- **Rustdoc on every `pub` item.**
- Use `./scripts/*.sh` wrappers (not raw cargo).

## Pre-landing

```sh
./scripts/pre-landing.sh philharmonic-policy
```

Must pass green.

## Git

You do NOT commit, push, branch, tag, or publish. Leave
the working tree dirty in the submodule (and in the parent
if `Cargo.lock` changes). Claude commits via
`./scripts/commit-all.sh` post-review.

Read-only git is fine (`log`, `diff`, `show`, `status`).

## Verification loop

```sh
# Phase 0 — cooldown check
./scripts/xtask.sh crates-io-versions -- coset
./scripts/xtask.sh crates-io-versions -- ciborium
./scripts/xtask.sh crates-io-versions -- ed25519-dalek
./scripts/xtask.sh crates-io-versions -- proptest

# Build + test
./scripts/pre-landing.sh philharmonic-policy
cargo test -p philharmonic-policy --all-targets
cargo doc -p philharmonic-policy --no-deps

# Status
git -C philharmonic-policy status --short
git -C . status --short
```

## Missing-context gating

- If `philharmonic_types::CanonicalJson` doesn't exist or
  doesn't provide `as_str()` / `bytes()` / `from_value()`:
  grep the crate, adapt, and document. STOP if the type is
  fundamentally different.
- If `coset 0.4.x` API for `CoseSign1Builder`,
  `verify_signature`, protected-header inspection has changed
  materially from what `philharmonic-connector-service/src/verify.rs`
  uses: adapt within idiomatic patterns; STOP if the API
  shape is fundamentally different.
- If `ciborium` encoding of `CanonicalJson` (via its serde
  impl) doesn't produce a CBOR text string: investigate and
  adapt. STOP if the encoding shape is fundamentally wrong.
- If any dep is yanked or fails 3-day cooldown: pin prior
  version and note.
- If any architecturally-significant surprise: STOP and flag.

## Action safety

- No `cargo publish`, no `git push`, no branch creation,
  no tags.
- No edits outside `philharmonic-policy/` except `Cargo.lock`
  regeneration in the workspace root.
- No destructive ops.
- No new crypto primitives beyond Ed25519 + COSE_Sign1 as
  specified. If the round surfaces a need for anything else,
  STOP and flag.

## Deliverables

1. Updated `philharmonic-policy/Cargo.toml` with version
   0.2.0 + cooldown-checked deps.
2. `src/api_token.rs` — full module per §"In scope" #2.
3. `src/lib.rs` — mod declaration + re-exports.
4. `CHANGELOG.md` — `[Unreleased]` entry.
5. `tests/api_token_vectors.rs` — known-answer + negative +
   round-trip + proptest tests.
6. `tests/vectors/api_token/` — committed reference vector
   files.
7. Unit tests colocated in `src/api_token.rs`.

Working tree: dirty. Do not commit.

## Structured output contract

1. **Summary** (3-6 sentences). What landed, which proposal
   items are fully implemented, any deviations.
2. **Files touched** — every file added / modified.
3. **Verification results** — `pre-landing.sh` output, test
   counts (unit + integration + proptest + doctest), `cargo
   doc` clean.
4. **Residual risks / TODOs** — anything that didn't fit,
   any adaptation from the proposal (with justification).
5. **Git state** — `git -C philharmonic-policy status --short`
   and `git -C . status --short`.
6. **Dep versions used** — exact versions of `coset`,
   `ciborium`, `ed25519-dalek`, `proptest`,
   `philharmonic-types`, `zeroize`. Note cooldown status.
7. **Vector verification** — confirm the 2 positive vectors
   match the proposal's expected shapes; confirm all 19
   negative vectors trigger the correct error variant.

## Default follow-through policy

- Carry through to pre-landing-green before returning. Do
  not return red.
- If pre-landing fails: fix and re-run.
- If a `CanonicalJson` method doesn't exist as expected:
  adapt and document.
- If proptest setup is complex: fall back to a hand-written
  generator over ~20 diverse shapes and document why proptest
  was skipped.

## Completeness contract

- Every type, function, constant, and error variant in the
  proposal r2 §"What lands" exists and is public.
- Every test in §"Test-vector plan" (2 positive + 19
  negative + round-trip + proptest/fuzz) exists and runs
  green.
- `ApiSigningKey` does NOT derive `Debug`; manual redacted
  impl confirmed.
- 14-step verify order matches proposal r2 step-for-step.
- `mint_ephemeral_api_token` validates kid-profile,
  claims-size, and lifetime invariants before signing.
- Crate at version `0.2.0`, NOT published.

---

## Outcome

Pending — will be updated after Codex run.
