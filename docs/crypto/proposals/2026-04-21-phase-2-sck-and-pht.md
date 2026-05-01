# Gate-1 crypto proposal: SCK encryption + `pht_` API tokens (Phase 2 Wave 2)

**Date:** 2026-04-21
**Authors:** Claude (for Yuka's Gate-1 review).
**Scope:** the two crypto-sensitive paths in `philharmonic-policy`
per `.claude/skills/crypto-review-protocol`:
1. SCK AES-256-GCM over `TenantEndpointConfig.encrypted_config`.
2. `pht_` long-lived API token generation, parsing, and storage hash.
**References:**
- `docs/design/09-policy-and-tenancy.md` §"Tenant endpoint configs",
  §"Long-lived API token format".
- `docs/design/11-security-and-cryptography.md` §"Cryptographic
  primitives", §"Substrate at-rest endpoint config encryption",
  §"Long-lived API tokens".
- `ROADMAP.md` §5 "Crypto review protocol".
- `.claude/skills/crypto-review-protocol/SKILL.md`.

**Requesting sign-off on:** primitives + versions, construction
details (nonce, AAD, wire layout, storage), zeroization points,
key-handling story, and the test-vector plan. Once signed off,
Claude drafts the Wave-2 Codex prompt embedding the approved
vectors as part of it (per the protocol: Codex doesn't generate
the reference values it's also verified against).

---

## Part A — SCK AES-256-GCM over `TenantEndpointConfig.encrypted_config`

### A.1 Primitive + library versions

- `aes-gcm = "0.10"` (RustCrypto) — `Aes256Gcm`.
- `rand_core = "0.6"` + `getrandom = "0.2"` — `OsRng` for fresh nonces.
- `zeroize = "1"` — `Zeroizing<[u8; 32]>` for the SCK, `Zeroizing<Vec<u8>>`
  for the plaintext blob in memory.

No other crypto crate. No `unsafe`. No custom primitives.

### A.2 Construction

Input to encryption:
- `sck: &[u8; 32]` — the 256-bit AES key held in deployment secret
  storage.
- `plaintext: &[u8]` — the admin-submitted JSON blob (realm, impl,
  config, credentials — the whole thing).
- `tenant_id: Uuid` — the owning tenant.
- `config_uuid: Uuid` — the `TenantEndpointConfig` entity's public
  UUID.
- `key_version: i64` — the `key_version` scalar (what generation
  of the SCK is encrypting this).

Procedure:
1. Draw `nonce: [u8; 12]` from `OsRng`. (96-bit nonce, as required
   by AES-GCM.)
2. Build `aad: Vec<u8>` = `tenant_id.as_bytes() (16) ||
   config_uuid.as_bytes() (16) || key_version.to_be_bytes() (8)`
   — exactly 40 bytes. Big-endian for `key_version` for wire
   determinism across platforms.
3. `Aes256Gcm::new(&sck)` → encrypt `plaintext` with `nonce` and
   `aad`. The aes-gcm crate returns `ciphertext || tag(16)` as a
   single `Vec<u8>`.
4. Emit wire bytes: `[version:u8=0x01] [nonce:12] [ciphertext+tag:N]`
   — total = 1 + 12 + len(plaintext) + 16 bytes.
5. Zeroize the plaintext buffer after use (it lives only briefly
   in memory; `Zeroizing<Vec<u8>>` at the caller's layer is
   acceptable).

Decryption reverses exactly: parse version byte (fail unless
`0x01`), split nonce and ciphertext+tag, reconstruct AAD the same
way, decrypt. The AEAD fails loudly on any mismatch (tag, nonce,
key, AAD) — no silent corruption path.

### A.3 Why each choice

- **Nonce: random per-encryption.** GCM's safety limit under
  random nonces is ~2^32 encryptions per key (birthday bound for
  96-bit nonces). For any realistic SCK usage (tens of thousands
  of configs encrypted over a key's lifetime) this is far below
  the limit. Counter-based nonces are rejected because the SCK is
  shared across API processes — synchronizing a counter across
  processes would introduce complexity without a security
  benefit.
- **AAD: `tenant_id || config_uuid || key_version`.** Binds the
  ciphertext to its storage context. If someone copy-pastes the
  ciphertext bytes from entity A into entity B in the substrate,
  decryption fails because the AAD won't match. Cheap defense-in-
  depth against substrate-level tampering.
- **Format-version byte.** Cheap future-proofing. If we later
  switch to AES-256-GCM-SIV or XChaCha20-Poly1305, a new version
  byte (0x02, …) lets both ciphertexts coexist during migration
  without a distinct content schema.
- **Big-endian `key_version`.** Deterministic wire encoding that
  doesn't depend on build target endianness.

### A.4 Key handling

- **Where it lives:** deployment secret storage, path configurable
  via environment variable (e.g. `PHILHARMONIC_SCK_PATH`). File
  contents are 32 raw bytes. Loaded once at API-process startup.
- **In memory:** `Zeroizing<[u8; 32]>` wrapper. Zeroized on drop.
- **Rotation:** new SCK generated, `key_version` incremented, a
  migration task walks every `TenantEndpointConfig` entity and
  appends a new revision with the blob re-encrypted under the new
  SCK and the bumped `key_version`. Old SCK is retained until
  every entity has been migrated, then retired. Rotation itself
  is out of Phase 2 scope (deployment tooling concern) — but the
  wire format supports it by including `key_version` in both the
  AAD and the `TenantEndpointConfig.key_version` scalar.
- **Zeroization gaps:** the JSON plaintext transits through
  `serde_json::from_slice` / `to_vec`, which allocate
  intermediate `String`s and `Vec<u8>`s. We cannot Zeroize
  inside serde internals. Accept this as a known limitation of
  Rust's crypto ecosystem; the plaintext window is short-lived.

### A.5 Public API shape (proposed, subject to Yuka's sign-off)

```rust
/// Opaque wrapper around the loaded SCK, zeroized on drop.
pub struct Sck { key: Zeroizing<[u8; 32]> }

impl Sck {
    pub fn from_bytes(bytes: [u8; 32]) -> Self { ... }
    pub fn from_file(path: &Path) -> Result<Self, PolicyError> { ... }
}

/// Encrypted blob wire format: [version:u8] [nonce:12] [ciphertext+tag].
pub fn sck_encrypt(
    sck: &Sck,
    plaintext: &[u8],
    tenant_id: Uuid,
    config_uuid: Uuid,
    key_version: i64,
) -> Result<Vec<u8>, PolicyError>;

pub fn sck_decrypt(
    sck: &Sck,
    wire: &[u8],
    tenant_id: Uuid,
    config_uuid: Uuid,
    key_version: i64,
) -> Result<Zeroizing<Vec<u8>>, PolicyError>;
```

Return type of `sck_decrypt` is `Zeroizing<Vec<u8>>` so the
plaintext is zeroized when the caller drops it.

### A.6 Test vectors for SCK

Three triplets, committed as hex in `tests/crypto_vectors/sck.rs`.
Each vector fixes `sck`, `plaintext`, `nonce`, `tenant_id`,
`config_uuid`, `key_version` → exact `expected_wire_bytes`.

Reference values produced via Python `cryptography` (a standard
AES-GCM implementation independent of RustCrypto). The Python
reference code is committed alongside the vectors as
`tests/crypto_vectors/gen_sck_vectors.py` so we can re-run it
if the reference implementation ever needs to be reproduced.

Vector-1: simple JSON blob (`{"realm":"llm","impl":"x","config":{}}`).
Vector-2: non-ASCII JSON (unicode characters in a value).
Vector-3: edge — one-character blob (`{}`).

Per-vector assertions:
- `sck_encrypt(inputs) == expected_wire_bytes`.
- `sck_decrypt(expected_wire_bytes, same_metadata) == plaintext`.
- `sck_decrypt` with a different tenant_id / config_uuid /
  key_version returns `Err(_)` (AAD-binding test).
- `sck_decrypt` with a bit-flipped tag byte returns `Err(_)`.

---

## Part B — `pht_` long-lived API token generation + storage hash

### B.1 Primitive + library versions

- `rand_core = "0.6"` + `getrandom = "0.2"` — `OsRng` to draw the
  32 random bytes.
- `base64 = "0.22"` — `base64::engine::general_purpose::URL_SAFE_NO_PAD`
  for the encoding.
- `sha2 = "0.10"` (RustCrypto) — `Sha256::digest()` for storage hash.
- `zeroize = "1"` — `Zeroizing<String>` for the generated plaintext
  token (returned to caller, zeroized when their copy drops).

No constant-time equality is needed for this code. Storage lookup
goes through the substrate's content-addressed index, which
performs B-tree equality at the storage layer (no timing leak in
our code). This is different from comparing MAC tags or
signatures — please confirm this reasoning.

### B.2 Construction

Generation:
1. Draw `raw: [u8; 32]` from `OsRng`.
2. Encode via `URL_SAFE_NO_PAD` → 43-char string.
3. `token = format!("pht_{}", encoded)` → 47-char total.
4. `hash = Sha256::digest(token.as_bytes())` → 32 bytes.
5. Return `(Zeroizing::new(token), hash)`.

Parsing (for verifying an incoming token before storage lookup):
1. Exact-length check: `s.len() == 47` (else `Err`).
2. Prefix check: `s.starts_with("pht_")` (else `Err`).
3. Extract the 43-char tail; base64url-decode to exactly 32 bytes
   (else `Err`).
4. Return `Sha256::digest(s.as_bytes())` — the 32-byte hash for
   substrate lookup.

Constants:

```rust
pub const TOKEN_PREFIX: &str = "pht_";
pub const TOKEN_BYTES: usize = 32;
pub const TOKEN_ENCODED_LEN: usize = 43;  // base64url unpadded of 32 bytes
pub const TOKEN_FULL_LEN: usize = TOKEN_PREFIX.len() + TOKEN_ENCODED_LEN; // 47
```

### B.3 Why each choice

- **32 bytes from OsRng.** 256 bits of entropy. `OsRng` is the
  standard CSPRNG wrapper across RustCrypto.
- **Hash the WHOLE token (including `pht_` prefix).** Matches the
  design doc verbatim. The hash space for a leaked stored-hash +
  known prefix is still 2^256.
- **Plaintext never persisted.** Zeroizing wrapper on the return
  value encourages caller-side zeroization after display.

### B.4 Key handling

Not a key. No long-term storage of randomness. The only handling
concern is the plaintext token's brief existence in the creating
client's memory and transit back to them. Out of scope for this
crate (the transport/UI layer handles disposal).

### B.5 Public API shape (proposed)

```rust
/// Newtype wrapper around the SHA-256 hash of a `pht_` token.
/// Transparent bytes; comparison is via the substrate index.
pub struct TokenHash(pub [u8; 32]);

/// Generate a fresh `pht_` token. Returns (zeroizing token for
/// one-shot display to the creating client, storage hash).
pub fn generate_api_token() -> (Zeroizing<String>, TokenHash);

/// Validate a token string's format and compute its storage hash.
/// Errors on wrong length, missing prefix, or invalid base64url.
pub fn parse_api_token(s: &str) -> Result<TokenHash, PolicyError>;
```

### B.6 Test vectors for `pht_`

Three triplets, committed as hex in
`tests/crypto_vectors/pht.rs`. Each vector fixes the 32-byte
RNG seed → expected token string → expected SHA-256 hash.

For generation, we can't fix the RNG directly from `OsRng`; the
test calls a crate-internal `generate_api_token_from_bytes(raw:
[u8; 32])` which takes the raw bytes directly. The public
`generate_api_token()` is a thin wrapper that draws from `OsRng`
and forwards. The seam is `pub(crate)` so only tests cross it.

Reference values produced via Python (32 bytes in → `base64.urlsafe_b64encode`
strip padding → prepend `pht_` → `hashlib.sha256` of the whole
string). Python code committed as
`tests/crypto_vectors/gen_pht_vectors.py`.

Per-vector assertions:
- `generate_api_token_from_bytes(raw) == (expected_token,
  expected_hash)`.
- `parse_api_token(&expected_token) == Ok(expected_hash)`.
- `parse_api_token("too short") == Err(_)`.
- `parse_api_token("php_<43-char-b64>") == Err(_)` (wrong prefix).
- `parse_api_token("pht_<43-char-with-invalid-b64-chars>") == Err(_)`.

---

## Hard constraints (verbatim from the protocol)

- No `unsafe` blocks in `philharmonic-policy` or its immediate
  dependents.
- No custom primitives; only the RustCrypto suite plus `base64`.
- Key material is zeroized (`Zeroizing` wrappers on every in-memory
  byte of SCK; `Zeroizing<Vec<u8>>` for decrypted plaintext blobs;
  `Zeroizing<String>` for freshly-generated `pht_` tokens).
- No signatures or MACs over attacker-controlled data introduced
  in this Phase.

## Surface for Yuka's attention

1. **AAD composition for SCK.** Proposed: `tenant_id || config_uuid
   || key_version_be_bytes`. Alternative considered: just
   `key_version` (lighter, still distinguishes generations but
   doesn't prevent cross-entity replay). I've chosen the heavier
   AAD because the cost is trivial and the defense is
   non-trivial. Confirm.
2. **Wire layout `[version:u8=0x01] [nonce:12] [ciphertext+tag]`.**
   Confirm the 1-byte version prefix is worth the cost (makes
   future migration a single `match` arm; adds 1 byte per config
   blob; no downside I can see).
3. **No constant-time equality in `pht_`.** My reasoning: hash
   lookup goes through the substrate's content-addressed B-tree
   index, so the equality check doesn't happen in our code.
   Confirm — if the actual lookup path does linear-scan byte
   compares, I'll add `subtle::ConstantTimeEq` to the proposal.
4. **`generate_api_token_from_bytes` seam for testing.** Tests
   need deterministic inputs; the `pub(crate)` seam lets them
   supply raw bytes directly. Alternative: inject an RNG trait.
   Trait injection is more flexible but heavier; `pub(crate)` fn
   is simpler and hides cleanly. Confirm this is acceptable.
5. **Serde zeroization gap on SCK plaintext.** Plaintext JSON
   passes through `serde_json` which allocates non-zeroizing
   `String`/`Vec<u8>` internally. We cannot reach inside serde.
   The window is short (submit handler or lowerer decrypt →
   immediate next step). Confirm this is acceptable for v1.
6. **Key rotation tooling out of scope.** Phase 2 ships the
   encryption + the `key_version` scalar; the migration task that
   rotates SCK (walks all configs, re-encrypts, bumps `key_version`)
   lands in a later phase (likely Phase 8 with the API layer).
   Flag if this splits the wrong way.

## After Gate 1 sign-off

Claude drafts the Wave-2 Codex prompt embedding these vectors
verbatim. Codex implements; Claude reviews; Yuka does Gate-2
line-by-line review before `philharmonic-policy 0.1.0` publishes.

The Wave-1 Codex prompt (non-crypto foundation — 6 entity kinds +
permission evaluation) can proceed in parallel; it explicitly
excludes `TenantEndpointConfig`, SCK code, and `pht_` code.
Approval of the Wave-1 prompt is independent of this gate.
