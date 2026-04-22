# Gate-1 proposal — Phase 5 Wave A: COSE_Sign1 connector authorization tokens

**Date:** 2026-04-22
**Revision:** 2 (revised 2026-04-22 after Codex's security review)
**Phase:** 5 (connector triangle), Wave A (signing-only half)
**Author:** Claude Code (on Yuka's review queue)
**Status:** Awaiting Gate-1 sign-off — no implementation yet
**Wave split decision:** ROADMAP §Phase 5 (commit `e292918`)
**Security review:** `docs/codex-reports/2026-04-22-0003-phase-5-wave-a-cose-sign1-tokens-security-review.md`.
Resolutions are summarized in §"Codex security review resolutions"
at the end of this document; the body has been updated inline.

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

## Replay threat model

The Wave A verification path is deliberately stateless — no
`jti`, no server-side replay cache. This is a considered choice,
not an oversight. Codex's security review flagged the absence as
a HIGH finding; the finding is factually correct, and we accept
the narrow threat in exchange for statelessness.

### Threats considered

1. **External wire-level replay / MITM.** Mitigated by TLS. The
   `philharmonic-connector-router` terminates TLS at
   `<realm>.connector.<domain>` and forwards to service
   instances on the internal network. An external attacker can't
   intercept a valid `(token, payload)` pair in the first place.

2. **Log-based replay.** If an attacker obtains historical
   `(token, payload)` pairs from logs, audit trails, or a
   breached observability stack, they can replay them while
   `exp` is still in the future. Mitigated by a tight `exp`
   window — default 120 seconds from mint. The attack window
   closes within seconds of natural token expiry.

3. **Internal compromise** (rogue operator with wire access, a
   compromised service replica). Anyone who has the capability
   to replay at this level already has the capability to mint
   new tokens or bypass the connector layer entirely. Replay
   detection buys nothing against this threat.

4. **Accidental double-fire** (network retry, lost ack, client
   reconnect). This is the one realistic case where the same
   `(token, payload)` hits the service twice. It's a
   **correctness** concern (re-executed side effects), not a
   security concern. Addressed at the implementation layer via
   protocol-native idempotency: `http_forward` can thread
   `instance_id` / `step_seq` as idempotency headers; `email_send`
   uses RFC 5322 `Message-Id`; payment connectors use
   vendor-specific idempotency keys; `sql_query` is the caller's
   problem (SQL has transactions). The framework exposes
   `instance_id`, `step_seq`, `config_uuid` in
   `ConnectorCallContext` specifically so each impl can derive
   a natural idempotency key.

### Why not a server-side jti cache

A `jti` claim plus a server-side replay cache would close the
log-based replay window before `exp`. Costs:

- Breaks the **"stateless where feasible"** workspace principle.
- Requires a distributed cache across service replicas (Redis
  or equivalent) for HA; adds a hard runtime dep.
- Cache memory and TTL management become operational concerns
  (eviction policy, memory ceilings, cross-region consistency).
- Marginal benefit over tight `exp` + TLS + protocol-layer
  idempotency.

The tradeoff goes the other way. Stateless stays.

### Properties this decision commits to

- **`exp` is mandatory and defaults to 120 s** from mint time.
  Lowerers that want a shorter window can configure it; longer
  windows require explicit justification and operator sign-off.
- **TLS is mandatory** on all legs: lowerer → router, router →
  service. Plaintext HTTP between any two legs is out of scope
  for v1 and SHOULD be rejected at config-load time in
  production deployments.
- **Per-impl idempotency is documented** in the
  `philharmonic-connector-impl-*` crate READMEs. An impl that
  performs non-idempotent side effects without an
  idempotency-key mechanism is considered to have a bug — the
  framework is not responsible for the duplicate behavior.

This section exists so the threat-model decision is explicit
and re-reviewable later. If the deployment model changes (e.g.
we add a third-party hosted router, or weaken TLS assumptions),
the replay model should be revisited.

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

The service-side `verify_token(cose_bytes, payload_bytes,
service_realm)` runs checks in this order, stopping at the
first failure. Ordering is deliberate: algorithm and key-level
checks fail before expensive crypto, signature verification
fails before any untrusted payload content is trusted, and all
content-level checks run over verified claim bytes.

1. **Parse** the COSE_Sign1 bytes via
   `coset::CoseSign1::from_slice`. Malformed → reject
   (`TokenVerifyError::Malformed`).

2. **Pin algorithm.** Read `alg` from the protected header;
   require `alg == -8` (EdDSA per RFC 9053 §2.2). Any other
   value rejects as `TokenVerifyError::AlgorithmNotAllowed`.
   This is defense-in-depth against COSE / JWT-style
   algorithm-confusion regressions if a dependency changes
   behavior.

3. **Kid lookup.** Extract `kid` from the protected header;
   look up the verifier key in the `MintingKeyRegistry`.
   Unknown kid → `TokenVerifyError::UnknownKid`.

4. **Key validity window.** The registry entry carries
   `not_before` / `not_after` (`UnixMillis`). Reject if `now <
   not_before` or `now >= not_after`
   (`TokenVerifyError::KeyOutOfWindow`). Operators that want a
   kid immediately inactive can retire it by removing the
   entry; the window check catches future-dated keys accepted
   early and retired keys that weren't removed.

5. **Payload size ceiling.** Before hashing, enforce
   `payload_bytes.len() <= MAX_PAYLOAD_BYTES`. Default is
   `1_048_576` (1 MiB), configurable per service. Oversize →
   `TokenVerifyError::PayloadTooLarge { limit, actual }`.
   Keeps the SHA-256 work attacker-bounded.

6. **Signature verification.** Use
   `coset::CoseSign1::verify_signature` with the Ed25519
   verifying key from step 3. Bad signature →
   `TokenVerifyError::BadSignature`. **No claim content is
   trusted before this step passes.**

7. **Claim payload decode.** Decode the claim payload from
   CBOR into `ConnectorTokenClaims`. Malformed → treat as
   `TokenVerifyError::Malformed` (the signature was valid over
   something that still didn't match our schema, which means
   either a version skew or a subtle encoding drift — reject
   either way).

8. **Kid consistency.** Require `claims.kid ==
   protected.kid` (both are signature-covered, but duplication
   invites drift). Mismatch →
   `TokenVerifyError::KidInconsistent`. Cheap; catches schema
   bugs and forensic-log confusion before they propagate.

9. **Expiry.** Check `claims.exp > UnixMillis::now()`. Expired
   → `TokenVerifyError::Expired`.

10. **Payload-hash binding.** Compute
    `SHA-256(payload_bytes)`. Compare in constant time with
    `claims.payload_hash` via `subtle::ConstantTimeEq`
    (Open Question #3 resolution: we pull `subtle = "2"`).
    Mismatch → `TokenVerifyError::PayloadHashMismatch`.

11. **Realm binding (mandatory).** Check `claims.realm ==
    service_realm`. Mismatch →
    `TokenVerifyError::RealmMismatch`. The service knows its
    own realm at boot; it is never a caller-optional check.
    This closes the cross-realm / audience-confusion vector
    that Codex's review flagged as a HIGH issue.

Only after all eleven pass is a verified `ConnectorCallContext`
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
- **File-permission check before the read.** On Unix
  (`cfg(unix)`), `stat(2)` the file and fail closed unless:
  - the owning uid matches the current process's uid (i.e. the
    key file isn't owned by someone else), AND
  - the mode's group and other bits are both zero (i.e.
    `mode & 0o077 == 0`, matching the 0600 / 0400 class).
  Non-compliant permissions → the lowerer refuses to start with
  `SigningKeyFilePermissions` error, naming the file and the
  observed mode. On Windows (not a supported production host;
  see conventions), skip the check with a warning. This catches
  world- or group-readable key files, which on multi-user or
  misconfigured hosts are the most common secret-exfiltration
  path. (The check is analogous to OpenSSH's client
  `StrictHostKeyChecking`-adjacent file-mode refusals.)
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
pub struct MintingKeyEntry {
    pub vk: ed25519_dalek::VerifyingKey,
    pub not_before: UnixMillis,
    pub not_after: UnixMillis,
}

pub struct MintingKeyRegistry {
    by_kid: HashMap<String, MintingKeyEntry>,
}

impl MintingKeyRegistry {
    pub fn lookup(&self, kid: &str) -> Option<&MintingKeyEntry>;
    pub fn insert(&mut self, kid: String, entry: MintingKeyEntry);
}
```

Public keys are **not** sensitive and don't need zeroization.

Registered at boot from a config file (one entry per minting
authority, each with `kid`, `public_key_hex`, `not_before`,
`not_after`). Rotation is additive: a new kid gets a new
`MintingKeyEntry` inserted; old kids stay registered until
all in-flight tokens issued under them have expired.
`not_before` / `not_after` are **enforced** at verification
time (see verify step 4 above) — they are not advisory. This
means a future-dated key cannot be used early, and an operator
can retire a kid cleanly by setting `not_after` in the past
without needing to synchronously remove the entry.

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

One vector per rejection reason in the verification order. Each
must fail with the specific `TokenVerifyError` variant named.

- `wave_a_bad_alg.hex` — same claims + key, but protected
  header re-encoded with `alg = -7` (ES256). Must fail at
  step 2 with `AlgorithmNotAllowed`.
- `wave_a_unknown_kid.hex` — `kid` in the protected header
  replaced with a kid not in the registry. Step 3,
  `UnknownKid`.
- `wave_a_key_out_of_window.hex` — valid token but the
  registry entry for that kid has `not_after` in the past.
  Step 4, `KeyOutOfWindow`.
- `wave_a_payload_too_large.hex` — payload_bytes of size
  `MAX_PAYLOAD_BYTES + 1`. Step 5, `PayloadTooLarge`.
- `wave_a_tampered_sig.hex` — last byte of the signature
  flipped. Step 6, `BadSignature`.
- `wave_a_tampered_payload.hex` — one byte of the claim
  payload flipped. Step 6, `BadSignature` (the signature no
  longer covers the modified payload).
- `wave_a_kid_inconsistent.hex` — protected header `kid` and
  `claims.kid` differ. Step 8, `KidInconsistent`. (Synthesized
  by signing a CBOR-encoded claim with `claims.kid = "A"` but
  placing `"B"` in the protected header — signature valid but
  the two kids mismatch.)
- `wave_a_expired.hex` — `exp` set to 1 (long in the past).
  Step 9, `Expired`.
- `wave_a_payload_hash_mismatch.hex` — valid signature over
  one claim's `payload_hash`, but service verifies with
  different payload bytes of the same length. Step 10,
  `PayloadHashMismatch`.
- `wave_a_realm_mismatch.hex` — `claims.realm = "llm"` but
  service_realm is `"sql"`. Step 11, `RealmMismatch`.

Ten negative vectors, one per verification step that has a
reject path.

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

3. ~~**`subtle` crate for constant-time `payload_hash`
   compare.**~~ **Decided (r2):** use `subtle = "2"` for the
   `payload_hash` equality in verify step 10. Already in the
   dep tree transitively via `ed25519-dalek`, so no new runtime
   surface. Closing.

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

## Codex security review resolutions

Codex ran an independent design-level security review of r1 of
this proposal. The full report is at
`docs/codex-reports/2026-04-22-0003-phase-5-wave-a-cose-sign1-tokens-security-review.md`.
Seven findings; Claude's evaluation and the r2 resolution per
finding:

| # | Finding | Severity | r2 resolution |
|---|---------|----------|---------------|
| 1 | Replay resistance not specified (no `jti`, no server-side cache) | HIGH | **Accept the risk, document explicitly.** Threat is narrow given TLS on every leg, tight 120 s `exp`, and protocol-layer idempotency as the impl's responsibility. Server-side `jti` cache breaks "stateless where feasible" and adds an HA cache dep for marginal benefit. See §"Replay threat model". |
| 2 | `not_before` / `not_after` on registry entries defined but not checked at verify | HIGH | **Fixed.** Enforced at verify step 4 (`KeyOutOfWindow`). Registry now carries an explicit `MintingKeyEntry` with the window. |
| 3 | No audience binding; optional `realm` check; weak issuer-key binding | HIGH | **Partially fixed.** Realm check is now mandatory at verify step 11 (`RealmMismatch`). No `aud` claim added — `realm` acts as audience in this architecture. Issuer-bound registry entries not adopted for v1 (small benefit, adds operator config; revisit if a concrete issuer-confusion scenario surfaces). |
| 4 | `alg` not explicitly pinned on verify | MEDIUM | **Fixed.** Explicit `alg == -8` (EdDSA) check at verify step 2 (`AlgorithmNotAllowed`). |
| 5 | Signing-key file handling omits permission checks | MEDIUM | **Fixed.** 0600-class permission + matching-uid check before the file read on `cfg(unix)`. `SigningKeyFilePermissions` error on mismatch. |
| 6 | Unbounded payload hashing (DoS pressure) | MEDIUM | **Fixed.** Hard `MAX_PAYLOAD_BYTES` ceiling (default 1 MiB) enforced at verify step 5 before the SHA-256 work (`PayloadTooLarge`). |
| 7 | Duplicated `kid` in protected header and claims without equality check | LOW | **Fixed.** Equality check at verify step 8 (`KidInconsistent`). Keeping both locations because `ConnectorTokenClaims` ships in `philharmonic-connector-common 0.1.0` with `kid` already in the claim set; removing it would be an API break we don't want mid-v1. |

Six of seven findings are code / design fixes landed inline
above. The seventh (replay) is a deliberate threat-model
decision documented in §"Replay threat model". The report's
"positive notes" (signature-first ordering, protected-header
`alg`/`kid`, explicit negative vector planning) all carried
through into r2.

## Requesting Gate-1 approval

For this proposal to unblock dispatch:

- Confirm or adjust the two remaining Open Questions (#1
  zeroization approach, #2 cross-check reference). #3 is
  decided.
- Confirm the replay threat-model decision in §"Replay threat
  model" — stateless, exp-bound, impl-layer idempotency.
- Confirm the revised 11-step verification order and, in
  particular, the post-review additions (alg pin, key window,
  payload size ceiling, kid equality, mandatory realm).
- Flag any field of the claim set you want CBOR-encoded
  differently than the defaults (e.g. `Uuid` as text vs.
  bytes).
- Say "Gate-1 approved" and I'll archive the Codex prompt +
  dispatch. No code before that.
