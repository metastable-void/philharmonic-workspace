# Phase 5 Wave A — COSE_Sign1 connector authorization tokens

**Date:** 2026-04-22
**Slug:** `phase-5-wave-a-cose-sign1-tokens`
**Round:** 01
**Subagent:** `codex:codex-rescue`

## Motivation

Implement the Ed25519 + COSE_Sign1 half of the Phase 5 connector
triangle. Two crates receive code:

- `philharmonic-connector-client` — lowerer mints
  `ConnectorSignedToken` over `ConnectorTokenClaims`.
- `philharmonic-connector-service` — service verifies a token +
  caller-supplied payload bytes, returning a narrowed
  `ConnectorCallContext`.

This is a **Gate-1-approved** crypto-sensitive task. The approved
construction (primitives, verification order, zeroization, error
taxonomy) is frozen by the proposal; do not deviate without
flagging.

## Gate-1 status (freeze before writing code)

- **Proposal (authoritative):**
  `docs/design/crypto-proposals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md`
  (revision 4).
- **Approval record:**
  `docs/design/crypto-approvals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md`.
- **Security review** (addressed in r2):
  `docs/codex-reports/2026-04-22-0003-phase-5-wave-a-cose-sign1-tokens-security-review.md`.
- **Reference vectors (pre-committed, do NOT regenerate):**
  `docs/crypto-vectors/wave-a/` — your Rust implementation must
  reproduce `wave_a_claims.cbor.hex`, `wave_a_protected.hex`,
  `wave_a_signature.hex`, and `wave_a_cose_sign1.hex` byte-for-
  byte.

If the code you're about to write would contradict the proposal,
STOP and flag. The proposal wins; your implementation must match
it or the dispatch goes back to Claude.

## References (read before coding)

- `docs/design/crypto-proposals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md` — the authoritative construction spec.
- `docs/design/crypto-approvals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md` — Yuka's approval with the library-crates-take-bytes caveat.
- `docs/design/11-security-and-cryptography.md` — threat model and construction context.
- `docs/design/13-conventions.md` §Library crate boundaries — why both crates take bytes, not file paths.
- `docs/design/13-conventions.md` §Panics and undefined behavior — the no-panic rule is load-bearing on crypto paths.
- `philharmonic-connector-common 0.1.0` (source in `philharmonic-connector-common/`) — already-published upstream types (`ConnectorTokenClaims`, `ConnectorCallContext`, `ConnectorSignedToken`, `ImplementationError`).

## Scope

### In scope

**`philharmonic-connector-client`** (new code in `src/`):

- `LowererSigningKey` struct holding
  `Zeroizing<[u8; 32]>` seed + `kid: String`. Construct via
  `LowererSigningKey::from_seed(seed, kid)`. The library accepts
  **bytes**, not a file path — file I/O, permission checks, and
  config-file parsing are a lowerer-bin concern and are explicitly
  out of scope for Wave A. Per workspace convention (see
  `docs/design/13-conventions.md` §Library crate boundaries).
- `mint_token(&self, claims: &ConnectorTokenClaims) -> Result<ConnectorSignedToken, MintError>`.
  Serializes claims via `ciborium` (canonical CBOR per RFC 8949
  §4.2; struct-field declaration order), builds the COSE_Sign1
  protected header (`alg = -8` EdDSA, `kid = claims.kid` as
  bstr), constructs `Sig_structure1` per RFC 9052 §4.4, signs
  via `ed25519-dalek`, returns the wrapped `ConnectorSignedToken`.
- `MintError` (`thiserror`-derived) with variants for
  serialization failure, signing failure, and any input
  validation. No variant is panicable.
- Per-call `SigningKey` reconstruction: every `mint_token` call
  builds a transient `ed25519_dalek::SigningKey` via
  `SigningKey::from_bytes(seed.as_ref())` and drops it at end of
  call. See proposal §Zeroization points (option (a) was
  chosen). Do NOT cache a long-lived `SigningKey`.

**`philharmonic-connector-service`** (new code in `src/`):

- `MintingKeyEntry { vk, not_before, not_after }` plus a
  `MintingKeyRegistry` exposing `new()`, `insert(kid, entry)`,
  `lookup(&kid)`. Library accepts pre-parsed entries; config-
  file parsing stays in the service bin (not this crate).
- `verify_token(cose_bytes, payload_bytes, service_realm,
  registry, now)` performing the 11-step verification order
  exactly as specified in the proposal §"Service-side
  verification order". Short-circuit at the first failure. Each
  rejection maps to a distinct `TokenVerifyError` variant.
- `TokenVerifyError` (`thiserror`-derived) with ALL of:
  `Malformed`, `AlgorithmNotAllowed`, `UnknownKid { kid }`,
  `KeyOutOfWindow { now, not_before, not_after }`,
  `PayloadTooLarge { limit, actual }`, `BadSignature`,
  `KidInconsistent { protected, claims }`, `Expired { exp, now }`,
  `PayloadHashMismatch`, `RealmMismatch { expected, found }`. No
  variant is panicable.
- Successful verification returns `ConnectorCallContext` (from
  connector-common), constructed from the verified claim fields.
- Constant-time `payload_hash` comparison via the `subtle` crate
  (`ConstantTimeEq::ct_eq`). See proposal Open Q #3.
- Configurable `MAX_PAYLOAD_BYTES` constant, defaulting to
  `1_048_576` (1 MiB). Enforced at step 5 (BEFORE the hash).
  Expose as a named constant; also expose a way for the service
  bin to override (e.g. pass it to `verify_token` or hold it in a
  builder). Match the proposal.

**Tests (both crates):**

- Known-answer vector tests using the pre-committed values in
  `docs/crypto-vectors/wave-a/`. Load either via
  `include_str!("../../../docs/crypto-vectors/wave-a/wave_a_claims.cbor.hex")`
  followed by trim + `hex::decode`, or as inline `hex!(...)`
  literals. Either is fine.
- Positive path: sign with the committed seed + claims; assert
  the resulting bytes equal `wave_a_cose_sign1.hex`. Verify
  those same bytes with the committed `VerifyingKey`; assert
  successful `ConnectorCallContext` return with expected field
  values.
- Ten negative-path vectors, each constructed by perturbing the
  positive vector, each asserting the exact
  `TokenVerifyError` variant. See §"Negative vectors" below.
- Where useful, unit tests on submodule boundaries (e.g. alg
  pin rejects `-7`, kid-consistency check detects mismatch).

### Out of scope

- **No COSE_Encrypt0, no hybrid KEM, no HKDF, no AES-GCM.**
  Those are Wave B. This is signing + verification only.
- **No router, no transport, no real connector impls.**
- **No file I/O, no config-file parsing, no permission checks.**
  Those live in the lowerer / service bin crates, which are out
  of scope here.
- **No publish.** The two crates stay at `0.0.0`. Claude handles
  the version bump + publish after Wave B lands and end-to-end
  tests pass.

## Construction (binding)

From the proposal — read the full §Construction and §Service-side
verification order sections; the summaries below are for your
quick reference, not substitutes.

### Token shape (CBOR payload of the COSE_Sign1)

The payload is `ciborium`-serialized `ConnectorTokenClaims`,
declaration order of fields:

`iss`, `exp`, `kid`, `realm`, `tenant`, `inst`, `step`,
`config_uuid`, `payload_hash`.

Wire expectations (r4 after `philharmonic-types 0.3.5`):
- Strings → CBOR tstr.
- `Uuid` → 16-byte bstr (uuid crate's `is_human_readable()==false` branch).
- `UnixMillis` (i64, `#[serde(transparent)]`) → CBOR uint for positive values.
- `u64` (`step`) → CBOR uint.
- `Sha256` → 32-byte bstr (REQUIRES `philharmonic-types >= 0.3.5`).

### Protected header

CBOR map `{1: -8, 4: <utf8-bytes-of-kid>}`, bstr-wrapped.

### Sig_structure1 (RFC 9052 §4.4)

`["Signature1", body_protected_bytes, external_aad=h'', payload_bytes]`,
CBOR-serialized. Use `coset::CoseSign1Builder::create_signature`
or equivalent; do not hand-roll the structure.

### COSE_Sign1 envelope

Array `[protected_bytes, {} unprotected, payload_bytes, signature_bytes]`.
No outer CBOR tag.

### 11-step verification order

From the proposal §Service-side verification order (r2). Do NOT
reorder:

1. Parse COSE_Sign1 → `Malformed` on failure.
2. Pin `alg == -8` (EdDSA) → `AlgorithmNotAllowed`.
3. Kid lookup in registry → `UnknownKid`.
4. Key validity window → `KeyOutOfWindow`.
5. Payload size ceiling → `PayloadTooLarge`.
6. Ed25519 signature verify → `BadSignature`.
7. Claim payload CBOR decode → `Malformed`.
8. Protected-header kid == claims.kid → `KidInconsistent`.
9. `exp` vs `now` → `Expired`.
10. Constant-time `SHA-256(payload_bytes) == claims.payload_hash`
    via `subtle::ConstantTimeEq` → `PayloadHashMismatch`.
11. `claims.realm == service_realm` → `RealmMismatch`.

Only after ALL eleven pass is a `ConnectorCallContext` returned.

## Reference vectors — use AS COMMITTED

These live in `docs/crypto-vectors/wave-a/`. The Rust tests
assert equality with these values; you are explicitly **NOT** to
regenerate or modify them.

- `wave_a_seed.hex` — `9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60`
- `wave_a_public.hex` — `d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a`
- `wave_a_payload_plaintext.hex` — 27 bytes of `"phase-5-wave-a-test-payload"` as ASCII
- `wave_a_payload_hash.hex` — `1db993b4b6e574cbf9f69632e60658cc03a16febf6f63d072ce0e91b65688617`
- `wave_a_claims.cbor.hex` — 207 bytes of CBOR
- `wave_a_protected.hex` — 38 bytes
- `wave_a_sig_structure1.hex` — 262 bytes (for signature-path debugging if needed; not the canonical artifact for tests)
- `wave_a_signature.hex` — 64 bytes Ed25519 signature
- `wave_a_cose_sign1.hex` — 317 bytes final COSE_Sign1

Claim-set inputs that produced those vectors (mirror these in
your test fixtures):

```rust
// ConnectorTokenClaims inputs — matches docs/crypto-vectors/wave-a/README.md
let claims = ConnectorTokenClaims {
    iss: "lowerer.main".to_owned(),
    exp: UnixMillis(1_924_992_000_000), // 2031-01-01T00:00:00Z
    kid: "lowerer.main-2026-04-22-3c8a91d0".to_owned(),
    realm: "llm".to_owned(),
    tenant: Uuid::parse_str("11111111-2222-4333-8444-555555555555").unwrap(),
    inst: Uuid::parse_str("66666666-7777-4888-8999-aaaaaaaaaaaa").unwrap(),
    step: 7,
    config_uuid: Uuid::parse_str("bbbbbbbb-cccc-4ddd-8eee-ffffffffffff").unwrap(),
    payload_hash: Sha256::of(b"phase-5-wave-a-test-payload"),
};
```

If your `mint_token` output diverges from the committed hex by
even one byte, that's a bug — either in your CBOR encoding order
(must be declaration order), your protected-header encoding, or
your Sig_structure1 construction. **Do not "fix" the test
vectors to make the test pass.** Instead, report the divergence
in your final summary so Claude can diagnose it (ciborium's map
ordering vs cbor2's, protected-header wrapping, etc).

## Negative vectors (synthesize at test time)

Ten cases, one per rejection step. Each derived from the positive
vector by a local perturbation. Assert the specific error
variant.

1. `alg == -7` (ES256) in the protected header →
   `AlgorithmNotAllowed`. Synthesize by re-encoding the protected
   bucket with `alg=-7` and re-signing (or by constructing the
   COSE_Sign1 outer structure manually with the tampered
   protected bytes; signature verify will fail either way, but
   the alg pin must fire BEFORE signature verify per step order).
2. Protected-header `kid` replaced with a kid not in the
   registry → `UnknownKid`. Same construction as (1) but only
   kid changes.
3. Registry entry with `not_after` in the past → `KeyOutOfWindow`.
   Uses the positive COSE_Sign1 bytes; alters only the registry.
4. `payload_bytes.len() == MAX_PAYLOAD_BYTES + 1` →
   `PayloadTooLarge`.
5. Last byte of the signature flipped → `BadSignature`.
6. One byte of the claim payload flipped (same total length so
   step 5's size check passes) → `BadSignature`.
7. Protected header `kid = "A"`, claim payload with `kid = "B"`,
   correctly signed (so steps 1-6 pass) → `KidInconsistent` at
   step 8.
8. `exp = 1` (long in the past) → `Expired`.
9. `payload_hash` claim is correct for plaintext P1, but service
   verifies with different plaintext P2 of the same length →
   `PayloadHashMismatch`.
10. `claims.realm = "llm"`, `service_realm = "sql"` →
    `RealmMismatch`.

For (1), (2), (7), and (9), you'll need to re-encode and/or
re-sign; the positive vector alone doesn't cover them. That's
fine — synthesize inside the test file using the committed seed.

## Dependencies

### `philharmonic-connector-client/Cargo.toml`

Regular `[dependencies]`:

- `philharmonic-connector-common = "0.1"` — `ConnectorTokenClaims`,
  `ConnectorSignedToken` wrapper (reuse — do NOT redefine claim
  types here).
- `philharmonic-types = "0.3.5"` or `"0.3"` — for `Uuid`,
  `Sha256`, `UnixMillis`. The serde CBOR shape requires
  `>= 0.3.5` (enforce as `"0.3.5"` to be explicit).
- `ed25519-dalek = "2"` — latest `2.2.0` as of 2026-04-22. No
  feature flags beyond defaults.
- `coset = "0.4"` — latest `0.4.2`. Already transitive via
  connector-common but declare directly since we use
  `CoseSign1Builder` etc.
- `ciborium = "0.2"` — latest `0.2.2`. For claim-payload CBOR
  serialization.
- `zeroize = { version = "1", features = ["derive"] }` — latest
  `1.8.2`. For `Zeroizing<[u8; 32]>`.
- `thiserror = "2"` — error derives.

`[dev-dependencies]`:

- `hex = "0.4"` — parsing the committed hex literals in tests.
- `hex-literal = "1"` (or latest; look up) — optional, for
  `hex!(...)` literals if you choose that form over
  `include_str!`. Pick one pattern; match what philharmonic-
  policy's `tests/crypto_vectors.rs` uses if it's installed
  there.

### `philharmonic-connector-service/Cargo.toml`

Regular `[dependencies]`:

- `philharmonic-connector-common = "0.1"`.
- `philharmonic-types = "0.3.5"`.
- `ed25519-dalek = "2"` — for `VerifyingKey`.
- `coset = "0.4"`.
- `ciborium = "0.2"` — for claim-payload CBOR deserialization.
- `sha2 = "0.11"` — latest `0.11.0`. For the payload-hash check.
- `subtle = "2"` — latest `2.6.1`. For constant-time hash
  equality.
- `thiserror = "2"`.

`[dev-dependencies]`:

- `hex = "0.4"`.
- Possibly `hex-literal` (same as above).

**Version-lookup rule:** every number above is a hint. Before
pinning, run `./scripts/xtask.sh crates-io-versions -- <crate>`
and use the actual latest for the minor you're selecting. This
rule caught a `coset = "0.3"` vs. `"0.4"` mistake in Phase 3
and a `Sha256` CBOR-shape mismatch today; don't repeat the
pattern.

## Module layout (both crates, tune if needed)

### `philharmonic-connector-client/src/`

```
lib.rs          // re-exports + crate docs
signing.rs      // LowererSigningKey + mint_token
error.rs        // MintError
```

### `philharmonic-connector-service/src/`

```
lib.rs          // re-exports + crate docs
registry.rs     // MintingKeyEntry + MintingKeyRegistry
verify.rs       // verify_token (the 11-step sequence)
context.rs      // ConnectorCallContext construction (thin)
error.rs        // TokenVerifyError
```

Tests under `tests/` in each crate:

- `philharmonic-connector-client/tests/signing_vectors.rs` —
  positive + any synthesis helpers.
- `philharmonic-connector-service/tests/verify_vectors.rs` —
  positive + all 10 negative cases.

## Workspace conventions (authoritative:
`docs/design/13-conventions.md`)

- **Edition 2024, MSRV 1.88.** Both `Cargo.toml` already set.
- **License `Apache-2.0 OR MPL-2.0`.** Both crates already have
  both `LICENSE-*` files.
- **`thiserror`** for library error enums. No `anyhow`.
- **No panics in library code.** Every `src/**/*.rs` path: no
  `.unwrap()` / `.expect()` on `Result`/`Option`, no `panic!` /
  `unreachable!` / `todo!` / `unimplemented!`, no unbounded
  indexing, no unchecked integer arithmetic, no lossy `as` casts.
  Narrow exceptions require inline justification (unlikely any
  are legitimate here). Tests can `.unwrap()` freely.
- **Library crates take bytes, not file paths.** Both `Lowerer-
  SigningKey::from_seed` and the registry's insertion API take
  already-read byte values or already-parsed struct values. No
  `&Path`, no `std::fs`, no file-permission logic. See
  `docs/design/13-conventions.md` §Library crate boundaries.
- **Re-export discipline.** Re-export types from direct
  dependencies that appear in each crate's own public API —
  `ConnectorTokenClaims`, `ConnectorSignedToken`, `Sha256`,
  `Uuid`, `UnixMillis`, `VerifyingKey`, etc.
- **Rustdoc coverage.** Every public item gets a doc comment.
- **No `unsafe`.** Not in either crate. If you're tempted, flag —
  we don't open `unsafe` on crypto paths.

## Zeroization (from the proposal §Zeroization points)

- `LowererSigningKey` owns `Zeroizing<[u8; 32]>` for the seed.
  On drop, the seed buffer is zeroed by `Zeroizing`.
- Every `mint_token` call reconstructs a transient
  `ed25519_dalek::SigningKey` via `SigningKey::from_bytes`.
  This `SigningKey` lives on the stack for the sign call and is
  dropped immediately. Do NOT store a long-lived `SigningKey`.
  (`ed25519_dalek::SigningKey 2.x` does not itself zeroize on
  drop; the whole point of the per-call pattern is to keep the
  un-zeroable type from living longer than one sign.)
- Signing-time intermediates (Ed25519's `r` nonce) are inside
  `ed25519-dalek`; we have nothing to zero there.
- Public keys (`VerifyingKey`, registry entries) are not
  sensitive — no zeroization.

Flag if you find any code path where the seed escapes the
`Zeroizing` wrapper or lives longer than a sign call.

## Pre-landing

Before concluding:

```sh
./scripts/pre-landing.sh philharmonic-connector-client
./scripts/pre-landing.sh philharmonic-connector-service
```

Pre-landing runs `fmt --check` + `check` + `clippy -D warnings`
+ workspace test + the per-modified-crate `--ignored` phase.
Both crates should pass cleanly. If an `--ignored` failure is
due to unrelated testcontainers / env issues, say so in your
summary; do not suppress the check.

Also run miri on both crates — crypto paths get extra scrutiny:

```sh
./scripts/miri-test.sh philharmonic-connector-client
./scripts/miri-test.sh philharmonic-connector-service
```

If miri flags something in third-party deps (`ed25519-dalek`,
`coset`, `ciborium`), note it but don't attempt to patch
upstream. If miri flags something in the new code, fix it.

## Git

You do NOT commit, push, branch, tag, or publish. Leave the
working tree dirty. Claude runs `./scripts/commit-all.sh`,
`./scripts/push-all.sh`, and — when Wave B lands later — the
publish.

## Deliverables

1. `philharmonic-connector-client/src/` — module tree above.
2. `philharmonic-connector-client/Cargo.toml` — dependencies per
   §Dependencies. Version stays at `0.0.0`.
3. `philharmonic-connector-client/README.md` — rewrite from stub.
   Purpose (mint COSE_Sign1 authorization tokens for the Phase 5
   connector triangle), quick example of `from_seed` +
   `mint_token`, one-sentence caveat that real lowerers read the
   seed from a file / KMS in the bin crate, not in this library.
4. `philharmonic-connector-client/CHANGELOG.md` — `[Unreleased]`
   section listing everything in the initial implementation.
5. `philharmonic-connector-client/tests/signing_vectors.rs`.
6. `philharmonic-connector-service/src/` — module tree above.
7. `philharmonic-connector-service/Cargo.toml` — dependencies per
   §Dependencies. Version stays at `0.0.0`.
8. `philharmonic-connector-service/README.md` — rewrite from
   stub. Purpose (verify COSE_Sign1 authorization tokens), quick
   example of building a `MintingKeyRegistry` and calling
   `verify_token`, one-sentence caveat that config parsing is a
   bin-crate concern.
9. `philharmonic-connector-service/CHANGELOG.md` — `[Unreleased]`.
10. `philharmonic-connector-service/tests/verify_vectors.rs`.
11. Passing
    `./scripts/pre-landing.sh philharmonic-connector-client`
    and
    `./scripts/pre-landing.sh philharmonic-connector-service`.
12. Passing miri on both crates (barring upstream-crate noise
    which is flagged not fixed).

## Structured output contract

Report in your final summary:

- Files changed (paths + approximate line counts).
- Dependency pins actually chosen (verified against crates.io on
  2026-04-22 via `./scripts/xtask.sh crates-io-versions`).
- Confirmation that the reference vectors in
  `docs/crypto-vectors/wave-a/` were loaded as committed
  (not regenerated).
- Byte-for-byte match between your `mint_token` output and
  `wave_a_cose_sign1.hex`. If there's a mismatch, DO NOT
  "fix" the vectors — report the divergence in detail (which
  field, which bytes, what your ciborium-produced CBOR looks
  like) so Claude can diagnose.
- All 10 negative vectors pass (each asserting the specific
  error variant per step).
- `pre-landing.sh` results for both crates.
- `miri-test.sh` results for both crates. Any third-party noise
  flagged (ed25519-dalek, coset, ciborium, subtle) but not
  fought.
- Any ambiguity you resolved and the call you made. Any
  deviation from the proposal — there should be none without an
  explicit flag.
- Zeroization sanity: the seed lives in `Zeroizing<[u8; 32]>`
  the whole time it exists; no transient `SigningKey` is held
  longer than one `mint_token` call.

## Default follow-through policy

Complete the full task end-to-end. Do not stop at a "ready for
review" checkpoint if pre-landing + miri pass cleanly and the
vectors match. If you hit a blocker (vector mismatch, clippy
flagging a no-panic violation you can't fix without a design
call, miri UB in NEW code you wrote), stop and report — do not
commit workarounds, do not silence lints, do not regenerate
vectors.

## Completeness contract

"Done" means:

- Both crates compile cleanly with `clippy -D warnings`.
- The one positive vector (`wave_a_cose_sign1.hex`) round-trips
  sign-and-verify with the committed keypair.
- All 10 negative vectors reject with the exact specified error
  variant.
- Miri passes on both crates (modulo upstream noise that you
  flag).
- Rustdoc is complete on all `pub` items.
- README + CHANGELOG written.
- No `.unwrap()` / `.expect()` / `panic!()` in `src/`.
- No `unsafe` in either crate.
- No file I/O in either library.

Partial state is a blocker, not a deliverable. If you can't
finish, leave the working tree in a buildable state and report
what's missing.

## Verification loop

After writing any non-trivial chunk:

```sh
./scripts/rust-lint.sh philharmonic-connector-client
./scripts/rust-lint.sh philharmonic-connector-service
```

(fmt + check + clippy — no tests). Then run tests:

```sh
./scripts/rust-test.sh philharmonic-connector-client
./scripts/rust-test.sh philharmonic-connector-service
```

Then pre-landing (full) before concluding.

## Missing-context gating

If anything in the proposal is ambiguous or contradicts itself,
STOP and surface the contradiction. Do not resolve by picking
the interpretation that's easiest to implement. Specific
pressure points:

- CBOR encoding of a specific claim field not covered by the
  types' existing tests (e.g. a weird `UnixMillis` edge case).
- Whether the 11-step order has subtle sub-step ordering
  requirements (e.g. kid-lookup-for-signature-check vs kid-
  lookup-for-window-check — they're the same kid, same
  lookup, but the error variant differs if one fires first).
- Any `subtle` API nuance (is `ct_eq` returning `Choice` that
  needs unwrapping via `bool::from(...)`, or a direct `bool`).

Flag it in your final summary with enough context for Claude to
answer without re-reading the proposal.

## Action safety

No destructive git operations. No cargo publish. No push. No
branch creation. No tag operation. No modification to files
outside the two submodule crates (`philharmonic-connector-
client/`, `philharmonic-connector-service/`) except as a LAST
RESORT for Cargo.toml plumbing and ONLY with a justification in
your summary.

The reference vectors in `docs/crypto-vectors/wave-a/` are
read-only for this dispatch. If you think they're wrong,
report — do not edit.

## Outcome

**Completed 2026-04-22** — Codex delivered in one round. Commits
`9634f68` (client) + `bcf8ea6` (service) + parent pointer bump
`ac02232`.

Files landed:
- `philharmonic-connector-client/` — `src/signing.rs` (new, ~73),
  `src/error.rs` (new, ~15), `src/lib.rs` updated, `Cargo.toml`,
  `README.md`, `CHANGELOG.md`, `tests/signing_vectors.rs` (new,
  ~124). 7 files, 284 insertions.
- `philharmonic-connector-service/` — `src/verify.rs` (new,
  ~129), `src/registry.rs` (new, ~42), `src/context.rs` (new,
  ~16), `src/error.rs` (new, ~49), `src/lib.rs` updated,
  `Cargo.toml`, `README.md`, `CHANGELOG.md`,
  `tests/verify_vectors.rs` (new, ~337). 9 files, 658 insertions.

Verification (Codex-reported + Claude Gate-2 confirmed):
- `mint_token` output matches `wave_a_cose_sign1.hex`
  byte-for-byte, plus every intermediate vector
  (`wave_a_claims.cbor.hex`, `wave_a_protected.hex`,
  `wave_a_sig_structure1.hex`, `wave_a_signature.hex`).
- All 10 negative vectors reject with the exact error variant
  specified in the proposal.
- `./scripts/pre-landing.sh` passed on both crates.
- `./scripts/miri-test.sh` passed on both crates.

Claude Gate-2 review: PASS.
- 11-step verify sequence: exact order, one distinct error
  variant per rejection path.
- `subtle::ConstantTimeEq` correctly used for the `payload_hash`
  compare at step 10.
- Zeroization: seed in `Zeroizing<[u8; 32]>` for
  `LowererSigningKey`'s lifetime; transient
  `ed25519_dalek::SigningKey` reconstructed per `mint_token`
  call (option (a) from the proposal).
- No `.unwrap()` / `.expect()` / `panic!` / `unreachable!` /
  `todo!` / `unimplemented!` on any reachable path in `src/` of
  either crate.
- No `unsafe`.
- No file I/O in either library (workspace convention §Library
  crate boundaries).
- Re-exports + rustdoc: complete.
- Versions verified against crates.io 2026-04-22.

Flagged follow-up (not a Gate-2 blocker):
- `ConnectorCallContext.issued_at` has no corresponding claim
  in `ConnectorTokenClaims`. Codex set it to `now` at verify
  time; this is semantically "time verified" not "time issued."
  Deferred to Wave B or `philharmonic-connector-common 0.2.0`
  (would require adding an `iat` claim — breaking). See
  `docs/notes-to-humans/2026-04-22-0011-phase-5-wave-a-claude-review.md`.

Minor observations (not blockers):
- `MintingKeyRegistry::insert` silently replaces on duplicate kid
  (standard HashMap behavior). `RealmRegistry` in connector-common
  rejects duplicates. Either policy is defensible; the service bin
  can add a wrapper if strict semantics are wanted.
- Crates remain at `0.0.0` per the proposal; publish is deferred
  until Wave B lands and end-to-end tests pass.
