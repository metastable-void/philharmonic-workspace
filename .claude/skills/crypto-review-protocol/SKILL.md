---
name: crypto-review-protocol
description: Use whenever work touches a crypto-sensitive path in this workspace — SCK encrypt/decrypt, COSE_Sign1 signing/verification, COSE_Encrypt0 encryption/decryption, the ML-KEM-768 + X25519 + HKDF + AES-256-GCM hybrid construction, payload-hash binding, or `pht_` API token generation. These paths require Yuka's personal review at BOTH the approach-design stage (before any code is written) and the code-review stage (before any publish). Neither gate is waivable. Trigger on intent ("implement SCK", "add the COSE_Sign1 token", "mint a pht_ token") — don't wait to be told it's crypto.
---

# Crypto review protocol

Philharmonic's security rests on a small number of cryptographic
constructions. Yuka reviews them personally. Not because Claude (or
Codex) can't write the code, but because crypto bugs are almost
never caught by tests that aren't *specifically* designed for
correctness-vs-a-reference, and the pre-approval gate is the
cheapest place to catch misuse.

Authoritative source: `ROADMAP.md` §5 "Crypto review protocol".
Also relevant: `docs/design/11-security-and-cryptography.md`
(threat model, construction specifics).

## Crypto-sensitive paths (the trigger list)

If the task touches any of these, this skill applies:

- **SCK encrypt/decrypt** in `philharmonic-policy` —
  AES-256-GCM over `TenantEndpointConfig.encrypted_config`, SCK
  loaded from deployment secret storage.
- **COSE_Sign1** signing and verification — the connector
  authorization token (`philharmonic-connector-client` mints,
  `philharmonic-connector-service` verifies).
- **COSE_Encrypt0** encryption and decryption — the encrypted
  payload channel between lowerer and connector service.
- **ML-KEM-768 + X25519 + HKDF + AES-256-GCM hybrid** — the hybrid
  KEM construction the encrypt/decrypt paths use.
- **Payload-hash binding** — the `payload_hash` claim in the
  COSE_Sign1 token, SHA-256 of the COSE_Encrypt0 bytes, verified
  against the claim server-side.
- **`pht_` API token generation** — 32 random bytes → 43-char
  base64url → `pht_` prefix, SHA-256 for storage.

If the task touches *anything adjacent* to these (looks up SCK,
holds key material, parses a token) flag it too. False positives
here are cheap; false negatives are catastrophic.

## Two gates (both mandatory)

### Gate 1: pre-approval of approach, before coding

Before Claude writes any of the primitives above — and before any
Codex prompt that implements them — produce a **short written
proposal** and get Yuka's sign-off. The proposal names:

- The exact primitives and library versions (e.g.
  `ml-kem = "0.2"`, `x25519-dalek = "2.0"`,
  `aes-gcm = "0.10"`, `ed25519-dalek = "2.1"`, `hkdf = "0.12"`,
  `sha2 = "0.10"` — all RustCrypto).
- The construction order: KEM shared secret || ECDH shared secret
  → HKDF-SHA256 → AES-256-GCM key.
- HKDF inputs: salt, IKM ordering, info string.
- AEAD associated data: which fields bind which claims.
- Nonce scheme: random-per-encryption, 12 bytes, never reused.
- Key derivation / rotation story: where keys live, how rotation
  happens, `kid` encoding.
- Zeroization points: which `Zeroizing<_>` wrappers go where.
- Test-vector plan (see below).

Deliver the proposal as a Markdown file under
`docs/design/crypto-proposals/<date>-<topic>.md` or as a comment
in the relevant GitHub issue; either way it gets committed before
code starts. Wait for explicit sign-off. A pre-approved approach
doesn't skip Gate 2.

### Gate 2: post-review of code, before publish

Once the implementation exists:

- Yuka reviews the actual code line-by-line, plus the committed
  test vectors.
- All review comments resolved before `cargo publish`.
- A clean Gate 2 does not retroactively bless an approach that
  was never through Gate 1.

## Test vector discipline

**Round-trip tests alone are insufficient.** Encrypt-then-decrypt
or sign-then-verify can pass while both sides are wrong in
matching ways. For every crypto operation, commit tests with
known inputs and exact expected outputs:

```rust
// tests/crypto_vectors.rs
#[test]
fn sck_encrypts_to_known_ciphertext() {
    let sck = hex!("00112233...");
    let nonce = hex!("0011...");
    let plaintext = br#"{"realm":"llm",...}"#;
    let expected = hex!("aabbccdd...");
    let actual = sck_encrypt(&sck, &nonce, plaintext).unwrap();
    assert_eq!(actual, expected);
}
```

Generate expected outputs once — by hand, or with a reference
implementation (Python `cryptography`, a spec's published test
vectors where available). Commit the hex-encoded bytes. Subsequent
test runs verify the implementation didn't drift.

Minimums:

- **SCK:** ≥3 vectors covering distinct SCKs, plaintexts, nonces.
- **COSE_Sign1:** known Ed25519 keypair, known claims → known
  signature bytes.
- **COSE_Encrypt0:** known ML-KEM-768 + X25519 keypairs, known
  plaintext → known ciphertext bytes.
- **Hybrid KEM:** known KEM + ECDH shared secrets → known HKDF
  output → known AES-GCM key (intermediate-value vector, not just
  final ciphertext).

## Hard constraints

- **No `unsafe` blocks** in crypto code or its immediate
  dependents.
- **No custom primitives.** Only the RustCrypto suite:
  `ml-kem`, `x25519-dalek`, `aes-gcm`, `ed25519-dalek`, `hkdf`,
  `sha2`. If something you need isn't there, flag before reaching
  for anything else.
- **Key material is zeroized.** Use `zeroize::Zeroizing` or
  `secrecy::SecretBox` for every in-memory key. Flag any place a
  key might linger.
- **Signatures/MACs over untrusted input are suspicious.** Flag
  them explicitly in the Gate 1 proposal.

## What to surface to Yuka up-front

When opening Gate 1, state explicitly:

1. Your understanding of the hybrid KEM construction — KEM-then-
   ECDH or ECDH-then-KEM for the HKDF IKM, HKDF info string, AEAD
   associated data choice. Confirm before implementing; getting
   these wrong is the canonical way hybrid constructions fail.
2. Any `unsafe` usage you're contemplating.
3. Any key handling that can't be zeroized (e.g. because it's
   borrowed from an FFI type).
4. Any signatures or MACs over data that isn't fully
   attacker-controlled-but-checked (most input is attacker-
   controlled; the question is whether it's authenticated first).

## If Codex is doing the implementation

Write the Codex prompt (see the `codex-prompt-archive` skill)
**after** Gate 1 is cleared, not before. The prompt must:

- Link to Yuka's approved proposal.
- Include the test vectors' expected hex as part of the prompt,
  not as a "generate expected outputs" task — Codex should not be
  generating the reference values it's also being verified
  against.
- Explicitly prohibit changing any primitive or construction
  choice without flagging.
- Require Codex to flag (not fix) any `unsafe` or zeroization gap
  it encounters in neighboring code.

The archived prompt file counts as documentation of what Codex
was told; Gate 2 reviews that too.

## Do not

- Do not publish a crypto-sensitive crate without Gate 2 signed
  off. No `cargo publish` — not even `--dry-run` publish — until
  review is complete and comments are resolved.
- Do not skip Gate 1 because "it's basically the same as last
  time." Restate the construction each time; drift is caught by
  restating, not by memory.
- Do not let test vectors be round-trip-only. That failure mode
  is the whole reason this section exists.
- Do not roll your own primitives, even for "small" utility code
  (MAC, HKDF, whatever). RustCrypto or flag.
