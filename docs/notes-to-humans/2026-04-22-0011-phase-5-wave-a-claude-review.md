# Phase 5 Wave A — Claude Gate-2 review

**Date:** 2026-04-22
**Status:** Gate-2 PASS — **approved by Yuka 2026-04-22**
**Approval record:** `docs/design/crypto-approvals/2026-04-22-phase-5-wave-a-cose-sign1-tokens-01.md`
**Accuracy check:** `docs/codex-reports/2026-04-22-0004-phase-5-wave-a-claude-review-accuracy-check.md`
  (Codex audited this note for factual accuracy — verdict
  "materially accurate" with two precision nits recorded under
  §Corrigenda below).
**Subject:** Codex's COSE_Sign1 mint + verify implementation for
`philharmonic-connector-client` and `philharmonic-connector-service`.

## Summary

Codex delivered in one round per the Gate-1-approved proposal
([r4](../design/crypto-proposals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md)
and [approval](../design/crypto-approvals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md)).
Positive KAT matches the pre-committed vectors byte-for-byte,
all 10 negative paths reject with the exact error variant,
pre-landing + miri pass on both crates.

Archived prompt +
outcome:
[docs/codex-prompts/2026-04-22-0004-phase-5-wave-a-cose-sign1-tokens.md](../codex-prompts/2026-04-22-0004-phase-5-wave-a-cose-sign1-tokens.md).

Landed:
- `philharmonic-connector-client` commit `9634f68` — 7 files,
  284 insertions (`src/signing.rs`, `src/error.rs`, tests,
  Cargo.toml, README, CHANGELOG, lib.rs).
- `philharmonic-connector-service` commit `bcf8ea6` — 9 files,
  658 insertions (`src/verify.rs`, `src/registry.rs`,
  `src/context.rs`, `src/error.rs`, tests, plumbing).
- Parent pointer bump `ac02232`.

Crates stay at `0.0.0`; publish deferred to Wave B end-to-end.

## What I checked line-by-line (and why each passes)

### 1. 11-step verification order — matches proposal exactly

`verify.rs` runs the sequence in the exact order the proposal
specifies, short-circuiting at the first failure. Each step
maps to one distinct `TokenVerifyError` variant:

| Step | Check | Variant |
|------|-------|---------|
| 1 | COSE_Sign1 parse | `Malformed` |
| 2 | `alg == -8` (EdDSA) | `AlgorithmNotAllowed` |
| 3 | Kid lookup in registry | `UnknownKid { kid }` |
| 4 | Key validity window (`now ∈ [not_before, not_after)`) | `KeyOutOfWindow { now, not_before, not_after }` |
| 5 | Payload size ≤ `MAX_PAYLOAD_BYTES` (default 1 MiB) | `PayloadTooLarge { limit, actual }` |
| 6 | Ed25519 signature verify | `BadSignature` |
| 7 | Claim CBOR decode | `Malformed` |
| 8 | `claims.kid == protected.kid` | `KidInconsistent { protected, claims }` |
| 9 | `exp > now` | `Expired { exp, now }` |
| 10 | Constant-time `SHA-256(payload) == claims.payload_hash` | `PayloadHashMismatch` |
| 11 | `claims.realm == service_realm` | `RealmMismatch { expected, found }` |

Step 10 uses `subtle::ConstantTimeEq::ct_eq` wrapped in
`bool::from(...)`. That's the right API shape — `ct_eq` returns
`subtle::Choice` which is an opaque 1-bit container; `bool::from`
is the documented way to convert.

### 2. Library-takes-bytes discipline — observed

Both crates take bytes:
- `LowererSigningKey::from_seed(Zeroizing<[u8; 32]>, String)` —
  no `&Path`, no `std::fs`, no permission logic.
- `MintingKeyRegistry::insert(kid, MintingKeyEntry)` — no
  `load_from_file`, no config-file parsing.

File I/O and config parsing stay in the (not-yet-scoped)
lowerer / service bin crates per workspace convention §Library
crate boundaries.

### 3. Zeroization — correct

- `LowererSigningKey` owns `Zeroizing<[u8; 32]>` for its
  lifetime. On drop, the seed buffer is zeroed automatically.
- Every `mint_token` call reconstructs a transient
  `ed25519_dalek::SigningKey` via `SigningKey::from_bytes(&self.seed)`
  and drops it at end of function. No long-lived `SigningKey`
  cache. This is option (a) from Open Q #1.
- No seed escapes the `Zeroizing` wrapper anywhere I could find.

### 4. No-panic discipline — clean

Ran `rg -n '\.unwrap\(|\.expect\(|panic!|unreachable!|todo!|unimplemented!|unsafe' src/` in both crates: **zero hits** in library code. Tests use `.expect(...)` freely (allowed).

No `as` casts on untrusted widths; no unbounded indexing; errors
propagate via typed `Result`.

### 5. Vectors — matched byte-for-byte

`signing_vectors.rs` asserts equality against every committed hex
file:
- `wave_a_claims.cbor.hex` (the ciborium-serialized claims)
- `wave_a_protected.hex` (the COSE protected-header bytes)
- `wave_a_sig_structure1.hex` (`tbs_data(b"")`, the
  Sig_structure1 we signed)
- `wave_a_signature.hex` (the Ed25519 signature)
- `wave_a_cose_sign1.hex` (the final envelope)

If the Rust ciborium map ordering, header-builder output, or
Sig_structure1 construction had drifted from what the Python
pycose/cbor2 reference emits, one of those asserts would fail.
None did — which is the strongest cross-implementation evidence
we can get without a live pycose cross-check inside CI.

### 6. Negative vectors — 10/10 pass with exact variant asserts

Each `negative_NN_*` test builds a perturbation, calls
`verify_token`, and `assert_eq!`s the specific error variant
(including its payload fields where applicable). No test uses
`matches!(...)`-with-wildcard-payloads, so a regression in error
construction (e.g. dropping the `kid` field) would trip the test.

### 7. Dependency pins — verified against crates.io today

Both Cargo.toml dep lists match what the proposal r4 pinned.
`philharmonic-types = "0.3.5"` (explicit, not caret) — required
so the CBOR-bstr `Sha256` serde is guaranteed.

## One flagged follow-up (not a Gate-2 blocker)

`ConnectorCallContext` in `philharmonic-connector-common 0.1.0`
has an `issued_at: UnixMillis` field. But `ConnectorTokenClaims`
has no `iat`-equivalent claim — only `exp`. Codex had no source
of truth for `issued_at` at verify time and set it to `now`.

That's semantically wrong: `issued_at = now` means "time
verified," not "time issued." Consumers who rely on `issued_at`
to compute clock skew, TTL remaining, or audit timestamps will
get misleading values.

### Options for the fix (not decided — flagging only)

- **(A)** Add an `iat: UnixMillis` claim to
  `ConnectorTokenClaims`. Breaking change to the already-shipped
  `philharmonic-connector-common 0.1.0` → bump to `0.2.0`,
  regenerate all Wave A test vectors. Clean but expensive now
  that 0.1.0 has landed.
- **(B)** Make `ConnectorCallContext.issued_at` optional
  (`Option<UnixMillis>`). Breaking API change but doesn't require
  a claim-set shape change. Also 0.2.0-tier.
- **(C)** Document that `issued_at` is "verification time" for
  Wave A and leave the API as-is; revisit when the connector-
  common schema gets a 0.2.0 bump for other reasons.
- **(D)** Derive `issued_at = exp - DEFAULT_VALIDITY_WINDOW` from
  a fixed assumption. Unreliable (assumes universal 120s TTL).

### My lean

**(C) for now, (A) later.** Wave A hasn't shipped to real
callers; the `issued_at` field exists but nobody reads it yet.
Documenting the Wave A constraint is low-cost; bundling the fix
with Wave B's inevitable connector-common 0.2.0 (which will add
the encrypted-payload claim surface anyway) amortizes the
breaking-change cost.

Awaiting your call; no change needed to land the Wave A code.

## Minor notes (none are blockers)

1. **`MintingKeyRegistry::insert` silently replaces on duplicate
   `kid`** (standard `HashMap::insert`). Returns the previous
   `MintingKeyEntry` wrapped in `Option`, so the caller can
   detect replacement. Compare to `RealmRegistry::insert` in
   connector-common, which returns an error on duplicate
   (`DuplicateKid`). Either policy is defensible; the service
   bin can wrap this one if it wants strict semantics.

2. **No `MintingKeyRegistry::remove(kid)`.** Operational key
   retirement can be done via `not_after` in the past (handled
   by step 4), so explicit removal isn't strictly required. If
   a service bin wants to drop entries on rotation for memory
   hygiene, it's one `HashMap::remove` away — could be added
   later without API break.

3. **`ConnectorCallContext` is re-exported from
   connector-service but constructed internally via
   `build_call_context`.** That's the narrowed-verified
   interface the proposal called for. Works as specified.

## What's next

- **Wave B**: hybrid KEM (ML-KEM-768 + X25519 + HKDF-SHA256 +
  AES-256-GCM) + COSE_Encrypt0 + end-to-end integration test.
  Per ROADMAP, blocked on Wave A (done now) and post-Golden-Week
  timing. **No crates-io publish for the connector crates until
  Wave B lands and end-to-end tests pass** — Wave A-approved
  code stays at `0.0.0` in the meantime, per the Gate-1 proposal
  and Yuka's Gate-2 approval.
- **Codex prompt for Wave B**: to be drafted after Yuka's own
  Gate-1 for Wave B's hybrid construction (separate proposal
  doc, not this one).
- **connector-common 0.2.0 decision** (A / B / C / D above):
  Yuka confirmed **(C) for now, (A) later** in the Gate-2
  approval. `issued_at = now` stays for Wave A; an `iat` claim
  gets added to `ConnectorTokenClaims` when Wave B forces the
  `philharmonic-connector-common 0.2.0` bump.

No immediate action needed from Yuka to land Wave A; the code is
committed and pushed.

## Corrigenda (after Codex accuracy check)

Codex audited this note against the actual code in both
submodules and flagged two precision-level issues
(`docs/codex-reports/2026-04-22-0004-phase-5-wave-a-claude-review-accuracy-check.md`).
Neither changes the Gate-2 conclusion; recording here for the
record.

1. **Step 3 can also return `Malformed`.** The error-variant
   table lists step 3 (kid lookup) as mapping only to
   `UnknownKid { kid }`. In fact `verify.rs:52-53` also returns
   `TokenVerifyError::Malformed` if the protected-header `kid`
   bytes fail to decode as UTF-8. The proposal's 11-step order
   fires `Malformed` here rather than `UnknownKid` because a
   non-UTF-8 kid can't even be compared; the behavior is
   correct, just not what the table suggested.

2. **`philharmonic-types = "0.3.5"` is still caret syntax.** The
   note called the pin "explicit, not caret." In Cargo's version
   grammar `"0.3.5"` is equivalent to `^0.3.5` — it floors at
   `>= 0.3.5` within the `0.3.x` series but still accepts
   `0.3.6`, `0.3.7`, etc. as compatible. A truly non-caret pin
   would be `"=0.3.5"`. The intent (floor at 0.3.5 so the
   CBOR-bstr `Sha256` serde is guaranteed) is correct; the
   label was wrong.
