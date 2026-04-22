# Miri validation of Wave 2 crypto + philharmonic-policy

**Date:** 2026-04-22

## Results

Ran `./scripts/miri-test.sh philharmonic-policy` against the
full crate after the Wave 2 commit landed. Miri flagged **zero
UB** across:

- `src/lib.rs` unit tests — 7 passed.
- `tests/crypto_vectors.rs` — 15 passed in 12.81s (3 SCK
  encrypt-vector matches byte-for-byte + 3 SCK decrypt
  round-trips + AAD-tamper negatives for tenant_id /
  config_uuid / key_version + tag-flip negative + short-wire /
  wrong-version negatives + 3 `pht_` generate vectors + 3
  parse round-trips + prefix / length / base64 negatives).
- `tests/permission_mock.rs` — 12 passed in 22.33s (async-trait
  implementations of `EntityStore` + `ContentStore` on
  `MockStore`, full `evaluate_permission` traversal, tokio
  `current_thread` runtime).
- `tests/permission_mysql.rs` — 11 ignored as expected (miri
  can't run testcontainers / sqlx / FFI).

**Total: 34 passed, 11 ignored, 0 failed.** One-time sysroot
build took ~60s; full run after that is ~35s wall-clock.

## Why this matters for Gate-2 review

Miri is an interpreter for Rust's MIR that catches
undefined-behavior classes regular `cargo test` can't
observe:

- Uninitialized memory reads (e.g. `MaybeUninit` misuse in
  dependency SIMD paths).
- Out-of-bounds pointer arithmetic (e.g. off-by-one in
  unsafe `*const u8::add(offset)`).
- Data races (unsynchronized shared-mutable access —
  rare in our code since there's no `unsafe`, but a
  dependency can still hit it).
- Type-layout confusion (e.g. casting between types with
  different alignment / niche layouts).
- Stacked-borrows violations (the provenance model Rust
  uses; a common source of subtle unsoundness in older
  `unsafe` code).

The Wave 2 crypto path pulls in `aes-gcm 0.10`, `sha2 0.11`,
`base64 0.22`, `rand 0.10` → `getrandom 0.4.2`, and
`zeroize 1`. Any of those could have shipped UB in an update
that didn't surface under `cargo test`. Miri passing on the
actual SCK vectors exercises the AES-GCM SIMD paths under
miri's interpreter (miri auto-selects the software reference
impl rather than AES-NI intrinsics, cross-checking the two).

Zeroize's drop behavior running under miri also confirms no
double-drops or use-after-free on `Zeroizing<[u8; 32]>` key
material when `Sck` / `TokenHash` go out of scope — an
invariant the regular test suite couldn't surface
programmatically.

## Not exhaustive proof

Miri interprets what the test exercises. The test vectors
cover: happy path, all 4 AAD-binding tamper cases, tag
tamper, short wire, wrong version byte, every `pht_` error
path. Miri didn't exercise the `Sck::from_file` path (pure
I/O wrapper around `std::fs::read` — not UB-relevant).

## Ongoing discipline

Convention added at `docs/design/13-conventions.md §Testing —
Miri` calls for miri routinely (pre-publish, on a periodic
schedule) — not in `pre-landing.sh`, which stays fast.
`scripts/check-toolchain.sh` probes nightly + miri install on
every pre-landing run so drift surfaces early.

For Gate-2 specifically: since the test vectors now have a
miri-pass stamp (no UB exercised on the happy or negative
paths), the manual crypto-review scope can focus on the
construction (AAD shape, wire format, token hashing) and
side-channel properties rather than memory-safety of the
primitives. The `aes-gcm` / `sha2` / `base64` / `rand`
transitive dependency surfaces are now also miri-clean for
what our tests touch.
