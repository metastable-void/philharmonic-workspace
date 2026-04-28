# Phase 8 B0 ephemeral API token primitives security review

**Date:** 2026-04-28
**Prompt:** Direct in-session request to fully audit `docs/design/crypto-proposals/2026-04-28-phase-8-ephemeral-api-token-primitives.md`

## Scope

Reviewed:

- `docs/design/crypto-proposals/2026-04-28-phase-8-ephemeral-api-token-primitives.md`

Compared against:

- Wave A's COSE_Sign1 proposal and landed implementation:
  `philharmonic-connector-client/src/signing.rs` and
  `philharmonic-connector-service/src/verify.rs`.
- Ephemeral-token design requirements in
  [`docs/design/09-policy-and-tenancy.md`](../design/09-policy-and-tenancy.md),
  [`docs/design/10-api-layer.md`](../design/10-api-layer.md), and
  [`docs/design/11-security-and-cryptography.md`](../design/11-security-and-cryptography.md).
- RFC-level shape for COSE_Sign1 and EdDSA, only to confirm the
  proposal's stated signing construction.

Focus: design-level security properties for the proposed B0 signing
and verification primitives, plus security-sensitive handoff
requirements that the proposal assigns to B1/G consumers.

## Summary

The Ed25519 + COSE_Sign1 mechanics are mostly sound and match Wave A:
protected `alg = -8`, protected `kid`, empty external AAD,
signature verification before claim trust, and bounded token bytes.

But I do **not** recommend Gate-1 sign-off on r1 unchanged. The
proposal leaves several security invariants entirely to later
consumers without making them mandatory, and in two places the
threat model is materially too optimistic. The highest-risk fixes are:

1. Require the B1 consumer to check that the `authority` entity belongs
   to the signed `tenant`, not only that the token tenant matches the
   request tenant.
2. Add issuer/audience/key binding instead of accepting any registry
   key for any `iss`.
3. Add `iat` or another verify-time lifetime invariant so "ephemeral"
   cannot silently become "valid until arbitrary far-future `exp`".
4. Forbid secret-bearing `Debug` output on `ApiSigningKey`; do not copy
   Wave A's current `#[derive(Debug)]` pattern.
5. Replace the current `serde_json::Value` / CBOR determinism story
   with an explicit canonical JSON or canonical CBOR policy.

## Findings

### 1. High: B1 checks omit the authority-to-tenant binding

Proposal lines 89-94 say B0 returns verified claims and leaves
substrate checks to B1: authority lookup, authority-epoch check,
tenant-scope match, and instance-scope match. Lines 111-119 repeat
that authority lookup/epoch/retirement and tenant/instance enforcement
are out of B0.

That list is missing the critical relation:

> The minting authority named by `claims.authority` must be pinned to
> the same tenant as `claims.tenant`.

Doc 09 models `MintingAuthority` with a pinned `tenant` entity slot
(`docs/design/09-policy-and-tenancy.md`:454-456). Doc 10 says per-request
verification looks up the authority, checks retirement/suspension/epoch,
then checks token tenant against the request tenant
(`docs/design/10-api-layer.md`:139-148), but it also does not explicitly
say to reject if `authority.tenant != claims.tenant`.

If B1 implements exactly the proposal text, a token can carry:

```text
authority = authority-from-tenant-B
tenant    = tenant-A
```

and still pass these separate checks if the authority exists, is not
retired, has the matching epoch, and the request is for tenant A. The
minting endpoint should never mint such a token, but the verifier must
not rely on mint correctness for a cross-tenant security invariant. The
API signing key is deployment-wide, so the signed claim is not
tenant-isolated by key material.

**Recommended fix:** make `authority.tenant == claims.tenant` a
normative B1 check, placed immediately after authority lookup and before
authority epoch acceptance. Add a negative B1 test:

- authority belongs to tenant B;
- token claims tenant A;
- request tenant A;
- signature and epoch valid;
- request must reject.

This is not a B0 primitive check unless B0 grows a substrate callback,
which it should not. It does need to be explicit in the proposal's B1
handoff contract before Gate-1 sign-off.

### 2. High: no issuer/audience/key binding; `iss` is signed but unused

The claim shape includes `iss` (`proposal`:217-230), but the verify
order never checks it (`proposal`:306-342). The verifying registry entry
is only `{ vk, not_before, not_after }` keyed by `kid`
(`proposal`:419-434). The replay/threat-model section says "no realm /
audience binding" because tenant and instance are enforced by the
consumer (`proposal`:357-361).

Tenant matching is not the same as issuer/audience binding. Without an
issuer or deployment audience check, any verifying key in the registry
is effectively trusted to mint for any `iss` string and for any API
deployment that loads that public key. This creates avoidable
cross-environment and cross-issuer confusion:

- staging/prod or regional deployments accidentally sharing a key;
- a future multi-issuer API layer using the same primitive;
- audit logs recording an `iss` value that was never authenticated
   against the key entry's configured identity.

Wave A fixed the analogous problem with mandatory realm binding in the
service verify path. B0 does not need a connector-style realm, but it
does need a verifier-side statement of who this token is for and who is
allowed to issue it.

**Recommended fix:** choose one of these before sign-off:

- Add `issuer: String` and `audience: String` to
  `ApiVerifyingKeyEntry`, then after signature/CBOR decode enforce
  `claims.iss == entry.issuer` and `claims.aud == entry.audience`.
- Or keep the claim set unchanged but make
  `verify_ephemeral_api_token` take an `expected_issuer` and
  `expected_audience`/deployment ID, and reject mismatch.

If adding an `aud` claim is too much for B0, at minimum bind
`claims.iss` to the registry entry. Leaving `iss` unchecked should not
pass Gate-1.

### 3. High: the verifier accepts arbitrary far-future `exp`; no `iat`
or max-age invariant exists

Doc 11 says ephemeral API tokens have a system-wide maximum lifetime of
24h, optionally lower per authority
(`docs/design/11-security-and-cryptography.md`:164-165). The proposal
deliberately leaves lifetime ceilings to the mint endpoint (`proposal`:
688-695). B0 only checks `claims.exp > now` (`proposal`:339-340).

That means the primitive will accept a validly signed token with
`exp = year 9999` unless a later consumer adds its own check. The
proposal's own split makes B0 the reusable security primitive; it
should not normalize a token that violates the central "ephemeral"
property.

Checking only "remaining lifetime <= 24h" is also insufficient without
an issued-at time: a token with a far-future `exp` would become accepted
during its final 24h window. The current claim set has no `iat`, unlike
the connector-token claim set after Wave A hardening.

**Recommended fix:** add an `iat: UnixMillis` claim and make verify
enforce:

- `iat <= now + allowed_clock_skew`;
- `exp > now`;
- `exp > iat`;
- `exp - iat <= max_token_lifetime` using checked arithmetic.

Keep the per-authority configured max in G/B1 if needed, but B0 should
at least enforce the system maximum from doc 11, either as a constant
or as a parameter to `verify_ephemeral_api_token`.

If Yuka decides not to add `iat`, the proposal should explicitly record
that B0 cannot enforce max lifetime, only expiry, and that this is a
deliberate defense-in-depth tradeoff. I would not recommend that
choice.

### 4. Medium-high: `ApiSigningKey` must not derive `Debug`

The proposal says `ApiSigningKey` is "same shape as
`LowererSigningKey`" (`proposal`:69-72), and Wave A's landed
`LowererSigningKey` currently has `#[derive(Clone, Debug)]` in
`philharmonic-connector-client/src/signing.rs`.

That pattern should not be copied. A `Debug` implementation for a
secret-bearing type is a log-leak footgun. `ApiSigningKey` owns the
32-byte Ed25519 seed in `Zeroizing<[u8; 32]>`; if a derived formatter
prints the inner array now or after a dependency behavior change, a
single tracing/debug statement can expose the API signing seed.

**Recommended fix:** explicitly state in the proposal that
`ApiSigningKey` must either:

- not implement `Debug`; or
- implement a redacted `Debug` that prints only `kid` and a redacted
  seed marker.

Gate-2 should check this directly. I also recommend opening a separate
follow-up for the existing Wave A `LowererSigningKey` derive, but that
is outside this B0 proposal.

### 5. Medium-high: CBOR determinism and `serde_json::Value` are not
pinned tightly enough

The proposal says the whole claim payload is "canonical per RFC 8949
§4.2 deterministic encoding" and that `ciborium` produces deterministic
output for the relevant Rust types (`proposal`:255-259). Later Q2
admits `serde_json::Value::Object` ordering is uncertain and
recommends a sorted-key intermediate (`proposal`:664-674).

This needs to be resolved before Gate-1, not left as an open question.
The signed bytes are safe against tampering, but the semantic contract
still matters:

- Known-answer vectors should be reproducible across languages.
- Audit hashes and forensic comparisons should not depend on incidental
  map insertion order.
- The verifier should not accept non-JSON-ish CBOR values inside
  `claims` if doc 09 says the field is JSON-shaped.
- Duplicate-map-key behavior should not be left to generic CBOR
  deserialization behavior.

This workspace already has `philharmonic_types::CanonicalJson`, which
canonicalizes JSON via JCS and is designed for stable JSON semantics.
Using raw `serde_json::Value` inside a signed CBOR map is the least
pinned option.

**Recommended fix:** decide one canonical policy in the proposal:

- Prefer: represent injected claims as `CanonicalJson` or canonical JSON
  bytes in the token claim, and convert to `serde_json::Value` only when
  constructing `AuthContext`.
- Or: keep CBOR-native `serde_json::Value`, but implement explicit
  recursive canonicalization before signing, reject duplicate/non-string
  map keys where applicable, reject non-finite floats, and add vectors
  for nested objects with deliberately shuffled keys.

Either way, remove the unconditional claim that `ciborium` alone gives
the desired deterministic RFC 8949 profile for free.

### 6. Medium: B0 verify should enforce the injected-claims size
contract, or expose a limit parameter

Doc 09 caps `claims` at 4 KB (`docs/design/09-policy-and-tenancy.md`:
487-488 and 535). The proposal says G enforces the cap before minting
and B0 deliberately does not (`proposal`:122-126, 410-415, 647-649).
B0 only limits total COSE bytes to 16 KiB (`proposal`:347-352).

The 16 KiB token cap is useful for parser/CPU bounding, but it is not
the same invariant as "injected claims are capped at 4 KB." Once B0
returns `EphemeralApiTokenClaims`, B1 will attach `claims` to
`AuthContext::Ephemeral` and workflow execution context. If a future
minting path, test helper, or operational tool signs a larger
`claims` value, verify will normalize it as valid.

**Recommended fix:** have B0 enforce a payload/claims size limit during
verification, ideally using the same serialized representation chosen
in finding 5. If the limit must remain deployment-configurable, add a
`verify_ephemeral_api_token_with_limits(...)` variant and keep the
default wrapper at the doc 09 cap.

### 7. Medium: the replay threat model overstates what token leakage
implies

The proposal says:

> If a token leaks to an attacker who has TLS-piercing access, they also
> have the ability to mint new tokens.

That is not generally true for browser-resident or partner-system
bearer tokens. A token can leak through XSS, browser storage exposure,
logs, crash reports, referrers, debugging proxies, or accidental
copy/paste without the attacker also holding the long-lived minting
authority credential.

I agree that v1 can choose stateless bearer tokens with no `jti` and no
server-side replay cache. But the threat model should say that a leaked
ephemeral token is replayable until expiry/epoch invalidation, within
its permission and instance scope. That is a deliberate bearer-token
tradeoff, not equivalent to minting compromise.

**Recommended fix:** rewrite the replay section to name bearer-token
exfiltration directly, and tie mitigations to:

- short `exp` / max-age;
- instance scope as the recommended browser default;
- authority epoch bump for incident response;
- B1/H rate limiting and generic auth failures;
- deployment guidance: do not persist browser tokens longer than needed.

This does not necessarily require adding `jti`, but the current wording
should not be signed off.

### 8. Medium: external auth error behavior is not specified

B0 exposes distinct errors: malformed, bad algorithm, unknown kid,
key out of window, bad signature, kid inconsistent, expired
(`proposal`:306-340 and 568-593). Typed library errors are useful for
tests and internal logs.

But if B1 maps those variants directly to externally visible HTTP
bodies, status codes, or timing, attackers can probe key IDs, key
validity windows, parser behavior, and token expiry. Wave B's review
handled the analogous issue by requiring the binary/service layer to
collapse crypto failures to a generic external response while preserving
typed internal logs.

**Recommended fix:** add a normative B1 requirement:

- all ephemeral-token verify failures return the same external auth
  failure shape, normally HTTP 401 with a generic `invalid_token`-style
  body;
- no `kid`, validity-window, signature, or expiry details in the
  external response;
- internal logs may keep typed variants but must avoid logging bearer
  token bytes or secret material.

### 9. Low-medium: reject non-empty unprotected headers and unexpected
protected headers

Minting creates an empty unprotected header (`proposal`:261-268), but
verification does not say to reject non-empty unprotected headers.
Because unprotected headers are not signature-covered, an attacker can
add misleading metadata even though the protected `alg` and `kid` still
drive verification.

This is unlikely to break the current verifier if it ignores
unprotected headers, but it creates forensic and future-maintenance
risk: later code may inspect the `CoseSign1` object and accidentally
prefer or log attacker-controlled unprotected metadata.

**Recommended fix:** define a strict COSE profile:

- protected headers may contain exactly `alg` and `kid` unless a future
  revision explicitly adds more;
- unprotected headers must be empty;
- any `crit` header or unknown protected header rejects until the
  application intentionally supports it.

Add negative vectors for non-empty unprotected `kid`/`alg` and for an
unexpected protected header.

### 10. Low-medium: `kid` needs a length/character profile

`kid` is a free-form UTF-8 string (`proposal`:456-462), extracted before
signature verification for lookup (`proposal`:314-317). `MAX_TOKEN_BYTES`
bounds total size, so this is not an unbounded allocation problem, but a
16 KiB UTF-8 `kid` can still pollute logs and error values.

Free-form strings also make operator configuration mistakes easier,
especially if visually confusable Unicode appears in key IDs.

**Recommended fix:** keep wire encoding as UTF-8 bytes, but define a
profile such as:

- ASCII only;
- `[A-Za-z0-9._:-]`;
- length 1..128 bytes;
- reject invalid profile before registry lookup.

If Yuka wants maximum flexibility, at least cap length and avoid
including the raw unknown `kid` in external responses.

### 11. Low: `authority_epoch` type must reconcile with the substrate
scalar type

The proposal uses `authority_epoch: u64` (`proposal`:223-224). Doc 09's
`MintingAuthority` entity sketch stores `epoch` as `ScalarType::I64`
(`docs/design/09-policy-and-tenancy.md`:457-459).

This is mostly an implementation-consistency issue, but it sits on a
revocation boundary. A sloppy `as` cast from signed substrate value to
unsigned claim value would be exactly the kind of width/sign bug the
workspace Rust rules try to avoid.

**Recommended fix:** align the wire type with the substrate type, or
state the conversion rule explicitly:

- substrate epoch must be non-negative;
- conversion uses `u64::try_from(...)`;
- negative or out-of-range values fail closed;
- B1 tests cover negative substrate epoch, even if such a value should
  never be produced normally.

## Positive notes

- Protected `alg = -8` and protected `kid` match the COSE profile used
  successfully in Wave A (`proposal`:261-268).
- Signature input matches COSE_Sign1 `Sig_structure1` with empty
  external AAD (`proposal`:270-295).
- Verification order is directionally right: parse, algorithm pin,
  key lookup/window, signature, then claim decode and content checks
  (`proposal`:297-343).
- The `MAX_TOKEN_BYTES` pre-parse guard is the right resource-exhaustion
  shape for a self-contained token (`proposal`:347-352).
- The library/bin split is correct: no key-file I/O in
  `philharmonic-policy`, deployment code supplies bytes
  (`proposal`:375-379 and 443-448).
- Known-answer and negative-vector planning is strong; it just needs
  additional vectors for the findings above.

## Gate recommendation

**Do not sign off Gate-1 on r1 unchanged.**

I would sign off after a revision that:

1. Adds explicit B1 authority-tenant binding.
2. Binds `iss` to the verifying key entry and preferably adds/verifies
   an audience/deployment claim.
3. Adds `iat` and verify-time max lifetime enforcement, or records a
   very explicit Yuka-approved exception.
4. Prohibits secret-bearing `Debug` for `ApiSigningKey`.
5. Resolves `serde_json::Value` / CBOR determinism by using
   `CanonicalJson`, canonical JSON bytes, or a specified custom
   canonicalization step.
6. Specifies generic external HTTP error behavior for B1.
7. Adds strict COSE-header profile checks and a bounded `kid` profile.

No code changes were made in this review beyond this note.
