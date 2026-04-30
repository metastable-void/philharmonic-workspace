# Gate-1 proposal: real `ConfigLowerer` implementation

**Date**: 2026-04-30
**Author**: Claude Code
**Status**: Gate-1 APPROVED by Yuka (2026-04-30, inline)

---

## Summary

Implement a real `ConfigLowerer` (replacing `StubLowerer`) that
wraps the existing `philharmonic-connector-client` library to
produce per-step COSE_Sign1 authorization tokens and
COSE_Encrypt0 encrypted payloads. **No new cryptographic
primitives are introduced.** The implementation calls
already-approved, Gate-2-passed library functions:

- `LowererSigningKey::mint_token()` — COSE_Sign1 with Ed25519
  (Phase 5 Wave A, Gate-2 approved 2026-04-22)
- `encrypt_payload()` — hybrid ML-KEM-768 + X25519 + HKDF +
  AES-256-GCM COSE_Encrypt0 (Phase 5 Wave B, Gate-2 approved
  2026-04-23)

The lowerer is a **caller** of these primitives, not a new
construction. It lives in the `philharmonic` meta-crate (bin
target code), not in a library crate.

## What the `ConfigLowerer` trait requires

From `philharmonic-workflow/src/lowerer.rs`:

```rust
#[async_trait]
pub trait ConfigLowerer: Send + Sync {
    async fn lower(
        &self,
        abstract_config: &JsonValue,
        instance_id: EntityId<WorkflowInstance>,
        step_seq: u64,
        subject: &SubjectContext,
    ) -> Result<JsonValue, ConfigLoweringError>;
}
```

Input: abstract endpoint config (JSON, contains `realm`,
`config_uuid`, `endpoint_config` fields referencing a
`TenantEndpointConfig` entity). Output: lowered config (JSON)
containing the COSE_Sign1 token bytes (hex) and COSE_Encrypt0
encrypted payload bytes (hex) ready for the connector service.

## Proposed implementation

### Struct: `ConnectorConfigLowerer`

Lives in `philharmonic/src/bin/philharmonic_api/lowerer.rs`
(bin-level code, not a library crate).

```rust
pub struct ConnectorConfigLowerer {
    signing_key: LowererSigningKey,
    realm_keys: HashMap<String, RealmPublicKey>,
    issuer: String,
    token_lifetime_ms: u64,  // default 600_000 (10 min)
}
```

### Flow inside `lower()`

1. **Parse abstract config**: extract `realm`, `config_uuid`,
   and `endpoint_config` (the decrypted endpoint config JSON —
   decrypted by the API layer via SCK before reaching the
   workflow engine).

2. **Look up realm public key**: `realm_keys[&realm]`. Error if
   not found.

3. **Build `ConnectorTokenClaims`**:
   ```rust
   ConnectorTokenClaims {
       iss: self.issuer.clone(),
       exp: UnixMillis(now_ms + self.token_lifetime_ms),
       iat: UnixMillis(now_ms),
       kid: self.signing_key.kid().to_string(),
       realm: realm.clone(),
       tenant: subject.tenant_id.internal().as_uuid(),
       inst: instance_id.internal().as_uuid(),
       step: step_seq,
       config_uuid: config_uuid,
       payload_hash: /* computed after encrypt, see below */
   }
   ```

4. **Serialize plaintext payload**: JSON containing `realm`,
   `impl` (connector implementation name), `config` (the
   endpoint config content), and `request` (the step request).

5. **Encrypt payload**: call `encrypt_payload(plaintext,
   &realm_key, aad_inputs, &mut OsRng)`. This produces a
   `ConnectorEncryptedPayload` (COSE_Encrypt0 bytes).

6. **Compute payload hash**: SHA-256 of the encrypted payload
   bytes.

7. **Set `payload_hash` in claims**, then mint the token:
   `signing_key.mint_token(&claims)`.

8. **Return lowered config** as JSON:
   ```json
   {
     "token": "<hex-encoded COSE_Sign1 bytes>",
     "encrypted_payload": "<hex-encoded COSE_Encrypt0 bytes>"
   }
   ```

### Crypto primitives used

| Primitive | Library | Version | Gate |
|---|---|---|---|
| Ed25519 signing (COSE_Sign1) | `ed25519-dalek` | 2.x | Wave A Gate-2 ✅ |
| ML-KEM-768 encapsulation | `ml-kem` | 0.2 | Wave B Gate-2 ✅ |
| X25519 ECDH | `x25519-dalek` | 2.x | Wave B Gate-2 ✅ |
| HKDF-SHA256 | `hkdf` + `sha2` | 0.13 / 0.11 | Wave B Gate-2 ✅ |
| AES-256-GCM (COSE_Encrypt0) | `aes-gcm` | 0.10 | Wave B Gate-2 ✅ |
| SHA-256 (payload hash) | `sha2` | 0.11 | Phase 2 Gate-2 ✅ |

**No new primitives. No new constructions. No new library
dependencies.** Everything is called through the existing
`philharmonic-connector-client` API (`mint_token` +
`encrypt_payload`).

### Key material handling

- **`LowererSigningKey`**: already wraps `Zeroizing<[u8; 32]>`
  with per-call `SigningKey` reconstruction (reviewed in
  Wave A Gate-2).
- **`RealmPublicKey`**: public key material — no zeroization
  needed.
- **RNG**: `rand::rngs::OsRng` for encrypt_payload's nonce
  and KEM randomness.
- No new key material is created by the lowerer; it uses keys
  loaded at bin startup from config files (already implemented
  in the connector bin).

### What this does NOT do

- Does **not** implement any new cryptographic primitive.
- Does **not** modify `philharmonic-connector-client` or any
  other library crate.
- Does **not** introduce `unsafe` code.
- Does **not** handle key rotation (the lowerer uses whatever
  keys are in its config; rotation happens via SIGHUP
  config reload, already implemented).

### Test plan

Since the lowerer calls existing Gate-2-approved functions,
**no new crypto test vectors are needed**. The existing test
vectors in `philharmonic-connector-client` and
`philharmonic-connector-service` cover the primitives.

The lowerer itself should have:
- A unit test verifying that `lower()` returns a JSON object
  with `token` and `encrypted_payload` hex strings.
- An integration test (in the e2e suite) verifying the full
  pipeline: lowerer produces output → connector service
  `verify_and_decrypt` succeeds on that output.

### Configuration

The API bin's `ApiConfig` gains:
- `lowerer_signing_key_path: Option<PathBuf>` — Ed25519 seed
  for the lowerer (32 bytes). May be the same key as the API
  signing key, or a separate one.
- `lowerer_signing_key_kid: Option<String>`
- `realm_public_keys: Vec<RealmPublicKeyConfig>` — kid, realm,
  mlkem public key path, x25519 public key path, validity
  window.
- `lowerer_token_lifetime_ms: u64` — default 600_000 (10 min).

## Decisions (resolved 2026-04-30)

1. **Lowerer signing key**: left to deployment — may be the
   same or separate from the API signing key. Config accepts
   an independent path/kid.

2. **Token lifetime**: **600 seconds (10 minutes)**, not 60.
   Yuka's rationale: dependencies on HTTP requests often block
   longer than expected; 60s is too tight. 600s gives
   comfortable margin for slow upstream connectors without
   opening an unreasonable replay window.

3. **Gate-1 sufficiency**: Yuka approved inline — "trivial
   since it's calling existing Gate-2-approved functions, no
   new crypto." Full Gate-1 proposal + approval recorded here
   for the audit trail; implementation proceeds immediately.
