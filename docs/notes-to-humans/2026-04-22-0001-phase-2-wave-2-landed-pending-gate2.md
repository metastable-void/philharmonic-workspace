# Phase 2 Wave 2 — crypto foundation: impl ready for Gate-2 review

**Date:** 2026-04-22

## Status

Wave 2 code is on disk and passes every local gate short of
Yuka's Gate-2 review. Nothing committed — working tree is dirty,
awaiting line-by-line crypto review before commit.

- Tier 1 (crypto unit tests, `tests/crypto_vectors.rs`) — 15/15
  green. Includes all 3 SCK wire-format vectors (byte-for-byte
  match against Python reference), all 3 `pht_` token vectors
  (byte-for-byte match against stdlib `hashlib`), and every
  error-path negative enumerated in the prompt.
- Tier 3 (mock round-trip, `tests/permission_mock.rs`) — 12/12
  green (11 Wave 1 + 1 new `TenantEndpointConfig`).
- Tier 2 (real MySQL, `tests/permission_mysql.rs`) — 11/11 green
  in 143s (10 Wave 1 + 1 new `tenant_endpoint_config_entity_round_trip`).
- `./scripts/pre-landing.sh --no-ignored philharmonic-policy`
  clean.

## Dep-resolution amendment (Gate-1 updated in same session)

Codex hit a hard blocker on `rand_core = "0.10.1"` with
`features = ["os_rng"]` — that feature flag was removed in the
0.10 series. Yuka authorized the swap to `rand = "0.10"`
default features in-session; the Gate-1 approval doc is
amended accordingly (see
`docs/design/crypto-approvals/2026-04-21-phase-2-sck-and-pht.md`).

The rand 0.10 API has several renames vs. the 0.9 approval
text:

- `OsRng` → `SysRng` (re-exported from `getrandom`).
- `RngCore` trait split into `Rng` (infallible) + `TryRng`
  (fallible); `SysRng` only implements `TryRng` / `TryCryptoRng`.
- The `os_rng` feature was renamed to `sys_rng` (enabled by
  default).

Usage pattern ended up as:
```rust
use rand::TryRng;
use rand::rngs::SysRng;
SysRng.try_fill_bytes(buf).expect("OS RNG failure — system entropy unavailable");
```

Matches how `ThreadRng` handles OS RNG failure internally and
how RustCrypto / `ring` treat OS-RNG failure (unrecoverable
panic — you can't mint crypto material without entropy).

## Test-quality finding Codex shipped (caught by pre-landing, worth flagging)

The original `rotate_uuid_by_one_byte` helper used
`bytes.rotate_left(1)`. All three SCK test vectors use
uniform-byte UUIDs (`11111111-...`, `33333333-...`,
`55555555-...`), and rotating a uniform-byte array by one
position is the identity. Result: the tenant_id and
config_uuid tamper-negative tests were **vacuous** — they
round-tripped the same UUID, so the AAD binding of those
fields never actually fired, and the tests passed green.

This almost shipped. The construction itself is correct (AAD
binds tenant_id and config_uuid as specified), but the test
suite would not have caught a silent regression where tenant_id
or config_uuid dropped out of the AAD. Fixed to
`flip_one_bit_in_uuid` with `bytes[0] ^= 0x01`; with that fix,
the previously-trivially-green tests now fail before the fix
and pass after — confirming AAD binding on both fields works.

Takeaway: for future crypto dispatches, assert that "wrong X"
tamper helpers actually produce a different value for the
specific vectors in use. A `debug_assert_ne!(original, tampered)`
inside the helper would have caught this.

## Encrypt-error variant — semantic quirk worth noting

`src/sck.rs:118` maps `Aes256Gcm::encrypt` failures to
`PolicyError::SckDecryptFailed` — the "decrypt" variant
repurposed for the encrypt path. The prompt's approved error
variant list doesn't include an `SckEncryptFailed` variant,
and `aes-gcm`'s `encrypt` with a `Vec<u8>` output is provably
infallible for any plaintext we'd actually pass, so this path
is unreachable in practice. But the semantic naming is off.

Options for Gate-2 to decide (all small):
1. Leave it — the path is unreachable, no runtime consequence.
2. Add `PolicyError::SckEncryptFailed` and map the encrypt
   error there.
3. Treat encrypt as infallible at the type level —
   `.expect("aes-gcm encrypt cannot fail for Vec<u8> output")`
   and drop the `Result` wrapper from the encrypt path.

Option 1 or 3 keeps the approved error variant list closed.
Option 2 amends it. No strong preference from Claude; flagging
for Yuka.

## Integration-test `pub(crate)` seam pattern

`tests/crypto_vectors.rs` accesses `sck_encrypt_with_nonce` and
`generate_api_token_from_bytes` (both `pub(crate)`) via
`#[path = "../src/sck.rs"] mod sck_internal;`. Rust treats this
as a second compilation of the file, so unused public items
(e.g. `sck_encrypt`, `Sck::from_file`, the `TOKEN_BYTES`
constant) fire dead-code warnings that `clippy -D warnings`
promotes to errors. Added `#[allow(dead_code)]` on the two
`mod` lines. If Yuka objects to the attribute, the alternative
is to expose the seams as `pub` in a dedicated `#[doc(hidden)]
pub mod internal` — more surface area, same effect. Kept the
minimal change.

## Dep tree (for Gate-2)

`cargo tree -p philharmonic-policy --depth 1` declared direct
deps (crypto-relevant rows only):

```
├── aes-gcm v0.10.3
├── base64 v0.22.1
├── rand v0.10.1
├── sha2 v0.11.0
├── uuid v1.23.1
└── zeroize v1.8.2
```

`rand_core 0.10.1` and `getrandom 0.4.2` are pulled in
transitively via `rand`. The original Gate-1 acceptance
criterion that listed them as direct deps is superseded by the
2026-04-22 amendment.

`rg '\bunsafe\b' philharmonic-policy/src` — no hits. `rg
'\banyhow\b' philharmonic-policy/src` — no hits. `rg
'println!|eprintln!' philharmonic-policy/src` — no hits.

## File map (pending commit — Claude's list for Gate-2 review)

New files:
- `philharmonic-policy/src/sck.rs`
- `philharmonic-policy/src/token.rs`
- `philharmonic-policy/tests/crypto_vectors.rs`
- `philharmonic-policy/tests/crypto_vectors/gen_sck.py`
- `philharmonic-policy/tests/crypto_vectors/gen_pht.py`
- `philharmonic-policy/tests/crypto_vectors/README.md`

Modified by Codex:
- `philharmonic-policy/Cargo.toml` (deps; version still `0.0.0`)
- `philharmonic-policy/src/lib.rs` (exports)
- `philharmonic-policy/src/error.rs` (new variants)
- `philharmonic-policy/src/entity.rs` (TenantEndpointConfig)
- `philharmonic-policy/tests/permission_mock.rs` (+1 test)
- `philharmonic-policy/tests/permission_mysql.rs` (+1 test)

Modified by Claude post-hand-off (housekeeping / blocker fixes):
- `philharmonic-policy/Cargo.toml` — `rand_core` + `getrandom`
  → `rand = "0.10"` (per Yuka's in-session authorization).
- `philharmonic-policy/src/sck.rs` — `rand_core::{OsRng, RngCore}`
  → `rand::{TryRng, rngs::SysRng}`; `fill_random` now calls
  `try_fill_bytes(...).expect(...)`.
- `philharmonic-policy/src/token.rs` — same RNG import swap;
  also fixed borrow-after-move on `token` in
  `generate_api_token_from_bytes` (was
  `(token, TokenHash(hash_token(&token)))`, now hash bound
  to a local first).
- `philharmonic-policy/tests/crypto_vectors.rs` — renamed
  `rotate_uuid_by_one_byte` (vacuous for uniform-byte UUIDs)
  to `flip_one_bit_in_uuid` with `bytes[0] ^= 0x01`; added
  `#[allow(dead_code)]` on the two `#[path]` `mod` declarations.

Gate-1 doc:
- `docs/design/crypto-approvals/2026-04-21-phase-2-sck-and-pht.md`
  — appended 2026-04-22 amendment.

## What's next

Yuka's Gate-2 review of the crypto code, line-by-line, before
`philharmonic-policy 0.1.0` ships. Specific things worth
Yuka's eyes:

1. The AAD construction in `src/sck.rs:127-133` (`build_aad`).
2. The wire format encoder + decoder in `src/sck.rs:44-125`.
3. The `Sck::from_file` length check in `src/sck.rs:30-42`.
4. The token-string hashing in `src/token.rs` (hashes full
   `pht_`-prefixed string, not just decoded bytes).
5. The encrypt-error-variant question raised above.
6. The three SCK + three `pht_` vectors, cross-checked against
   the Python generators.

After Gate-2 clears, Claude bumps `philharmonic-policy` to
`0.1.0`, commits via `./scripts/commit-all.sh`, runs
`./scripts/check-api-breakage.sh`, publishes, tags, pushes.
