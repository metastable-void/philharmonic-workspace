# Gate-1 proposal — Phase 5 Wave A: COSE_Sign1 connector authorization tokens

**Date:** 2026-04-22
**Revision:** 3 (revised 2026-04-22 after Yuka's Gate-1 sign-off)
**Phase:** 5 (connector triangle), Wave A (signing-only half)
**Author:** Claude Code (on Yuka's review queue)
**Status:** **Gate-1 approved 2026-04-22** — implementation unblocked
**Approval record:** `docs/design/crypto-approvals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md`
**Wave split decision:** ROADMAP §Phase 5 (commit `e292918`)
**Security review:** `docs/codex-reports/2026-04-22-0003-phase-5-wave-a-cose-sign1-tokens-security-review.md`.
Resolutions are summarized in §"Codex security review resolutions";
the body is updated inline.

## Revisions

- **r1** (2026-04-22): first draft.
- **r2** (2026-04-22): applied six of seven findings from Codex's
  independent design-level security review; documented the
  stateless-replay threat-model decision.
- **r3** (2026-04-22): Gate-1 approved by Yuka with a workspace-
  level caveat — library crates take bytes, not file paths.
  Refactored the minting-side and registry sections accordingly
  (the `philharmonic-connector-client` and
  `philharmonic-connector-service` lib APIs accept bytes; file I/O
  and permission checks live in the respective bin crates). Open
  Questions resolved: Q1 → option (a), Q2 → pycose 2.x approved.
  The workspace rule itself landed in
  `docs/design/13-conventions.md` §Library crate boundaries and
  a short bullet in `CLAUDE.md`.

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

Per workspace convention §Library crate boundaries, file I/O and
permission checks live in the lowerer **bin** crate. The
`philharmonic-connector-client` **library** accepts the seed
bytes and nothing filesystem-shaped.

**Library API** (`philharmonic-connector-client`):

```rust
pub struct LowererSigningKey {
    seed: Zeroizing<[u8; 32]>,
    kid: String,
}

impl LowererSigningKey {
    /// Construct from a 32-byte Ed25519 seed. The seed is owned
    /// by the `Zeroizing` wrapper and zeroed on drop.
    pub fn from_seed(seed: Zeroizing<[u8; 32]>, kid: String) -> Self;

    pub fn kid(&self) -> &str;

    pub fn mint_token(
        &self,
        claims: &ConnectorTokenClaims,
    ) -> Result<ConnectorSignedToken, MintError>;
}
```

Per the resolution of Open Question #1 (option a, approved
2026-04-22), the library holds the raw 32-byte seed in
`Zeroizing<[u8; 32]>` and reconstructs a transient
`ed25519_dalek::SigningKey` via `SigningKey::from_bytes` on
every `mint_token` call. Ed25519 key schedule is cheap; at
lowerer throughput (one sign per connector call) the overhead
is negligible and zeroization is guaranteed by
`Zeroizing::drop`.

**Binary responsibilities** (lowerer bin crate — wherever the
lowerer's `main` lives):

- Read the configured seed source (typically a file path from
  the lowerer's config; could equally be an env var, a KMS
  fetch, or stdin in a test harness).
- If the source is a filesystem path, perform a
  **file-permission check** before the read. On `cfg(unix)`,
  `stat(2)` the file and fail closed unless:
  - the owning uid matches the current process's uid (i.e. the
    key file isn't owned by someone else), AND
  - `mode & 0o077 == 0` (no group or other bits — matching the
    0600 / 0400 class).
  Non-compliant permissions → the bin refuses to start,
  surfacing the file path and the observed mode in an operator-
  readable error. Analogous to OpenSSH's client file-mode
  refusals. On Microsoft Windows (not a supported production
  host; see conventions), the bin may skip the check with a
  warning.
- Read bytes into `Zeroizing<[u8; 32]>` (the bin can use a
  scratch `Vec<u8>`, `copy_from_slice` into the zeroized
  buffer, and drop the `Vec` — or go straight into the fixed
  buffer if the reader allows).
- Hand the `Zeroizing<[u8; 32]>` to
  `LowererSigningKey::from_seed`, which now owns the zeroizing
  buffer for the remainder of the process lifetime.

This split keeps `philharmonic-connector-client` unit-testable
with in-memory seeds (the RFC 8032 test vector seed for Wave A's
known-answer tests), free of filesystem-portability concerns,
and composable with non-file secret sources.

### Verifying side (service)

Same split: file I/O and config parsing live in the service
**bin**; the `philharmonic-connector-service` **library**
exposes a programmatic registry.

**Library API** (`philharmonic-connector-service`):

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
    pub fn new() -> Self;
    pub fn insert(&mut self, kid: String, entry: MintingKeyEntry);
    pub fn lookup(&self, kid: &str) -> Option<&MintingKeyEntry>;
}
```

Public keys are **not** sensitive and don't need zeroization.

The library ships no `load_from_file`, `load_from_toml`, or
similar. The service **bin** is responsible for whatever
configuration format it chooses (TOML, JSON, env-derived, KMS-
backed) — it parses that format, constructs `MintingKeyEntry`
values, and calls `insert` per minting authority at boot. This
keeps the library independent of any particular config
serialization choice, and keeps Wave A's unit tests free of
fixture files.

Rotation is additive: a new kid gets a new `MintingKeyEntry`
inserted; old kids stay registered until all in-flight tokens
issued under them have expired. `not_before` / `not_after` are
**enforced** at verification time (see verify step 4 above) —
they are not advisory. A future-dated key cannot be used early,
and an operator can retire a kid cleanly by setting
`not_after` in the past without synchronously removing the
entry.

### `kid` encoding

`kid` is a free-form UTF-8 string, signed as part of the
protected header. Suggested format: `<issuer-slug>-<utc-date>-<rand-hex-8>`
(e.g. `lowerer.main-2026-04-22-3c8a91d0`). Not pinned as a
wire format — the registry uses exact-string equality.

## Zeroization points

**Private keys only** (public keys need no zeroization):

- `Zeroizing<[u8; 32]>` owns the 32-byte Ed25519 seed. The
  lowerer bin allocates and populates it (see §"Minting side")
  and hands ownership to `LowererSigningKey::from_seed`. The
  library holds the `Zeroizing` wrapper for its lifetime; when
  the `LowererSigningKey` drops, the seed is zeroed.
- Every `mint_token` call reconstructs a transient
  `ed25519_dalek::SigningKey` via
  `SigningKey::from_bytes(seed.as_ref())` and drops it at end-
  of-call. This is the approved pattern per Open Question #1
  (option a, resolved 2026-04-22): `ed25519_dalek::SigningKey`
  does not itself zeroize on drop in 2.x, so we never keep one
  around — the authoritative copy of key material lives only
  in our `Zeroizing` wrapper.
- Signing-time intermediates (the `r` nonce in Ed25519) are
  derived inside `ed25519-dalek` and aren't exposed to our
  code, so there's nothing for us to zero. The library is the
  trusted boundary here.

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

3. **Key handling that can't be zeroized.** None in the
   landed design. `ed25519_dalek::SigningKey` (2.x) doesn't
   itself implement `Zeroize`, so we never hold one longer
   than a single `mint_token` call — the authoritative copy of
   key material lives in our `Zeroizing<[u8; 32]>` wrapper,
   and the transient `SigningKey` is reconstructed per sign.
   See §"Resolved open questions" #1.

4. **Signatures over untrusted input.** The sign side takes
   trusted input (engine-assembled claim values), so
   straightforward. The verify side takes
   attacker-controlled COSE_Sign1 bytes and
   attacker-controlled payload bytes. Signature verification
   gates everything — no claim field is trusted before the
   signature check passes, which is standard COSE /
   JWT-equivalent discipline.

## Resolved open questions

All three Open Questions from r1/r2 are resolved. Retained here
for archaeology.

1. **Ed25519 private-key zeroization → option (a).** Resolved
   2026-04-22 by Yuka at Gate-1. Hold the raw 32-byte seed in
   `Zeroizing<[u8; 32]>`; reconstruct the transient
   `ed25519_dalek::SigningKey` via `from_bytes` on every sign
   call. One key schedule per sign; at Wave A lowerer
   throughput (one sign per connector call) the overhead is
   negligible, and zeroization is guaranteed by
   `Zeroizing::drop`. Rejected alternatives: caching a
   `SigningKey` with a custom drop-zero wrapper (fragile —
   requires access to `ed25519-dalek` internals we don't
   control), and accepting process-lifetime residency
   (weakest).

2. **`pycose` 2.x as cross-check reference → approved.**
   Resolved 2026-04-22 by Yuka at Gate-1. Vector generation
   and cross-implementation cross-check will run Python
   `pycose` 2.x against the Rust implementation for the final
   COSE_Sign1 bytes.

3. **`subtle` crate for constant-time `payload_hash`
   compare → approved.** Resolved in r2. Use `subtle = "2"`
   for the `payload_hash` equality in verify step 10. Already
   in the dep tree transitively via `ed25519-dalek`, so no new
   runtime surface.

## What lands (Wave A)

Library source files (no code written yet):

- `philharmonic-connector-client/src/signing.rs` —
  `LowererSigningKey` (`from_seed(Zeroizing<[u8;32]>, kid)`)
  and `mint_token`. No file I/O.
- `philharmonic-connector-service/src/verify.rs` —
  `MintingKeyRegistry` (`new` / `insert` / `lookup`) and the
  `verify_token` function. No file I/O, no config-file
  parsing.
- `philharmonic-connector-service/src/context.rs` — the
  verified `ConnectorCallContext` construction from claim
  fields.
- Tests: `philharmonic-connector-client/tests/signing_vectors.rs`,
  `philharmonic-connector-service/tests/verify_vectors.rs`,
  `tests/vectors/*.hex` / `*.json` committed alongside. All
  tests feed bytes in directly — no fixture files on disk at
  the library boundary.

What does **not** land in Wave A:

- File-reading / permission-checking code for the lowerer seed
  file. That's a lowerer-bin concern and lands with the
  lowerer binary (not yet scoped in this workspace —
  `philharmonic-lowerer` is a later phase).
- Config-file parsing for the service's `MintingKeyRegistry`.
  Same reason: service-bin concern.
- No `ConfigLowerer` connector impl (that's Wave B, which
  needs encryption to produce the payload bytes).
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
| 5 | Signing-key file handling omits permission checks | MEDIUM | **Fixed.** 0600-class permission + matching-uid check before the file read on `cfg(unix)`. In r3 this check lives in the **lowerer bin**, not `philharmonic-connector-client` — the library now accepts a seed byte buffer and does no file I/O (see §"Minting side" and conventions §Library crate boundaries). |
| 6 | Unbounded payload hashing (DoS pressure) | MEDIUM | **Fixed.** Hard `MAX_PAYLOAD_BYTES` ceiling (default 1 MiB) enforced at verify step 5 before the SHA-256 work (`PayloadTooLarge`). |
| 7 | Duplicated `kid` in protected header and claims without equality check | LOW | **Fixed.** Equality check at verify step 8 (`KidInconsistent`). Keeping both locations because `ConnectorTokenClaims` ships in `philharmonic-connector-common 0.1.0` with `kid` already in the claim set; removing it would be an API break we don't want mid-v1. |

Six of seven findings are code / design fixes landed inline
above. The seventh (replay) is a deliberate threat-model
decision documented in §"Replay threat model". The report's
"positive notes" (signature-first ordering, protected-header
`alg`/`kid`, explicit negative vector planning) all carried
through into r2.

## Gate-1 outcome

Approved by Yuka on 2026-04-22. See
`docs/design/crypto-approvals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md`
for the approval record.

Approval included one workspace-level caveat — library crates
take bytes, not file paths. The r3 minting-side and registry
sections reflect that split; the general rule landed in
`docs/design/13-conventions.md` §Library crate boundaries and
as a bullet in `CLAUDE.md`.

Open Questions:

- **#1 zeroization approach** → option (a): seed in
  `Zeroizing<[u8; 32]>`, `SigningKey` reconstructed per sign.
- **#2 cross-check reference** → pycose 2.x approved.
- **#3 `subtle` for `payload_hash` compare** → approved in r2.

Replay threat-model (§"Replay threat model") and the 11-step
verification order (§"Service-side verification order") were
approved as proposed in r2.

## Next steps

1. Archive the Codex implementation prompt for Wave A at
   `docs/codex-prompts/YYYY-MM-DD-NNNN-phase-5-wave-a-...md`
   per the codex-prompt-archive skill. The prompt must link
   this proposal and the approval record, spell out the 10
   negative vectors, and carry the test-vector seed + claim
   set as committed hex (so Codex is not generating the
   reference values it's being verified against).
2. Dispatch Codex via the `codex:codex-rescue` plugin.
3. Gate-2 review on the returned code before any
   `cargo publish`. The crates stay at `0.0.0` through Wave A;
   publish happens when Wave B lands and end-to-end tests pass.
