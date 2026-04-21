# Phase 2 — `philharmonic-policy` Wave 2: crypto foundation

**Date:** 2026-04-21
**Slug:** `phase-2-wave-2-crypto-foundation`
**Round:** 01 (initial dispatch; Wave 1 has landed at parent
commit `65fc3c4`, submodule `philharmonic-policy@790c23d`).
**Subagent:** `codex:codex-rescue`

## Gate-1 crypto approval is in place

This dispatch implements the two crypto-sensitive paths proposed
in [`docs/design/crypto-proposals/2026-04-21-phase-2-sck-and-pht.md`](../design/crypto-proposals/2026-04-21-phase-2-sck-and-pht.md)
and approved by Yuka (with version amendments) at
[`docs/design/crypto-approvals/2026-04-21-phase-2-sck-and-pht.md`](../design/crypto-approvals/2026-04-21-phase-2-sck-and-pht.md).

The approval tightened two library versions over the proposal:

| Crate       | Proposal       | Approved         |
|-------------|----------------|------------------|
| `sha2`      | `0.10`         | **`0.11`**       |
| `rand_core` | `0.6`          | **`0.10.1`**     |
| `getrandom` | `0.2`          | **`0.4.2`**      |
| `aes-gcm`   | `0.10`         | `0.10` (confirmed) |
| `zeroize`   | `1`            | `1` (confirmed)  |
| `base64`    | `0.22`         | `0.22` (no change) |

**Do not change any primitive, library, or construction choice
without flagging.** If a version above doesn't resolve against
other workspace crates or has an API change the proposal didn't
anticipate, stop and report — don't pick a different version or a
different primitive.

## Motivation

Closes Phase 2 by landing the three crypto-sensitive deliverables:

1. The `TenantEndpointConfig` entity kind (the seventh kind in
   `09-policy-and-tenancy.md`, held out of Wave 1 so its content
   — the `encrypted_config` blob — wouldn't land without the
   crypto review that protects it).
2. SCK AES-256-GCM encrypt/decrypt primitives (`Sck`,
   `sck_encrypt`, `sck_decrypt`) — the key that protects every
   `encrypted_config` in the substrate.
3. `pht_` long-lived API token generation + parsing + storage hash
   (`generate_api_token`, `parse_api_token`, `TokenHash`) — the
   credential shape for long-lived automation clients.

After this dispatch the crate is ready for Yuka's Gate-2 code
review; only after Gate-2 clears does `philharmonic-policy 0.1.0`
ship to crates.io. **This dispatch does NOT publish** — version
stays at `0.0.0` in your diff. Claude will bump + publish after
Gate-2 sign-off.

## References

- `docs/design/crypto-proposals/2026-04-21-phase-2-sck-and-pht.md`
  — the approved proposal. Read first; primitives + construction
  are pinned there.
- `docs/design/crypto-approvals/2026-04-21-phase-2-sck-and-pht.md`
  — the approval, including the version amendments above.
- `docs/design/09-policy-and-tenancy.md` — `TenantEndpointConfig`
  entity shape (§"Tenant endpoint configs") and `pht_` token
  format (§"Long-lived API tokens").
- `docs/design/11-security-and-cryptography.md` — §"Cryptographic
  primitives", §"Substrate at-rest endpoint config encryption",
  §"Long-lived API tokens" (threat model and rationale).
- `docs/design/13-conventions.md` — workspace conventions (you
  already know these from Wave 1).
- `README.md` §"Terminology and language" — prose conventions.
- `ROADMAP.md` §5 "Crypto review protocol" — the gate rules.
- `.claude/skills/crypto-review-protocol/SKILL.md` — the protocol
  skill invoked for this work.
- `philharmonic-policy/src/entity.rs` (from Wave 1) — the six
  existing entity kinds; match the structural style when adding
  `TenantEndpointConfig`.
- `philharmonic-policy/tests/common/mock.rs` (from Wave 1) — the
  mock substrate you'll reuse for `TenantEndpointConfig` round-trip
  tests.

## Scope

### In scope (Wave 2)

- **`TenantEndpointConfig` entity kind** matching `09-policy-and-tenancy.md`
  §"`TenantEndpointConfig` entity kind":
  - `KIND = uuid!("19d1a8f5-6ef0-49b0-adf5-48e1cd3daea9")` —
    reserved during Wave 1 and must be used verbatim.
  - `NAME = "tenant_endpoint_config"`.
  - `CONTENT_SLOTS`: `display_name`, `encrypted_config`.
  - `ENTITY_SLOTS`: `tenant` → `Tenant`, `SlotPinning::Pinned`.
  - `SCALAR_SLOTS`: `key_version` (`I64`, **NOT indexed** —
    third argument `false`), `is_retired` (`Bool`, indexed).
  - Export from `src/lib.rs` alongside the other six.
- **SCK module** (`src/sck.rs`), public API exactly as approved:
  ```rust
  pub struct Sck { /* Zeroizing<[u8; 32]> inside */ }
  impl Sck {
      pub fn from_bytes(bytes: [u8; 32]) -> Self;
      pub fn from_file(path: &std::path::Path) -> Result<Self, PolicyError>;
  }

  pub fn sck_encrypt(
      sck: &Sck,
      plaintext: &[u8],
      tenant_id: uuid::Uuid,
      config_uuid: uuid::Uuid,
      key_version: i64,
  ) -> Result<Vec<u8>, PolicyError>;

  pub fn sck_decrypt(
      sck: &Sck,
      wire: &[u8],
      tenant_id: uuid::Uuid,
      config_uuid: uuid::Uuid,
      key_version: i64,
  ) -> Result<zeroize::Zeroizing<Vec<u8>>, PolicyError>;

  /// Crate-internal deterministic seam for vector tests. Takes
  /// the nonce as input instead of drawing from OsRng. Public
  /// tests cross this seam via `pub(crate)`.
  pub(crate) fn sck_encrypt_with_nonce(
      sck: &Sck,
      plaintext: &[u8],
      nonce: &[u8; 12],
      tenant_id: uuid::Uuid,
      config_uuid: uuid::Uuid,
      key_version: i64,
  ) -> Result<Vec<u8>, PolicyError>;
  ```
- **`pht_` token module** (`src/token.rs`), public API exactly
  as approved:
  ```rust
  pub const TOKEN_PREFIX: &str = "pht_";
  pub const TOKEN_BYTES: usize = 32;
  pub const TOKEN_ENCODED_LEN: usize = 43;
  pub const TOKEN_FULL_LEN: usize = 47;

  pub struct TokenHash(pub [u8; 32]);

  pub fn generate_api_token() -> (zeroize::Zeroizing<String>, TokenHash);
  pub fn parse_api_token(s: &str) -> Result<TokenHash, PolicyError>;

  /// Crate-internal deterministic seam for vector tests.
  pub(crate) fn generate_api_token_from_bytes(
      raw: [u8; 32],
  ) -> (zeroize::Zeroizing<String>, TokenHash);
  ```
- **`PolicyError` additions** via new variants with `thiserror`
  `#[from]` or explicit `#[error(...)]`:
  - `SckIo(#[from] std::io::Error)` — surfaces `Sck::from_file`
    I/O failures.
  - `SckKeyFileLength { expected: usize, actual: usize }` — SCK
    key file has wrong length.
  - `SckCiphertextTooShort { len: usize, required: usize }` —
    wire bytes shorter than `1 + 12 + 16`.
  - `SckUnsupportedVersion { byte: u8 }` — wire version byte not
    `0x01`.
  - `SckDecryptFailed` — AEAD tag/nonce/AAD/key mismatch. **Do
    not include underlying error detail** — return the same
    opaque variant for every failure so the decrypt path is
    side-channel clean.
  - `TokenWrongLength { expected: usize, actual: usize }`.
  - `TokenWrongPrefix`.
  - `TokenInvalidBase64` (no underlying error; opaque).
  - `TokenDecodedWrongLength { expected: usize, actual: usize }`.
  Keep existing variants; don't reorder.
- **Test vectors module**: `tests/crypto_vectors/gen_sck.py`,
  `tests/crypto_vectors/gen_pht.py`, and Rust tests that load the
  vectors below *exactly as given* and assert equality. See
  §"Test vectors" for the committed values.
- **Three test tiers, same discipline as Wave 1**:
  - **Tier 1 — crypto unit tests** (fast, no `#[ignore]`): every
    vector + every negative case enumerated in §"Test plan"
    runs on every `cargo test --workspace`. No substrate is
    involved; these are pure-crypto checks.
  - **Tier 2 — TenantEndpointConfig round-trip** (real MySQL,
    `#[ignore]`): single additional test in
    `tests/permission_mysql.rs` exercising the new entity kind.
    Populate every slot, read back, validate. Extends the
    existing Wave 1 suite.
  - **Tier 3 — Mock-substrate round-trip** (fast, no `#[ignore]`):
    single additional test in `tests/permission_mock.rs` exercising
    the new entity kind via the existing `MockStore`. Extends Wave
    1's suite.

### Out of scope (any phase)

- **Publishing.** Version stays at `0.0.0`. Do not edit the
  `version` in `philharmonic-policy/Cargo.toml`.
- **Commits / pushes / tags / branch ops.** Claude drives Git via
  `./scripts/*.sh` after review. Leave the working tree dirty.
- **Key rotation tooling.** Producing a new SCK, walking existing
  entities, and re-encrypting under the new `key_version` is
  deployment tooling for a later phase (likely Phase 8 with the
  API layer). The construction this dispatch ships *supports*
  rotation (via the `key_version` scalar + the `key_version` AAD
  component), but the migration task itself is not delivered.
- **COSE paths.** COSE_Sign1 and COSE_Encrypt0 are separate
  crypto-sensitive paths that live in `philharmonic-connector-*`
  and need their own Gate-1 review. Don't start them.
- **Any change to Wave 1 code.** If you find a bug in the Wave 1
  code, stop and flag it — don't fix it inline. The Wave 1 work
  already landed with Gate-2 equivalent review for its non-crypto
  scope; any changes there need Yuka's call.

## Dependencies — Cargo.toml

Add to `philharmonic-policy/Cargo.toml` `[dependencies]`:

```toml
aes-gcm = "0.10"
sha2 = "0.11"
base64 = "0.22"
rand_core = { version = "0.10.1", features = ["os_rng"] }
getrandom = "0.4.2"
zeroize = { version = "1", features = ["derive"] }
```

Exact version strings. `rand_core = "0.10.1"` exists and is the
latest in the 0.10 series as of 2026-04-21. `getrandom = "0.4.2"`
same. If resolution fails against the rest of the workspace
(e.g. an upstream pin), **stop and report**; do not downgrade
without flagging.

**Do not add** `secrecy`, `subtle`, `hkdf`, `ml-kem`, `x25519-dalek`,
`ed25519-dalek`, or any other crypto crate. The proposal's argument
against constant-time equality for `pht_` tokens is approved;
substrate-level B-tree lookup does the comparison, not our code.

Dev-dependencies already in place from Wave 1 stay. No new
dev-deps needed — the crypto tests are pure-Rust, no container.

## `TenantEndpointConfig` entity

Add to `src/entity.rs` alongside the six existing impls (keep the
authoring style identical — `pub struct Foo;` + `impl Entity for
Foo { ... }`, blank line between entities).

```rust
pub struct TenantEndpointConfig;

impl Entity for TenantEndpointConfig {
    const KIND: Uuid = uuid!("19d1a8f5-6ef0-49b0-adf5-48e1cd3daea9");
    const NAME: &'static str = "tenant_endpoint_config";
    const CONTENT_SLOTS: &'static [ContentSlot] = &[
        ContentSlot::new("display_name"),
        ContentSlot::new("encrypted_config"),
    ];
    const ENTITY_SLOTS: &'static [EntitySlot] =
        &[EntitySlot::of::<Tenant>("tenant", SlotPinning::Pinned)];
    const SCALAR_SLOTS: &'static [ScalarSlot] = &[
        ScalarSlot::new("key_version", ScalarType::I64, false),
        ScalarSlot::new("is_retired", ScalarType::Bool, true),
    ];
}
```

`key_version` is NOT indexed (third arg = `false`). Matches the
design doc verbatim. Export from `src/lib.rs` alongside the rest.

## SCK construction (from the approved proposal, verbatim)

### Encrypt

Input:
- `sck: &Sck` — holds `Zeroizing<[u8; 32]>`.
- `plaintext: &[u8]`.
- `tenant_id: Uuid`.
- `config_uuid: Uuid`.
- `key_version: i64`.

Procedure:
1. For `sck_encrypt`: draw `nonce: [u8; 12]` via `rand_core`'s
   `OsRng`. For `sck_encrypt_with_nonce`: take `nonce` as an
   argument (vector-test seam).
2. Build AAD (40 bytes exactly):
   ```
   aad = tenant_id.as_bytes() (16)
       || config_uuid.as_bytes() (16)
       || key_version.to_be_bytes() (8, signed i64, big-endian)
   ```
   `Uuid::as_bytes` returns RFC-4122 big-endian byte order — this
   is what the vectors assume. `i64::to_be_bytes` is the
   big-endian two's-complement representation.
3. `Aes256Gcm::new(key)` where `key = sck.key.as_ref()`.
   Encrypt plaintext with `Nonce::from_slice(&nonce)` and
   `Payload { msg: plaintext, aad: &aad }`. The result is
   `ciphertext || tag(16)` as a single `Vec<u8>`.
4. Emit wire bytes:
   ```
   [version:u8 = 0x01] [nonce:12] [ciphertext_and_tag]
   ```
   Total length = `1 + 12 + plaintext.len() + 16`.
5. Return `Ok(wire_bytes)`.

### Decrypt

Input: `sck`, `wire: &[u8]`, plus the same `tenant_id`,
`config_uuid`, `key_version` the caller knows from the
`TenantEndpointConfig` entity context.

Procedure:
1. Length check: `wire.len() >= 1 + 12 + 16` (29 bytes). Fail
   `SckCiphertextTooShort` if not.
2. Version check: `wire[0] == 0x01`. Fail `SckUnsupportedVersion`
   if not.
3. Parse: `nonce = wire[1..13]`, `ciphertext_and_tag = wire[13..]`.
4. Rebuild AAD exactly the same way as encrypt.
5. `Aes256Gcm::new(key).decrypt(...)`. On any error, return
   `SckDecryptFailed` (opaque — do not include the underlying
   `aes_gcm::Error`).
6. Wrap plaintext in `Zeroizing::new(plaintext_bytes)` and return.

### Hard constraints (from the protocol)

- No `unsafe`.
- Key material is `Zeroizing<[u8; 32]>` inside `Sck`; don't copy
  out.
- Decrypt result is `Zeroizing<Vec<u8>>`; callers zeroize on
  drop.
- No logging of plaintext or key bytes anywhere in the module
  (not via `println!`, not via `eprintln!`, not via `tracing`
  macros either — `tracing` isn't a dep, so that one shouldn't
  come up, but flag if you see temptation).
- Error path returns the same variant regardless of which AEAD
  step failed (tag mismatch vs AAD mismatch vs wrong key). This
  is the side-channel cleanliness Yuka specified.

### `Sck::from_file` behavior

- Open path, read bytes.
- If file length != 32 → `SckKeyFileLength { expected: 32,
  actual: <len> }`.
- Wrap in `[u8; 32]`, pass to `from_bytes`.
- I/O errors propagate via `SckIo(#[from] std::io::Error)`.

No specific permission-mode check on the file. OS file
permissions are the operator's responsibility.

## `pht_` construction (from the approved proposal, verbatim)

### Generation

`generate_api_token_from_bytes(raw: [u8; 32])`:
1. Base64url-encode `raw` with no padding (`URL_SAFE_NO_PAD`):
   ```rust
   use base64::engine::general_purpose::URL_SAFE_NO_PAD;
   use base64::Engine as _;
   let encoded: String = URL_SAFE_NO_PAD.encode(&raw);
   debug_assert_eq!(encoded.len(), 43);
   ```
2. Prepend the prefix: `let token = format!("{TOKEN_PREFIX}{encoded}")`.
   Length must be exactly `TOKEN_FULL_LEN = 47`.
3. Compute `hash = Sha256::digest(token.as_bytes())` → `[u8; 32]`.
4. Return `(Zeroizing::new(token), TokenHash(hash.into()))`.

`generate_api_token()`:
1. Fill a local `[u8; 32]` buffer from `rand_core`'s OS RNG
   (through the `os_rng` feature of `rand_core = 0.10.1`).
2. Call `generate_api_token_from_bytes`.
3. Zeroize the intermediate `[u8; 32]` buffer before return.

### Parsing

`parse_api_token(s: &str)`:
1. If `s.len() != TOKEN_FULL_LEN` → `TokenWrongLength { expected:
   47, actual: s.len() }`.
2. If `!s.starts_with(TOKEN_PREFIX)` → `TokenWrongPrefix`.
3. Base64url-decode the 43-char tail (`URL_SAFE_NO_PAD.decode`).
   On decode error → `TokenInvalidBase64` (opaque).
4. If decoded.len() != `TOKEN_BYTES` → `TokenDecodedWrongLength
   { expected: 32, actual: <len> }`.
5. `hash = Sha256::digest(s.as_bytes())` — hashing the **whole
   token string including the `pht_` prefix**, not just the
   decoded bytes. Matches the design doc.
6. Return `Ok(TokenHash(hash.into()))`.

### Hard constraints

- `TokenHash` is `pub struct TokenHash(pub [u8; 32])` — a simple
  newtype. Don't `#[derive(PartialEq)]` — substrate lookup is
  by bytes at the storage layer; if the API evolves a
  constant-time equality need later, it will be added then.
  Actually — scratch that: we need `PartialEq` so that test
  assertions can compare two `TokenHash` values. `#[derive(Clone,
  Copy, Debug, PartialEq, Eq)]` is fine. The proposal's "no
  constant-time equality" comment is about not using
  `subtle::ConstantTimeEq` on lookup; derived `PartialEq` for
  debug/test ergonomics is ok.
- No `unsafe`.
- Token string `Zeroizing<String>` — the token must be zeroized
  when the caller drops it. `Zeroizing<String>` satisfies that.

## Test vectors — commit these exact hex values

Both Python reference scripts go under
`philharmonic-policy/tests/crypto_vectors/`. The Rust tests
encode the same values as hex literals. Commit **all three
forms**: the Python scripts, the expected hex in the Rust tests,
and a short README in `tests/crypto_vectors/README.md` that
explains how to re-run the Python to reproduce the hex (for
future auditability).

### Python reference — `tests/crypto_vectors/gen_sck.py`

Commit this file verbatim. Produces the SCK vectors below when
run with Python 3.11+ + pyca `cryptography`:

```python
#!/usr/bin/env python3
"""Reference vector generator for SCK AES-256-GCM wire format.

Independent of the RustCrypto suite so the Rust implementation
is checked against an external reference, not itself.

Wire format produced:
    [version:u8=0x01] [nonce:12] [ciphertext || tag(16)]

AAD = tenant_id.bytes (16, RFC 4122 big-endian)
    || config_uuid.bytes (16, RFC 4122 big-endian)
    || key_version.to_bytes(8, 'big', signed=True)     # i64 big-endian

Library: pyca/cryptography (OpenSSL backing) — AES-256-GCM.
"""

import uuid
from cryptography.hazmat.primitives.ciphers.aead import AESGCM

def enc(sck: bytes, plaintext: bytes, nonce: bytes,
        tenant_id: uuid.UUID, config_uuid: uuid.UUID,
        key_version: int) -> bytes:
    assert len(sck) == 32
    assert len(nonce) == 12
    aad = tenant_id.bytes + config_uuid.bytes + key_version.to_bytes(8, "big", signed=True)
    assert len(aad) == 40
    aead = AESGCM(sck)
    ct_and_tag = aead.encrypt(nonce, plaintext, aad)  # tag appended
    return bytes([0x01]) + nonce + ct_and_tag

def emit(label, **kwargs):
    wire = enc(**kwargs)
    print(f"# {label}")
    print(f"#   sck         = {kwargs['sck'].hex()}")
    print(f"#   nonce       = {kwargs['nonce'].hex()}")
    print(f"#   tenant_id   = {kwargs['tenant_id']}")
    print(f"#   config_uuid = {kwargs['config_uuid']}")
    print(f"#   key_version = {kwargs['key_version']}")
    print(f"#   plaintext   = {kwargs['plaintext']!r}")
    print(f"#   plaintext hex = {kwargs['plaintext'].hex()}")
    print(f"#   wire len    = {len(wire)} bytes")
    print(f"#   wire hex    =")
    hx = wire.hex()
    for i in range(0, len(hx), 64):
        print(f"#       {hx[i:i+64]}")
    print()

if __name__ == "__main__":
    print(f"# generated by gen_sck.py (cryptography {__import__('cryptography').__version__})\n")

    emit("vector 1 — simple JSON, 38-byte plaintext",
         sck=bytes(range(0x00, 0x20)),
         plaintext=br'{"realm":"llm","impl":"x","config":{}}',
         nonce=bytes(range(0x10, 0x1c)),
         tenant_id=uuid.UUID("11111111-1111-1111-1111-111111111111"),
         config_uuid=uuid.UUID("22222222-2222-2222-2222-222222222222"),
         key_version=1)

    emit("vector 2 — Unicode JSON (Japanese characters in a value)",
         sck=bytes(range(0x20, 0x40)),
         plaintext='{"display_name":"テスト"}'.encode("utf-8"),
         nonce=bytes(range(0x30, 0x3c)),
         tenant_id=uuid.UUID("33333333-3333-3333-3333-333333333333"),
         config_uuid=uuid.UUID("44444444-4444-4444-4444-444444444444"),
         key_version=7)

    emit("vector 3 — edge case, shortest conceivable JSON blob",
         sck=bytes(range(0x40, 0x60)),
         plaintext=b"{}",
         nonce=bytes(range(0x50, 0x5c)),
         tenant_id=uuid.UUID("55555555-5555-5555-5555-555555555555"),
         config_uuid=uuid.UUID("66666666-6666-6666-6666-666666666666"),
         key_version=42)
```

### SCK vectors (produced by the script above; use verbatim)

**Vector 1** — simple JSON, 38-byte plaintext:
- `sck` = `000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f`
- `nonce` = `101112131415161718191a1b`
- `tenant_id` = `11111111-1111-1111-1111-111111111111`
- `config_uuid` = `22222222-2222-2222-2222-222222222222`
- `key_version` = `1`
- `plaintext` = `{"realm":"llm","impl":"x","config":{}}` (ASCII, 38 bytes)
- `plaintext_hex` = `7b227265616c6d223a226c6c6d222c22696d706c223a2278222c22636f6e666967223a7b7d7d`
- `expected_wire` (67 bytes):
  ```
  01101112131415161718191a1b06dcea7328a55791f0576471625b4571be3d3e
  6239f875c9c5d5c004313a32b237cb6d7888db25e1be0219d64b83d1645de6ba
  45571a
  ```

**Vector 2** — Unicode JSON (Japanese characters):
- `sck` = `202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f`
- `nonce` = `303132333435363738393a3b`
- `tenant_id` = `33333333-3333-3333-3333-333333333333`
- `config_uuid` = `44444444-4444-4444-4444-444444444444`
- `key_version` = `7`
- `plaintext` = `{"display_name":"テスト"}` (UTF-8, 28 bytes)
- `plaintext_hex` = `7b22646973706c61795f6e616d65223a22e38386e382b9e38388227d`
- `expected_wire` (57 bytes):
  ```
  01303132333435363738393a3b2139141263b0bd4717f60c3c9ab83b272911b5
  ca50617dd5a1df20cbebab062b3c79213ac04003daab844519
  ```

**Vector 3** — edge case, 2-byte plaintext:
- `sck` = `404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f`
- `nonce` = `505152535455565758595a5b`
- `tenant_id` = `55555555-5555-5555-5555-555555555555`
- `config_uuid` = `66666666-6666-6666-6666-666666666666`
- `key_version` = `42`
- `plaintext` = `{}` (ASCII, 2 bytes)
- `plaintext_hex` = `7b7d`
- `expected_wire` (31 bytes):
  ```
  01505152535455565758595a5bed36f3f497b1eae71aaa363947390d6f3a91
  ```

### Python reference — `tests/crypto_vectors/gen_pht.py`

Commit verbatim:

```python
#!/usr/bin/env python3
"""Reference vector generator for the pht_ API token construction.

Independent of RustCrypto — uses stdlib base64 + hashlib so the
Rust implementation is checked against an external reference.

Construction:
    raw    = 32 bytes from OsRng  (fixed here for test vectors)
    token  = "pht_" + base64url_no_pad(raw)          # 47 chars
    hash   = SHA-256(token.encode("ascii"))           # 32 bytes
"""

import base64
import hashlib

def gen(raw: bytes) -> tuple[str, bytes]:
    assert len(raw) == 32
    encoded = base64.urlsafe_b64encode(raw).rstrip(b"=").decode("ascii")
    assert len(encoded) == 43
    token = "pht_" + encoded
    assert len(token) == 47
    digest = hashlib.sha256(token.encode("ascii")).digest()
    return token, digest

def emit(label, raw):
    token, digest = gen(raw)
    print(f"# {label}")
    print(f"#   raw          = {raw.hex()}")
    print(f"#   token        = {token!r}")
    print(f"#   sha256(token)= {digest.hex()}")
    print()

if __name__ == "__main__":
    print("# generated by gen_pht.py (stdlib base64 + hashlib)\n")

    emit("vector 1 — sequential 0x00..0x1f",
         bytes(range(0x00, 0x20)))

    emit("vector 2 — reverse-sequential 0xff..0xe0",
         bytes(range(0xff, 0xdf, -1)))

    emit("vector 3 — alternating 0xa5/0x5a pattern",
         bytes([0xa5 if i % 2 == 0 else 0x5a for i in range(32)]))
```

### `pht_` vectors (produced by the script above; use verbatim)

**Vector 1** — sequential `0x00..0x1f`:
- `raw` = `000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f`
- `token` = `pht_AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8`
- `sha256(token)` = `642b986e2d7c4afd6922e6228a93c46fc9e831d0569a750c5f97aaedd6799a85`

**Vector 2** — reverse-sequential `0xff..0xe0`:
- `raw` = `fffefdfcfbfaf9f8f7f6f5f4f3f2f1f0efeeedecebeae9e8e7e6e5e4e3e2e1e0`
- `token` = `pht___79_Pv6-fj39vX08_Lx8O_u7ezr6uno5-bl5OPi4eA`
- `sha256(token)` = `7004403c0e97e82ff4aef986720abe8146217905df403b23b9c22d32b291d10e`

**Vector 3** — alternating `0xa5/0x5a` pattern:
- `raw` = `a55aa55aa55aa55aa55aa55aa55aa55aa55aa55aa55aa55aa55aa55aa55aa55a`
- `token` = `pht_pVqlWqVapVqlWqVapVqlWqVapVqlWqVapVqlWqVapVo`
- `sha256(token)` = `c72d9531bdcd158fb10093899d4694b1bb885916dc8c0c16fbd4008d5e174521`

### Rust test file — `tests/crypto_vectors.rs`

Create this file alongside `permission_mock.rs` and
`permission_mysql.rs`. Skeleton (fill in with the vectors above;
use `hex_literal::hex!` or a hex-decoding helper — whichever
you prefer, as long as the literals in the test source match
the hex above byte-for-byte):

- For each SCK vector: assert `sck_encrypt_with_nonce(...)` returns
  `expected_wire` byte-for-byte. Assert `sck_decrypt(expected_wire,
  same_metadata)` returns plaintext. Assert decrypt with a
  tampered `tenant_id` (rotate UUID by 1 byte) returns `Err`.
  Assert decrypt with a tampered `config_uuid` returns `Err`.
  Assert decrypt with a tampered `key_version` returns `Err`.
  Assert decrypt with a flipped last-byte (in the tag region)
  returns `Err`. Assert decrypt with a truncated wire (length <
  29) returns `Err(SckCiphertextTooShort)`. Assert decrypt with
  wire version byte = `0x02` returns `Err(SckUnsupportedVersion)`.
- For each `pht_` vector: assert
  `generate_api_token_from_bytes(raw) == (expected_token,
  TokenHash(expected_hash))`. Assert
  `parse_api_token(expected_token) == Ok(TokenHash(expected_hash))`.
- Additional `pht_` negatives (cover the full error enumeration):
  - Wrong length: `parse_api_token("pht_short")` →
    `TokenWrongLength`.
  - Wrong prefix: construct `"php_" + 43 valid base64url chars` →
    `TokenWrongPrefix`.
  - Invalid base64: `"pht_" + 43 chars including an invalid
    char like `!`` → `TokenInvalidBase64`.
  - Valid base64 that decodes to wrong length: construct a 47-char
    string `"pht_" + <43 chars that decode to 32 bytes... but
    wait, 43 chars unpadded always decode to 32 bytes if the
    chars are all valid>`. This negative is in practice
    unreachable via `parse_api_token` once the length + prefix +
    base64 checks pass — `TokenDecodedWrongLength` is defensive.
    Flag if you find a triggering input; otherwise a `#[test]`
    that constructs the error variant directly to prove it
    compiles is enough.

`dev-dependencies` additions permitted: `hex-literal = "0.4"` (or
`"1"` if the major has advanced) for readable hex constants. No
other dev-dep changes.

### README — `tests/crypto_vectors/README.md`

Short text (≤30 lines) explaining:
- These vectors are the external reference against which the
  Rust crypto implementations are verified.
- Python scripts (`gen_sck.py`, `gen_pht.py`) reproduce the
  expected values, run on any system with Python 3 + pyca
  `cryptography` installed.
- If RustCrypto's behavior ever drifts from these vectors,
  **the Rust implementation has a bug**, not the Python. Don't
  "update" the vectors to match drift.
- The vectors themselves live as hex literals in
  `tests/crypto_vectors.rs` so `cargo test` doesn't need
  Python. The Python is for future re-verification.

## Test plan — comprehensive, enumerated

Tier 1 (crypto unit tests, in `tests/crypto_vectors.rs`):

1. `sck_encrypt_with_nonce(vector_1) == vector_1.expected_wire`.
2. `sck_encrypt_with_nonce(vector_2) == vector_2.expected_wire`.
3. `sck_encrypt_with_nonce(vector_3) == vector_3.expected_wire`.
4. `sck_decrypt(vector_1.expected_wire, vector_1.metadata) ==
   vector_1.plaintext` (and 2, 3).
5. For each vector, decrypt with wrong `tenant_id` → `Err`.
6. For each vector, decrypt with wrong `config_uuid` → `Err`.
7. For each vector, decrypt with wrong `key_version` → `Err`.
8. For each vector, decrypt with flipped tag byte → `Err`.
9. Decrypt with length-28 wire → `Err(SckCiphertextTooShort)`.
10. Decrypt with version-byte `0x02` → `Err(SckUnsupportedVersion)`.
11. `generate_api_token_from_bytes(vector_1) == (token, hash)`.
12. Same for vectors 2, 3.
13. `parse_api_token(vector_1.token) == Ok(hash)` (and 2, 3).
14. `parse_api_token("pht_")` → `TokenWrongLength`.
15. `parse_api_token("php_AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8")` →
    `TokenWrongPrefix` (47 chars, wrong prefix).
16. `parse_api_token("pht_!!ECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8")` →
    `TokenInvalidBase64` (47 chars, prefix ok, base64 invalid).
17. Round-trip `generate_api_token()` → `parse_api_token` returns
    the same `TokenHash`. (Non-deterministic, just a sanity
    check that the public wrapper works end-to-end.)

Tier 2 (MySQL integration, one additional test):

18. `tenant_endpoint_config_entity_round_trip` in
    `tests/permission_mysql.rs`: populate content + entity +
    scalar slots (encrypt a small blob via `sck_encrypt` with a
    fixed SCK to exercise the primitive end-to-end), append
    revision, read back, validate every slot including the
    scalar `key_version = 3` and scalar `is_retired = false`,
    and decrypt the blob via `sck_decrypt` to confirm the wire
    bytes round-trip through real MySQL. `#[ignore = "requires
    MySQL testcontainer"]` on the test, same pattern as the
    rest.

Tier 3 (mock, one additional test):

19. `tenant_endpoint_config_round_trip_mock` in
    `tests/permission_mock.rs`: same shape as 18, against
    `MockStore`. No encryption content needed — plain bytes for
    `encrypted_config` are fine (the mock doesn't care), but
    populate every slot the entity declares.

## Acceptance criteria (before Claude commits your work)

- `cargo fmt --check` clean on `philharmonic-policy`.
- `cargo check --workspace` passes at the workspace root.
- `cargo clippy --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes. Tier 1 (crypto vectors) + Tier
  3 (mock round-trips) + the Wave 1 tiers all run here; none are
  `#[ignore]`'d.
- `cargo test -p philharmonic-policy -- --ignored` passes against
  MySQL testcontainers. This runs Wave 1's 10 MySQL tests plus
  the one new `tenant_endpoint_config_entity_round_trip` —
  expect 11 green.
- `cargo tree -p philharmonic-policy` includes `aes-gcm 0.10.*`,
  `sha2 0.11.*`, `rand_core 0.10.1` or later, `getrandom 0.4.2`
  or later, `zeroize 1.*`, `base64 0.22.*`, as *declared direct
  deps*. Resolver may upgrade patches within a minor series;
  that's fine.
- No `unsafe` anywhere in the crate (grep proves it).
- No `anyhow` in the crate.
- No `println!`/`eprintln!` in library code.
- No `subtle`, `secrecy`, `hkdf`, `ml-kem`, `x25519-dalek`,
  `ed25519-dalek`, or any other crypto crate declared as a
  direct dep.
- `version` in `philharmonic-policy/Cargo.toml` is still
  `"0.0.0"` — **don't bump**. Claude bumps after Gate-2.
- The hex-literal test assertions match the vectors above
  byte-for-byte.

If any criterion fails, flag it in your final summary — **do
not work around it** by changing the vectors, by swapping a
primitive, or by silencing a test.

## Flag-vs-fix policy (from the protocol)

The crypto-review-protocol skill requires you to **flag, not fix**,
certain kinds of issue:

- `unsafe` blocks in neighboring code (anywhere you look, not
  just what you're writing).
- Zeroization gaps in neighboring code.
- Signatures or MACs over untrusted input that isn't fully
  authenticated first.
- Primitive or version choices that don't resolve — report the
  failure, don't pick a substitute.
- Test vectors that don't match — **that's a bug in the Rust
  code you wrote**, not in the vector. Flag, don't edit the
  vector.
- Bugs in Wave 1 code (entity shapes, permission evaluation,
  mock store). Flag, don't fix — Yuka reviews those separately.

## Git handling

**Do not run any Git command that changes state** — no `commit`,
`push`, `add`, `checkout`, `reset`, `stash`, `tag`, nothing. Leave
the working tree dirty. Claude inspects the diff, runs
`./scripts/pre-landing.sh philharmonic-policy`, runs the
`--ignored` MySQL suite, then Yuka gate-2-reviews, then Claude
drives `./scripts/commit-all.sh` + `./scripts/push-all.sh`.

Read-only `git status`, `git diff`, `git log` are fine.

## Final summary format

When you finish (or hit a wall), write a short summary covering:

- **File map**: every file created or modified, with one-line
  description.
- **API added**: paste the final signatures for `Sck`,
  `sck_encrypt`, `sck_decrypt`, `sck_encrypt_with_nonce`,
  `generate_api_token`, `parse_api_token`,
  `generate_api_token_from_bytes`, `TokenHash`, plus the new
  `PolicyError` variants and the `TenantEndpointConfig` entity
  impl.
- **Test results per tier**:
  - Tier 1 (crypto unit tests): count passed / total, any
    that required iteration.
  - Tier 3 (mock round-trip): pass/fail on the one new test.
  - Tier 2 (MySQL): pass/fail on the 11 ignored tests (10 from
    Wave 1 + 1 new).
- **Crypto-crate ban check**: paste the `cargo tree -p
  philharmonic-policy | grep -iE '...'` output confirming only
  the approved direct deps are added.
- **`unsafe` / `anyhow` / `println!` checks**: `rg '\bunsafe\b'`,
  `rg '\banyhow\b'`, `rg 'println!|eprintln!' -g 'src/**'`
  output — each should be empty or near-empty (clippy-level
  false positives in comments are ok).
- **Flag list**: anything you flagged rather than fixed —
  Wave 1 code you noticed bugs in, zeroization gaps you found
  elsewhere, `unsafe` in neighboring code, vector mismatches,
  resolver failures, design-doc ambiguity. Empty list if none.
- **Trait-method surface**: same as Wave 1 — list the
  `philharmonic-store` / `philharmonic-types` method names you
  called. Handy for Yuka's Gate-2 review.

No commits, no pushes, no publish. Claude handles all of that
post-review.

---

## Prompt (verbatim text to send to Codex)

<task>
Implement Phase 2 Wave 2 of the Philharmonic workspace — the crypto foundation of `philharmonic-policy`. Read the full spec in this repo at:

- `docs/codex-prompts/2026-04-21-0002-phase-2-wave-2-crypto-foundation.md` — this file; **read verbatim before touching code**. It contains the approved primitives, the test vectors you must match byte-for-byte, and the flag-vs-fix policy.
- `docs/design/crypto-proposals/2026-04-21-phase-2-sck-and-pht.md` — Claude's proposal (with construction details + rationale).
- `docs/design/crypto-approvals/2026-04-21-phase-2-sck-and-pht.md` — Yuka's approval (with the version amendments: sha2 0.11, rand_core 0.10.1, getrandom 0.4.2).
- `docs/design/09-policy-and-tenancy.md` §"Tenant endpoint configs", §"Long-lived API tokens".
- `docs/design/11-security-and-cryptography.md` — threat model.
- `docs/design/13-conventions.md` — workspace conventions.
- `README.md` §"Terminology and language" — prose conventions.
- `philharmonic-policy/src/entity.rs` (Wave 1) — authoring style for `Entity` impls; add `TenantEndpointConfig` alongside the six existing kinds.
- `philharmonic-policy/tests/common/mock.rs` (Wave 1) — the `MockStore` you'll reuse.

Three deliverables:

1. **`TenantEndpointConfig` entity kind** with `KIND = uuid!("19d1a8f5-6ef0-49b0-adf5-48e1cd3daea9")` (reserved in Wave 1; use verbatim). Slot shape: content `display_name` + `encrypted_config`; entity `tenant` → `Tenant` (pinned); scalars `key_version: I64 (NOT indexed)` + `is_retired: Bool (indexed)`. Export from `src/lib.rs`.

2. **SCK AES-256-GCM module** (`src/sck.rs`): `Sck` (holding `Zeroizing<[u8; 32]>`), `Sck::from_bytes`, `Sck::from_file`, `sck_encrypt`, `sck_decrypt` (returning `Zeroizing<Vec<u8>>`), and a `pub(crate)` `sck_encrypt_with_nonce` seam for deterministic vector tests. Wire format `[0x01][nonce:12][ciphertext+tag]`; AAD `tenant_id.as_bytes() || config_uuid.as_bytes() || key_version.to_be_bytes()` (40 bytes). Decrypt failures return the same opaque `SckDecryptFailed` variant regardless of which AEAD step rejected (side-channel cleanliness).

3. **`pht_` token module** (`src/token.rs`): `TokenHash(pub [u8; 32])`, `generate_api_token` (draws from OS RNG, returns `(Zeroizing<String>, TokenHash)`), `parse_api_token` (47-char total; prefix `pht_`; 43-char base64url no-pad tail; hash the whole token string), and a `pub(crate)` `generate_api_token_from_bytes([u8; 32])` seam for vector tests.

Dependencies (add exactly these in `[dependencies]`, no others):
- `aes-gcm = "0.10"`
- `sha2 = "0.11"`
- `base64 = "0.22"`
- `rand_core = { version = "0.10.1", features = ["os_rng"] }`
- `getrandom = "0.4.2"`
- `zeroize = { version = "1", features = ["derive"] }`

**Do not add** `secrecy`, `subtle`, `hkdf`, `ml-kem`, `x25519-dalek`, `ed25519-dalek`, or any other crypto crate. The Gate-1 approval explicitly ruled these out for this wave.

`dev-dependencies`: existing Wave 1 deps stay. Add `hex-literal = "0.4"` (or whatever the current stable major is at resolution) for readable hex test constants; nothing else.

Test vectors — commit verbatim:
- The three SCK vectors listed in the prompt file's §"SCK vectors" section. Each fixes sck/nonce/tenant_id/config_uuid/key_version/plaintext → exact expected wire bytes. Assert `sck_encrypt_with_nonce` produces those exact bytes and `sck_decrypt` round-trips them. Plus the AAD-binding negatives and tag-tamper negative from the prompt's §"Test plan".
- The three `pht_` vectors listed in the prompt file's §"`pht_` vectors" section. Each fixes raw 32 bytes → exact expected token string + SHA-256 hash. Assert `generate_api_token_from_bytes` and `parse_api_token` produce those. Plus the length / prefix / base64 negatives.
- The two Python reference scripts (`gen_sck.py`, `gen_pht.py`) under `philharmonic-policy/tests/crypto_vectors/`. A short `README.md` in that directory explaining how to re-run them. The Python is for audit-reproducibility — `cargo test` doesn't touch Python.

**Hard constraints** (from the crypto-review protocol — non-waivable):
- No `unsafe` anywhere in the crate.
- No `anyhow` (library crate).
- No `println!`/`eprintln!`/`tracing` in library code.
- Key material is `Zeroizing<[u8; 32]>` inside `Sck`; decrypt returns `Zeroizing<Vec<u8>>`; `pht_` tokens are `Zeroizing<String>`.
- Decrypt errors return the same opaque variant (no information about which AEAD step failed).
- Don't hand-roll any primitive, even a "small utility" one. RustCrypto only.
- Don't modify any Wave 1 code. If you see a Wave 1 bug, **flag it, don't fix it** — Yuka reviews Wave 1 changes separately.

**Flag-vs-fix** per the crypto-review protocol: if you encounter (a) `unsafe` in neighboring code, (b) zeroization gaps, (c) signatures/MACs over unauthenticated untrusted input, (d) test vectors that don't match — flag in your final summary, don't "fix" or "update" to make things pass.

**Version handling**: if `rand_core = "0.10.1"` or `getrandom = "0.4.2"` don't resolve against the workspace, **stop and report**; do not pick a substitute version. These were specifically approved. Same for any other dependency resolution failure.

**Publishing**: `version` stays at `"0.0.0"`. Claude bumps after Yuka's Gate-2 review.

**Prose conventions apply** (`README.md §Terminology and language`): no `master`/`slave` for technical relationships, no gendered defaults, prefer `allowlist`/`denylist`, GNU/Linux (OS) vs. Linux kernel, no `win*` shorthand, prefer "free software"/"FLOSS" over standalone "open-source". External identifiers (HTTP `Authorization`, DB `MASTER`) are used literally.

**Git handling**: do not run any state-changing git command. Leave the working tree dirty. Claude drives `./scripts/commit-all.sh` + `./scripts/push-all.sh` after review.

When done, write a short summary covering: the file map (created / modified); the final public API signatures for every new symbol; test results per tier (Tier 1 crypto unit tests from `cargo test --workspace`, Tier 3 mock round-trip from same, Tier 2 MySQL from `cargo test -p philharmonic-policy -- --ignored` — expect 11 total now); `cargo tree | grep` output proving only the approved crypto crates are declared direct deps; `rg` output confirming no `unsafe` / `anyhow` / `println!` in library code; the flag list (empty if nothing); and the `philharmonic-store` / `philharmonic-types` trait-method surface you called.

Don't publish. Don't commit. Don't push.
</task>
