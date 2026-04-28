# Gate-1 proposal — Phase 8 sub-phase B0: ephemeral API token primitives

**Date:** 2026-04-28 (Tue)
**Revision:** 1 (first draft)
**Phase:** 8 (`philharmonic-api`), sub-phase B0 (primitives only;
B1 is the consumer middleware in `philharmonic-api`)
**Author:** Claude Code (on Yuka's review queue)
**Status:** **Awaiting Gate-1 sign-off**
**Approval record:** `docs/design/crypto-approvals/2026-04-28-phase-8-ephemeral-api-token-primitives.md` (will be created on sign-off)
**Blocker note that triggered this:** [`docs/notes-to-humans/2026-04-28-0003-ephemeral-token-primitives-gap.md`](../../notes-to-humans/2026-04-28-0003-ephemeral-token-primitives-gap.md)
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

These belong to later sub-phases or to deployment:

- **The auth middleware itself** — sub-phase B1, in
  `philharmonic-api`. B1 calls `verify_ephemeral_api_token`
  and the existing `parse_api_token` (for `pht_`), populates
  `RequestContext.auth`.
- **Authority lookup, authority-epoch enforcement,
  authority-retired check, tenant-suspended check** — B1 +
  later. These are substrate-state checks; they belong with
  the consumer that holds the substrate handle.
- **Tenant-scope enforcement** (claim's `tenant` matches the
  request's `RequestScope::Tenant(...)`) — sub-phase C
  (authz).
- **Instance-scope enforcement** (claim's `instance` vs the
  URL's instance) — sub-phase C / D.
- **Token minting endpoint** — sub-phase G. G uses the
  `mint_ephemeral_api_token` primitive.
- **Permission-clipping logic** at mint time — sub-phase G.
  The B0 mint primitive trusts its caller's claim values; the
  G consumer is responsible for clipping `permissions` to the
  authority's envelope and capping `claims` at 4 KB before
  calling the primitive.
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

Same as Wave A's Phase 5 replay decision (
[§"Replay threat model" in the Wave A proposal](2026-04-22-phase-5-wave-a-cose-sign1-tokens.md#replay-threat-model)):
the verify primitive is deliberately stateless. No `jti`, no
server-side replay cache. Mitigation rests on:

- **TLS** between the client and the API — assumed by
  deployment.
- **Tight `exp`** — ephemeral API tokens have at-most-24h
  natural expiry per
  [doc 11 §"Ephemeral API tokens"](../11-security-and-cryptography.md#ephemeral-api-tokens),
  often much shorter (per-minting-authority configurable).
- **Authority epoch bump** — for compromise response, not
  per-token replay defense, but it caps the blast radius.

The threat model carries through unchanged from Wave A. If a
token leaks to an attacker who has TLS-piercing access, they
also have the ability to mint new tokens. The sole new
realistic-replay window vs Wave A is "same token, different
endpoint within `exp`" — that's fine because the API enforces
permissions and instance-scope per-call. A leaked ephemeral
token replayed against an endpoint within its permission
envelope is doing exactly what the token was authorized to do.

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
`EphemeralApiTokenClaims`. Field set per
[`docs/design/09-policy-and-tenancy.md` §"Ephemeral token claims"](../09-policy-and-tenancy.md#ephemeral-token-claims):

```rust
pub struct EphemeralApiTokenClaims {
    pub iss: String,
    pub exp: UnixMillis,
    pub sub: String,
    pub tenant: Uuid,
    pub authority: Uuid,
    pub authority_epoch: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<Uuid>,
    pub permissions: Vec<String>,
    pub claims: serde_json::Value,
    pub kid: String,
}
```

CBOR encoding conventions match Wave A:
- `Uuid` (`tenant`, `authority`, `instance`) → 16-byte CBOR
  byte string.
- `UnixMillis` (`exp`) → CBOR unsigned integer for positive
  values (always positive post-epoch).
- `u64` (`authority_epoch`) → CBOR unsigned integer.
- `String` (`iss`, `sub`, `kid`, each entry of `permissions`)
  → CBOR text string.
- `Vec<String>` (`permissions`) → CBOR array of text strings.
- `serde_json::Value` (`claims`) → CBOR using `ciborium`'s
  serde integration. The encoder maps:
  `null → null`, `bool → bool`, `i64 / u64 → CBOR integer`,
  `f64 → CBOR float`, `String → text`, `Array → array`,
  `Object → map`. **No CBOR tags** are emitted (ciborium's
  default behavior). No binary-blob support — JSON-shape only,
  matching the doc 09 spec ("free-form, tenant-defined").

`#[serde(skip_serializing_if = "Option::is_none")]` on
`instance` keeps absent-instance tokens compact and matches
the doc 09 spec that the field is optional. The verify side
deserializes either shape.

The CBOR encoding is canonical per RFC 8949 §4.2 deterministic
encoding — `ciborium` produces deterministic output for
struct types built from the basic Rust types above. Open
question Q2 confirms that this is true for
`serde_json::Value` round-trips through ciborium.

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
checks run over verified claim bytes.

1. **Parse** the COSE_Sign1 bytes via
   `coset::CoseSign1::from_slice`. Malformed → reject
   (`ApiTokenVerifyError::Malformed`).

2. **Pin algorithm.** Read `alg` from the protected header;
   require `alg == -8` (EdDSA per RFC 9053 §2.2). Any other
   value → `ApiTokenVerifyError::AlgorithmNotAllowed`.

3. **Kid lookup.** Extract `kid` from the protected header;
   look up the verifier key in the
   `ApiVerifyingKeyRegistry`. Unknown kid →
   `ApiTokenVerifyError::UnknownKid`.

4. **Key validity window.** The registry entry carries
   `not_before` / `not_after` (`UnixMillis`). Reject if `now <
   not_before` or `now >= not_after`
   (`ApiTokenVerifyError::KeyOutOfWindow`).

5. **Signature verification.** Use
   `coset::CoseSign1::verify_signature` with the Ed25519
   verifying key from step 3. Bad signature →
   `ApiTokenVerifyError::BadSignature`. **No claim content is
   trusted before this step passes.**

6. **Claim payload decode.** Decode the claim payload from
   CBOR into `EphemeralApiTokenClaims`. Malformed → treat as
   `ApiTokenVerifyError::Malformed`.

7. **Kid consistency.** Require `claims.kid ==
   protected.kid`. Both are signature-covered, but duplication
   invites drift. Mismatch →
   `ApiTokenVerifyError::KidInconsistent`.

8. **Expiry.** Check `claims.exp > now`. Expired →
   `ApiTokenVerifyError::Expired { exp, now }`.

Only after all eight pass is `Ok(EphemeralApiTokenClaims)`
returned.

**Notably absent vs Wave A:**

- No `MAX_PAYLOAD_BYTES` ceiling. There is no auxiliary
  payload — only the COSE_Sign1 itself. We **do** add a
  ceiling on the COSE bytes themselves
  (`MAX_TOKEN_BYTES`, default 16 KiB) at step 0 before the
  parse to keep the SHA / parse work bounded, matching Wave
  A's defense-in-depth posture. Step 0 → `TokenTooLarge`.
- No `payload_hash` check. Wave A binds the encrypted payload
  via `payload_hash`; ephemeral API tokens have no auxiliary
  payload, so there's nothing to bind beyond the claim CBOR
  itself (which is signature-covered).
- No `realm` / audience binding. Ephemeral API tokens are
  scoped by `tenant` (and optionally `instance`), not by realm
  / service-side audience. Tenant-match enforcement happens at
  the consumer (sub-phase C) where the request's
  `RequestScope` is known.
- No authority-epoch / authority-retired / tenant-suspended
  enforcement. These are substrate-state checks done by the
  B1 consumer; the verify primitive returns the claims and
  lets B1 cross-check.

So the verify primitive is **pure**: only
signature-and-CBOR-and-time concerns. State concerns belong
to the consumer.

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
- Serializes the claims to CBOR via `ciborium`.
- Builds the COSE_Sign1 with `alg = -8` and `kid =
  claims.kid` in the protected header.
- Signs by reconstructing a transient
  `ed25519_dalek::SigningKey::from_bytes(&seed)` per call (Q1
  resolution from Wave A; carries through here).

The minting primitive does **not** enforce permission
envelopes or `claims` size caps. Sub-phase G (the minting
endpoint) is responsible for clipping `permissions` to the
authority's envelope and capping `claims` at 4 KB before
calling this primitive. Letting the primitive enforce those
would couple it to authority/tenant state it doesn't have.

### Verifying side

```rust
pub struct ApiVerifyingKeyEntry {
    pub vk: ed25519_dalek::VerifyingKey,
    pub not_before: UnixMillis,
    pub not_after: UnixMillis,
}

pub struct ApiVerifyingKeyRegistry {
    by_kid: HashMap<String, ApiVerifyingKeyEntry>,
}

impl ApiVerifyingKeyRegistry {
    pub fn new() -> Self;
    pub fn insert(&mut self, kid: String, entry: ApiVerifyingKeyEntry);
    pub fn lookup(&self, kid: &str) -> Option<&ApiVerifyingKeyEntry>;
}

pub fn verify_ephemeral_api_token(
    cose_bytes: &[u8],
    registry: &ApiVerifyingKeyRegistry,
    now: UnixMillis,
) -> Result<EphemeralApiTokenClaims, ApiTokenVerifyError>;
```

Public keys aren't sensitive; no zeroization. The library
ships no `load_from_file`, `load_from_toml`, etc. The
deployment binary parses whatever config format it chooses,
constructs `ApiVerifyingKeyEntry` values, and calls `insert`
per signing-key generation at boot. Same library/bin split
discipline as Wave A.

Rotation is additive: a new kid → new registry entry; old
kids stay registered until tokens issued under them have
expired. `not_before` / `not_after` are **enforced** at
verify step 4 — they're not advisory. Matches doc 11
§"Ephemeral API token signing key rotation" exactly.

### `kid` encoding

Free-form UTF-8 string, signed as part of the protected
header. Suggested format: `<api-issuer-slug>-<utc-date>-<rand-hex-8>`
(e.g. `api.tenant-2026-04-28-a1b2c3d4`). Not pinned as a wire
format — registry uses exact-string equality. Same pattern as
Wave A.

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
named.

- `api_token_too_large.hex` — COSE_Sign1 of `MAX_TOKEN_BYTES
  + 1` bytes (constructed by padding with garbage trailing
  bytes that still serialize as a valid CBOR major type).
  Step 0, `TokenTooLarge`.
- `api_bad_alg.hex` — same claims + key, protected header
  re-encoded with `alg = -7` (ES256). Step 2,
  `AlgorithmNotAllowed`.
- `api_unknown_kid.hex` — `kid` in the protected header
  replaced with a kid not in the registry. Step 3,
  `UnknownKid`.
- `api_key_out_of_window.hex` — valid token, registry entry
  has `not_after` in the past. Step 4, `KeyOutOfWindow`.
- `api_tampered_sig.hex` — last byte of the signature
  flipped. Step 5, `BadSignature`.
- `api_tampered_payload.hex` — one byte of the claim payload
  flipped. Step 5, `BadSignature`.
- `api_kid_inconsistent.hex` — protected header `kid` and
  `claims.kid` differ. Step 7, `KidInconsistent`.
- `api_expired.hex` — `exp` set to 1 (long in the past).
  Step 8, `Expired`.

Eight negative vectors. (No `payload_hash_mismatch` or
`realm_mismatch` — those checks don't exist in this
construction.)

### Round-trip + property tests (in addition)

- `mint(claims) → verify(...)` returns the same claims for
  both positive vectors. Asserts canonical encoding stability.
- A property-test loop over arbitrary `EphemeralApiTokenClaims`
  values (using `proptest` if Yuka approves the dep —
  otherwise a hand-written generator over a few dozen
  shapes): mint then verify must round-trip. This catches
  CBOR-encoding drift on `serde_json::Value` shapes the hand-
  written vectors don't exercise.

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

5. **`serde_json::Value` for `claims`** is the one design
   choice that's *not* identical to Wave A. Justification:
   (a) doc 09 explicitly says `claims` is "free-form,
   tenant-defined"; (b) `philharmonic-api/src/auth.rs:31`
   already types the runtime field as `serde_json::Value`,
   so matching the surface keeps the mint→sign→verify
   round-trip JSON-shape-stable; (c) ciborium's serde
   integration handles `serde_json::Value` shapes correctly
   without CBOR tag emission. The 4 KB cap is enforced by
   sub-phase G's minting endpoint, not by the primitive (per
   §"Out of scope").

## Open questions

Tagged for Yuka's call at sign-off.

**Q1.** Use `subtle = "2"` for the kid-equality check at
verify step 7? Wave A used `subtle` for the payload-hash
compare specifically because that compares
attacker-controlled bytes against
trusted bytes; here, both kids come from the same
signature-validated payload, so timing-side-channel risk is
nil. My recommendation: **plain `==`**, document the
reasoning in a code comment. Yuka's call.

**Q2.** Confirm `serde_json::Value`'s round-trip through
ciborium is deterministic for the JSON-equivalent shapes we
expect. The ciborium docs say "Yes for primitive types and
small structs"; for `serde_json::Value::Object` the field
order on serialization matters because CBOR maps preserve
insertion order while JSON objects don't. **My
recommendation: pin the on-the-wire ordering by serializing
through a sorted-key intermediate** (a small canonicalization
step in the mint primitive). Yuka's call on whether to do
this in B0 or defer to B1 / G when it matters for audit
binding.

**Q3.** pycose 2.x cross-check, same as Wave A's Q2? My
recommendation: **yes** — at minimum for the two positive
COSE_Sign1 hex vectors. Same script vehicle as Wave A's
vector generator (the workspace has experience with it).

**Q4.** `proptest` as a new dev-dep on
`philharmonic-policy`? Cooldown-checked, RustCrypto-style
code (it's QuickCheck-shaped, not crypto). **My
recommendation: yes** — it's the only practical way to
catch CBOR-shape edge cases in `serde_json::Value` (e.g.
deeply nested objects, negative integers in arrays, etc.).

**Q5.** Should the verify primitive also enforce a maximum
`exp - now` window? E.g., reject any token whose remaining
lifetime exceeds 24 h, on the principle that ephemeral
tokens shouldn't ever be that long-lived. My recommendation:
**no — leave the lifetime ceiling to sub-phase G's mint
endpoint** (it knows the per-authority configured max).
Adding the check at verify time would couple the primitive
to the deployment-configured policy. Yuka's call.

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

## What lands (sub-phase B0)

Library source files (no code written yet):

- `philharmonic-policy/src/api_token.rs` — module containing
  `EphemeralApiTokenClaims`, `ApiSigningKey`,
  `ApiSignedToken`, `ApiVerifyingKeyEntry`,
  `ApiVerifyingKeyRegistry`,
  `mint_ephemeral_api_token`, `verify_ephemeral_api_token`,
  `ApiTokenMintError`, `ApiTokenVerifyError`,
  `MAX_TOKEN_BYTES`.
- `philharmonic-policy/src/lib.rs` — re-export the new public
  surface from the new module.
- `philharmonic-policy/Cargo.toml` — version bump
  `0.1.0 → 0.2.0`, dep additions for `coset`, `ciborium`,
  `ed25519-dalek` (already transitive but now direct), and
  optionally `proptest` as a dev-dep.
- `philharmonic-policy/CHANGELOG.md` — `[Unreleased]` entry
  describing the addition.
- `philharmonic-policy/tests/api_token_vectors.rs` — known-
  answer + negative vectors per §"Test-vector plan".
- `philharmonic-policy/tests/vectors/api_token/*.hex /
  *.json` — committed reference vectors.

What does **not** land in B0:

- Auth middleware in `philharmonic-api` — sub-phase B1.
- File-reading / KMS-fetching code for the API signing key —
  deployment / bin concern.
- Token minting endpoint — sub-phase G.
- `cargo publish` — Gate-2 review first; per Q6 the actual
  publish defers to Phase 8 close.

## Codex security review (optional, recommended)

Wave A ran an independent Codex security review of r1 of the
proposal before Yuka's Gate-1 sign-off; seven findings
surfaced and six landed inline as r2 fixes. The pattern
worked well there. **Recommendation:** run the same kind of
review on this proposal (r1) before sign-off, archived under
`docs/codex-reports/2026-04-28-NNNN-phase-8-b0-ephemeral-api-token-primitives-security-review.md`.

The proposal text above is small enough that a focused review
should add ≤ ~30 minutes of Codex time + Yuka's review of
the report. Catches things like: "did I miss algorithm
pinning in step N", "should I require min-`exp` window?",
"does the kid registry need `not_before` semantics that
match Wave A?", etc.

If Yuka prefers to skip and sign off directly on r1, that's
also fine — the proposal is closely modeled on Wave A's
already-reviewed structure.

## Next steps after Gate-1 sign-off

1. (If approved per Q6 (b)) sub-phase B0 Codex prompt drafted
   under `docs/codex-prompts/YYYY-MM-DD-NNNN-phase-8-b0-...md`,
   archived per the codex-prompt-archive skill, committed.
2. Codex dispatch via `codex:codex-rescue`. Round produces
   the source files + tests + vectors per §"What lands".
3. **Gate-2 code review** on the returned code before any
   merge into main. Code-line-level review of:
   - The mint and verify functions vs the proposal.
   - Each negative-vector test asserts the named
     `ApiTokenVerifyError` variant.
   - Zeroization wrappers appear at the seed and only at the
     seed.
   - No `unsafe` / `unwrap` / `expect` on reachable paths.
   - The CBOR serialization for `serde_json::Value` produces
     the canonical shape we expected (re-confirm Q2's
     resolution against the actual hex bytes).
4. Sub-phase B1 prompt drafted, archived, dispatched.
5. (Optional, end of Phase 8) publish `philharmonic-policy
   0.2.0` and `philharmonic-api 0.1.0` together at sub-phase
   I.
