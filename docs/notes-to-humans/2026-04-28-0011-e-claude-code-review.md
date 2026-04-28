# Sub-phase E — Claude code review

**Author:** Claude Code · **Audience:** Yuka ·
**Date:** 2026-04-28 (Tue) JST afternoon

Crypto-touching sub-phase (SCK encrypt/decrypt call sites).
Code-review gate fires.

## Verdict

**PASSES.** 6 endpoint-config handlers with correct SCK
usage, no plaintext/ciphertext leakage, generic crypto error
collapsing. Ready for Yuka's crypto call-site review.

## Security checklist (crypto focus)

- **`sck_encrypt` call** (line 428, `encrypt_config`):
  canonicalizes config JSON via `CanonicalJson::from_value`,
  encrypts the canonical bytes with tenant_id + config_uuid +
  key_version as AAD, stores the wire ciphertext as content.
  Error → generic `"endpoint config encryption failed"` 500.
  ✅
- **`sck_decrypt` call** (line 196, `read_endpoint_decrypted`):
  reads stored `key_version` from the revision (not the
  builder's current version), loads `encrypted_config` content
  blob, decrypts with tenant_id + config_uuid + key_version.
  Error → generic `"endpoint config decryption failed"` 500.
  ✅
- **No plaintext in metadata reads** — `read_endpoint_metadata`
  returns only `display_name`, `is_retired`, `key_version`,
  timestamps. Never touches `encrypted_config`. ✅
- **No ciphertext in any response** — raw `encrypted_config`
  hash is internal; never serialized to response bodies. ✅
- **No plaintext/ciphertext in logs** — zero `tracing::` calls
  in the entire endpoint module. ✅
- **SCK decrypt error → generic message** — `map_err(|_| ...)`
  discards the specific AES-GCM error (which could otherwise
  leak whether it was a tag mismatch, nonce issue, etc.). ✅
- **`key_version` from stored revision on decrypt** — not from
  the builder's current key_version. This is correct: each
  config revision carries the key_version it was encrypted
  under, and that's what decrypt needs. A key-rotated
  deployment where old revisions have a different key_version
  will still decrypt correctly. ✅

## What Yuka should focus on

1. **AAD binding correctness** — `sck_encrypt` and
   `sck_decrypt` both pass `(tenant_id, config_uuid,
   key_version)` as AAD components. Confirm these match the
   `build_aad` function in `philharmonic-policy/src/sck.rs`.
2. **The `require_sck` gate** — if SCK is not configured, all
   endpoint-config routes return 500. Confirm this is the
   right behavior (vs. 501 or disabling routes entirely).
3. **`display_name` as required content** — metadata response
   treats `display_name` as required (errors if missing); the
   create handler always stores it. Rotate preserves it or
   updates it.

## Test coverage

8 new integration tests in `tests/endpoint_config.rs`:
- Create → read metadata → read decrypted → assert plaintext
- Create → rotate → read decrypted → assert new config
- Create → retire → assert retired
- List → paginated
- Read decrypted without permission → 403
- Metadata read without `endpoint:read_decrypted` → succeeds
  (metadata-only)
- Cross-tenant read → 404
- Missing SCK → 500

59 total tests green. Clippy clean. No panics on library paths.
