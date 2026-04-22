# Gate-1 proposal — Phase 5 Wave A: COSE_Sign1 connector authorization tokens

**Date:** 2026-04-22
**Phase:** 5 (connector triangle), Wave A (signing-only half)
**Author:** Claude Code (on Yuka's review queue)
**Status:** Awaiting Gate-1 sign-off — no implementation yet
**Wave split decision:** ROADMAP §Phase 5 (commit `e292918`)

## Scope

Wave A lands the Ed25519 + COSE_Sign1 layer of the connector
triangle:

- Lowerer side (`philharmonic-connector-client`): mint a signed
  `ConnectorSignedToken` over a `ConnectorTokenClaims` payload.
- Service side (`philharmonic-connector-service`): verify the
  token's signature, expiry, and `payload_hash` binding against
  caller-supplied payload bytes.

Out of scope for Wave A (deferred to Wave B):

- Payload encryption (COSE_Encrypt0, ML-KEM-768, X25519, HKDF,
  AES-256-GCM).
- Generating real encrypted payloads — Wave A tests use
  arbitrary payload bytes and verifies that
  `SHA-256(bytes) == claims.payload_hash`.
- Router.
- End-to-end cross-crate integration (comes after Wave B).

## Primitives and library versions

All RustCrypto. Versions verified against crates.io on
2026-04-22 via `./scripts/xtask.sh crates-io-versions`:

- **`ed25519-dalek = "2"`** — latest `2.2.0`. Used for
  Ed25519 signing and verification. No feature flags beyond the
  defaults; `rand_core` feature for in-process key generation
  in tests only.
- **`coset = "0.4"`** — latest `0.4.2`. Already landed as a
  dep in `philharmonic-connector-common 0.1.0`. Provides
  `CoseSign1`, `CoseSign1Builder`, `ProtectedHeader`,
  `Algorithm`, `HeaderBuilder`, and the CBOR framing. We'll
  reuse the `ConnectorSignedToken(CoseSign1)` newtype from
  connector-common rather than touching `coset` types directly
  in the new code.
- **`sha2 = "0.11"`** — latest `0.11.0`. For the payload-hash
  check on the service side. Matches the version already in
  `philharmonic-policy 0.1.0`.
- **`zeroize = { version = "1", features = ["derive"] }`** —
  latest `1.8.2`. Wraps Ed25519 private-key bytes in
  `Zeroizing<[u8; 32]>`. Matches the version already in
  `philharmonic-policy 0.1.0`.

No new primitives. No `unsafe`. No custom MAC / KDF / AEAD.

## Construction

### Token shape

The payload of the COSE_Sign1 is a CBOR encoding of
`ConnectorTokenClaims` from `philharmonic-connector-common 0.1.0`:

```rust
pub struct ConnectorTokenClaims {
    pub iss: String,
    pub exp: UnixMillis,
    pub kid: String,
    pub realm: String,
    pub tenant: Uuid,
    pub inst: Uuid,
    pub step: u64,
    pub config_uuid: Uuid,
    pub payload_hash: Sha256,
}
```

The struct is serde-derived, round-trip-tested in
connector-common's serde suite. Wave A pins the CBOR encoding
as the wire form:

- `Uuid` fields serialize as 16-byte byte strings (CBOR major
  type 2), not UTF-8 textual UUIDs. This matches how
  `serde_cbor` / `ciborium` handle `uuid::Uuid` by default when
  the `serde` feature is on; we'll test this explicitly in
  vector tests.
- `UnixMillis` serializes as an unsigned 64-bit integer
  (millis since epoch).
- `Sha256` serializes as a 32-byte byte string (major type 2).

The CBOR encoding is canonical per RFC 8949 §4.2 deterministic
encoding — `ciborium` produces deterministic output by default
for simple struct types.

### Protected headers

The COSE_Sign1 `protected` bucket carries two fields:

- `alg = -8` (EdDSA per RFC 9053 §2.2).
- `kid = claims.kid` as a UTF-8 byte string. Binding `kid` in
  the protected header (in addition to in the payload) matches
  COSE convention and makes the kid itself signature-covered.

`unprotected` header is empty — everything security-relevant
goes in the protected bucket.

### Signature input

The COSE_Sign1 signature is computed over the
`Sig_structure1` per RFC 9052 §4.4:

```
Sig_structure1 = [
    context: "Signature1",
    body_protected: serialized protected header bucket,
    external_aad: h'' (empty — we don't use external AAD),
    payload: serialized ConnectorTokenClaims CBOR,
]
```

`coset::CoseSign1Builder::create_signature` handles this
encoding; we pass it the claim bytes, a closure that runs
Ed25519 signing, and the protected header builder.

### External AAD

Empty. Every field that needs to be bound is already in the
payload (`payload_hash`, `realm`, `tenant`, `inst`, `step`,
`config_uuid`). External AAD would duplicate what's already
signed, so leaving it empty keeps the construction simpler and
matches the RFC 9052 recommended pattern for JWT-like tokens.

### Service-side verification order

The service-side `verify_token` runs checks in this order,
stopping at the first failure:

1. Parse the COSE_Sign1 bytes via `coset::CoseSign1::from_slice`.
2. Extract `kid` from protected header; look up the verifier
   key in the `MintingKeyRegistry`. Unknown kid → reject.
3. Verify the Ed25519 signature using
   `coset::CoseSign1::verify_signature`. Bad signature →
   reject. **This must come before any payload-content check**
   so that malformed payloads can't leak timing side channels.
4. Decode the claim payload from CBOR.
5. Check `exp > now_millis()` where `now_millis()` uses
   `UnixMillis::now()` (monotonic enough for expiry checks;
   we're not making real-time-accuracy claims). Expired →
   reject.
6. Compute `SHA-256(payload_bytes)` where `payload_bytes` is
   the caller-supplied opaque payload. Compare with
   `claims.payload_hash` in constant time via
   `subtle::ConstantTimeEq` (pull `subtle = "2"` as a small new
   dep, or use `ed25519-dalek`'s transitive `subtle`
   re-export — TBD at implementation time, no construction
   impact).
7. If the caller passed an `expected_realm`, check
   `claims.realm == expected_realm`. Mismatch → reject.

Only after all seven pass is a verified `ConnectorCallContext`
returned. The context is built from the claim fields that the
service needs to dispatch (`tenant`, `inst`, `step`,
`config_uuid`, `iss`-as-issuer, `exp`). The service does **not**
pass the full claim set through to the implementation; the
`ConnectorCallContext` struct from connector-common is the
narrowed, already-verified interface.

## Key management

### Minting side (lowerer)

Each lowerer binary loads its Ed25519 keypair at boot:

- Source: a file path in the lowerer's configuration
  (`signing_key_path`). 32-byte seed; PEM or raw binary —
  decide at implementation time, default to raw binary for
  simplicity.
- Read bytes into `Zeroizing<[u8; 32]>` immediately after the
  file read; `std::fs::read` returns `Vec<u8>` which lives
  long enough for a `copy_from_slice` into the zeroized
  buffer, then drop the `Vec`.
- Construct `ed25519_dalek::SigningKey::from_bytes(&buf)`. The
  `SigningKey` type itself does not zeroize on drop (as of
  2.2.0); wrap it in a local `Zeroizing`-aware newtype at the
  lowerer's public surface:

  ```rust
  pub struct LowererSigningKey {
      inner: SigningKey,   // ed25519_dalek type
      kid: String,
  }

  impl Drop for LowererSigningKey {
      fn drop(&mut self) {
          // SigningKey::to_bytes returns a [u8; 32] snapshot we
          // can zero; but the internal field is private, so we
          // zero a copy and rely on the Drop-on-move semantics
          // of the inner SigningKey itself. Flag for Yuka: is
          // the implicit zero-before-drop in SigningKey sufficient,
          // or do we need to patch ed25519-dalek for explicit
          // zeroization? See Open Questions §1 below.
      }
  }
  ```

  **This is Open Question #1 below.** Pending Yuka's guidance,
  the default plan is to use `secrecy::SecretBox<[u8; 32]>`
  holding the seed and regenerating the `SigningKey` on each
  sign call — slower but guaranteed zeroization. Benchmarks
  TBD.

### Verifying side (service)

The `philharmonic-connector-service` holds a
`MintingKeyRegistry`:

```rust
pub struct MintingKeyRegistry {
    by_kid: HashMap<String, ed25519_dalek::VerifyingKey>,
}

impl MintingKeyRegistry {
    pub fn lookup(&self, kid: &str) -> Option<&VerifyingKey>;
    pub fn insert(&mut self, kid: String, vk: VerifyingKey);
}
```

Public keys are **not** sensitive and don't need zeroization.

Registered at boot from a config file (one entry per minting
authority, each with `kid`, `public_key_hex`, `not_before`,
`not_after`). Rotation is additive: a new kid gets a new
`VerifyingKey` inserted; the old kid stays registered until
all in-flight tokens issued under it have expired (`exp` past
`now`). Validity windows (`not_before` / `not_after` on the
service-side registry entry) let operators retire a kid
cleanly.

### `kid` encoding

`kid` is a free-form UTF-8 string, signed as part of the
protected header. Suggested format: `<issuer-slug>-<utc-date>-<rand-hex-8>`
(e.g. `lowerer.main-2026-04-22-3c8a91d0`). Not pinned as a
wire format — the registry uses exact-string equality.

## Zeroization points

**Private keys only** (public keys need no zeroization):

- `Zeroizing<[u8; 32]>` on the seed buffer between file read
  and `SigningKey::from_bytes` call.
- `LowererSigningKey` wrapper (see Open Questions §1 for the
  SigningKey-specific zeroization story).
- Signing-time intermediates (the `r` nonce in Ed25519) are
  derived inside `ed25519-dalek` and aren't exposed to our
  code, so there's nothing for us to zero. This is fine —
  the library is the trusted boundary.

## Test-vector plan

Per the crypto-review skill's vector discipline. Known-answer
tests, not round-trip. Commit vectors as hex-encoded byte
strings.

### Ed25519 keypair

Committed as `tests/vectors/wave_a_signing.json`:

```json
{
  "seed_hex": "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
  "public_key_hex": "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
}
```

This is RFC 8032 §7.1 TEST 1 — a published test keypair. Using
a public vector makes cross-implementation cross-checks easier.

### Claim set

```json
{
  "iss": "lowerer.main",
  "exp_millis": 1924992000000,
  "kid": "lowerer.main-2026-04-22-3c8a91d0",
  "realm": "llm",
  "tenant_uuid": "11111111-2222-4333-8444-555555555555",
  "inst_uuid": "66666666-7777-4888-8999-aaaaaaaaaaaa",
  "step": 7,
  "config_uuid": "bbbbbbbb-cccc-4ddd-8eee-ffffffffffff",
  "payload_hash_hex": "abababab...32-byte-hex"
}
```

`payload_hash_hex` is the SHA-256 of a known plaintext, e.g.
`b"phase-5-wave-a-test-payload"`. Committing the hash + the
plaintext lets future vector-generators reproduce the hash.

### Expected CBOR claim bytes

Hex-encoded canonical CBOR of the claim set. Generated by
running the implementation once, verified by hand against RFC
8949's deterministic-encoding rules (major types, integer
minimality, etc.). Committed as
`tests/vectors/wave_a_claims.cbor.hex`.

### Expected COSE_Sign1 bytes

Hex-encoded final COSE_Sign1 structure, sealing protected
headers + payload + signature. Generated by signing the CBOR
with the committed seed; cross-checked against a Python `cose`
(pycose) implementation producing the same bytes.

Committed as `tests/vectors/wave_a_cose_sign1.hex`.

### Negative-path vectors

- `wave_a_tampered_sig.hex` — last byte of the signature
  flipped; must fail verification.
- `wave_a_tampered_payload.hex` — one byte of the claim
  payload flipped; must fail verification.
- `wave_a_tampered_kid.hex` — `kid` in the protected header
  replaced with a kid not in the registry; must fail at the
  registry lookup step.
- `wave_a_expired.hex` — same construction but `exp` set to 1
  (long in the past); must pass signature verification but
  fail the expiry check.
- `wave_a_payload_hash_mismatch.hex` — valid signature over
  one claim's `payload_hash`, but service verifies with
  different payload bytes; must fail the hash-mismatch check.

Five negative vectors covering each rejection reason in the
verification order.

## Explicit confirmations (per crypto-review skill)

1. **Understanding of the signing construction.** COSE_Sign1
   per RFC 9052 §4.4. Signature is over the CBOR-encoded
   `Sig_structure1 = ["Signature1", body_protected_bytes,
   external_aad=h'', payload_bytes]`. Ed25519 per RFC 8032
   is deterministic — no per-signature randomness required
   from us. COSE algorithm ID `-8` (EdDSA) per RFC 9053 §2.2.
   **Wave A does NOT involve a hybrid KEM, HKDF, AEAD, or
   symmetric key derivation.** Those all belong to Wave B.

2. **`unsafe` usage.** None planned. `ed25519-dalek 2.x` uses
   `unsafe` internally (via its RustCrypto dependency chain);
   we don't add any of our own.

3. **Key handling that can't be zeroized.** See Open Question
   §1 — `ed25519_dalek::SigningKey` doesn't itself implement
   `Zeroize`. Two fallback approaches proposed; awaiting
   Yuka's preference.

4. **Signatures over untrusted input.** The sign side takes
   trusted input (engine-assembled claim values), so
   straightforward. The verify side takes
   attacker-controlled COSE_Sign1 bytes and
   attacker-controlled payload bytes. Signature verification
   gates everything — no claim field is trusted before the
   signature check passes, which is standard COSE /
   JWT-equivalent discipline.

## Open questions

1. **Ed25519 private-key zeroization.** `ed25519_dalek::SigningKey`
   (version 2.x) does not implement `Zeroize` or zero its
   internal secret on drop. Options:

   - (a) Hold the raw 32-byte seed in a `Zeroizing<[u8; 32]>`
     and reconstruct the `SigningKey` via `from_bytes` on
     every sign call. Costs a key schedule per sign; at
     lowerer throughput (one sign per connector call) this is
     negligible.
   - (b) Cache the `SigningKey` and manually zero its
     internal state at drop via a custom wrapper. Requires
     access to `ed25519-dalek` internals we don't control;
     fragile.
   - (c) Accept that the `SigningKey` lives in RAM for the
     process lifetime and trust Linux's process-isolation
     boundary. Simplest, weakest.

   **My lean: (a).** The cost is trivial, the zeroization is
   guaranteed, the wrapper is a few lines. Want Yuka's call
   before implementation.

2. **`pycose` cross-check for the COSE_Sign1 vector.** The
   vector plan assumes we have a non-Rust reference
   implementation to cross-check against for the canonical
   CBOR / COSE_Sign1 bytes. `pycose` 2.x (Python) is the
   obvious candidate. Flagging because it's the one external
   tool in the plan — if Yuka wants a different reference
   (e.g. `go-cose`), say so and I'll adjust.

3. **`subtle` crate for constant-time `payload_hash` compare.**
   SHA-256 digest comparisons that *feed* a signature check
   are already covered by the signature's integrity — but
   the payload-hash check on the service side is a separate
   equality. Using `ConstantTimeEq` is cheap insurance
   against speculative side channels. Small new dep (`subtle
   = "2"`), already in the dependency tree transitively via
   `ed25519-dalek`. Flagging as a decision, not a question —
   will use unless Yuka objects.

## What lands (Wave A)

Source files (no code written yet):

- `philharmonic-connector-client/src/signing.rs` — `LowererSigningKey`
  + `mint_token`.
- `philharmonic-connector-service/src/verify.rs` —
  `MintingKeyRegistry` + `verify_token`.
- `philharmonic-connector-service/src/context.rs` — the
  verified `ConnectorCallContext` construction from claim
  fields.
- Tests: `philharmonic-connector-client/tests/signing_vectors.rs`,
  `philharmonic-connector-service/tests/verify_vectors.rs`,
  `tests/vectors/*.hex` / `*.json` committed alongside.

What does not ship in Wave A:

- No `ConfigLowerer` impl (that's Wave B, which needs
  encryption to produce the payload bytes).
- No publish — crates stay at `0.0.0` until Wave B's
  end-to-end tests pass.

## Requesting Gate-1 approval

For this proposal to unblock dispatch:

- Confirm or adjust each of the three Open Questions.
- Flag any field of the claim set you want CBOR-encoded
  differently than the defaults (e.g. `Uuid` as text vs.
  bytes).
- Confirm the verification order is correct, especially the
  signature-first-before-any-payload-content-check ordering.
- Say "Gate-1 approved" and I'll archive the Codex prompt +
  dispatch. No code before that.
