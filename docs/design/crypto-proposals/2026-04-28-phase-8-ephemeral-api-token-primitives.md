# Gate-1 proposal — Phase 8 sub-phase B0: ephemeral API token primitives

**Date:** 2026-04-28 (Tue)
**Revision:** 2 (post Codex security review)
**Phase:** 8 (`philharmonic-api`), sub-phase B0 (primitives only;
B1 is the consumer middleware in `philharmonic-api`)
**Author:** Claude Code (on Yuka's review queue)
**Status:** **Gate-1 approved 2026-04-28** — implementation unblocked
**Approval record:** `docs/design/crypto-approvals/2026-04-28-phase-8-ephemeral-api-token-primitives.md` (will be created on sign-off)
**Blocker note that triggered this:** [`docs/notes-to-humans/2026-04-28-0003-ephemeral-token-primitives-gap.md`](../../notes-to-humans/2026-04-28-0003-ephemeral-token-primitives-gap.md)
**Codex security review:** [`docs/codex-reports/2026-04-28-0001-phase-8-b0-ephemeral-api-token-primitives-security-review.md`](../../codex-reports/2026-04-28-0001-phase-8-b0-ephemeral-api-token-primitives-security-review.md)
  — eleven findings; resolutions summarized in §"Codex security
  review resolutions". The body below is the r2 incarnation.
**Yuka's Tue-morning calls** (recorded against `2026-04-28-0003`'s D1/D2/D3/D4):
  - **D1 = (a)** — primitives live in **`philharmonic-policy`**
    (next to `Sck`, `sck_encrypt`, `sck_decrypt`,
    `parse_api_token`, `generate_api_token`).
  - **D2 = (b)** — sub-phase B is **split into B0 (primitives)
    + B1 (consumer)**. This proposal covers only B0.
  - **D3 = (a)** — fresh Gate-1 proposal (this document).
  - **D4** — continue toward 2026-05-02 target anyway; the
    cut-list in
    [`2026-04-28-0002-pre-gw-target-may-2-end-to-end.md`](../../notes-to-humans/2026-04-28-0002-pre-gw-target-may-2-end-to-end.md)
    moves up if proposal sign-off slips.

## Revisions

- **r1** (2026-04-28 morning): first draft. Closely modeled on
  Phase 5 Wave A's COSE_Sign1 proposal.
- **r2** (2026-04-28 midday, post Codex security review): eleven
  findings landed as inline fixes. Highlights:
  - Added `iss` binding to the verifying-key registry entry
    (Finding 2). Verify rejects `claims.iss != entry.issuer`.
  - Added `iat` claim + `MAX_TOKEN_LIFETIME_MILLIS` ceiling
    (Finding 3). Verify enforces `iat ≤ now + skew`,
    `exp > iat`, `exp − iat ≤ MAX_TOKEN_LIFETIME_MILLIS`.
  - `ApiSigningKey` does **not** derive `Debug`; carries a
    redacted manual `fmt::Debug` impl (Finding 4).
  - Replaced `claims: serde_json::Value` with `claims:
    CanonicalJson` (RFC 8785 JCS via
    `philharmonic_types::CanonicalJson`) for deterministic
    on-the-wire bytes (Finding 5). 4 KiB cap enforced at both
    mint and verify (Finding 6).
  - Replay threat-model section rewritten to acknowledge
    bearer-token exfiltration as a real risk separate from
    minting compromise (Finding 7).
  - Added strict COSE-header profile (only `alg` + `kid`
    protected; unprotected empty; `crit` rejects) (Finding 9).
  - Added `kid` length/character profile
    (`[A-Za-z0-9._:-]`, 1..=128 bytes) checked before registry
    lookup (Finding 10).
  - `authority_epoch` wire type stays `u64`; B1 substrate
    conversion rule pinned to `u64::try_from` with
    fail-closed on negative/out-of-range (Finding 11).
  - B1 handoff contract gained two normative items:
    `authority.tenant == claims.tenant` (Finding 1) and
    generic external-error collapsing for verify failures
    (Finding 8).
  - Open questions Q2 (CBOR determinism) and Q5 (max-lifetime
    in primitive) resolved by the above. Q1/Q3/Q4/Q6 stand.

## Why this proposal exists

The 2026-04-28 morning Phase 8 approach approval (recorded in
[`2026-04-28-0001-phase-8-decisions-confirmed.md`](../../notes-to-humans/2026-04-28-0001-phase-8-decisions-confirmed.md))
asserted that ephemeral-API-token COSE_Sign1 mint + verify
already exists as wave-A primitive code in
`philharmonic-connector-common` and/or `philharmonic-policy`,
and that the API crate is "a caller, not an implementer".

That assumption is incorrect.
[`2026-04-28-0003-ephemeral-token-primitives-gap.md`](../../notes-to-humans/2026-04-28-0003-ephemeral-token-primitives-gap.md)
documents the audit. Workspace-wide, what exists is:

- ✅ Long-lived `pht_` token utilities in
  `philharmonic-policy::token`.
- ✅ Connector authorization tokens (different claim shape,
  hard-bound to `realm` + `payload_hash`) in
  `philharmonic-connector-{client,service}`.
- ❌ **Ephemeral API tokens** — not implemented anywhere.

So sub-phase B is not "just a caller". The
ephemeral-API-token primitives need to be built first, and that
is **new framework crypto** even if the underlying primitives
(Ed25519 + COSE_Sign1) are RustCrypto / coset code already in
the dep tree. Per the
[`crypto-review-protocol`](../../../.claude/skills/crypto-review-protocol/SKILL.md)
skill, **Gate 1: pre-approval of approach, before coding**
fires. Hence this proposal.

The shape closely mirrors the Phase 5 Wave A proposal at
[`2026-04-22-phase-5-wave-a-cose-sign1-tokens.md`](2026-04-22-phase-5-wave-a-cose-sign1-tokens.md).
Same primitive choices (Ed25519 + COSE_Sign1), same
verification-order discipline, same zeroization pattern, same
test-vector approach. Differences are confined to the claim
shape (no realm, no payload_hash) and the host crate
(`philharmonic-policy`, not the connector crates).

## Scope

### In scope (sub-phase B0)

A new module **`philharmonic-policy::api_token`** introducing:

- `EphemeralApiTokenClaims` — the CBOR-serialized claim
  struct, fields per
  [`docs/design/09-policy-and-tenancy.md` §"Ephemeral token claims"](../09-policy-and-tenancy.md#ephemeral-token-claims).
- `ApiSigningKey` — wraps a 32-byte Ed25519 seed in
  `Zeroizing<[u8; 32]>` plus a `kid: String`. Same shape as
  `LowererSigningKey` in `philharmonic-connector-client`,
  modeled on the same approval pattern (Wave A r3 split).
- `ApiSignedToken` — newtype wrapper for a `coset::CoseSign1`,
  parallel to `ConnectorSignedToken`.
- `ApiVerifyingKeyEntry` — the verifying-key record (`vk:
  ed25519_dalek::VerifyingKey`, `not_before: UnixMillis`,
  `not_after: UnixMillis`).
- `ApiVerifyingKeyRegistry` — `HashMap<String,
  ApiVerifyingKeyEntry>` with `new` / `insert` / `lookup`,
  parallel to `MintingKeyRegistry`.
- `mint_ephemeral_api_token(&ApiSigningKey,
  &EphemeralApiTokenClaims) -> Result<ApiSignedToken,
  ApiTokenMintError>` — the minting primitive.
- `verify_ephemeral_api_token(cose_bytes: &[u8], registry:
  &ApiVerifyingKeyRegistry, now: UnixMillis) ->
  Result<EphemeralApiTokenClaims, ApiTokenVerifyError>` — the
  verifying primitive.

Both functions are **pure crypto / serialization** —
substrate-free, async-free, no I/O. They take bytes and
return claims; substrate-state checks (authority lookup,
authority-epoch check, tenant-scope match,
instance-scope-vs-URL match) live in the **B1** consumer in
`philharmonic-api` where the substrate access lives.

A **`philharmonic-policy` crate version bump from 0.1.0 →
0.2.0**. Justification: this is a meaningful new module +
public surface (about a dozen new public types and two new
public functions), substantial enough to warrant a minor bump
in the 0.x series even though the change is purely additive to
the published 0.1.0 surface.

### Out of scope (sub-phase B0)

These belong to later sub-phases or to deployment. Every
B1/G/H handoff item below is a **normative requirement** on
the consumer — B0 cannot enforce it, but the next sub-phase
must. The B1 / G / H Codex prompts will pin each item with
explicit tests.

#### B1 (auth middleware in `philharmonic-api`) handoff contract

These are substrate-state checks B0 cannot perform — the
primitive is substrate-free by design. They are required for
correctness of the auth flow and must land in B1 with tests.

- **Authority lookup.** Look up `claims.authority` in the
  substrate. Reject if not found, retired, or its tenant is
  suspended.
- **Authority-epoch enforcement.** Reject if the authority's
  current `epoch` does not equal `claims.authority_epoch`.
  (Doc 11 §"Ephemeral API tokens".)
- **Authority-tenant binding.** **Reject if
  `authority.tenant != claims.tenant`.** This is the check
  that prevents a token signed under tenant B's authority
  from being accepted as a tenant A scope. Without it, the
  deployment-wide signing key would let any authority's mint
  pass any tenant's verify when `RequestScope::Tenant`
  matches `claims.tenant` only. Place this check
  **immediately after authority lookup, before epoch
  acceptance**. Negative test required: authority belongs to
  tenant B, claims.tenant = A, request scope = A, signature
  valid → reject.
- **Tenant-scope match against `RequestScope`.** Sub-phase C
  (authz) cross-checks `claims.tenant` against the
  request's `RequestScope::Tenant(...)`. (Mismatch → reject.)
- **Instance-scope match against URL.** If `claims.instance`
  is `Some(_)`, sub-phase C / D verifies that any instance
  ID in the request URL equals it.
- **Generic external auth-failure response.** Verify failures
  (whether from the B0 primitive's typed
  `ApiTokenVerifyError` variants or from any of the B1
  substrate checks above) MUST collapse to a single external
  shape: HTTP `401`, body
  `{"error":{"code":"unauthenticated","message":"invalid
  token","correlation_id":"..."}}`. **No leaking of `kid`,
  validity-window, signature, expiry, or which substrate
  check failed.** Internal logs may keep typed variants;
  external responses MUST NOT. This protects against
  attackers probing the verify decision tree.
  (Codex review Finding 8; pattern from Wave B's
  service-side handling.)
- **`authority_epoch` width conversion.** Substrate stores
  `MintingAuthority.epoch` as `i64` per doc 09; the wire claim
  is `u64`. Conversion: `u64::try_from(stored_i64)`. Negative
  or out-of-range stored epoch fails closed at the lookup
  step (treated like a substrate corruption — auth rejected,
  internal log records the issue). Negative test required.

#### G (minting endpoint) handoff contract

- **Permission clipping at mint.** G clips requested
  `permissions` to the authority's envelope before calling
  `mint_ephemeral_api_token`. Out-of-envelope atoms are
  silently stripped and audited (per doc 09).
- **`claims` size cap.** G enforces the 4 KiB cap on the
  caller-supplied injected claims **before** canonicalizing
  via `CanonicalJson`, returning an explicit error to the
  minting caller if exceeded. Defense-in-depth: B0's
  `mint_ephemeral_api_token` *also* enforces the cap on the
  resulting `CanonicalJson` size, but G should fail early.
- **`iat` set to mint time.** G sets `claims.iat` to
  `UnixMillis::now()` and `claims.exp` to whatever the
  per-authority lifetime configuration dictates, capped at
  the system maximum. Both populated correctly is a G
  requirement; B0's verify will reject if `exp - iat` exceeds
  the system maximum but that's a defense, not a substitute
  for G doing it right.
- **`authority_epoch` from substrate.** G reads the
  authority's current `epoch` and writes it into
  `claims.authority_epoch`. Same `i64 → u64` conversion rule
  as the verify path (see B1 contract).
- **Audit record content.** G writes an `AuditEvent` with
  subject + authority + tenant + instance — but NOT the full
  `claims` (especially not the `injected_claims` JSON),
  matching doc 09's audit guidance.

#### Out of scope for both B0 and the immediate B1/G follow-up

- **Signing-key file I/O / KMS integration / startup loading**
  — deployment / bin concern. The B0 library accepts seed
  bytes; the deployment binary reads the file (or KMS, or env
  var) and supplies the bytes. Same workspace rule as Wave A
  (libraries take bytes, not file paths).
- **No `cargo publish`** during B0. The new philharmonic-policy
  0.2.0 publishes only after Gate-2 code review on the B0 round.
- **No new crypto** beyond what's named in §"Primitives and
  library versions" below.

### Replay threat model

The verify primitive is deliberately stateless. No `jti`, no
server-side replay cache. Mitigation rests on a layered
defense, not on Wave A's "if a token leaks then minting is
also compromised" framing — which is **wrong** for browser-
resident or partner-system bearer tokens. Ephemeral API tokens
travel over many channels Wave A's connector tokens don't:
browser localStorage, server logs, crash reports, debug
proxies, copy-paste, referrer headers, third-party SDK
internals. Any of those exfiltrate the bearer without
exfiltrating the API signing key.

**Threat:** a leaked ephemeral API token is replayable, by
whoever holds the bytes, against any API endpoint covered by
the token's `permissions` and `instance` scope, until `exp`
or until an authority-epoch bump invalidates outstanding
tokens.

This is the standard bearer-token tradeoff. We take it
deliberately for v1. Mitigations layered against it:

- **Short `exp`.** The system maximum is 24h per
  [doc 11 §"Ephemeral API tokens"](../11-security-and-cryptography.md#ephemeral-api-tokens);
  per-authority configuration goes shorter. The verify
  primitive enforces `exp - iat ≤ MAX_TOKEN_LIFETIME_MILLIS`
  (24h) so even a buggy mint can't issue longer-lived tokens.
- **Instance scope as the recommended browser default.**
  Doc 09 §"Instance-scoped ephemeral tokens" makes this the
  norm: token + browser session ↔ one workflow instance. A
  leaked token can only call `execute_step` on its single
  instance.
- **Permission clipping.** Each token's `permissions` list
  was clipped at mint to the minting authority's envelope.
  Even with the token, an attacker can only do what that
  envelope allowed.
- **Authority epoch bump.** If a leak is detected, bumping
  `MintingAuthority.epoch` invalidates all outstanding tokens
  under that authority within the verify path's epoch check
  (B1 / sub-phase C).
- **Rate limiting on auth-failure churn.** Sub-phase H rate
  limits per tenant per endpoint. Generic external auth
  failures (Finding 8 below) prevent attackers from
  efficiently probing whether a stolen token is still valid.
- **Deployment guidance.** Browser tokens must not persist
  past the session that needs them; server-side stores must
  not log bearer bytes. Documented in B1's deployment notes.

What we explicitly do **not** do, and why:

- **No `jti` + replay cache.** Same Wave A reasoning, with
  bearer-token framing now made explicit. The defenses above
  cap blast radius without requiring a distributed cache.
  Trade-off documented; revisit if a deployment with stricter
  replay requirements surfaces.
- **No TLS-piercing assumption.** Wave A's connector tokens
  travel an internal lowerer→service path where TLS
  termination + internal-network assumptions are realistic.
  Ephemeral API tokens travel arbitrarily far through tenant
  systems; we cannot assume TLS protects them once minted.
  The verify primitive's correctness must not depend on
  channel security after mint.

This section exists so the threat-model decision is explicit
and re-reviewable. If the deployment model adds e.g. a
zero-trust browser-token replay store, this section needs an
update.

## Primitives and library versions

All RustCrypto + coset, all already in the workspace dep tree.
Versions verified against crates.io on **2026-04-28** via
`./scripts/xtask.sh crates-io-versions`. To be re-verified
the day Codex dispatches B0; pinned as part of the Codex
prompt.

- **`ed25519-dalek = "2"`** — Ed25519 sign/verify. Already in
  `philharmonic-connector-client` and
  `philharmonic-connector-service` at the same major. No new
  feature flags.
- **`coset = "0.4"`** — COSE_Sign1 framing. Already in
  `philharmonic-connector-common`. Reuses the same coset
  surface (`CoseSign1Builder`, `HeaderBuilder`, `iana`,
  `Algorithm`, `CborSerializable`) the connector code uses.
- **`ciborium = "0.2"`** — CBOR encode/decode for the claim
  payload. Already a transitive dep via `coset`, but B0 will
  need it as a direct dep on `philharmonic-policy` for
  `ciborium::ser::into_writer` /
  `ciborium::de::from_reader`. The added direct dep mirrors
  what `philharmonic-connector-client` and
  `philharmonic-connector-service` already do.
- **`subtle = "2"`** — already in the dep tree via
  `ed25519-dalek`. Used here only if a constant-time
  comparison surfaces as needed; for B0's claim-equality
  checks (kid consistency) plain `==` on `String` is fine
  because the claim payload was just signature-validated, so
  there's no untrusted-input timing-side-channel concern. We
  pull `subtle` only if Yuka prefers extra defense-in-depth
  (open question Q1 below).
- **`zeroize = { version = "1", features = ["derive"] }`** —
  for `ApiSigningKey`'s seed wrapping. Already in
  `philharmonic-policy 0.1.0` (used by `Sck` and the
  pht_-token surface).
- **`philharmonic-types`** — for `Uuid`, `UnixMillis`. Bump
  to **`0.3.x`** matching what the rest of the workspace
  uses; `0.3.5+` recommended for the human-readable-aware
  Sha256 serde even though we don't use Sha256 in B0 (keeps
  policy's dep set in lockstep with connector-common).
- **`philharmonic-policy`** itself — bumps `0.1.0 → 0.2.0`.
  Existing public surface untouched; only additive.

**No new primitives.** No `unsafe`. No custom MAC / KDF /
AEAD. No KEM. (Wave B's hybrid KEM construction is
unaffected; B0 is sign-only, parallel to Wave A.)

## Construction

### Token shape

The COSE_Sign1 payload is a CBOR encoding of
`EphemeralApiTokenClaims`. Field set adapted from
[`docs/design/09-policy-and-tenancy.md` §"Ephemeral token claims"](../09-policy-and-tenancy.md#ephemeral-token-claims),
with the additions tagged below:

```rust
pub struct EphemeralApiTokenClaims {
    pub iss: String,
    pub iat: UnixMillis,            // r2 / Finding 3
    pub exp: UnixMillis,
    pub sub: String,
    pub tenant: Uuid,
    pub authority: Uuid,
    pub authority_epoch: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<Uuid>,
    pub permissions: Vec<String>,
    pub claims: CanonicalJson,      // r2 / Finding 5
    pub kid: String,
}
```

The two r2 changes vs r1:

- **`iat` added.** Required for the verify primitive to
  enforce `exp - iat ≤ MAX_TOKEN_LIFETIME_MILLIS` (doc 11's
  24h system maximum). Without it, B0 cannot bound token
  lifetime independently — the lifetime invariant would have
  to live in G, and a buggy G could mint validly-signed
  tokens with arbitrary far-future `exp`. Added as a
  precondition for B0 owning the lifetime invariant.
  (Codex review Finding 3.)
- **`claims: CanonicalJson` (was `serde_json::Value`).**
  The injected-claims field carries RFC 8785 (JCS)
  canonical JSON bytes via
  [`philharmonic_types::CanonicalJson`](../../../philharmonic-types/src/canonical.rs).
  `CanonicalJson` already exists in the workspace at
  `philharmonic-types 0.3.x` with stable serde
  Serialize/Deserialize that round-trips through
  canonicalization. Rationale: `serde_json::Value::Object`
  CBOR encoding via ciborium does not pin map insertion
  order, so identical JSON content can produce different
  CBOR bytes — and once those bytes are signed, two
  semantically-identical tokens would have different
  signature bytes, breaking known-answer vectors and
  forensic comparisons. JCS sorts keys
  lexicographically and pins a canonical numeric/string
  representation, fixing the determinism cleanly.
  (Codex review Finding 5; resolves Q2 from r1.)

The wire encoding of `claims` is then a single CBOR text
string carrying the canonical JSON bytes (as
`CanonicalJson` serializes to a UTF-8 string of canonical
JSON). The whole token claim is signed; the
canonicalization is performed at mint time by the caller (G)
and re-canonicalized at verify time only as a
defense-in-depth check. Verify-side handling:

- Compute `serde_jcs::to_vec(serde_json::from_str(claims.as_str())?)`
  as a normalization step.
- Compare the recanonicalized bytes against the wire bytes.
  Mismatch → reject as `ApiTokenVerifyError::ClaimsNotCanonical`.
  This catches a non-canonical-JCS mint at the verify
  boundary, but normal G should always produce canonical
  bytes so this is a safety net.

CBOR encoding of the **outer struct** matches Wave A
conventions:
- `Uuid` (`tenant`, `authority`, `instance`) → 16-byte CBOR
  byte string.
- `UnixMillis` (`iat`, `exp`) → CBOR unsigned integer for
  positive values (always positive post-epoch).
- `u64` (`authority_epoch`) → CBOR unsigned integer.
- `String` (`iss`, `sub`, `kid`, each entry of `permissions`)
  → CBOR text string.
- `Vec<String>` (`permissions`) → CBOR array of text strings.
- `CanonicalJson` (`claims`) → CBOR text string (see above).
- `Option<Uuid>` (`instance`) → either omitted (skip-if-none)
  or 16-byte CBOR byte string.

`#[serde(skip_serializing_if = "Option::is_none")]` on
`instance` keeps absent-instance tokens compact and matches
the doc 09 spec that the field is optional. The verify side
deserializes either shape.

The struct's CBOR encoding is canonical per RFC 8949 §4.2
deterministic encoding for the field types above (no
recursive maps inside the struct itself; only fixed-shape
fields). The previous r1 concern about `serde_json::Value`
ordering is moot — that field is now a string of canonical
JSON, and CBOR text-string encoding is unique per byte
sequence.

### Protected headers

The COSE_Sign1 `protected` bucket carries:

- `alg = -8` (EdDSA per RFC 9053 §2.2).
- `kid = claims.kid` as a UTF-8 byte string.

`unprotected` header is empty. Same shape as Wave A.

### Signature input

The COSE_Sign1 signature is computed over the
`Sig_structure1` per RFC 9052 §4.4. Unchanged from Wave A:

```
Sig_structure1 = [
    context: "Signature1",
    body_protected: serialized protected header bucket,
    external_aad: h'' (empty),
    payload: serialized EphemeralApiTokenClaims CBOR,
]
```

`coset::CoseSign1Builder::create_signature` handles the
encoding; we pass the claim bytes, a closure that runs Ed25519
signing, and the protected header builder. Identical pattern
to `LowererSigningKey::mint_token`.

### External AAD

Empty. Every field that needs to be bound is already in the
payload (`kid`, `tenant`, `authority`, `authority_epoch`,
`instance`, `permissions`). External AAD would duplicate
what's already signed; matches Wave A's RFC-recommended JWT-
like pattern.

### Verification order

The verify primitive runs checks in this order, stopping at
the first failure. Ordering is deliberate, mirroring Wave A's
sign-side discipline: algorithm and key-level checks fail
before expensive crypto, signature verification fails before
any untrusted payload content is trusted, all content-level
checks run over verified claim bytes. r2 expands the order
from r1's eight steps to thirteen, addressing Codex review
Findings 2, 3, 6, 9, 10.

1. **Token-byte ceiling.** Reject if `cose_bytes.len() >
   MAX_TOKEN_BYTES` (default 16 KiB).
   `ApiTokenVerifyError::TokenTooLarge`. Bounds parse / SHA /
   allocation work before any expensive operation.

2. **Parse** the COSE_Sign1 bytes via
   `coset::CoseSign1::from_slice`. Malformed → reject
   (`ApiTokenVerifyError::Malformed`).

3. **Strict header profile.** *(r2 / Finding 9.)*
   - Protected header MUST contain exactly `alg` and `kid`
     and nothing else; any unknown protected header label,
     and any `crit` header, → reject
     (`ApiTokenVerifyError::HeaderProfileViolation`).
   - Unprotected header MUST be empty (no labels, no
     values). Non-empty unprotected → reject (same error
     variant).
   This closes a forensic / future-maintenance hole where an
   attacker adds misleading metadata in the unprotected
   bucket that later code might accidentally inspect. Add
   negative vectors covering both shapes.

4. **Pin algorithm.** Read `alg` from the protected header;
   require `alg == -8` (EdDSA per RFC 9053 §2.2). Any other
   value → `ApiTokenVerifyError::AlgorithmNotAllowed`.

5. **Kid format profile.** *(r2 / Finding 10.)* Extract the
   raw `kid` bytes from the protected header. Reject unless:
   - bytes are valid UTF-8;
   - length is between 1 and 128 inclusive;
   - every byte is in `[A-Za-z0-9._:-]`.
   Failures → `ApiTokenVerifyError::KidProfileViolation`. Cap
   keeps logs tidy and operator-config typos easy to spot;
   character profile rejects visually-confusable Unicode
   `kid`s before they reach the registry. Negative vectors
   for over-length, control-char, and non-ASCII `kid`s.

6. **Kid lookup.** Look up the verifier key in the
   `ApiVerifyingKeyRegistry`. Unknown kid →
   `ApiTokenVerifyError::UnknownKid`.

7. **Key validity window.** The registry entry carries
   `not_before` / `not_after` (`UnixMillis`). Reject if `now <
   not_before` or `now >= not_after`
   (`ApiTokenVerifyError::KeyOutOfWindow`).

8. **Signature verification.** Use
   `coset::CoseSign1::verify_signature` with the Ed25519
   verifying key from step 6. Bad signature →
   `ApiTokenVerifyError::BadSignature`. **No claim content is
   trusted before this step passes.**

9. **Claim payload decode.** Decode the claim payload from
   CBOR into `EphemeralApiTokenClaims` (with `claims:
   CanonicalJson`). Malformed → treat as
   `ApiTokenVerifyError::Malformed`.

10. **Canonicalization re-check.** *(r2 / Finding 5.)* Take
    the decoded `claims: CanonicalJson` value, parse it as
    JSON, re-canonicalize via JCS, compare byte-for-byte with
    the wire bytes. Mismatch →
    `ApiTokenVerifyError::ClaimsNotCanonical`. Defense-
    in-depth — ensures a non-canonical mint can't slip
    through and produce divergent audit hashes. Constant-time
    not required (claims are signature-validated by step 8).

11. **Issuer binding.** *(r2 / Finding 2.)* Require
    `claims.iss == registry_entry.issuer`. Mismatch →
    `ApiTokenVerifyError::IssuerMismatch`. The registry entry
    declares which issuer string the deployment is willing to
    accept tokens for under this kid; without this check, a
    leaked verifying key trusted in the registry would
    accept any `iss`. Negative vector required.

12. **Kid consistency.** Require `claims.kid ==
    protected.kid`. Both are signature-covered, but
    duplication invites drift. Mismatch →
    `ApiTokenVerifyError::KidInconsistent`.

13. **Lifetime invariants.** *(r2 / Finding 3.)* All using
    checked arithmetic; any overflow / underflow rejects
    with `ApiTokenVerifyError::LifetimeInvariantViolation`:
    - `claims.iat <= now + ALLOWED_CLOCK_SKEW_MILLIS`
      (default 60 000 ms / 60 s) — token not minted in the
      future.
    - `claims.exp > now` — not expired.
      `ApiTokenVerifyError::Expired { exp, now }`.
    - `claims.exp > claims.iat` — well-formed lifetime.
    - `claims.exp - claims.iat <= MAX_TOKEN_LIFETIME_MILLIS`
      (default 24 h, doc 11 §"Ephemeral API tokens") —
      ephemeral means ephemeral. A primitive-level ceiling
      makes a buggy G unable to mint long-lived tokens.

14. **Injected-claims size cap.** *(r2 / Finding 6.)* Require
    `claims.bytes().len() <= MAX_INJECTED_CLAIMS_BYTES`
    (default 4 096 = 4 KiB, doc 09 §"Ephemeral token
    claims"). Oversize →
    `ApiTokenVerifyError::ClaimsTooLarge { limit, actual }`.
    G enforces this at mint too; B0 enforces at verify as a
    defense against a bypassed G or a hand-crafted token
    that signed valid bytes but exceeded the cap.

Only after all fourteen pass is
`Ok(EphemeralApiTokenClaims)` returned.

**Constants — pinned defaults:**

```rust
pub const MAX_TOKEN_BYTES: usize = 16 * 1024;            // 16 KiB
pub const MAX_INJECTED_CLAIMS_BYTES: usize = 4 * 1024;   //  4 KiB
pub const MAX_TOKEN_LIFETIME_MILLIS: i64 = 24 * 60 * 60 * 1000; // 24 h
pub const ALLOWED_CLOCK_SKEW_MILLIS: i64 = 60_000;       // 60 s
pub const KID_MIN_LEN: usize = 1;
pub const KID_MAX_LEN: usize = 128;
```

A `verify_ephemeral_api_token_with_limits` variant accepts
caller-supplied lifetime / size / skew limits for deployments
that want stricter values; the default
`verify_ephemeral_api_token` wraps it with the constants
above. A deployment **cannot** loosen the constants past the
pinned defaults — `with_limits` clamps to `min(default,
caller_supplied)` and treats a caller value greater than the
default as a configuration error.

**Notably absent vs Wave A:**

- No `payload_hash` check. Wave A binds an encrypted payload
  via `payload_hash`; ephemeral API tokens have no auxiliary
  payload, so there's nothing to bind beyond the claim CBOR
  itself (signature-covered).
- No `realm` binding. The role realm played in Wave A
  (service-side audience) is taken here by `iss` binding
  against the registry entry (step 11) plus, at the
  consumer, tenant-scope match against `RequestScope`.
- No authority lookup, authority-epoch enforcement, or
  authority-tenant binding. These are substrate-state
  checks done by the B1 consumer (see "B1 handoff
  contract" in §"Out of scope (sub-phase B0)" above); the
  verify primitive returns the claims and lets B1
  cross-check.
- No instance-scope-vs-URL match. That's a sub-phase C / D
  concern.

So the verify primitive is **pure**:
signature-and-CBOR-and-time concerns plus the kid/iss
binding. Substrate state concerns belong to the consumer.

## Key management

### Minting side

Per workspace convention §Library crate boundaries (and Wave
A's r3 split), `philharmonic-policy` accepts seed bytes — no
file I/O. The deployment binary loads the seed however it
sees fit (file-permission-checked filesystem path, KMS
fetch, env var, etc.) and constructs `ApiSigningKey`.

```rust
pub struct ApiSigningKey {
    seed: Zeroizing<[u8; 32]>,
    kid: String,
}

// r2 / Finding 4: explicitly NOT derive Debug. Manual redacted
// impl prints the kid and a marker for the seed.
impl std::fmt::Debug for ApiSigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiSigningKey")
            .field("kid", &self.kid)
            .field("seed", &"<redacted; 32 bytes>")
            .finish()
    }
}

impl ApiSigningKey {
    pub fn from_seed(seed: Zeroizing<[u8; 32]>, kid: String) -> Self;
    pub fn kid(&self) -> &str;
}

pub fn mint_ephemeral_api_token(
    signing_key: &ApiSigningKey,
    claims: &EphemeralApiTokenClaims,
) -> Result<ApiSignedToken, ApiTokenMintError>;
```

The mint function:
- Asserts `claims.kid == signing_key.kid()` —
  `ApiTokenMintError::KidMismatch` on disagreement (catches a
  bug class where the caller forgot to set the claim's kid to
  match the signing key).
- Validates the same `kid` profile as the verify side
  (`[A-Za-z0-9._:-]`, length 1..=128). Out-of-profile kid in
  the signing key OR in the claims →
  `ApiTokenMintError::KidProfileViolation`.
- Validates `claims.claims.bytes().len() <=
  MAX_INJECTED_CLAIMS_BYTES` (4 KiB). Oversize →
  `ApiTokenMintError::ClaimsTooLarge`. Defense-in-depth — G
  is supposed to enforce this earlier but B0 fails closed if
  G missed.
- Validates the lifetime invariants: `claims.iat <=
  now + ALLOWED_CLOCK_SKEW_MILLIS` (the mint caller's
  responsibility to set `iat` correctly, but B0 sanity-checks
  using a `now: UnixMillis` parameter), `claims.exp >
  claims.iat`, `claims.exp - claims.iat <=
  MAX_TOKEN_LIFETIME_MILLIS`. Violations →
  `ApiTokenMintError::LifetimeInvariantViolation`. The mint
  signature gains the `now` parameter:
  ```rust
  pub fn mint_ephemeral_api_token(
      signing_key: &ApiSigningKey,
      claims: &EphemeralApiTokenClaims,
      now: UnixMillis,                      // r2
  ) -> Result<ApiSignedToken, ApiTokenMintError>;
  ```
- Serializes the claims to CBOR via `ciborium`. Because
  `claims.claims: CanonicalJson` is already canonical, the
  outer CBOR encoding is the only ordering concern, and
  ciborium's struct serialization is field-order-stable.
- Builds the COSE_Sign1 with `alg = -8` and `kid =
  claims.kid` in the protected header. Unprotected header
  empty.
- Signs by reconstructing a transient
  `ed25519_dalek::SigningKey::from_bytes(&seed)` per call (Q1
  resolution from Wave A; carries through here).

The minting primitive does **not** enforce permission
envelopes or per-authority lifetime configuration. Sub-phase
G (the minting endpoint) is responsible for clipping
`permissions` to the authority's envelope and computing the
final `exp` from the per-authority lifetime configuration
before calling this primitive. The primitive enforces only
the **system maximum** lifetime (24 h, doc 11) — letting it
also carry per-authority state would couple it to
substrate-loaded data it doesn't have.

### Verifying side

```rust
pub struct ApiVerifyingKeyEntry {
    pub vk: ed25519_dalek::VerifyingKey,
    pub issuer: String,                    // r2 / Finding 2
    pub not_before: UnixMillis,
    pub not_after: UnixMillis,
}

pub struct ApiVerifyingKeyRegistry {
    by_kid: HashMap<String, ApiVerifyingKeyEntry>,
}

impl ApiVerifyingKeyRegistry {
    pub fn new() -> Self;
    pub fn insert(&mut self, kid: String, entry: ApiVerifyingKeyEntry)
        -> Result<(), RegistryInsertError>;
    pub fn lookup(&self, kid: &str) -> Option<&ApiVerifyingKeyEntry>;
}

pub fn verify_ephemeral_api_token(
    cose_bytes: &[u8],
    registry: &ApiVerifyingKeyRegistry,
    now: UnixMillis,
) -> Result<EphemeralApiTokenClaims, ApiTokenVerifyError>;

pub fn verify_ephemeral_api_token_with_limits(
    cose_bytes: &[u8],
    registry: &ApiVerifyingKeyRegistry,
    now: UnixMillis,
    limits: &VerifyLimits,
) -> Result<EphemeralApiTokenClaims, ApiTokenVerifyError>;

pub struct VerifyLimits {
    pub max_token_bytes: usize,            // clamped <= MAX_TOKEN_BYTES
    pub max_injected_claims_bytes: usize,  // clamped <= MAX_INJECTED_CLAIMS_BYTES
    pub max_token_lifetime_millis: i64,    // clamped <= MAX_TOKEN_LIFETIME_MILLIS
    pub allowed_clock_skew_millis: i64,    // clamped <= ALLOWED_CLOCK_SKEW_MILLIS
}
```

`ApiVerifyingKeyEntry::issuer` declares which `iss` value
this kid is permitted to sign for. Verify step 11 enforces
`claims.iss == entry.issuer`. Without this, any verifying key
in the registry would be effectively trusted to mint for any
issuer string — a problem for staging/prod sharing keys, for
multi-issuer future deployments, and for audit hygiene.
(Codex review Finding 2.)

`ApiVerifyingKeyRegistry::insert` returns
`Result<(), RegistryInsertError>`. The insert validates the
`kid` against the format profile (same constraints as the
verify side) and rejects duplicate-kid inserts. Bin code
that catches the error fails closed at startup rather than
silently accepting a malformed registry.

Public keys aren't sensitive; no zeroization. The library
ships no `load_from_file`, `load_from_toml`, etc. The
deployment binary parses whatever config format it chooses,
constructs `ApiVerifyingKeyEntry` values, and calls `insert`
per signing-key generation at boot. Same library/bin split
discipline as Wave A.

Rotation is additive: a new kid → new registry entry; old
kids stay registered until tokens issued under them have
expired. `not_before` / `not_after` are **enforced** at
verify step 7 — they're not advisory. Matches doc 11
§"Ephemeral API token signing key rotation" exactly.

### `kid` encoding

*(r2 / Finding 10. Tightened from r1's free-form Wave-A
parity.)*

`kid` is signed as part of the protected header. To bound
log/operator-config pollution and to reject visually-
confusable Unicode in operator-supplied identifiers, we pin
a strict character profile:

- ASCII only (UTF-8 valid, all bytes in `[A-Za-z0-9._:-]`).
- Length 1..=128 bytes (`KID_MIN_LEN..=KID_MAX_LEN`).

Suggested format remains
`<api-issuer-slug>-<utc-date>-<rand-hex-8>`
(e.g. `api.tenant-2026-04-28-a1b2c3d4`). Registry uses
exact-string equality on already-validated kids.

Both the mint and verify primitives validate the profile
before any further processing: mint at construction time
(both for `signing_key.kid` and `claims.kid`), verify at
step 5 before registry lookup. `ApiVerifyingKeyRegistry::insert`
also validates incoming kids and rejects out-of-profile
entries at startup, so an operator typo never reaches a
verify path.

## Zeroization points

**Private keys only**:

- `Zeroizing<[u8; 32]>` owns the 32-byte Ed25519 seed in
  `ApiSigningKey`. The deployment binary populates it; the
  library holds the wrapper for the `ApiSigningKey`'s
  lifetime; `Zeroizing::drop` zeros on drop.
- Each `mint_ephemeral_api_token` call reconstructs a
  transient `ed25519_dalek::SigningKey` via
  `SigningKey::from_bytes(seed.as_ref())` and drops it at
  end-of-call. Same Wave-A-r1 zeroization pattern (Q1 option
  (a), already approved 2026-04-22).
- Signing-time intermediates (the `r` nonce in Ed25519) live
  inside `ed25519-dalek` and aren't exposed; nothing for us
  to zero.

The verify primitive holds no private key material — only
public verifying keys (which need no zeroization) and
signature/payload bytes.

The B1 consumer (sub-phase B1 in `philharmonic-api`) handles
the bearer token bytes from the `Authorization` header. Those
bytes are not secret in the same sense as a key — they're a
capability that's about to be checked — but to mirror the
`pht_` token treatment in `philharmonic-policy::token`, we
should `Zeroize` the parsed bearer string after verification.
This is a B1 concern, not B0; flagged here for cross-reference.

## Test-vector plan

Same discipline as Wave A. Known-answer vectors committed as
hex-encoded byte strings. Cross-checked against pycose 2.x
where possible (Q3 below).

Vectors live alongside the crate at
`philharmonic-policy/tests/vectors/api_token/`.

### Ed25519 keypair

Reuse RFC 8032 §7.1 TEST 1 (same as Wave A) — public test
keypair, easy to cross-check externally.

```json
{
  "seed_hex": "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
  "public_key_hex": "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
}
```

### Claim set

```json
{
  "iss": "philharmonic-api.example",
  "exp_millis": 1924992000000,
  "sub": "user-42",
  "tenant_uuid": "11111111-2222-4333-8444-555555555555",
  "authority_uuid": "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee",
  "authority_epoch": 7,
  "instance_uuid": "66666666-7777-4888-8999-aaaaaaaaaaaa",
  "permissions": ["workflow:instance_execute"],
  "claims": {
    "session_id": "demo-session-001",
    "role": "viewer"
  },
  "kid": "api.test-2026-04-28-deadbeef"
}
```

A second positive vector with `instance` absent:

```json
{
  "iss": "philharmonic-api.example",
  "exp_millis": 1924992000000,
  "sub": "user-42",
  "tenant_uuid": "11111111-2222-4333-8444-555555555555",
  "authority_uuid": "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee",
  "authority_epoch": 7,
  "permissions": ["workflow:list", "workflow:read"],
  "claims": {},
  "kid": "api.test-2026-04-28-deadbeef"
}
```

(Tests both code paths around the `Option<Uuid>`.)

### Expected CBOR claim bytes

Hex-encoded canonical CBOR of each claim set. Generated by
running the implementation once, verified against RFC 8949's
deterministic-encoding rules. Committed at
`tests/vectors/api_token/claims_with_instance.cbor.hex` and
`tests/vectors/api_token/claims_no_instance.cbor.hex`.

### Expected COSE_Sign1 bytes

Hex-encoded final COSE_Sign1 structure for each claim set,
signed under the RFC 8032 TEST 1 seed. Cross-checked against
pycose 2.x. Committed as
`tests/vectors/api_token/signed_with_instance.hex` and
`tests/vectors/api_token/signed_no_instance.hex`.

### Negative-path vectors

One vector per rejection reason in the verify order. Each
must fail with the specific `ApiTokenVerifyError` variant
named. r2 expands the list from r1's eight to fifteen,
covering the new steps.

- `api_token_too_large.hex` — COSE_Sign1 of `MAX_TOKEN_BYTES
  + 1` bytes (constructed by padding with garbage trailing
  bytes that still serialize as a valid CBOR major type).
  Step 1, `TokenTooLarge`.
- `api_unprotected_nonempty.hex` — *(r2 / Finding 9)* same
  claims + key, but the unprotected header carries
  `kid = "decoy"`. Step 3, `HeaderProfileViolation`.
- `api_protected_unknown_label.hex` — *(r2 / Finding 9)*
  protected header carries an extra unknown label besides
  `alg` + `kid`. Step 3, `HeaderProfileViolation`.
- `api_protected_crit.hex` — *(r2 / Finding 9)* protected
  header carries a `crit` label. Step 3,
  `HeaderProfileViolation`.
- `api_bad_alg.hex` — same claims + key, protected header
  re-encoded with `alg = -7` (ES256). Step 4,
  `AlgorithmNotAllowed`.
- `api_kid_too_long.hex` — *(r2 / Finding 10)* protected
  header `kid` of 129 bytes. Step 5, `KidProfileViolation`.
- `api_kid_invalid_chars.hex` — *(r2 / Finding 10)* protected
  header `kid` containing a slash or space (out of profile).
  Step 5, `KidProfileViolation`.
- `api_unknown_kid.hex` — `kid` in profile but not in the
  registry. Step 6, `UnknownKid`.
- `api_key_out_of_window.hex` — valid token, registry entry
  has `not_after` in the past. Step 7, `KeyOutOfWindow`.
- `api_tampered_sig.hex` — last byte of the signature
  flipped. Step 8, `BadSignature`.
- `api_tampered_payload.hex` — one byte of the claim payload
  flipped. Step 8, `BadSignature`.
- `api_claims_not_canonical.hex` — *(r2 / Finding 5)* mint
  signs an `EphemeralApiTokenClaims` whose `claims:
  CanonicalJson` byte payload is non-canonical (e.g.,
  reverse-ordered keys); verify re-canonicalizes and
  byte-mismatches. Step 10, `ClaimsNotCanonical`.
- `api_issuer_mismatch.hex` — *(r2 / Finding 2)* registry
  entry `issuer = "philharmonic-api.example"`; token's
  `claims.iss = "philharmonic-api.attacker"`. Step 11,
  `IssuerMismatch`.
- `api_kid_inconsistent.hex` — protected header `kid` and
  `claims.kid` differ. Step 12, `KidInconsistent`.
- `api_iat_in_future.hex` — *(r2 / Finding 3)* `iat` set
  120 s in the future of `now`, beyond the 60 s default
  skew window. Step 13, `LifetimeInvariantViolation`.
- `api_exp_before_iat.hex` — *(r2 / Finding 3)* `iat` ahead
  of `exp`. Step 13, `LifetimeInvariantViolation`.
- `api_lifetime_too_long.hex` — *(r2 / Finding 3)* `exp -
  iat` set to 25 h (one hour past the 24 h system maximum).
  Step 13, `LifetimeInvariantViolation`.
- `api_expired.hex` — `exp` set to 1 (long in the past).
  Step 13, `Expired`.
- `api_claims_too_large.hex` — *(r2 / Finding 6)* signed
  token whose `claims` injected-JSON bytes exceed 4 KiB
  (constructed by signing a 4 KiB + 1 canonical JSON
  payload). Step 14, `ClaimsTooLarge`.

Nineteen negative vectors total. Each is paired with a
matching unit test asserting the specific
`ApiTokenVerifyError` variant. The integration test that
runs the whole vector suite asserts that **every** non-
trivial verify error path is covered (no rejection step
without an associated negative vector).

### Round-trip + property tests (in addition)

- `mint(claims) → verify(...)` returns the same claims for
  both positive vectors. Asserts canonical encoding stability.
- A property-test loop over arbitrary `EphemeralApiTokenClaims`
  values (using `proptest` if Yuka approves the dep —
  otherwise a hand-written generator over a few dozen
  shapes): mint then verify must round-trip. With `claims:
  CanonicalJson`, the JCS canonicalization step is the
  primary thing being fuzzed — random JSON input through
  the canonicalize-then-sign-then-verify-then-recanonicalize
  loop should always succeed when properly canonicalized
  upstream and always reject (`ClaimsNotCanonical`) when
  given hand-crafted non-canonical bytes.

`proptest` would be a new dev-dep on `philharmonic-policy`.
Open question Q4 below.

## Explicit confirmations (per crypto-review skill)

1. **Understanding of the signing construction.** COSE_Sign1
   per RFC 9052 §4.4. Signature is over the CBOR-encoded
   `Sig_structure1 = ["Signature1", body_protected_bytes,
   external_aad=h'', payload_bytes]`. Ed25519 per RFC 8032 is
   deterministic. COSE algorithm ID `-8` (EdDSA) per RFC 9053
   §2.2. **B0 does NOT involve a hybrid KEM, HKDF, AEAD, or
   symmetric key derivation.**

2. **`unsafe` usage.** None planned. `ed25519-dalek 2.x` and
   `coset 0.4.x` use `unsafe` internally (RustCrypto / coset
   internals); we don't add any.

3. **Key handling that can't be zeroized.** None in the
   landed design. `ed25519_dalek::SigningKey` (2.x) doesn't
   itself implement `Zeroize`, so we never hold one longer
   than a single `mint_ephemeral_api_token` call. Same Wave A
   r1 pattern.

4. **Signatures over untrusted input.** The mint side takes
   trusted input (consumer-assembled claims). The verify
   side takes attacker-controlled COSE_Sign1 bytes; signature
   verification gates everything — no claim field is trusted
   before signature passes, standard COSE / JWT discipline.

5. **`claims: CanonicalJson` for the injected-claims
   field** *(updated in r2)*. Justification: (a) doc 09
   says `claims` is "free-form, tenant-defined" JSON, and
   `philharmonic_types::CanonicalJson` carries that shape
   with RFC 8785 / JCS canonicalization built in; (b) the
   canonical bytes are stable per JCS, eliminating the
   CBOR-map-ordering ambiguity that r1's `serde_json::Value`
   had; (c) the wire encoding is a single CBOR text string,
   the simplest possible binding shape; (d) the verify path
   re-canonicalizes and byte-compares, so a non-canonical
   mint cannot produce divergent audit bytes (verify step
   10). The 4 KiB injected-claims cap is enforced at both
   mint and verify (steps 6 / 14 respectively); the
   per-authority `permissions` envelope is enforced at sub-
   phase G's mint endpoint.

6. **No new crypto.** Same primitives, library versions, and
   construction shape as Wave A's already-reviewed surface
   (Ed25519 + COSE_Sign1 via coset). The only structurally-new
   bit vs Wave A is `CanonicalJson` for the injected-claims
   field — and that's a serialization layer, not a crypto
   primitive.

## Open questions

Tagged for Yuka's call at sign-off. r2 has resolved Q2 and
Q5 inline (see §"Revisions"); Q1, Q3, Q4, Q6 remain as
posed in r1, and r2 adds Q7 / Q8.

**Q1.** Use `subtle = "2"` for the kid-equality check at
verify step 12? Wave A used `subtle` for the payload-hash
compare specifically because that compares
attacker-controlled bytes against trusted bytes; here, both
kids come from the same signature-validated payload, so
timing-side-channel risk is nil. My recommendation:
**plain `==`**, document the reasoning in a code comment.
Yuka's call.

**Q2 (RESOLVED in r2).** ~~CBOR determinism for
`serde_json::Value`~~ → resolved by replacing
`claims: serde_json::Value` with `claims: CanonicalJson`
(RFC 8785 / JCS). On the wire, `claims` is a CBOR text
string holding canonical JSON; map-ordering ambiguity is
gone.

**Q3.** pycose 2.x cross-check, same as Wave A's Q2? My
recommendation: **yes** — at minimum for the two positive
COSE_Sign1 hex vectors. Same script vehicle as Wave A's
vector generator (the workspace has experience with it).

**Q4.** `proptest` as a new dev-dep on
`philharmonic-policy`? Cooldown-checked, RustCrypto-style
code (it's QuickCheck-shaped, not crypto). **My
recommendation: yes** — useful for fuzz-shape coverage of
the verify path (random byte inputs around the parser
boundaries, random JSON inputs through the
re-canonicalization check) even though `claims` is now
JCS-pinned.

**Q5 (RESOLVED in r2).** ~~Should the verify primitive
enforce a maximum `exp - now` window?~~ → yes, B0 owns the
24 h system-maximum invariant via `iat` + checked
arithmetic. G owns the per-authority finer-grained
ceiling. See verify step 13.

**Q6.** The B0 round produces no `cargo publish` — fine,
matches Wave A. But the B1 consumer in `philharmonic-api`
needs `philharmonic-policy 0.2.0`. Two paths:
- **(a)** Publish 0.2.0 immediately after B0 Gate-2 review,
  before B1 dispatches. Standard publish dance via
  `./scripts/publish-crate.sh`.
- **(b)** Pin B1 against a path-dep (workspace
  `[patch.crates-io]`) until the full Phase 8 lands; publish
  0.2.0 alongside `philharmonic-api` 0.1.0 at sub-phase I.
  Avoids a mid-Phase-8 publish; risk: a regression caught
  late in Phase 8 forces a 0.2.1 fix, but no users would
  have consumed 0.2.0 yet.

My recommendation: **(b) — patch.crates-io**, publish at
Phase 8 close. Matches the "publish at the end of the
phase" rhythm we've used for connector impls.

**Q7 (new in r2).** `ALLOWED_CLOCK_SKEW_MILLIS` default of
60 000 ms (60 s) reasonable? The verify primitive checks
`claims.iat <= now + ALLOWED_CLOCK_SKEW_MILLIS` — too
tight and benign clock drift between mint and verify hosts
causes spurious rejects; too loose and a "minted in the
future" attack window opens. 60 s matches conservative TLS
/ JWT defaults. Alternative defaults considered: 30 s
(tighter), 5 min (looser; rejected as too generous for an
ephemeral-token system). Yuka's call.

**Q8 (new in r2).** Should the proposal also recommend
opening a separate Wave A follow-up to remove
`#[derive(Debug)]` from `LowererSigningKey`? The Codex
reviewer flagged this as out-of-scope-for-B0-but-worth-doing.
My recommendation: **yes — open a separate notes-to-humans
issue** so it's tracked, but do not roll it into B0's scope
(B0 ships `philharmonic-policy 0.2.0`; the connector-client
fix would be a separate crate revision). The follow-up
note can land in the same commit as B0's Gate-1 sign-off
record.

## What lands (sub-phase B0)

Library source files (no code written yet):

- `philharmonic-policy/src/api_token.rs` — module containing
  the full B0 surface:
  - Types: `EphemeralApiTokenClaims` (with `iat` +
    `claims: CanonicalJson`), `ApiSigningKey` (no derived
    `Debug`, manual redacted impl), `ApiSignedToken`,
    `ApiVerifyingKeyEntry` (with `issuer`),
    `ApiVerifyingKeyRegistry`, `VerifyLimits`.
  - Functions: `mint_ephemeral_api_token`,
    `verify_ephemeral_api_token`,
    `verify_ephemeral_api_token_with_limits`.
  - Errors: `ApiTokenMintError` (variants
    `KidMismatch`, `KidProfileViolation`, `ClaimsTooLarge`,
    `LifetimeInvariantViolation`, `SerializationFailure`,
    `SigningFailure`), `ApiTokenVerifyError` (variants
    `TokenTooLarge`, `Malformed`, `HeaderProfileViolation`,
    `AlgorithmNotAllowed`, `KidProfileViolation`,
    `UnknownKid`, `KeyOutOfWindow`, `BadSignature`,
    `ClaimsNotCanonical`, `IssuerMismatch`,
    `KidInconsistent`, `LifetimeInvariantViolation`,
    `Expired`, `ClaimsTooLarge`), `RegistryInsertError`
    (variants `KidProfileViolation`, `DuplicateKid`).
  - Constants: `MAX_TOKEN_BYTES = 16 * 1024`,
    `MAX_INJECTED_CLAIMS_BYTES = 4 * 1024`,
    `MAX_TOKEN_LIFETIME_MILLIS = 24 * 60 * 60 * 1000`,
    `ALLOWED_CLOCK_SKEW_MILLIS = 60_000`,
    `KID_MIN_LEN = 1`, `KID_MAX_LEN = 128`.
- `philharmonic-policy/src/lib.rs` — re-export the new public
  surface from the new module.
- `philharmonic-policy/Cargo.toml` — version bump
  `0.1.0 → 0.2.0`, dep additions for `coset`, `ciborium`,
  `ed25519-dalek` (already transitive but now direct).
  `philharmonic-types ≥ 0.3.x` for `CanonicalJson`. Optional
  `proptest` as a dev-dep (Q4).
- `philharmonic-policy/CHANGELOG.md` — `[Unreleased]` entry
  describing the addition (the B0 module surface).
- `philharmonic-policy/tests/api_token_vectors.rs` — known-
  answer + 19 negative vectors per §"Test-vector plan".
- `philharmonic-policy/tests/vectors/api_token/*.hex /
  *.json` — committed reference vectors.
- `docs/notes-to-humans/<date>-<NNNN>-lowerer-signing-key-debug-derive-followup.md`
  — short Wave-A follow-up note opening the
  `LowererSigningKey: Debug` redaction work as a separate
  task. (Q8.)

What does **not** land in B0:

- Auth middleware in `philharmonic-api` — sub-phase B1.
- File-reading / KMS-fetching code for the API signing key —
  deployment / bin concern.
- Token minting endpoint — sub-phase G.
- `cargo publish` — Gate-2 review first; per Q6 the actual
  publish defers to Phase 8 close.

## Codex security review resolutions

Codex ran an independent design-level security review of r1
of this proposal. The full report is at
[`docs/codex-reports/2026-04-28-0001-phase-8-b0-ephemeral-api-token-primitives-security-review.md`](../../codex-reports/2026-04-28-0001-phase-8-b0-ephemeral-api-token-primitives-security-review.md).
Eleven findings; Claude's evaluation and the r2 resolution
per finding:

| #  | Finding | Severity | r2 resolution |
|----|---------|----------|---------------|
| 1  | B1 handoff omits the `authority.tenant == claims.tenant` binding; cross-tenant accept possible | HIGH | **Fixed.** §"Out of scope" §"B1 handoff contract" now lists this as a normative B1 check, placed immediately after authority lookup and before epoch acceptance. Required negative test enumerated. |
| 2  | No issuer/audience/key binding; `iss` is signed but unused | HIGH | **Fixed.** `ApiVerifyingKeyEntry` gains `issuer: String`. Verify step 11 enforces `claims.iss == entry.issuer`. New negative vector `api_issuer_mismatch.hex`. Audience claim deferred to v2; the registry being deployment-scoped acts as implicit deployment audience for v1. |
| 3  | Verifier accepts arbitrary far-future `exp`; no `iat` or max-age invariant | HIGH | **Fixed.** Added `iat: UnixMillis` to claims. Verify step 13 enforces `iat ≤ now + ALLOWED_CLOCK_SKEW_MILLIS`, `exp > now`, `exp > iat`, `exp - iat ≤ MAX_TOKEN_LIFETIME_MILLIS` (24 h, doc 11). Three new negative vectors: `api_iat_in_future.hex`, `api_exp_before_iat.hex`, `api_lifetime_too_long.hex`. |
| 4  | `ApiSigningKey` must not derive `Debug` | MEDIUM-HIGH | **Fixed.** Manual redacted `fmt::Debug` impl pinned in §"Minting side". Gate-2 will check this directly. Wave-A's existing `LowererSigningKey` derive is captured as a separate follow-up note (Q8). |
| 5  | CBOR determinism / `serde_json::Value` not pinned tightly enough | MEDIUM-HIGH | **Fixed.** `claims` field's type changes from `serde_json::Value` to `philharmonic_types::CanonicalJson` (RFC 8785 / JCS). Verify step 10 re-canonicalizes and byte-compares (`ClaimsNotCanonical`). New negative vector `api_claims_not_canonical.hex`. r1's Q2 collapses. |
| 6  | B0 verify should enforce the 4 KiB injected-claims size cap | MEDIUM | **Fixed.** Verify step 14 enforces `claims.bytes().len() ≤ MAX_INJECTED_CLAIMS_BYTES = 4096`. Mint also enforces (defense-in-depth). New negative vector `api_claims_too_large.hex`. `verify_ephemeral_api_token_with_limits` allows tighter (not looser) deployment overrides. |
| 7  | Replay threat model overstates what token leakage implies | MEDIUM | **Fixed.** §"Replay threat model" rewritten. The leaked-token replay risk for browser/partner systems is now named directly; mitigations are tied to short `exp`, instance-scope-as-default, permission clipping, authority-epoch bump, rate limiting, and deployment guidance. The "TLS-piercing access ⇔ minting compromise" claim is removed. |
| 8  | External auth error behavior unspecified | MEDIUM | **Fixed.** §"Out of scope" §"B1 handoff contract" now requires that all verify failures (B0 typed errors AND B1 substrate-state failures) collapse to a single external `401` with a generic `unauthenticated` body — no `kid`/window/signature/expiry leakage to external responses. Internal logs preserve typed variants. |
| 9  | Verify should reject non-empty unprotected and unexpected protected headers | LOW-MEDIUM | **Fixed.** New verify step 3 (strict header profile): protected may contain only `alg` + `kid`; unprotected must be empty; any `crit` or unknown protected header rejects. Three new negative vectors: `api_unprotected_nonempty.hex`, `api_protected_unknown_label.hex`, `api_protected_crit.hex`. |
| 10 | `kid` needs a length/character profile | LOW-MEDIUM | **Fixed.** `kid` profile pinned to `[A-Za-z0-9._:-]`, length 1..=128. Verify step 5 validates before registry lookup. Mint and `ApiVerifyingKeyRegistry::insert` also validate. Two new negative vectors: `api_kid_too_long.hex`, `api_kid_invalid_chars.hex`. |
| 11 | `authority_epoch` u64 wire vs i64 substrate; conversion undefined | LOW | **Fixed.** §"Out of scope" §"B1 handoff contract" pins the conversion: `u64::try_from(stored_i64)`; negative or out-of-range fails closed at lookup; B1 negative test required for negative substrate epoch. The wire type stays `u64` (avoids signing-side sign-extension footguns); B1 owns the boundary conversion. |

Eleven findings, eleven inline fixes. Five new constants
introduced (`MAX_TOKEN_BYTES`, `MAX_INJECTED_CLAIMS_BYTES`,
`MAX_TOKEN_LIFETIME_MILLIS`, `ALLOWED_CLOCK_SKEW_MILLIS`,
`KID_*_LEN`); six new error variants
(`HeaderProfileViolation`, `KidProfileViolation`,
`ClaimsNotCanonical`, `IssuerMismatch`,
`LifetimeInvariantViolation`, `ClaimsTooLarge`); eleven
additional negative vectors. Verify-order step count grew
from eight to fourteen.

The reviewer's "positive notes" (signature-first ordering,
protected-header `alg`/`kid`, library/bin split,
`MAX_TOKEN_BYTES` shape, vector planning) all carry through
into r2 unchanged.

## Next steps after Gate-1 sign-off

1. (If approved per Q6 (b)) sub-phase B0 Codex prompt drafted
   under `docs/codex-prompts/YYYY-MM-DD-NNNN-phase-8-b0-...md`,
   archived per the codex-prompt-archive skill, committed.
2. Codex dispatch via `codex:codex-rescue`. Round produces
   the source files + tests + vectors per §"What lands".
3. **Gate-2 code review** on the returned code before any
   merge into main. Code-line-level review of:
   - The mint and verify functions vs the r2 proposal text.
   - Each negative-vector test (19 of them) asserts the
     specific named `ApiTokenVerifyError` variant.
   - Zeroization wrappers appear at the seed and only at the
     seed; `ApiSigningKey` does not derive `Debug`; manual
     redacted impl prints only `kid` + a marker.
   - `claims: CanonicalJson` round-trips byte-stably; the
     re-canonicalization check at verify step 10 actually
     fires on a non-canonical input.
   - `iat`-aware lifetime checks use checked arithmetic; no
     `as` casts on the `i64`s.
   - No `unsafe` / `unwrap` / `expect` on reachable paths.
   - Strict header profile (verify step 3) rejects all three
     "extra metadata" shapes.
   - `kid` profile validator is consistent across mint, verify,
     and `ApiVerifyingKeyRegistry::insert`.
4. Sub-phase B1 prompt drafted, archived, dispatched. The B1
   prompt explicitly references the "B1 handoff contract" in
   this proposal and pins each item with tests (especially
   the authority-tenant binding negative test and the
   external-error-collapsing test).
5. (Optional, end of Phase 8) publish `philharmonic-policy
   0.2.0` and `philharmonic-api 0.1.0` together at sub-phase
   I.
