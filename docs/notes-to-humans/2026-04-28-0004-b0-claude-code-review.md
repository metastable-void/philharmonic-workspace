# Sub-phase B0 — Claude code review (pre-Gate-2)

**Author:** Claude Code · **Audience:** Yuka (for human Gate-2 review) · **Date:** 2026-04-28 (Tue) JST midday

This is the mandatory Claude-run review before Yuka's human
Gate-2 review. The code was produced by Codex round
`task-moi1ft52-gmwowo` (~52 min) and lives in the
`philharmonic-policy` submodule working tree.

## Verdict

**PASSES** Claude-level code review. No security issues found.
No proposal deviations that require Yuka's call. Ready for
human Gate-2 review.

## Security checklist

### Verify order matches r2 proposal (14 steps)

Traced through `src/api_token.rs` → `verify_internal`:

1. Token size ceiling (`MAX_TOKEN_BYTES`) — line 557. ✅
2. `CoseSign1::from_slice` parse — line 564. ✅
3. Strict header profile (`validate_header_profile`) — line
   566. Protected only `alg` + `kid`, unprotected empty,
   `crit` + extra labels rejected. ✅
4. Algorithm pin (`EdDSA` / `-8`) — line 568. ✅
5. Kid format profile (ASCII `[A-Za-z0-9._:-]`, 1..=128) —
   line 575-577. Validated before registry lookup. ✅
6. Kid registry lookup — line 579-584. ✅
7. Key validity window (`not_before` / `not_after`) — line
   586-592. ✅
8. Ed25519 signature verification via
   `CoseSign1::verify_signature` — line 594-601. **No claim
   content trusted before this step.** ✅
9. CBOR claim decode into `EphemeralApiTokenClaims` — line
   603-609. ✅
10. `CanonicalJson` re-check (`raw_injected_claims_text` vs
    decoded `claims.as_bytes()`) — line 611-614. ✅
11. Issuer binding (`claims.iss == entry.issuer`) — line
    616-621. ✅
12. Kid consistency (protected header vs claims kid) — line
    623-628. ✅
13. Lifetime invariants via `validate_lifetime`:
    `iat ≤ now + skew`, `exp > now`, `exp > iat`,
    `exp - iat ≤ MAX_TOKEN_LIFETIME_MILLIS`. All using
    `checked_add` / `checked_sub`. No `as` casts. — line
    630-640. ✅
14. Injected-claims size cap
    (`MAX_INJECTED_CLAIMS_BYTES`) — line 642-648. ✅

### ApiSigningKey Debug redaction

Lines 120-127: manual `fmt::Debug` impl. Prints `kid` +
`<redacted>`. Does NOT derive `Debug`. Confirmed by unit
test `signing_key_debug_redacts_seed` (asserts output
contains `<redacted>`, does NOT contain seed bytes). ✅

### Zeroization

- `ApiSigningKey.seed: Zeroizing<[u8; 32]>`. ✅
- `mint_ephemeral_api_token` reconstructs a transient
  `ed25519_dalek::SigningKey` via `from_bytes` per call
  (line 487), dropped at end of function. Same Wave A
  pattern. ✅
- No other private-key material held anywhere. ✅

### No panics in library src

Grep confirmed: all `.unwrap()` / `.expect()` are in
`#[cfg(test)]` blocks only. Library code returns `Result` /
`Option` everywhere. ✅

### No `unsafe`

None in `src/api_token.rs`. ✅

### CanonicalJson round-trip

- **Serialize**: manual `Serialize` impl (line 54-75) writes
  `claims` as `std::str::from_utf8(self.claims.as_bytes())`
  → CBOR text string.
- **Deserialize**: helper struct reads `claims: String`, then
  `CanonicalJson::from_bytes` re-canonicalizes on decode.
- **Re-check (verify step 10)**: `raw_injected_claims_text`
  extracts the raw wire-level text from the full CBOR
  payload (via second `ciborium::value::Value` parse),
  compares byte-for-byte against the re-canonicalized
  `CanonicalJson`. Non-canonical wire → mismatch → reject.
  Confirmed by `api_claims_not_canonical.hex` negative vector
  and `proptest_noncanonical_injected_claims_reject`. ✅

### VerifyLimits clamping

Lines 274-288: `min(caller, default)` for each field. Cannot
loosen past system defaults. Confirmed by unit test
`verify_limits_clamp_to_defaults`. ✅

## Test coverage

- **6 colocated unit tests** in `src/api_token.rs`: kid
  profile (valid + invalid), debug redaction, limits
  clamping, registry insert (duplicate + profile violation),
  CBOR round-trip stability. ✅
- **2 positive known-answer tests**: `with_instance` and
  `no_instance` vectors. Each mints, checks claim-CBOR hex
  against committed vector, checks COSE_Sign1 hex against
  committed vector, then verifies back to claims equality.
  Committed hex files under
  `tests/vectors/api_token/*.hex`. ✅
- **19 negative vectors**: one parametric test iterates all
  19, checks each generates the expected bytes AND triggers
  the specific `ApiTokenVerifyError` variant. All
  proposal-specified rejection steps covered. ✅
- **1 round-trip test**: fresh claims → mint → serialize →
  verify → assert claims equality. ✅
- **2 proptest fuzz tests**: random claims round-trip, and
  non-canonical JSON rejection. ✅

Total: 16 unit + 6 integration (incl. proptest) + 12 MySQL
= 34 tests all green. `pre-landing.sh philharmonic-policy`
passes.

## Noted observations (not defects)

- **Double CBOR parse for re-check.** `raw_injected_claims_text`
  deserializes the full CBOR payload a second time (as
  `ciborium::value::Value`) to extract the raw `claims` text
  for the canonicalization check. Not a security concern —
  the payload is already signature-validated and bounded by
  `MAX_TOKEN_BYTES` (16 KiB). A single-parse approach would
  require threading the raw bytes through the deserialization
  layer, which isn't worth the complexity.
- **`ApiSignedToken::to_bytes` clones.** Line 163:
  `self.0.clone().to_vec()`. `coset::CoseSign1` doesn't offer
  `to_vec` on `&self`, so a clone is needed. Performance-only
  concern (one extra ~KiB allocation per serialization); not
  a security issue.
- **19 negative vectors are generated, not hand-crafted hex.**
  The test function `negative_vectors()` programmatically
  mutates a positive token to produce each negative variant,
  then compares against committed hex files for regression.
  This is a good testing pattern — committed hex catches
  drift, programmatic generation ensures the tests are
  understandable. If a pycose cross-check is done later
  (Q3), the committed hex can be verified externally.

## What Yuka should focus on during Gate-2

1. **Verify step 10 (canonicalization re-check)**. The
   `raw_injected_claims_text` function (line 681-698)
   re-parses the full CBOR payload to extract the raw
   `claims` text. Confirm this is the right approach vs e.g.
   a custom CBOR decoder that captures the raw bytes during
   the initial parse.
2. **The `CanonicalJson::from_bytes` semantics** in the
   deserialization helper (line 95-96). Confirm this
   re-canonicalizes on decode as expected, so that a
   non-canonical wire value gets canonicalized during
   deserialization and then fails the byte-comparison in step
   10.
3. **The test-vector hex files**. If pycose 2.x cross-check
   is planned (Q3), the committed hex is the input.
   Otherwise, visual spot-check that the keypair.json
   matches RFC 8032 TEST 1 and the claim-set JSONs match the
   proposal.
4. **Ed25519 seed → SigningKey per-call reconstruction.**
   Line 487: `SigningKey::from_bytes(&signing_key.seed)`.
   Confirm this is the correct Wave-A-pattern zeroization
   choice (per the Q1 resolution from the original Wave A
   proposal, option (a), approved 2026-04-22).
