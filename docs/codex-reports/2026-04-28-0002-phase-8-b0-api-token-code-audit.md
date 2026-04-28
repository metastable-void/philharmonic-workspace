# Phase 8 B0 API token code audit

**Date:** 2026-04-28
**Prompt:** docs/codex-prompts/2026-04-28-0002-phase-8-sub-phase-b0-ephemeral-api-token-primitives.md
**Reviewed alongside:** docs/notes-to-humans/2026-04-28-0004-b0-claude-code-review.md
**Submodule commit reviewed:** philharmonic-policy `5d96e7b`

## Scope

I independently reviewed the Phase 8 sub-phase B0 implementation of
ephemeral API token primitives in `philharmonic-policy`, focusing on
correctness and security against the Gate-1 r2 proposal:

- `philharmonic-policy/src/api_token.rs`
- `philharmonic-policy/src/lib.rs`
- `philharmonic-policy/Cargo.toml`
- `philharmonic-policy/CHANGELOG.md`
- `philharmonic-policy/tests/api_token_vectors.rs`
- `philharmonic-policy/tests/vectors/api_token/*`

I also checked the relevant supporting types in `philharmonic-types`,
especially `CanonicalJson` and `UnixMillis`, and spot-checked `coset
0.4.2` behavior for `CoseSign1::verify_signature` and protected-header
original-data handling.

## Summary verdict

The implementation follows the Gate-1 r2 design in its core security
properties: Ed25519 + COSE_Sign1 signing, strict `kid` profile,
signature-before-claims ordering, issuer binding, lifetime checks with
checked arithmetic, injected-claims canonicalization checks, seed
zeroization, and redacted `ApiSigningKey` debug output.

I found two low-to-medium concerns around exact CBOR claim-payload
profiling. Neither lets an attacker forge or alter a token without the
signing key, because both issues require bytes already covered by a
valid Ed25519 signature. They do, however, weaken the intended
"payload is exactly the canonical claim CBOR" invariant and can create
audit / forensic drift if a valid signer or future minting bug emits
non-profile payload bytes.

## Findings

### 1. Claim-payload decoding accepts trailing bytes

Severity: low/medium.

`verify_internal` decodes the signed payload with:

```rust
let mut claims_reader = claims_payload;
let claims: EphemeralApiTokenClaims = ciborium::de::from_reader(&mut claims_reader)
    .map_err(|_| ApiTokenVerifyError::Malformed)?;
```

This decodes one CBOR value but does not check that
`claims_reader` is empty afterwards. `raw_injected_claims_text` has the
same pattern: it decodes one `ciborium::value::Value` from the payload
but does not verify that the reader consumed all bytes.

As a result, a valid signer can produce a token whose payload is:

```text
<valid EphemeralApiTokenClaims CBOR> || <extra signed CBOR bytes>
```

Verification will return `Ok(claims)` and silently ignore the trailing
bytes. This is not unsigned-attacker exploitable: changing those bytes
invalidates the COSE_Sign1 signature. The problem is profile drift. The
returned claims no longer represent the entire signed payload, and
audit tooling that hashes or records raw token payload bytes may see
data that the B0 verifier ignores.

Recommended fix: after each `ciborium::de::from_reader` call over a
slice, reject if the remaining reader slice is non-empty. A stronger
alternative is to re-serialize the decoded `EphemeralApiTokenClaims`
and compare it byte-for-byte with `claims_payload`, which would also
catch unknown signed fields and non-canonical outer CBOR encodings.

Relevant file: `philharmonic-policy/src/api_token.rs`.

### 2. Unknown signed claim fields are accepted and stripped

Severity: low.

`EphemeralApiTokenClaims` deserializes through an internal
`ClaimsHelper` struct without `#[serde(deny_unknown_fields)]`.
Consequently, a signed claim map containing extra fields is accepted,
and those fields are dropped from the returned
`EphemeralApiTokenClaims`.

Like the trailing-byte issue, this is not a signature bypass. It does
allow a valid signer or future minting bug to include signed data that
B0 verification ignores. That is undesirable for a fixed claim profile,
especially because this module is meant to provide stable known-answer
vectors and clear audit semantics.

Recommended fix: add `#[serde(deny_unknown_fields)]` to `ClaimsHelper`,
or use the stronger full-payload re-serialization comparison described
above.

Relevant file: `philharmonic-policy/src/api_token.rs`.

## Checks that looked correct

The 14-step verification order is implemented in the intended shape:

1. token byte ceiling before parsing;
2. `CoseSign1::from_slice` parse;
3. strict protected/unprotected header profile;
4. EdDSA algorithm pin;
5. protected `kid` UTF-8 / length / ASCII profile;
6. registry lookup by validated `kid`;
7. key validity window;
8. Ed25519 signature verification;
9. claim CBOR decode;
10. injected-claims canonicalization re-check;
11. issuer binding;
12. protected-header `kid` versus signed-claim `kid` consistency;
13. lifetime invariants;
14. injected-claims size cap.

No claim content is trusted before signature verification passes.

`ApiSigningKey` does not derive `Debug`; its manual debug output prints
the `kid` and a redaction marker for the seed. The seed is stored as
`Zeroizing<[u8; 32]>`, and minting reconstructs an
`ed25519_dalek::SigningKey` transiently per call.

`validate_lifetime` uses checked addition and subtraction for skew and
lifetime arithmetic. It rejects future `iat`, expired tokens,
`exp <= iat`, and lifetimes beyond `MAX_TOKEN_LIFETIME_MILLIS`.

`VerifyLimits::clamped` uses `min(caller, default)` for token size,
injected-claims size, max lifetime, and clock skew, so callers cannot
loosen the default policy through `verify_ephemeral_api_token_with_limits`.

The `kid` profile is consistently applied on mint, verify, and registry
insert. It enforces length `1..=128` bytes and ASCII
`[A-Za-z0-9._:-]`.

The strict COSE header profile rejects non-empty unprotected headers,
protected `crit`, known-but-unwanted protected fields, and unknown
protected labels. Coset preserves parsed protected-header original bytes
for signature verification, so the verifier checks the actual signed
protected header bytes.

The committed tests cover two positive known-answer vectors, nineteen
negative vectors, round-trip mint/verify, and property tests for random
claims plus non-canonical injected JSON rejection.

## Verification run

I ran:

```sh
./scripts/pre-landing.sh philharmonic-policy
```

Result: passed.

The wrapper printed an early `rustup check` warning:

```text
error: could not create temp file /home/ubuntu/.rustup/tmp/...: Read-only file system (os error 30)
```

The script continued and completed successfully with:

```text
=== pre-landing: all checks passed ===
```

This run included `rust-lint`, workspace tests, doctests, and
`philharmonic-policy` ignored tests. In this environment, the ignored
MySQL-backed policy tests ran and passed.

I also attempted:

```sh
./scripts/rust-test.sh philharmonic-policy api_token
```

That was an invalid wrapper invocation and printed the usage line:

```text
Usage: ./scripts/rust-test.sh [--include-ignored|--ignored] [<crate-name>]
```

No code or test files were modified by that failed invocation.

## Gate-2 note

This code is in the crypto-sensitive path: COSE_Sign1 signing and
verification for ephemeral API tokens. Even with the low-severity
findings above, Yuka's personal Gate-2 review remains required before
the work should be treated as accepted.
