# Gate-1 proposal — Phase 5 Wave B: hybrid KEM + COSE_Encrypt0 payload encryption

**Date:** 2026-04-22
**Revision:** 3 (Gate-1 approved; Q answers folded in)
**Phase:** 5 (connector triangle), Wave B (encryption half)
**Author:** Claude Code (on Yuka's review queue)
**Status:** **Gate-1 approved 2026-04-22** — implementation unblocked
**Approval record:** `docs/design/crypto-approvals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md`
**Prereq (landed):** Wave A COSE_Sign1
  (`docs/design/crypto-proposals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md`,
  approved
  `docs/design/crypto-approvals/2026-04-22-phase-5-wave-a-cose-sign1-tokens-01.md`).
**Wave split decision:** ROADMAP §Phase 5 (commit `e292918`).
**Security review:**
  `docs/codex-reports/2026-04-22-0005-phase-5-wave-b-hybrid-kem-cose-encrypt0-security-review.md`.
  Resolutions are summarized in §"Codex security review resolutions"
  at the end of this document; the body has been updated inline.

## Revisions

- **r1** (2026-04-22): first draft. Writing before bed — expect
  Yuka to flag things. Open Questions are called out explicitly
  at the bottom and are load-bearing.
- **r2** (2026-04-22): applied Codex's independent security
  review. Three HIGH and three MEDIUM findings triaged:
  four fixes landed inline, two concerns (replay CPU-amplification
  and error-variant oracle) resolved via service-bin-layer
  requirements + library-internal-vs-external-surface
  documentation rather than library-API changes.
  Summary table at §"Codex security review resolutions".
- **r3** (2026-04-22): **Gate-1 approved by Yuka**
  (`docs/design/crypto-approvals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md`).
  All eight Open Questions answered; each answer confirmed
  Claude's proposed lean. Specifically:
  - Q#1 HKDF IKM order → `kem_ss || ecdh_ss` (ML-KEM first).
  - Q#2 ML-KEM sk zeroization → `Zeroizing<[u8; 2400]>` + per-decrypt reconstruction.
  - Q#3 X25519 ephemeral source → `OsRng` in public API + test-only override.
  - Q#4 AAD content → SHA-256 of canonical CBOR map of `(realm, tenant, inst, step, config_uuid, kid)`.
  - Q#5 AEAD nonce → random 12 bytes from `OsRng`.
  - Q#6 `rand_core` version pin → acknowledged; defer to implementation time, flag if it fractures.
  - Q#7 COSE alg id → `alg = 3` (A256GCM) + custom text-keyed headers for `kem_ct` / `ecdh_eph_pk`.
  - Q#8 distinguish AEAD / AAD / kem-ct failures → document-only, all lumped to `DecryptionFailed`.
  The body of this document already captured all eight leans
  at r2; r3 is a status / provenance bump rather than a design
  change. No further revisions expected before Codex dispatch.

## Scope

Wave B lands the encryption half of the connector triangle:
hybrid ML-KEM-768 + X25519 KEM, HKDF-SHA256 derivation,
AES-256-GCM content encryption, packaged as `COSE_Encrypt0`.
It composes with Wave A's `COSE_Sign1` token such that the token
commits (via `payload_hash` = SHA-256 of the COSE_Encrypt0
bytes) to a specific ciphertext, and the ciphertext's AEAD
associated-data binds back to the token's call-context fields.

Wave B is where the roadmap's "crates publish as 0.1.0" gate
unlocks for the three triangle crates
(`philharmonic-connector-client`, `philharmonic-connector-service`,
`philharmonic-connector-router`). That happens only after a
green end-to-end test mints + encrypts + transmits + verifies +
decrypts a known-answer plaintext.

### In scope (Wave B)

**New crypto construction (the reviewable surface):**

- Hybrid KEM: ML-KEM-768 encapsulate against a per-realm public
  key, in parallel with X25519 ECDH against a per-realm X25519
  public key. Two shared secrets → HKDF-SHA256 → AES-256-GCM
  key.
- COSE_Encrypt0 envelope carrying the two KEM outputs
  (ML-KEM ciphertext + ephemeral X25519 public key) plus the
  AEAD ciphertext + tag.
- AEAD associated data bound to the call-context claim fields
  so that a valid ciphertext cannot be combined with a token
  for a different `(tenant, inst, step, config_uuid, realm,
  kid)` tuple.
- Realm-key registry on the service side (private ML-KEM +
  X25519 keypairs, by `kid`, with validity windows).
- 12-step verification+decryption order composing Wave A's
  11-step token verification with the 12th step (decrypt) and
  13th step (inner-realm belt-and-suspenders check).

**Upstream crate updates (forced by the `issued_at` follow-up):**

- `philharmonic-connector-common 0.2.0` adds an `iat: UnixMillis`
  claim to `ConnectorTokenClaims`. Yuka's Gate-2 approval on
  Wave A chose option **(A) later**; Wave B is the "later."
- Wave A reference vectors at `tests/crypto-vectors/wave-a/` get
  regenerated because the CBOR claim-map layout changes (9 →
  10 entries).
- Wave A's `verify_token` reads `claims.iat` for
  `ConnectorCallContext.issued_at` instead of `now`.

**Tests (both crates + router):**

- Known-answer hybrid-KEM vectors: fixed ML-KEM-768 keypair,
  fixed X25519 keypair, fixed ephemeral randomness, fixed
  plaintext → fixed ML-KEM ciphertext bytes, fixed ECDH shared
  secret, fixed HKDF output, fixed AEAD ciphertext.
- Known-answer COSE_Encrypt0 vectors: the KEM known-answers
  rolled up into the full envelope bytes.
- Wave-A × Wave-B composition vectors: the Wave A positive
  vector's `payload_hash` equals SHA-256 of the Wave B
  COSE_Encrypt0 bytes (i.e. regenerated Wave A vectors now
  point at a real Wave B payload instead of an arbitrary
  27-byte string).
- End-to-end integration test: lowerer-side `mint +
  encrypt → service-side decrypt + verify` round-trip with
  a known plaintext; asserts decrypted plaintext bytes equal
  the input.
- Negative vectors per rejection path (see §Negative-path
  vectors below).

**Router (minimal):**

- `philharmonic-connector-router` gets enough plumbing to
  forward an HTTP request carrying `Authorization: Bearer
  <token>` and `X-Encrypted-Payload: <payload>` to a
  connector-service instance. Pure dispatcher; no crypto.
- Not the focus of Gate-1 / Gate-2. Reviewed at the module-
  boundary level only.

### Out of scope (Wave B)

- **SCK encrypt / decrypt.** Already shipped in
  `philharmonic-policy 0.1.0`; re-used via SCK decrypt at step
  2 of the lowerer pipeline. Not re-reviewed.
- **Concrete connector implementations.** `http_forward`,
  `llm_openai_compat`, etc. are Phase 6.
- **End-to-end with a real remote upstream** (e.g. making an
  actual LLM call). The integration test terminates at the
  service-side dispatch stub; `execute()` is a
  known-plaintext-equals-input assertion.
- **Realm-key lifecycle** in production sense (HSM,
  auto-rotation, etc). Wave B lands the registry API; ops
  concerns are Phase 8 / v1 deployment scope.

## Replay threat model

Inherited from Wave A (§"Replay threat model" in
`docs/design/crypto-proposals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md`).
Wave B does not change it. Encryption adds confidentiality, not
replay resistance; a captured `(token, ciphertext)` pair can
still be replayed inside the token's `exp` window if an
attacker has wire access. All the Wave A mitigations (TLS on
every leg, tight `exp`, protocol-layer idempotency as the impl's
responsibility) apply unchanged.

New threats introduced by encryption:

1. **Ciphertext substitution with a valid token.** Mitigated by
   `claims.payload_hash = SHA-256(COSE_Encrypt0 bytes)` (Wave
   A) AND by the AEAD associated-data binding (Wave B, see
   §"AEAD associated data" below). Two independent checks; a
   failure in either rejects.
2. **Token substitution with a valid ciphertext.** Mitigated
   by the AAD binding: the AEAD decryption fails if the
   caller-provided token doesn't match the AAD the ciphertext
   was encrypted under.
3. **KEM harvest-now-decrypt-later** against a future quantum
   computer. This is the main reason ML-KEM-768 is in the
   hybrid and is specifically called out at
   `docs/design/11-security-and-cryptography.md` §Threat model
   §Harvest-now-decrypt-later. X25519 alone would be
   vulnerable; hybrid requires breaking both.
4. **KEM-only or ECDH-only downgrade.** The HKDF combines both
   shared secrets; there's no single-primitive fallback. Any
   middlebox attempt to strip one half fails AEAD decryption.
5. **Replay-driven CPU exhaustion.** Wave B's decrypt path is
   significantly more expensive per call than Wave A's signature
   check: each accepted token + ciphertext pair triggers an
   ML-KEM-768 decapsulation, an X25519 DH, an HKDF, and an
   AES-256-GCM decrypt. A captured valid `(token, payload)`
   pair can be replayed within `exp` to force that work
   repeatedly. Codex's security review (finding #4) flagged
   this as a material uplift from Wave A's replay acceptance.
   The library does not add a replay cache (stays stateless
   per Wave A); the mitigation is on the **service bin**:
   a) per-tenant and per-instance rate limits on decrypt
   attempts, b) per-source-IP rate limits at the router, and
   c) an early `404` / `429` before the decrypt pipeline for
   requests over a budget. These requirements are normative
   for the bin-layer implementation and will be echoed in the
   service bin's module docs and in
   `docs/design/08-connector-architecture.md`'s router
   section when that content lands.

Out of Wave B's threat-mitigation scope (explicit):

- **Side-channel attacks** (timing, power) against
  `ml-kem` / `x25519-dalek` / `aes-gcm` internals. We trust the
  RustCrypto implementations; custom side-channel hardening is
  not a v1 deliverable.
- **Post-compromise recovery.** A compromised realm private
  key means all past AND future ciphertexts encrypted to that
  kid are decryptable until rotation. This is the documented
  blast radius; the ops mitigation is fast kid rotation. No
  forward secrecy in the KEM output is added in v1.

## Primitives and library versions

All RustCrypto. Versions verified against crates.io on
2026-04-22 via `./scripts/xtask.sh crates-io-versions`:

- **`ml-kem = "0.2"`** — latest `0.2.3`. NIST ML-KEM-768
  (FIPS 203). The crate exposes `MlKem768` with `encapsulate`
  and `decapsulate`; we use the defaults and wrap a thin
  helper.
- **`x25519-dalek = "2"`** — latest `2.0.1`. Classical ECDH.
  No feature flags beyond defaults; `static_secrets` for the
  per-realm long-lived private key, ephemeral per-encryption
  public for the lowerer side.
- **`aes-gcm = "0.10"`** — latest `0.10.3`. AES-256-GCM AEAD.
- **`hkdf = "0.12"`** — latest `0.13.0` (new major; check
  compatibility before pinning to `"0.13"`). The HKDF-SHA256
  extract-and-expand is a thin wrapper; nothing exotic.
- **`sha2 = "0.11"`** — already in
  `philharmonic-connector-service`. Used for the HKDF
  instantiation and for the payload hash.
- **`zeroize = { version = "1", features = ["derive"] }`** —
  already in `philharmonic-connector-client`.
- **`secrecy = "0.10"`** — latest `0.10.3`. `SecretBox<T>` for
  the KEM-derived AEAD key. Wave A didn't need this (one
  32-byte seed sufficed); Wave B has multiple distinct key
  materials (ML-KEM SS, ECDH SS, HKDF output, AEAD key) each
  needing explicit drop-zero.
- **`rand_core = "0.6"`** — already transitive via
  `ed25519-dalek` and `x25519-dalek`. Verify the pin matches
  what `ml-kem 0.2` expects; `rand_core` had a 0.6 → 0.9
  bump recently and the ecosystem is mid-migration. **Open
  Question #6 below.**

Already-pinned from Wave A (no change):

- `ed25519-dalek = "2"`, `coset = "0.4"`, `ciborium = "0.2"`,
  `subtle = "2"`.

No new primitives. No custom MAC / KDF / AEAD. No `unsafe` in
any code we write.

## Construction

### Hybrid KEM

Notation:

- `kem_pk`, `kem_sk`: ML-KEM-768 public / private key (realm
  long-lived).
- `ecdh_pk`, `ecdh_sk`: X25519 public / private key (realm
  long-lived).
- `ecdh_eph_pk`, `ecdh_eph_sk`: X25519 ephemeral keypair
  (per-encryption, lowerer side).
- `kem_ct`: ML-KEM-768 ciphertext (1088 bytes per FIPS 203 for
  the 768 parameter set).
- `kem_ss`: ML-KEM-768 shared secret (32 bytes).
- `ecdh_ss`: X25519 shared secret (32 bytes).
- `ikm`: HKDF input keying material.
- `prk`: HKDF pseudorandom key.
- `aead_key`: 32-byte AES-256-GCM key (HKDF expand output).

**Encapsulate (lowerer side):**

```
(kem_ct, kem_ss)  = MLKEM768::encapsulate(kem_pk)
(ecdh_eph_pk, ecdh_eph_sk) = X25519::generate()
ecdh_ss           = X25519::diffie_hellman(ecdh_eph_sk, ecdh_pk)
```

**Decapsulate (service side):**

```
kem_ss  = MLKEM768::decapsulate(kem_sk, kem_ct)
ecdh_ss = X25519::diffie_hellman(ecdh_sk, ecdh_eph_pk)
```

### HKDF-SHA256

```
ikm  = kem_ss || ecdh_ss          (64 bytes; ML-KEM-first)
salt = b""                          (empty)
info = b"philharmonic/wave-b/hybrid-kem/v1/aead-key"
prk  = HKDF-Extract(salt, ikm)       (SHA-256, 32 bytes)
aead_key = HKDF-Expand(prk, info, 32 bytes)
```

- **IKM ordering**: ML-KEM shared secret first, then X25519.
  Rationale: keeps the PQ half at the highest-entropy position
  by convention; matches the order NIST's draft hybrid
  guidance has floated. **Open Question #1 — confirm or flip.**
- **Salt**: empty. No pre-shared entropy. Standard HPKE-style.
- **Info**: versioned domain-separation string. The `v1`
  suffix is deliberate: if we ever change anything about the
  KEM or AEAD construction, bumping to `v2` in the info string
  means old ciphertexts fail to decrypt cleanly instead of
  silently misbehaving. Also unambiguously distinguishes these
  keys from any future use of HKDF in the project (e.g.
  ephemeral API tokens).

`aead_key` is wrapped in `Zeroizing<[u8; 32]>` /
`SecretBox<[u8; 32]>` immediately and dropped when the AEAD
operation returns.

### AEAD (AES-256-GCM)

- **Key**: `aead_key` (above). 256-bit.
- **Nonce**: 12 bytes. **Randomly generated per encryption**
  from `OsRng`. Never reused. Included in the COSE_Encrypt0
  protected header so the service can recover it. Under
  random-nonce semantics the probability of reuse at our
  expected volume (one encryption per connector call, bounded
  by step-sequence uniqueness) is negligible.
  **Open Question #5 below — deterministic derivation vs
  random is the canonical AES-GCM tradeoff; random is my
  lean.**
- **Plaintext**: the decrypted `TenantEndpointConfig` payload
  bytes (the full admin-submitted config JSON, byte-identical
  as the lowerer received them after SCK decryption).
- **Associated data** (AAD): see §AEAD associated data below.
- **Ciphertext + tag**: 16-byte GCM tag appended.

### AEAD associated data

Per RFC 9052 §5.3, the AEAD's associated-data input for
COSE_Encrypt0 is the canonical CBOR of `Enc_structure`:

```
Enc_structure = [
    context: "Encrypt0",
    protected: bstr  (serialized protected-header bucket),
    external_aad: bstr,
]
```

We use the standard structure exactly. That covers the
protected header (alg, kid, IV, kem_ct, ecdh_eph_pk) under
the AEAD tag automatically — an attacker flipping a byte in
any header field causes tag verification to fail.

`external_aad` carries the **call-context binding digest**:
the value binds the ciphertext to the Wave-A-verified claim
fields so that valid ciphertexts can't be rehomed to tokens
for a different `(realm, tenant, inst, step, config_uuid, kid)`
tuple. Per-call context comes from claim fields available
**at encrypt-time** (i.e. NOT `payload_hash`, which would be
circular, and NOT `iss`/`exp`/`iat` which are not per-call
identifiers):

```
external_aad = SHA-256( CBOR_canonical({
    "realm":       claims.realm,
    "tenant":      claims.tenant,     (16-byte bstr)
    "inst":        claims.inst,       (16-byte bstr)
    "step":        claims.step,
    "config_uuid": claims.config_uuid,
    "kid":         claims.kid,
}) )
```

- 32 bytes (one SHA-256 digest).
- **Lowerer side** computes `external_aad` before encryption,
  using the values it's about to place in the token claims.
  `coset::CoseEncrypt0Builder::create_ciphertext(aad_buf,
  plaintext, ...)` takes `external_aad` and handles the
  `Enc_structure` serialization internally.
- **Service side** computes the same `external_aad` after
  Wave A token verification succeeds (so claim fields are
  verified-trusted). `coset::CoseEncrypt0::decrypt(aad_buf,
  ...)` rebuilds the `Enc_structure` and passes the full
  canonical CBOR as AEAD associated-data.

The "AAD binding to the token" phrasing in
`docs/design/11-security-and-cryptography.md` is satisfied by
this `external_aad` — the digest bridges the COSE_Encrypt0
to the COSE_Sign1's claim set.

**Open Question #4 below — `external_aad` field set and
encoding.** I've proposed SHA-256 of a canonical CBOR map to
get a fixed-width digest with unambiguous field inclusion.
Alternatives: raw concatenation of fields (length-prefixed),
or passing the claim-map canonical CBOR directly (variable
width; fine for AEAD). Yuka's call.

### COSE_Encrypt0 envelope

Structure per RFC 9052 §5.2:

```
COSE_Encrypt0 = [
    protected:   bstr (serialized map below),
    unprotected: map {},
    ciphertext:  bstr (AES-256-GCM output, ciphertext || tag),
]
```

Protected-header map:

| Label | Value | Meaning |
|-------|-------|---------|
| `1` (alg) | `3` (A256GCM per RFC 9053 §4.2.1) | content encryption |
| `4` (kid) | utf-8 bytes of `realm_kid` | selects the realm key pair |
| `5` (IV) | 12 bytes | AES-GCM nonce |
| `"kem_ct"` (text key) | bstr, 1088 bytes | ML-KEM-768 ciphertext |
| `"ecdh_eph_pk"` (text key) | bstr, 32 bytes | ephemeral X25519 public |

- `alg = 3` pins the **content** algorithm to A256GCM. The
  hybrid KEM is not an "algorithm" in the COSE registry; it's
  reflected by the presence of the custom `kem_ct` +
  `ecdh_eph_pk` labels. This is a pragmatic choice because
  COSE doesn't have a standard ML-KEM hybrid registration yet
  (the IETF `cose-pqc` draft is mid-flight). **Open Question
  #7 below.**
- Text-keyed headers are legal per RFC 9052 §1.4; we pick
  descriptive names rather than collide with the integer
  label space.
- `unprotected` header is empty. Everything security-relevant
  is signature-covered by Wave A (via `payload_hash` committing
  to the full COSE_Encrypt0 bytes).

### Lowerer-side pipeline (mint + encrypt)

From `ConnectorCallContext` pre-claims construction to final
`(token_bytes, payload_bytes)` pair ready for HTTP:

1. Fetch `TenantEndpointConfig` by UUID; SCK-decrypt (Phase 2
   code, already shipped).
2. Parse the decrypted JSON; read `realm`; keep everything
   else opaque.
3. Look up `(kem_pk, ecdh_pk, kem_kid)` for the destination
   realm in the `RealmRegistry` from `philharmonic-connector-
   common 0.2.0`.
4. Hybrid-KEM encapsulate: produces `kem_ct`, `kem_ss`,
   `ecdh_eph_pk`, `ecdh_ss`.
5. HKDF → `aead_key`.
6. Assemble AAD from call-context fields.
7. AES-256-GCM encrypt (random 12-byte nonce, AAD, SCK-
   plaintext). Produces `ciphertext || tag`.
8. Build COSE_Encrypt0 envelope bytes.
9. `payload_hash = SHA-256(cose_encrypt0_bytes)`.
10. Build `ConnectorTokenClaims` (Wave A, with `iat = now`
    from the 0.2.0 claim-set) and pass to
    `LowererSigningKey::mint_token` (Wave A, unchanged).
11. Return `(token_bytes, cose_encrypt0_bytes)`.

Zeroize between steps: `kem_ss`, `ecdh_ss`, the HKDF PRK, and
`aead_key` are all in `SecretBox` / `Zeroizing` for the
minimum lifetime needed.

### Service-side pipeline (verify + decrypt)

Extends Wave A's 11-step order with five more steps. All 11
Wave A steps run unchanged and short-circuit first; Wave B
only runs if the token is fully valid.

| Step | Action | Reject with |
|------|--------|-------------|
| 1–11 | Wave A token verify | (Wave A variants) |
| 12 | Parse `COSE_Encrypt0(payload_bytes)` | `EncryptedPayloadMalformed` |
| 12a | Protected-header strict validation (see below) | `EncryptedPayloadMalformed` |
| 13 | Read protected-header kid; look up in `RealmPrivateKeyRegistry`; window-check; check entry's `realm == service_realm` | `UnknownRealmKid` / `RealmKeyOutOfWindow` / `RealmKeyRealmMismatch` |
| 14 | Hybrid-KEM decapsulate; HKDF; compute `external_aad` from Wave-A-verified claims; AES-256-GCM decrypt (AEAD AAD = `Enc_structure` per RFC 9052) | `DecryptionFailed` |
| 15 | Parse decrypted plaintext; assert inner `realm` field equals `claims.realm` | `InnerRealmMismatch` |

Only after step 15 passes is the decrypted plaintext handed to
`impl` dispatch.

**Step 12a: strict protected-header validation.** Explicitly
required before any expensive crypto. Each condition rejects
with `EncryptedPayloadMalformed`:

- `alg == 3` exactly (A256GCM). No other value — not even other
  AEADs we might later support, without a version bump.
- `unprotected` map is empty. Any entry is rejected.
- Required labels present exactly once: `1` (alg), `4` (kid),
  `5` (IV), `"kem_ct"`, `"ecdh_eph_pk"`.
- Duplicate labels across the map are rejected (including
  duplicate integer labels and duplicate text labels).
- Unknown labels are rejected. No forward-compat tolerance at
  this layer; forward compat is handled by protocol version
  bumps.
- Exact byte-length bounds:
  - `kid`: 1 ≤ length ≤ 255 (sanity bound; real kids are
    ~40 chars).
  - `IV`: exactly 12 bytes.
  - `kem_ct`: exactly 1088 bytes (FIPS 203 ML-KEM-768
    ciphertext size).
  - `ecdh_eph_pk`: exactly 32 bytes.
- Ciphertext body: bounded by Wave A's `MAX_PAYLOAD_BYTES`
  (1 MiB default, enforced at Wave A step 5 — which checks
  the outer COSE_Encrypt0 byte length). No separate
  Wave-B-only ciphertext limit.

**Step 13: realm-kid lookup with realm check.** Per Codex's
review, the registry entry carries a `realm: RealmId` field
so that even if the wrong realm's key entries end up in a
service's registry (operator misconfig / key-distribution
mistake), the service refuses to decrypt:

```rust
let entry = registry.lookup(protected_kid)
    .ok_or(TokenVerifyError::UnknownRealmKid { kid })?;

if now < entry.not_before || now >= entry.not_after {
    return Err(TokenVerifyError::RealmKeyOutOfWindow { ... });
}
if entry.realm.as_str() != service_realm {
    return Err(TokenVerifyError::RealmKeyRealmMismatch { ... });
}
```

In v1 each service instance is for exactly one realm, so the
registry is homogeneous and this check is belt-and-suspenders.
For a hypothetical multi-realm service in the future, this
check is a prerequisite — it keeps realm-kid ownership
auditable at the library layer.

**Step 14 is the only cryptographic failure point in Wave B's
decrypt path.** The AEAD check, the tag verification, and the
AAD binding all fail into the same `DecryptionFailed` bucket —
they're indistinguishable to an attacker (constant-time-ish),
and distinguishing them server-side would only help with
debugging rather than security. I'm not proposing sub-variants
for "which part of decryption failed." **Open Question #8 —
whether the AEAD / AAD mismatch should be distinguishable for
operator troubleshooting, or lumped.**

### Internal vs external error surface

The library returns fine-grained `TokenVerifyError` variants
for every rejection path because tests and operator logs need
to tell them apart. The **service bin** (which consumes the
library and surfaces HTTP) SHOULD collapse all steps-12-to-15
failures into a **single generic HTTP error** (e.g.
`400 Bad Request` with `{ "error": "decryption_failed" }`) plus
a best-effort uniform response latency. This prevents an
attacker from using API-surface error variants to probe the
realm-kid namespace or key-window state.

This requirement is for the service bin, not the Wave B
library. The library exposes what happened so the bin can log
it; the bin decides what the external response looks like.
Documented here (and in the service bin's README / module
docs at implementation time) because otherwise the two crates'
behavior can diverge under independent development. See
§"Codex security review resolutions" finding #2.

The two kids are separate by design. In the claim set,
`claims.kid` identifies the **lowerer signing key** used to
mint the token (Wave A). In the COSE_Encrypt0 protected
header, `kid` identifies the **realm KEM + X25519 keypair**
used to encrypt the payload (Wave B). The two refer to
different key materials on different sides of the triangle;
they are expected to differ, and no `protected_kid ==
claims.kid` check is meaningful.

### Key management

Both sides get bytes-only library APIs (per workspace
convention §Library crate boundaries):

**Lowerer side** — `philharmonic-connector-client` exposes a
`RealmPublicKeyRegistry` (new in 0.2.0 — today's
`RealmRegistry` in `philharmonic-connector-common` stays where
it is and is re-exported / re-used). The lowerer bin reads
the realm-key distribution config (TOML / JSON / KMS) and
populates the registry programmatically, same pattern as
Wave A's `MintingKeyRegistry`.

**Service side** — `philharmonic-connector-service` exposes a
`RealmPrivateKeyRegistry`:

```rust
pub struct RealmPrivateKeyEntry {
    pub kem_sk: Zeroizing<[u8; 2400]>,   // ML-KEM-768 sk size per FIPS 203
    pub ecdh_sk: x25519_dalek::StaticSecret,
    pub realm: RealmId,                   // belt-and-suspenders per Codex r2 review
    pub not_before: UnixMillis,
    pub not_after: UnixMillis,
}

pub struct RealmPrivateKeyRegistry {
    by_kid: HashMap<String, RealmPrivateKeyEntry>,
}

impl RealmPrivateKeyRegistry {
    pub fn new() -> Self;
    pub fn insert(&mut self, kid: String, entry: RealmPrivateKeyEntry);
    pub fn lookup(&self, kid: &str) -> Option<&RealmPrivateKeyEntry>;
}
```

Private-key material is `Zeroizing`-wrapped (ML-KEM) or
wrapped by a type that zeroizes on drop (X25519's
`StaticSecret` does this natively per
`x25519-dalek 2.x`). The service bin reads private keys from
deployment secret storage, populates the registry, drops the
buffers.

**Why `realm` on the entry.** v1 deploys each service for one
realm, so `registry.lookup(kid)` uniqueness by `kid` alone is
sufficient in the common case. The per-entry `realm` field is
there for two reasons: (1) defensive against operator
misconfig where the wrong realm's key distribution lands in a
service's secret-store path, and (2) forward-looking toward
multi-realm deployments (not scoped for v1) where a single
registry would need `(realm, kid)` disambiguation. Step 13's
`entry.realm == service_realm` check is cheap and makes the
ownership explicit. See §"Codex security review resolutions"
finding #3.

### Zeroization points

- `kem_ss: Zeroizing<[u8; 32]>` — dropped after HKDF-extract.
- `ecdh_ss: Zeroizing<[u8; 32]>` — dropped after HKDF-extract.
- `prk: Zeroizing<[u8; 32]>` — dropped after HKDF-expand.
- `aead_key: SecretBox<[u8; 32]>` — lives only inside the
  encrypt / decrypt function stack frame.
- `kem_sk: Zeroizing<[u8; 2400]>` — owned by
  `RealmPrivateKeyEntry`; dropped when the entry is removed
  from the registry or the registry itself drops.
- `ecdh_sk: StaticSecret` — `x25519-dalek`'s type
  zeroizes-on-drop natively.
- Ephemeral `ecdh_eph_sk: EphemeralSecret` — consumed by the
  first `.diffie_hellman()` call; RustCrypto's design
  guarantees it can't be accidentally reused.

No key material should ever flow through a `Vec<u8>` that isn't
wrapped; if the implementation needs a `Vec` transiently
(e.g. a parse buffer), it goes into `Zeroizing::new(buf)` and
drops at the earliest possible scope.

## Test-vector plan

### Hybrid-KEM layer (pre-COSE)

- Fixed ML-KEM-768 keypair — generated with a committed
  deterministic seed and cross-checked against the `ml-kem`
  crate's own test vectors if available, else against a
  Python reference (the `mlkem-py` package or
  `cryptography`'s post-quantum support when it lands).
- Fixed X25519 keypair — the RFC 7748 §6.1 test vector (the
  same public one this project has used for X25519
  cross-checks elsewhere).
- Fixed ephemeral X25519 keypair (for the lowerer side of
  the encapsulation) — committed seed.
- Fixed ML-KEM encapsulation randomness → fixed `kem_ct` and
  `kem_ss`. (ML-KEM's encapsulate is deterministic given
  randomness.)
- Expected bytes committed at
  `tests/crypto-vectors/wave-b/wave_b_kem_ct.hex`,
  `wave_b_kem_ss.hex`, `wave_b_ecdh_ss.hex`,
  `wave_b_hkdf_prk.hex`, `wave_b_aead_key.hex`.

### COSE_Encrypt0 layer

- Fixed plaintext: a JSON blob in the shape of a real
  `TenantEndpointConfig` decryption (`{"realm": "llm",
  "impl": "llm_openai_compat", "config": {...}}`), 200–500
  bytes. Committed as `wave_b_plaintext.json`.
- Fixed AEAD nonce: 12 bytes from a committed deterministic
  source (test-only override of the OsRng).
- Fixed AAD: SHA-256 of the canonical-CBOR-encoded
  call-context map (from §AEAD associated data).
- Expected `wave_b_cose_encrypt0.hex`.

### Wave-A × Wave-B composition

- Regenerated Wave A positive vector at
  `tests/crypto-vectors/wave-a/` so `payload_hash` equals
  SHA-256 of the new `wave_b_cose_encrypt0.hex`, and the
  claim-set includes the new `iat` field.
- Expected `wave_a_cose_sign1.hex` regenerated from the new
  claim set; other Wave A intermediate vectors
  (`wave_a_claims.cbor.hex`, `wave_a_protected.hex`,
  `wave_a_sig_structure1.hex`, `wave_a_signature.hex`)
  regenerated.

### End-to-end integration test

- `#[ignore]`-gated (same two-tier pattern as Wave A).
- Round-trip: known plaintext → lowerer side produces
  `(token_bytes, cose_encrypt0_bytes)` → service side
  verifies Wave A + decrypts Wave B → asserts recovered
  plaintext bytes equal input.
- Runs without a real network; just in-memory roundtrip.

### Negative-path vectors (additions to Wave A's 10)

One vector per Wave B rejection path:

**Parse + header validation (step 12 / 12a):**

- `wave_b_encrypted_payload_malformed.hex` — truncated
  COSE_Encrypt0 bytes (CBOR parse fails). Step 12,
  `EncryptedPayloadMalformed`.
- `wave_b_alg_not_a256gcm.hex` — valid COSE_Encrypt0 with
  `alg = 1` (A128GCM, registered but disallowed here).
  Step 12a, `EncryptedPayloadMalformed`.
- `wave_b_unprotected_nonempty.hex` — valid envelope but
  `unprotected` map carries a single entry. Step 12a,
  `EncryptedPayloadMalformed`.
- `wave_b_kem_ct_wrong_length.hex` — `kem_ct` is 1087 bytes
  (one byte short of FIPS 203). Step 12a,
  `EncryptedPayloadMalformed`.
- `wave_b_ecdh_pk_wrong_length.hex` — `ecdh_eph_pk` is 31
  bytes. Step 12a, `EncryptedPayloadMalformed`.
- `wave_b_iv_wrong_length.hex` — nonce is 11 bytes. Step 12a,
  `EncryptedPayloadMalformed`.
- `wave_b_unknown_protected_label.hex` — an extra unknown
  text-keyed header is present. Step 12a,
  `EncryptedPayloadMalformed`.

**Registry + key lookup (step 13):**

- `wave_b_unknown_realm_kid.hex` — valid COSE_Encrypt0 with
  `kid` not in the service's realm registry. Step 13,
  `UnknownRealmKid`.
- `wave_b_realm_key_out_of_window.hex` — service's realm-key
  entry has `not_after` in the past. Step 13,
  `RealmKeyOutOfWindow`.
- `wave_b_realm_key_realm_mismatch.hex` — registry entry for
  the kid exists and is in-window but its `realm` field does
  not equal `service_realm` (simulates operator
  key-distribution misconfig). Step 13,
  `RealmKeyRealmMismatch`.

**Decryption (step 14):**

- `wave_b_decrypt_tag_tamper.hex` — valid envelope but last
  byte of the GCM tag is flipped. Step 14, `DecryptionFailed`.
- `wave_b_decrypt_kem_ct_tamper.hex` — one byte of `kem_ct`
  flipped → wrong `kem_ss` → wrong `aead_key` → tag fails.
  Step 14, `DecryptionFailed`.
- `wave_b_decrypt_ecdh_pk_tamper.hex` — one byte of
  `ecdh_eph_pk` flipped → wrong ECDH shared secret → wrong
  AEAD key. Step 14, `DecryptionFailed`.
- `wave_b_decrypt_aad_tamper.hex` — valid ciphertext, but
  one claim field differs between what was bound as
  `external_aad` and what the service recomputes (simulates
  a token with matching signature but a different
  `config_uuid`). Step 14, `DecryptionFailed` — AAD mismatch
  indistinguishable from tag tamper, by design.

**Inner-realm check (step 15):**

- `wave_b_inner_realm_mismatch.hex` — valid decryption, but
  the decrypted plaintext's inner `realm` field differs from
  `claims.realm`. Step 15, `InnerRealmMismatch`.

Fifteen Wave B negative vectors. Combined with Wave A's 10,
the service-side test file asserts each of 25 reject paths
hits its specific library-level error variant — independent of
whatever generic response the service bin may surface
externally (see §"Internal vs external error surface").

## `philharmonic-connector-common 0.2.0`

This is the "(A) later" half of Yuka's Gate-2 decision from
Wave A. Scope for the 0.2.0 bump:

- `ConnectorTokenClaims` gets a new field `pub iat: UnixMillis`
  after `exp` (keeping struct-field declaration order
  stable-by-prefix for readers). CBOR encoding now has 10 map
  entries instead of 9.
- `ConnectorCallContext.issued_at` is now populated from
  `claims.iat` instead of `now`. Existing semantics ("time
  issued") is finally accurate.
- Existing consumers (`philharmonic-connector-client`,
  `philharmonic-connector-service`) bump their pin to
  `philharmonic-connector-common = "0.2"`.
- Wave A reference vectors at `tests/crypto-vectors/wave-a/`
  regenerated as noted above.

No other changes to `connector-common` in 0.2.0. Keep the bump
surface tight.

## Explicit confirmations (per crypto-review skill)

1. **Understanding of the hybrid KEM construction.** KEM-then-
   ECDH concatenation for the HKDF IKM (ML-KEM first). Empty
   salt. `info = "philharmonic/wave-b/hybrid-kem/v1/aead-key"`.
   HKDF-Extract then HKDF-Expand yields a 32-byte AES-256-GCM
   key. AEAD associated-data is the canonical CBOR of COSE's
   `Enc_structure = ["Encrypt0", protected, external_aad]`
   (RFC 9052 §5.3), where `external_aad` is SHA-256 of a
   canonical CBOR map of `(realm, tenant, inst, step,
   config_uuid, kid)`. Using the full `Enc_structure` covers
   the protected header under the AEAD tag automatically.
   Confirm Q#1 (IKM order), Q#4 (external_aad content +
   encoding).

2. **`unsafe` usage.** None planned in our code. Upstream
   `ml-kem`, `x25519-dalek`, `aes-gcm`, `hkdf`, `sha2` all use
   `unsafe` internally (RustCrypto dependency chain); we don't
   add any of our own.

3. **Key handling that can't be zeroized.** `ml-kem 0.2.x`'s
   key types: check whether `DecapsulationKey` implements
   `ZeroizeOnDrop`. If not, hold the raw secret bytes in
   `Zeroizing<[u8; 2400]>` and reconstruct the decap key per
   decryption call — analogous to Wave A's per-sign
   `SigningKey` reconstruction. **Open Question #2 below.**
   `x25519-dalek`'s `StaticSecret` and `EphemeralSecret`
   zeroize on drop natively.

4. **Signatures / MACs over untrusted input.** The AEAD tag
   acts as the MAC; AAD binds untrusted claim fields AFTER
   they've been signature-verified (Wave A steps 1-11 run
   first). The ciphertext bytes and the `kem_ct` /
   `ecdh_eph_pk` header values are attacker-controlled-but-
   covered-by-`payload_hash` (Wave A) and by the AAD +
   tag (Wave B).

## Open questions

1. **HKDF IKM ordering: `kem_ss || ecdh_ss` or `ecdh_ss ||
   kem_ss`?** My lean: ML-KEM first, as proposed. Rationale:
   matches the ordering implied by NIST's draft hybrid KEM
   combiner guidance and by the RFC 9180 (HPKE) Hybrid KEM
   drafts. Not load-bearing for security (HKDF mixes both
   thoroughly); picking and committing is what matters.

2. **ML-KEM private-key zeroization pattern.** Mirror Wave A's
   option-(a) pattern — hold raw bytes in
   `Zeroizing<[u8; 2400]>` and reconstruct the decap key on
   demand — or trust `ml-kem 0.2.x`'s native zeroize-on-drop
   (need to confirm it has one). My lean: start with raw-bytes
   pattern (known-safe), migrate to native if the crate
   provides it.

3. **X25519 ephemeral key generation source.** `OsRng` per
   encryption call, or accept a `rand_core::RngCore` parameter
   at the `encrypt()` API boundary? Former is simpler and
   matches `x25519-dalek`'s `EphemeralSecret::random()`;
   latter is test-friendlier. My lean: `OsRng` in the public
   API, plus a `#[doc(hidden)]` test-only
   `encrypt_with_rng(...)` that tests use for known-answer
   vectors.

4. **AAD content and encoding.** The fields `(realm, tenant,
   inst, step, config_uuid, kid)` are my proposed set.
   Alternatives: just `payload_hash_precursor =
   SHA-256(claims_cbor_without_payload_hash)`. Concern with
   that: it's brittle — any future claim-field addition
   changes the AAD silently. A named-field map is auditable.
   My lean: the named-field map as proposed.

5. **AEAD nonce: random or deterministic?** My lean: random
   per encryption. Alternatives include
   `HKDF-Expand(prk, "nonce-info", 12)` — deterministic from
   the KEM output. Deterministic is nice in tests but a KEM-
   derived nonce that's tied to a single AEAD key is
   equivalent to a fresh random nonce for security purposes.
   Random is simpler and matches standard GCM guidance.

6. **`rand_core` version pin.** `ml-kem 0.2` and
   `ed25519-dalek 2` may have different `rand_core` majors
   in their public APIs; need to check the transitive dep
   graph doesn't fracture. If it does, we pick the lowest-
   common-denominator `rand_core` major and live with adapter
   types, OR wait for one of the upstreams to bump. Defer to
   implementation time; flag now.

7. **COSE alg identifier.** I proposed `alg = 3` (A256GCM)
   with custom text-keyed headers for the KEM outputs.
   Alternative: register a private-use `alg` value for
   "hybrid-ML-KEM-768-X25519-HKDF-SHA256-A256GCM" (COSE
   supports private-use integer alg values, e.g. `-65537` and
   below). Pros of private-use: self-describing, future COSE
   PQC standardization can alias onto it. Cons: registry
   conflict risk if two deployments pick the same number.
   My lean: stick with `alg = 3` + custom headers for Wave B;
   revisit when the IETF COSE PQC draft lands.

8. **Distinguish AEAD tag / AAD / kem-ct mismatch failures?**
   All currently fold into `DecryptionFailed`. Operator
   troubleshooting would benefit from sub-variants, but
   distinguishing them gives an attacker an oracle (timing +
   error-variant leak what layer failed). My lean: stay
   undistinguished; log-time forensics can reconstruct which
   by re-running with diagnostic flags off-path.

## What lands (Wave B)

Source files (no code written yet):

- `philharmonic-connector-common/src/lib.rs` — `iat` field on
  `ConnectorTokenClaims`; bump to 0.2.0.
- `philharmonic-connector-client/src/encrypt.rs` (new) —
  hybrid-KEM encapsulate + HKDF + AEAD encrypt + COSE_Encrypt0
  envelope build. Public API: `encrypt_payload(plaintext,
  realm_public_key_entry, aad_inputs) ->
  ConnectorEncryptedPayload`.
- `philharmonic-connector-client/src/error.rs` — extend with
  `EncryptError` variants.
- `philharmonic-connector-client/src/signing.rs` — no
  behavior change, but the `mint_token` demo / example in
  the README rewritten to feed the new `iat` claim.
- `philharmonic-connector-service/src/decrypt.rs` (new) —
  `RealmPrivateKeyRegistry`, `decrypt_payload`. Public API
  mirrors `verify_token`'s shape.
- `philharmonic-connector-service/src/verify.rs` — extend
  `verify_token` to optionally chain-decrypt (or split into
  `verify_token` + `decrypt_payload`; final shape TBD at
  implementation time; chaining is more ergonomic, split is
  more testable).
- `philharmonic-connector-service/src/error.rs` — extend
  `TokenVerifyError` with steps 12–15 variants.
- `philharmonic-connector-router/src/*.rs` (new) — minimal
  HTTP dispatcher.
- Tests: `philharmonic-connector-client/tests/encryption_vectors.rs`,
  `philharmonic-connector-service/tests/decryption_vectors.rs`,
  `philharmonic-connector-service/tests/e2e_roundtrip.rs`
  (`#[ignore]`-gated).
- Reference vectors: `tests/crypto-vectors/wave-b/` (new).
  Regenerated Wave A vectors at
  `tests/crypto-vectors/wave-a/` (in-place update with
  regeneration date in the README).

**Publish at Wave B end:** all three triangle crates
(`philharmonic-connector-client`, `philharmonic-connector-
service`, `philharmonic-connector-router`) bump to `0.1.0`
and publish after the end-to-end test passes. This is the
first time Wave A's code actually makes it to crates.io —
Wave A landed in-tree but deferred publish specifically for
this moment.

## Codex security review resolutions

Codex ran an independent design-level security review of r1 of
this proposal. The full report is at
`docs/codex-reports/2026-04-22-0005-phase-5-wave-b-hybrid-kem-cose-encrypt0-security-review.md`.
Six findings (three HIGH, three MEDIUM); Claude's evaluation
and the r2 resolution per finding:

| # | Finding | Severity | r2 resolution |
|---|---------|----------|---------------|
| 1 | COSE `Enc_structure` not specified; AEAD AAD is raw `external_aad` digest only, so protected headers aren't AEAD-authenticated | HIGH | **Fixed.** §"AEAD associated data" now specifies RFC 9052 `Enc_structure = ["Encrypt0", protected, external_aad]` as the AEAD AAD, with `external_aad` carrying the call-context digest. Covers the protected header under the AEAD tag, matches standards-compliant COSE implementations, removes the ordering dependency on Wave A step 10. |
| 2 | Step-13 distinct error variants (`UnknownRealmKid` vs `RealmKeyOutOfWindow`) leak realm-kid-namespace state to the external attacker surface | HIGH | **Partially fixed (library) + documented (bin).** Claude's threat-model analysis: the kid in the COSE_Encrypt0 protected header is signature-covered via `payload_hash`, so an attacker with a valid token cannot freely probe realm kids — Wave A step 10 catches any substitution. The observable attack surface only arises if distinct HTTP responses surface distinct library variants. The library keeps typed variants for tests/logs (critical for operator forensics); new §"Internal vs external error surface" mandates that the service bin collapse all steps-12-to-15 failures into a single generic HTTP response with uniform latency. Library API unchanged; normative requirement on the bin layer. |
| 3 | No claims↔decryption-key binding; registry shape keyed only by `kid`, no `protected_kid == claims.kid` check, no `registry_entry.realm == claims.realm` check | HIGH | **Partially fixed.** The "`protected_kid == claims.kid`" part of the finding doesn't apply — Wave A's `claims.kid` identifies the **lowerer signing key**, while COSE_Encrypt0's protected-header `kid` identifies the **realm KEM keypair**; they are different-purpose kids by design (documented in §"Internal vs external error surface"). The realm-binding half of the finding is fixed: `RealmPrivateKeyEntry` now carries a `realm: RealmId` field, and step 13 additionally checks `entry.realm == service_realm` (`RealmKeyRealmMismatch` variant). Registry keying stays `by_kid`; single-realm-per-service makes `(realm, kid)` keying unnecessary for v1 and keying by `kid` keeps the common-case lookup cheap. The `realm` field already gives us multi-realm future-compat. |
| 4 | Replay remains accepted from Wave A and now amplifies CPU-exhaustion risk: each replay drives ML-KEM decap + X25519 + HKDF + AEAD decrypt | MEDIUM | **Documented.** Wave A's stateless-replay decision stands; adding a replay cache would violate "stateless where feasible" and add an HA cache runtime dep for marginal benefit over TLS + tight `exp` + bin-layer rate limiting. Threat-model §"Replay-driven CPU exhaustion" (new in r2) names the amplification explicitly and makes per-tenant / per-instance rate limits a normative requirement at the service bin layer (not at the library). The bin's module docs will restate this requirement at implementation time. |
| 5 | Payload-size and structure bounds underspecified | MEDIUM | **Fixed.** New step 12a specifies hard byte-length bounds for every field (`kid` ≤ 255, `IV == 12`, `kem_ct == 1088`, `ecdh_eph_pk == 32`, ciphertext bounded by Wave A's 1 MiB outer limit). |
| 6 | Mandatory algorithm / parameter validation not explicit in decrypt sequence | MEDIUM | **Fixed.** New step 12a is an explicit strict-validation pass: `alg == 3` exactly, `unprotected` map empty, every required label present exactly once at exact type/length, duplicate labels rejected, unknown labels rejected. Runs before any crypto. |

Four of six findings are normative design fixes landed inline
above; two (replay amplification and error-variant
observability) are resolved by layering requirements between
the library and the service bin — the library exposes typed
behavior; the bin assembles the external surface. The report's
positive notes (token↔ciphertext commitment, documented
secret lifecycle, negative-vector planning) carried through
into r2.

## Requesting Gate-1 approval

For this proposal to unblock dispatch:

- **Required decisions**:
  - Q#1 HKDF IKM order (my lean: ML-KEM first).
  - Q#2 ML-KEM sk zeroization approach (my lean:
    `Zeroizing<[u8; 2400]>` + per-decrypt reconstruction).
  - Q#4 AAD content (my lean: SHA-256 of canonical CBOR map of
    the six fields above).
  - Q#5 AEAD nonce source (my lean: random via OsRng).
  - Q#7 COSE alg identifier (my lean: `alg = 3` + custom text-
    keyed headers for Wave B v1).

- **Acknowledgeable** (no decision needed if Yuka agrees with
  my leans):
  - Q#3 ephemeral X25519 key from OsRng (with test-only
    override).
  - Q#6 rand_core version — defer to implementation time;
    flag if it fractures.
  - Q#8 DecryptionFailed not sub-varianted.

- Confirm the replay threat-model decision stays Wave A's,
  with the r2-added CPU-amplification acknowledgement and
  normative bin-layer rate-limit requirement.
- Confirm the revised verify+decrypt order: Wave A's 11 steps,
  then 12 (parse), 12a (strict header validation per r2), 13
  (kid lookup + window + `entry.realm == service_realm` per
  r2), 14 (crypto, using `Enc_structure` AAD per r2), 15
  (inner-realm check).
- Confirm the internal-vs-external error-surface split:
  typed library variants, generic HTTP response from the bin
  (per r2 / finding #2).
- Confirm the `connector-common 0.2.0` bump scope is
  appropriate for Wave B (adding `iat`, nothing else).
- Confirm Wave A vectors regeneration is in scope of Wave B
  and not its own separate task.
- Flag anything in §Construction you want adjusted.
- Say "Gate-1 approved" (with a list of Q answers) and I'll
  generate the reference vectors, archive the Codex prompt, and
  dispatch. No code before that.
