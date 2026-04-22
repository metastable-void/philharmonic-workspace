# Phase 5 Wave B — hybrid KEM + COSE_Encrypt0 payload encryption

**Date:** 2026-04-22 (drafted); dispatch planned for next session
**Slug:** `phase-5-wave-b-hybrid-kem-cose-encrypt0`
**Round:** 01
**Subagent:** `codex:codex-rescue`

## Motivation

Implement the encryption half of the Phase 5 connector triangle.
Three crates receive substantive code:

- `philharmonic-connector-client` — lowerer-side hybrid-KEM
  encapsulate + AEAD encrypt, producing `ConnectorEncryptedPayload`.
  Composes with the Wave A `mint_token` so a lowerer can emit a
  paired `(token, ciphertext)` tuple.
- `philharmonic-connector-service` — service-side verify + decrypt
  pipeline, extending Wave A's 11-step order with steps 12, 12a,
  13, 14, 15 per the Gate-1-approved proposal.
- `philharmonic-connector-router` — minimal HTTP dispatcher (no
  crypto): terminate TLS at `<realm>.connector.<domain>`, forward
  `Authorization: Bearer <token>` + `X-Encrypted-Payload: <bytes>`
  to a connector-service instance in the realm.

This is a **Gate-1-approved** crypto-sensitive task. The approved
construction (primitives, verification order, AAD binding,
zeroization, error taxonomy) is frozen by the proposal; do not
deviate without flagging.

Wave B is when the roadmap's "triangle crates publish as 0.1.0"
gate unlocks. No publish as part of this dispatch — Claude
handles that after Gate-2 review on the returned code.

## Gate-1 status (frozen before writing code)

- **Proposal (authoritative):**
  `docs/design/crypto-proposals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md`
  (revision 3, Gate-1 approved).
- **Approval record:**
  `docs/design/crypto-approvals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md`.
- **Security review (addressed in r2):**
  `docs/codex-reports/2026-04-22-0005-phase-5-wave-b-hybrid-kem-cose-encrypt0-security-review.md`.
- **Reference vectors (pre-committed, do NOT regenerate):**
  - `docs/crypto-vectors/wave-b/` — every Wave B intermediate
    (ML-KEM pk / sk / ct / ss, X25519 keys + ECDH ss, HKDF IKM
    + AEAD key, external_aad digest, Enc_structure, protected
    header, ciphertext+tag, final COSE_Encrypt0 envelope,
    payload_hash).
  - `docs/crypto-vectors/wave-a/wave_a_composition_*.hex` —
    Wave A token regenerated with `payload_hash` pointing at
    the Wave B COSE_Encrypt0 (the end-to-end composition
    fixture).
  - `docs/crypto-vectors/wave-a/wave_a_*.hex` — self-contained
    Wave A vectors (unchanged by Wave B; your Wave A tests
    still need to pass against these).

If the code you're about to write would contradict the proposal,
STOP and flag. The proposal wins; your implementation must match
it or the dispatch goes back to Claude.

Wave B also lands the `philharmonic-connector-common 0.2.0`
bump (adds the `iat` claim). That bump already landed in-tree in
the earlier Wave-B-prep commit — `connector-client` and
`connector-service` already consume the new claim set. Your work
is the new encrypt/decrypt surface plus the router.

## References (read before coding)

- `docs/design/crypto-proposals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md` — the authoritative construction spec (979 lines, every section is load-bearing).
- `docs/design/crypto-approvals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md` — Yuka's Gate-1 with Q answers.
- `docs/design/11-security-and-cryptography.md` §§Cryptographic primitives / Encryption systems / Encrypted payload flow — threat-model context.
- `docs/design/13-conventions.md` §Library crate boundaries, §Panics and undefined behavior, §Script wrappers, §In-tree workspace tooling. Non-negotiable.
- `docs/crypto-vectors/wave-b/README.md` — explains every hex file and how the generator produced the bytes you must reproduce.
- `docs/crypto-vectors/wave-a/README.md` — the regenerated Wave A vectors (with `iat` added) plus the new Wave A × Wave B composition vectors.
- `philharmonic-connector-common 0.2.0` source (`philharmonic-connector-common/src/lib.rs`) — the shared vocabulary (`ConnectorTokenClaims` with `iat`, `RealmPublicKey`, `RealmRegistry`, `ConnectorEncryptedPayload`, `ConnectorCallContext`, `ImplementationError`, `MLKEM768_PUBLIC_KEY_LEN = 1184`).
- `philharmonic-connector-client/src/signing.rs` — the Wave A signing surface you're extending.
- `philharmonic-connector-service/src/verify.rs` — the Wave A 11-step verify you're chaining onto.

## Scope

### In scope

**`philharmonic-connector-client`** (new code in `src/`):

- `src/encrypt.rs` — hybrid-KEM encapsulate + AEAD encrypt +
  COSE_Encrypt0 envelope build.
  - Public API (exact shape TBD at implementation time; this is a
    sketch, refine for ergonomics):
    ```rust
    pub fn encrypt_payload(
        plaintext: &[u8],
        realm_key: &RealmPublicKey,          // from connector-common 0.2.0
        aad_inputs: AeadAadInputs<'_>,        // see §AEAD below
        rng: &mut impl CryptoRngCore,         // OsRng in production; test-only override
    ) -> Result<ConnectorEncryptedPayload, EncryptError>;
    ```
  - For deterministic known-answer tests, expose a `#[doc(hidden)]`
    `encrypt_payload_with_test_inputs(...)` that takes explicit
    ML-KEM `m` randomness, explicit X25519 ephemeral private bytes,
    and explicit AEAD nonce. This mirrors the Wave A vector tests'
    pattern of asserting byte-for-byte equality against committed
    hex.
  - Library takes bytes everywhere: `RealmPublicKey` is already a
    bytes-only struct (shipped in connector-common 0.1.0).
    Lowerer-bin file-I/O / config-parsing stays out of scope.
- `src/error.rs` — extend `MintError` (already present) with new
  `EncryptError` variants:
  - `SerializationFailure`, `KemEncapsulationFailure`,
    `HkdfFailure`, `AeadEncryptionFailure`,
    `MalformedRealmKey`, `InvalidInput`. `thiserror`-derived, no
    panicable variants.
- `src/signing.rs` — no new behavior; the Wave A implementation
  already threads `iat` through the claim set. Verify untouched.
- `src/lib.rs` — re-export `encrypt_payload`, the new error type,
  relevant bytes-owning wrappers.

**`philharmonic-connector-service`** (new code in `src/`):

- `src/realm_keys.rs` (new) — `RealmPrivateKeyEntry` +
  `RealmPrivateKeyRegistry`:
  ```rust
  pub struct RealmPrivateKeyEntry {
      pub kem_sk: Zeroizing<[u8; 2400]>,     // ML-KEM-768 dk
      pub ecdh_sk: x25519_dalek::StaticSecret,
      pub realm: RealmId,                      // per Codex r2 review
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
- `src/decrypt.rs` (new) — `decrypt_payload(cose_encrypt0_bytes,
  registry, service_realm, call_context) -> Result<Vec<u8>,
  TokenVerifyError>`. The plaintext is `Zeroizing<Vec<u8>>` on
  return so callers can drop it cleanly (or the decrypt returns
  `Zeroizing`-wrapped bytes and the caller decides what to do).
- `src/verify.rs` — extend `verify_token` (or add
  `verify_and_decrypt`, final shape TBD) to chain the full
  12 → 12a → 13 → 14 → 15 sequence after Wave A's 1–11. Keep the
  existing `verify_token` surface intact for callers that want
  token-only verification.
- `src/error.rs` — extend `TokenVerifyError` with:
  - `EncryptedPayloadMalformed` (steps 12 + 12a; malformed CBOR,
    unexpected alg, non-empty unprotected, missing / duplicate /
    unknown labels, wrong byte lengths).
  - `UnknownRealmKid { kid }` (step 13).
  - `RealmKeyOutOfWindow { now, not_before, not_after }` (step 13).
  - `RealmKeyRealmMismatch { expected, found }` (step 13, r2
    Codex-review addition).
  - `DecryptionFailed` (step 14; any of tag / AAD / kem_ct /
    ecdh_eph_pk tamper folds here, indistinguishable by design).
  - `InnerRealmMismatch { expected, found }` (step 15).
- `src/lib.rs` — re-exports.

**`philharmonic-connector-router`** (new code in `src/`):

- Minimal HTTP dispatcher. No crypto. No token / payload
  inspection beyond pass-through.
- Suggested: `axum` or `hyper` + `tokio`. Pick whichever
  matches the workspace's existing async story (no other router
  yet — this is a fresh choice; justify briefly in your final
  summary).
- Routes: single handler at `*` that forwards any request
  whose `Host` header matches `<realm>.connector.<domain>` to a
  configured upstream (connector-service instance) in that realm.
  Round-robin load-balance across upstreams if >1; simple is
  fine.
- Error surface: the router is an infra component; don't
  collapse error variants. Just forward 5xx if the upstream is
  unreachable.
- Tests: one tier-1 mock test using `tower::util::ServiceExt`
  or a stubbed hyper `Service` to confirm a request for
  `Host: llm.connector.example.com` dispatches to the correct
  upstream. No real network.
- Version stays at `0.0.0`. Claude handles the bump + publish at
  Gate-2 approval.

**Tests (all three crates):**

- Known-answer vector tests: load committed hex from
  `docs/crypto-vectors/wave-b/` and assert byte-for-byte equality
  with your implementation's output. At minimum:
  - ML-KEM `_keygen_internal(d, z)` → expected `pk` / `sk`.
  - ML-KEM `_encaps_internal(ek, m)` → expected `kem_ct` / `kem_ss`.
  - ML-KEM `decaps(sk, kem_ct)` round-trip → expected `kem_ss`.
  - X25519 realm + ephemeral public derivation → expected public
    bytes.
  - X25519 ECDH → expected `ecdh_ss`.
  - HKDF output → expected `aead_key`.
  - AAD digest → expected `external_aad`.
  - Protected header bytes → expected `wave_b_protected.hex`.
  - `Enc_structure` bytes → expected `wave_b_enc_structure.hex`.
  - AEAD encrypt → expected `ciphertext_and_tag`.
  - Final envelope → expected `wave_b_cose_encrypt0.hex`.
  - `payload_hash` → expected `wave_b_payload_hash.hex`.

- 15 negative-vector tests (per proposal §Negative-path vectors):
  synthesize at test time from the positive vectors by local
  perturbation. Each asserts the exact `TokenVerifyError` variant.
  List in the proposal's Negative-vectors section — follow it
  verbatim.

- End-to-end composition test (`#[ignore]`-gated if slow, but
  likely fast enough to run always):
  - Build Wave B plaintext → lowerer-side `encrypt_payload` →
    assert bytes match `wave_b_cose_encrypt0.hex`.
  - Compute `payload_hash = SHA-256(cose_encrypt0_bytes)` →
    assert match with `wave_a_composition_payload_hash.hex`.
  - Wave A `mint_token` with claims containing that
    `payload_hash` (+ the Wave A-composition claim set from the
    vectors) → assert bytes match `wave_a_composition_cose_sign1.hex`.
  - Service side: verify_token (Wave A 1–11) with committed
    token + payload bytes → success.
  - Service side: decrypt_payload (Wave B 12–15) with committed
    COSE_Encrypt0 bytes + verified call-context → assert plaintext
    bytes exactly equal the original (119-byte) plaintext.
  - Test verifies the end-to-end ≈ "mint + encrypt on one side,
    verify + decrypt on the other, no byte drift".

### Out of scope

- **No concrete connector implementations.** `http_forward`,
  `llm_openai_compat`, etc. are Phase 6.
- **No real network** in the end-to-end test; in-memory only.
- **No router production polish.** TLS termination details,
  graceful shutdown, metrics, tracing — those are Phase 8 /
  deployment concerns. Wave B lands the dispatch skeleton only.
- **No publish.** Claude does the version bumps + publish after
  Gate-2 review.
- **No SCK path.** SCK encrypt / decrypt is already in
  `philharmonic-policy 0.1.0`; not re-touched here.
- **No lowerer / service bin crates.** File I/O, config-file
  parsing, permission checks, rate limiting — all bin-layer
  concerns. Libraries stay bytes-only.

## Construction (binding)

Read §Construction of the proposal end-to-end. Summaries below
are for quick reference, not substitutes.

### Hybrid KEM

- ML-KEM-768 encapsulate / decapsulate (FIPS 203). Use
  `ml-kem::MlKem768` (RustCrypto crate). For deterministic tests,
  use the internal-randomness surface that takes `d`, `z`, `m`
  directly — mirrors `kyber-py._keygen_internal` /
  `_encaps_internal` used by the vector generator.
- X25519 classical ECDH. Use `x25519_dalek::StaticSecret` for
  realm long-lived, `x25519_dalek::EphemeralSecret` for the
  lowerer's per-encryption ephemeral.
- HKDF-SHA256:
  - IKM: `kem_ss || ecdh_ss` (ML-KEM first; Gate-1 Q#1).
  - Salt: `b""` (empty).
  - Info: `b"philharmonic/wave-b/hybrid-kem/v1/aead-key"`.
  - Output: 32 bytes, the AEAD key.

### AEAD

- AES-256-GCM (`aes_gcm::Aes256Gcm`, `aead` 0.5 / 0.10 API).
- Nonce: 12 bytes, random via `OsRng` in production; test-only
  override for vector reproduction.
- AEAD associated data: the canonical CBOR of
  `Enc_structure = ["Encrypt0", protected_bytes, external_aad]`
  per RFC 9052 §5.3. This covers the protected header under the
  AEAD tag automatically.
- `external_aad`: 32-byte SHA-256 digest of canonical CBOR of
  `(realm, tenant, inst, step, config_uuid, kid)` in that
  declaration order. Define a struct in code with those fields
  in that order so `ciborium` emits identical bytes.

### COSE_Encrypt0 envelope

- Protected header: CBOR map with labels, coset-order:
  - `1` (alg) → `3` (A256GCM)
  - `4` (kid) → UTF-8 bytes of the realm kid
  - `5` (IV) → 12-byte nonce
  - `"kem_ct"` → 1088 bytes
  - `"ecdh_eph_pk"` → 32 bytes
- Unprotected: empty map.
- Ciphertext: `ciphertext || tag` (AES-GCM native layout).

### 12–15 verify + decrypt order (service side)

Runs ONLY after Wave A's 11 steps pass. Short-circuit at first
failure.

1. Parse `COSE_Encrypt0` CBOR → `EncryptedPayloadMalformed`.
2. Strict protected-header validation (step 12a):
   - `alg == 3` exactly.
   - `unprotected` empty.
   - Required labels present exactly once: `1`, `4`, `5`,
     `"kem_ct"`, `"ecdh_eph_pk"`.
   - Duplicate labels rejected.
   - Unknown labels rejected (no forward-compat tolerance).
   - Exact lengths: `kid` 1..=255, `IV` 12, `kem_ct` 1088,
     `ecdh_eph_pk` 32.
   - Any failure → `EncryptedPayloadMalformed`.
3. Realm-kid lookup in `RealmPrivateKeyRegistry` (step 13):
   - Missing → `UnknownRealmKid`.
   - Out of window (`now < not_before || now >= not_after`) →
     `RealmKeyOutOfWindow`.
   - `entry.realm != service_realm` → `RealmKeyRealmMismatch`.
4. Hybrid-KEM decapsulate + HKDF + compute expected `external_aad`
   from Wave-A-verified claims + AEAD decrypt (step 14): any
   failure collapses to `DecryptionFailed`.
5. Parse decrypted plaintext as JSON; assert inner `realm` field
   equals `claims.realm` (step 15) →
   `InnerRealmMismatch { expected, found }`.

## Reference vectors — use AS COMMITTED

Same rule as Wave A. Load the hex files via `include_str!` + trim
+ `hex::decode` (or `hex_literal::hex!` if you prefer inline
literals — match whatever Wave A did; check
`philharmonic-connector-client/tests/signing_vectors.rs`).

- `docs/crypto-vectors/wave-b/wave_b_mlkem_keygen_d.hex` — 32 bytes.
- `docs/crypto-vectors/wave-b/wave_b_mlkem_keygen_z.hex` — 32 bytes.
- `docs/crypto-vectors/wave-b/wave_b_mlkem_encaps_m.hex` — 32 bytes.
- `docs/crypto-vectors/wave-b/wave_b_mlkem_public.hex` — 1184 bytes.
- `docs/crypto-vectors/wave-b/wave_b_mlkem_secret.hex` — 2400 bytes.
- `docs/crypto-vectors/wave-b/wave_b_mlkem_ct.hex` — 1088 bytes.
- `docs/crypto-vectors/wave-b/wave_b_mlkem_ss.hex` — 32 bytes.
- `docs/crypto-vectors/wave-b/wave_b_x25519_realm_sk.hex` — 32 bytes.
- `docs/crypto-vectors/wave-b/wave_b_x25519_realm_pk.hex` — 32 bytes.
- `docs/crypto-vectors/wave-b/wave_b_x25519_eph_sk.hex` — 32 bytes.
- `docs/crypto-vectors/wave-b/wave_b_x25519_eph_pk.hex` — 32 bytes.
- `docs/crypto-vectors/wave-b/wave_b_ecdh_ss.hex` — 32 bytes.
- `docs/crypto-vectors/wave-b/wave_b_hkdf_ikm.hex` — 64 bytes (= `kem_ss || ecdh_ss`).
- `docs/crypto-vectors/wave-b/wave_b_aead_key.hex` — 32 bytes.
- `docs/crypto-vectors/wave-b/wave_b_external_aad.hex` — 32 bytes.
- `docs/crypto-vectors/wave-b/wave_b_nonce.hex` — 12 bytes.
- `docs/crypto-vectors/wave-b/wave_b_plaintext.hex` — 119 bytes.
- `docs/crypto-vectors/wave-b/wave_b_protected.hex` — 1196 bytes.
- `docs/crypto-vectors/wave-b/wave_b_enc_structure.hex` — 1243 bytes.
- `docs/crypto-vectors/wave-b/wave_b_ciphertext_and_tag.hex` — 135 bytes.
- `docs/crypto-vectors/wave-b/wave_b_cose_encrypt0.hex` — 1338 bytes.
- `docs/crypto-vectors/wave-b/wave_b_payload_hash.hex` — 32 bytes.

Wave A × Wave B composition:

- `docs/crypto-vectors/wave-a/wave_a_composition_payload_hash.hex` — equal to `wave_b_payload_hash.hex`; sanity assert.
- `docs/crypto-vectors/wave-a/wave_a_composition_claims.cbor.hex` — 220 bytes.
- `docs/crypto-vectors/wave-a/wave_a_composition_sig_structure1.hex` — 275 bytes.
- `docs/crypto-vectors/wave-a/wave_a_composition_signature.hex` — 64 bytes.
- `docs/crypto-vectors/wave-a/wave_a_composition_cose_sign1.hex` — 330 bytes.

Call-context fields for the AAD / AAD assertions:

```rust
let aad_inputs = AeadAadInputs {
    realm: "llm",
    tenant: Uuid::parse_str("11111111-2222-4333-8444-555555555555").unwrap(),
    inst: Uuid::parse_str("66666666-7777-4888-8999-aaaaaaaaaaaa").unwrap(),
    step: 7,
    config_uuid: Uuid::parse_str("bbbbbbbb-cccc-4ddd-8eee-ffffffffffff").unwrap(),
    kid: "lowerer.main-2026-04-22-3c8a91d0",
};
```

The realm KEM kid used in vectors is
`"llm.default-2026-04-22-realmkey0"` (32 chars; distinct from
the Wave A lowerer signing kid).

If your implementation's output diverges from any committed hex
by even one byte, that's a bug — either in your ciborium map
ordering, your HKDF inputs, your AAD construction, or somewhere
in the crypto chain. **Do not "fix" the test vectors.** Report
the divergence in your final summary with enough detail for
Claude to diagnose (which file mismatches, first mismatched byte
offset, both `yours` and `expected` hex).

## Negative vectors (synthesize at test time)

Exactly 15, one per rejection path. Full recipes in the proposal
§Negative-path vectors; summary below.

**Parse + header validation (step 12 / 12a):**
1. Truncated `cose_encrypt0_bytes` → `EncryptedPayloadMalformed`.
2. `alg = 1` (A128GCM) instead of `3` → `EncryptedPayloadMalformed`.
3. Non-empty `unprotected` map → `EncryptedPayloadMalformed`.
4. `kem_ct` length 1087 (one byte short) → `EncryptedPayloadMalformed`.
5. `ecdh_eph_pk` length 31 → `EncryptedPayloadMalformed`.
6. `IV` length 11 → `EncryptedPayloadMalformed`.
7. Unknown extra text-keyed header label → `EncryptedPayloadMalformed`.

**Registry + key lookup (step 13):**
8. Protected-header `kid` not in registry → `UnknownRealmKid`.
9. Registry entry `not_after` in the past → `RealmKeyOutOfWindow`.
10. Registry entry `realm` differs from `service_realm` → `RealmKeyRealmMismatch`.

**Decryption (step 14):**
11. Last byte of GCM tag flipped → `DecryptionFailed`.
12. One byte of `kem_ct` flipped → `DecryptionFailed`.
13. One byte of `ecdh_eph_pk` flipped → `DecryptionFailed`.
14. Valid ciphertext but `claims.config_uuid` (part of AAD input) differs between mint and verify context → `DecryptionFailed` (AAD mismatch indistinguishable from tag tamper; by design).

**Inner-realm check (step 15):**
15. Decryption succeeds but plaintext inner `realm = "sql"` while `claims.realm = "llm"` → `InnerRealmMismatch`.

Assert each via `assert_eq!` on the exact variant (including
payload fields where applicable; match Wave A's discipline).

## Dependencies

### `philharmonic-connector-client/Cargo.toml`

Already-pinned from Wave A (no change):

- `philharmonic-connector-common = "0.2"`
- `philharmonic-types = "0.3.5"`
- `ed25519-dalek = "2"`
- `coset = "0.4"`
- `ciborium = "0.2"`
- `zeroize = { version = "1", features = ["derive"] }`
- `thiserror = "2"`

New for Wave B:

- `ml-kem = "0.2"` — latest `0.2.3`. FIPS 203 ML-KEM-768. Use the
  internal-randomness surface for deterministic tests.
- `x25519-dalek = "2"` — latest `2.0.1`.
- `hkdf = "0.12"` or `"0.13"` — latest `0.13.0`; check that
  `aes-gcm 0.10` / `sha2 0.11` don't fracture on `rand_core` +
  `digest` versions before pinning the major.
- `aes-gcm = "0.10"` — latest `0.10.3`.
- `sha2 = "0.11"` — already transitive via philharmonic-types;
  declare directly since we use it.
- `secrecy = "0.10"` — for `SecretBox<[u8; 32]>` around the AEAD
  key / HKDF PRK.
- `rand_core = "0.6"` or whatever the ecosystem demands — flag
  if it fractures (Gate-1 Q#6).

`[dev-dependencies]`:

- `hex = "0.4"` — unchanged.

### `philharmonic-connector-service/Cargo.toml`

Already-pinned from Wave A:

- `philharmonic-connector-common = "0.2"`
- `philharmonic-types = "0.3.5"`
- `ed25519-dalek = "2"`
- `coset = "0.4"`
- `ciborium = "0.2"`
- `sha2 = "0.11"`
- `subtle = "2"`
- `thiserror = "2"`

New for Wave B: same hybrid-KEM + AEAD + HKDF + zeroize stack as
the client.

### `philharmonic-connector-router/Cargo.toml`

Pick: `tokio = "1"` (rt-multi-thread, macros), `axum = "0.8"` or
`hyper = "1"` + `hyper-util`, `tower = "0.5"`. Look up latest
before pinning. Small surface; minimal deps.

**Version-lookup rule:** every pin above is a hint. Run
`./scripts/xtask.sh crates-io-versions -- <crate>` and use the
actual latest for the minor you're selecting. Third time this
rule has caught a stale pin this week (coset, Sha256 CBOR, ureq);
don't repeat the pattern.

## Module layout (all three crates; tune if needed)

### `philharmonic-connector-client/src/`

```
lib.rs          // re-exports + crate docs
signing.rs      // Wave A (unchanged)
encrypt.rs      // Wave B: LowererEncryption, encrypt_payload
error.rs        // MintError (Wave A) + EncryptError (Wave B)
```

### `philharmonic-connector-service/src/`

```
lib.rs
registry.rs     // MintingKeyRegistry (Wave A, unchanged)
realm_keys.rs   // RealmPrivateKeyEntry + RealmPrivateKeyRegistry (Wave B)
verify.rs       // Wave A 1-11 (unchanged surface; optional chain into decrypt)
decrypt.rs      // Wave B 12-15
context.rs      // Wave A build_call_context (unchanged)
error.rs        // TokenVerifyError extended with Wave B variants
```

### `philharmonic-connector-router/src/`

```
lib.rs          // crate docs
main.rs         // minimal async entrypoint (for the bin)
dispatch.rs     // forwarder logic
config.rs       // bytes-only Upstreams struct; no file I/O
```

Tests under `tests/` in each crate — match the Wave A pattern.

## Workspace conventions (authoritative:
`docs/design/13-conventions.md`)

Repeat of Wave A's list; the rules haven't changed:

- Edition 2024, MSRV 1.88.
- License `Apache-2.0 OR MPL-2.0`.
- `thiserror` for library errors. No `anyhow`.
- **No panics in library code.** Every `src/**/*.rs` path —
  no `.unwrap()` / `.expect()` / `panic!` / `unreachable!` /
  `todo!` / `unimplemented!` on reachable paths, no unbounded
  indexing, no unchecked integer arithmetic, no lossy `as`
  casts. Tests can `.unwrap()` freely.
- **Library crates take bytes, not file paths.** The two
  registries + the encrypt / decrypt APIs all take already-
  parsed bytes / typed values. No `&Path`, no `std::fs`, no
  config-file parsing.
- **Re-export discipline.** What's in each crate's public API,
  nothing more.
- **Rustdoc on every `pub` item.**
- **No `unsafe`.** Not in any of the three crates.
- **Use `./scripts/*.sh` wrappers.** No raw `cargo`. The
  pre-landing rules above describe exactly which scripts.

## Zeroization (from proposal §Zeroization points)

- `kem_ss: Zeroizing<[u8; 32]>` — dropped after HKDF-Extract.
- `ecdh_ss: Zeroizing<[u8; 32]>` — dropped after HKDF-Extract.
- HKDF PRK: `Zeroizing<[u8; 32]>` — dropped after HKDF-Expand.
- AEAD key: `SecretBox<[u8; 32]>` from `secrecy`.
- ML-KEM sk in `RealmPrivateKeyEntry.kem_sk: Zeroizing<[u8; 2400]>`.
- X25519 `StaticSecret` / `EphemeralSecret` — zeroize on drop
  natively.
- No key material leaves `Zeroizing` / `SecretBox` without an
  inline comment justifying it.

Flag any path where key material might linger in an un-zeroized
container.

## Pre-landing

```sh
./scripts/pre-landing.sh philharmonic-connector-client
./scripts/pre-landing.sh philharmonic-connector-service
./scripts/pre-landing.sh philharmonic-connector-router
./scripts/miri-test.sh philharmonic-connector-client
./scripts/miri-test.sh philharmonic-connector-service
./scripts/miri-test.sh philharmonic-connector-router
```

All six must pass before you conclude. Miri noise in third-party
crates (`ml-kem`, `aes-gcm`, `hkdf`) — flag but don't patch
upstream. Miri noise in our code — fix.

## Git

You do NOT commit, push, branch, tag, or publish. Leave the
working tree dirty in all three submodules. Claude runs the
`scripts/commit-all.sh` and `push-all.sh` after Gate-2 review.

## Deliverables

1. `philharmonic-connector-client/src/encrypt.rs` + `src/error.rs` updates + `src/lib.rs` re-exports + `Cargo.toml` deps + `CHANGELOG.md` [Unreleased] entry.
2. `philharmonic-connector-client/tests/encryption_vectors.rs` — KAT vs committed Wave B hex.
3. `philharmonic-connector-service/src/realm_keys.rs` + `src/decrypt.rs` + `src/error.rs` updates + `src/verify.rs` chaining + `src/lib.rs` re-exports + `Cargo.toml` deps + `CHANGELOG.md` [Unreleased] entry.
4. `philharmonic-connector-service/tests/decryption_vectors.rs` — positive + 15 negative vectors.
5. `philharmonic-connector-service/tests/e2e_roundtrip.rs` — end-to-end composition test (mint + encrypt → verify + decrypt, byte-for-byte vs committed vectors).
6. `philharmonic-connector-router/src/**/*.rs` — minimal dispatcher + `Cargo.toml` + `README.md` + `CHANGELOG.md` + one mock test.
7. All six pre-landing / miri runs clean.

## Structured output contract

Report in your final summary:

- Files changed per crate (paths + approximate line counts).
- Dependency pins actually chosen (verified via `./scripts/xtask.sh crates-io-versions` on dispatch day; list the numbers).
- Confirmation that every committed hex file in `docs/crypto-vectors/wave-{a,b}/` was loaded as-is, not regenerated.
- Byte-for-byte match vs every committed Wave B hex (enumerate them; any mismatch = detailed divergence report, not a silent fix).
- All 15 negative vectors pass with the exact variant.
- End-to-end roundtrip plaintext equals the input 119-byte JSON.
- `pre-landing.sh` + `miri-test.sh` results for all three crates.
- Zeroization sanity: every key-material variable lives in
  `Zeroizing` / `SecretBox` / a `Zeroize`-implementing type
  until drop.
- Any ambiguity you resolved and the call you made. Any
  deviation from the proposal — there should be none without an
  explicit flag.
- Router framework choice (axum vs hyper vs other) + one-line
  justification.

## Default follow-through policy

Complete the full task end-to-end. Do not stop at a "ready for
review" checkpoint if pre-landing + miri + all KAT + all
negative vectors + end-to-end roundtrip pass cleanly. If you hit
a blocker (vector mismatch, clippy / miri flagging an unfixable
issue, a dep pin that fractures), stop and report — do not
commit workarounds, do not silence lints, do not regenerate
vectors.

## Completeness contract

"Done" means:

- All three crates compile cleanly with `clippy -D warnings`.
- Every committed Wave B hex matches byte-for-byte.
- All 15 negative vectors reject with the exact specified
  variant.
- End-to-end roundtrip test passes (decrypted plaintext equals
  input).
- Miri passes on all three crates (modulo upstream noise you
  flag).
- Rustdoc complete on every `pub` item.
- README + CHANGELOG written for all three crates.
- No `.unwrap()` / `.expect()` / `panic!*` / `unsafe` in any
  `src/`.
- No file I/O in any library (router's `main.rs` is allowed a
  minimal arg parse for upstreams; extract to a bytes-taking lib
  fn where it matters).

Partial state is a blocker, not a deliverable.

## Verification loop

```sh
./scripts/rust-lint.sh philharmonic-connector-client
./scripts/rust-lint.sh philharmonic-connector-service
./scripts/rust-lint.sh philharmonic-connector-router
./scripts/rust-test.sh philharmonic-connector-client
./scripts/rust-test.sh philharmonic-connector-service
./scripts/rust-test.sh philharmonic-connector-router
```

Then full pre-landing + miri before concluding.

## Missing-context gating

Specific pressure points where STOP-and-ask is the right move:

- `ml-kem 0.2.x`'s exact API for deterministic keygen /
  encapsulate. The crate may expose the FIPS 203 internal
  surface under different names (`keygen_internal`,
  `encaps_internal`, or via explicit `MlKem768::try_from_seed`
  or similar). The vector generator used `kyber-py`'s `_keygen_internal(d, z)` /
  `_encaps_internal(ek, m)`; if the Rust crate diverges, flag
  the exact API surface so Claude can cross-check.
- HKDF `info` string endianness / encoding. It's ASCII as
  specified; should not ambiguity. Flag if your crate's HKDF
  builder forces something unexpected.
- AEAD AAD wrapping semantics. We specified the AEAD's AAD as
  the full `Enc_structure` canonical CBOR. Confirm `aes-gcm`'s
  `encrypt` / `decrypt` methods let you pass arbitrary AAD
  bytes — which they do via `Payload { msg, aad }` or the
  low-level nonce-key-AAD API.
- `x25519-dalek`'s `StaticSecret::from(&[u8; 32])` is the
  deterministic constructor for test vectors. Flag if 2.x
  changed this API in a breaking way.

## Action safety

No destructive git operations. No cargo publish. No push. No
branch creation. No tag operation. No modification to files
outside the three submodule crates except for test fixtures
inside `tests/` and documentation in each crate's own README /
CHANGELOG. The reference vectors in `docs/crypto-vectors/` are
read-only for this dispatch — if you think they're wrong,
report; do not edit.

## Outcome

Pending — will be updated after the Codex run completes. Expected
content: summary of what landed, file counts across all three
crates, pre-landing + miri verdicts, byte-for-byte vector match
confirmation, end-to-end roundtrip result, router framework
choice, any flagged ambiguity or dep-graph friction (especially
around `rand_core` versions), residual risks.
