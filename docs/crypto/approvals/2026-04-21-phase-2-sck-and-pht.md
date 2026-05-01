# Gate-1 crypto approval

I approve the proposal, with the following changes.

## A1. SCK AES-256-GCM primitives

there is rand_core 0.10.1 and getrandom 0.4.2. please use the up-to-date versions.

aes-gcm = "0.10" and zeroize = "1" is okay.

## B1.

use sha2 = "0.11"

## Finally

it's okay otherwise. please continue.

---

## Amendment — 2026-04-22

`rand_core = "0.10.1"` with `features = ["os_rng"]` does not resolve
— that feature was removed in the 0.10 series. Yuka authorized
switching to `rand = "0.10"` (default features) as the
replacement; the low-level `rand_core` + `getrandom` direct deps
are dropped in favor of `rand`'s high-level re-exports.

Notes on the actual API used:

- `rand 0.10` renamed `OsRng` → `SysRng` (re-exported from
  `getrandom`) and exposes it at `rand::rngs::SysRng`. The old
  `RngCore` trait split into `Rng` (infallible) and `TryRng`
  (fallible); `SysRng` implements `TryRng` / `TryCryptoRng`
  only.
- RNG access site pattern:
  ```rust
  use rand::TryRng;
  use rand::rngs::SysRng;
  SysRng.try_fill_bytes(buf).expect("OS RNG failure — system entropy unavailable");
  ```
  Same panic-on-OS-RNG-failure behavior as `rand::rngs::ThreadRng`
  uses internally, and matches how RustCrypto / `ring` treat
  OS-RNG failure (unrecoverable).

Construction (AAD shape, wire format, SHA-256 on token, etc.)
is unchanged — only the crate naming the OS RNG source was
amended.
