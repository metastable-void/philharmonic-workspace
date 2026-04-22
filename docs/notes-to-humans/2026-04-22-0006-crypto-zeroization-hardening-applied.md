# H1 + H2 zeroization hardening applied — `philharmonic-policy`

**Date:** 2026-04-22

## What landed

Applied the two defense-in-depth zeroization hardening items
from the Gate-2 review (note 0005) per your approval.

### H1 — `generate_api_token_from_bytes` now takes `&[u8; 32]`

[philharmonic-policy/src/token.rs](../../philharmonic-policy/src/token.rs):

- Signature change: `generate_api_token_from_bytes(raw: [u8; 32])`
  → `generate_api_token_from_bytes(raw: &[u8; 32])`. `pub(crate)`
  visibility unchanged.
- `generate_api_token` now wraps the RNG-filled buffer in
  `Zeroizing<[u8; 32]>` from the start and passes a reference
  into the helper:
  ```rust
  let mut raw = Zeroizing::new([0_u8; TOKEN_BYTES]);
  fill_random(raw.as_mut_slice());
  generate_api_token_from_bytes(&raw)
  ```
  No more stack-copy of the 32 credential bytes into the
  helper's parameter.
- Removed `use zeroize::{Zeroize, ...}` — the explicit
  `.zeroize()` call on the raw buffer is no longer needed
  because the `Zeroizing` wrapper zeroizes on drop. The scope
  end is the same moment (function return) so the deterministic
  zero-out timing is preserved.
- Vector-test call site in `tests/crypto_vectors.rs:247`
  updated to `generate_api_token_from_bytes(&raw)`.

### H2 — `Sck::from_file` constructs `Sck` directly with `Zeroizing<[u8; 32]>`

[philharmonic-policy/src/sck.rs](../../philharmonic-policy/src/sck.rs):

- `from_file` no longer routes through `Self::from_bytes`.
  Instead constructs `Self { key }` directly after wrapping
  the intermediate buffer in `Zeroizing` at declaration time:
  ```rust
  let mut key = Zeroizing::new([0_u8; SCK_KEY_LEN]);
  key.copy_from_slice(bytes.as_slice());
  Ok(Self { key })
  ```
- The buffer is now `Zeroizing<[u8; 32]>` the moment it
  exists, and moves (not copies) into the `Sck` struct. No
  post-return stack residue.
- `Sck::from_bytes` (the public constructor taking a bare
  `[u8; 32]`) is unchanged. External callers with their own
  key material can still use it; they accept the zeroization
  responsibility on their side.

## Test results

All four gates green after the hardening:

- **`./scripts/pre-landing.sh --no-ignored philharmonic-policy`** — all
  checks passed. Per-binary counts:
  - `src/lib.rs` unit tests: 10 passed (unchanged)
  - `tests/crypto_vectors.rs`: 15 passed (unchanged)
  - `tests/permission_mock.rs`: 14 passed (unchanged)
  - workspace test (full family): 33 + 116 passed
- **`./scripts/rust-test.sh --ignored philharmonic-policy`** —
  12 passed in 170.26s against MySQL testcontainer
  (unchanged).
- **`./scripts/miri-test.sh philharmonic-policy sck_ pht_`** —
  15 crypto_vectors tests passed under miri in 12.90s, 0 UB.
  Essentially identical runtime to the pre-hardening run
  (12.81s yesterday) — the wrapper change doesn't affect
  miri's interpretation cost.
- **Test vectors still match byte-for-byte.** The SCK and
  `pht_` vectors committed earlier survive the hardening
  unchanged, which was the intended property (behavior-
  preserving, memory-hygiene-only change).

## Subtle observations worth preserving

### "Immediate zeroize" vs "scope-end zeroize" are the same here

The old `generate_api_token` called `raw.zeroize()` explicitly
on line 22 after the helper returned. The new code relies on
`Zeroizing`'s `Drop` impl at scope end. Both fire at the
same moment (function return, right before `generated` is
returned to the caller), so there's no semantic shift in the
timing of zeroization. The change is purely "don't copy to a
second location that isn't wrapped" — the original location
was zeroized then and still is now (via Drop instead of
manual call).

### The `from_bytes` constructor stays public

`Sck::from_bytes(bytes: [u8; 32])` accepts a bare `[u8; 32]` by
value, wraps it in `Zeroizing` internally, and returns the
`Sck`. That's the "external-caller-owns-the-bytes" path —
they're responsible for zeroizing their source. `from_file`
no longer uses it because the file path controls its own
buffer lifecycle and can skip the intermediate unprotected
copy. If Yuka prefers, `from_bytes` could be further
hardened to accept `Zeroizing<[u8; 32]>`, but that would
break the ergonomic call from tests (which have
hex-literal-constructed `[u8; 32]` vectors). Leaving it as-is
for now — the ergonomic trade-off goes the other way there.

### Zeroization is probabilistic against attackers at this layer anyway

Worth keeping in mind: the `Zeroizing<T>` wrapper is
best-effort — compilers can elide writes to memory that's
about to be freed, and Rust's MIR doesn't have a "memset that
must not be optimized away" primitive. The `zeroize` crate
uses volatile writes + memory fences to discourage the
optimizer, but on an aggressive LLVM pass it's not 100%
ironclad. What we get from H1/H2 is: the number of memory
locations that could retain key material is reduced, and the
remaining ones are inside `Zeroizing` wrappers that the
optimizer is nudged (not forced) to zero. Defense-in-depth
layer, not a hard guarantee.

For stronger memory-hygiene guarantees we'd need something
like `mlock(2)` (pinned pages, swap-proof) or OS-level
scrubbing — out of scope for this crate and probably a
deployment-time concern at best.

### Miri doesn't exercise the zeroization itself

Miri confirms no UB introduced by the refactor, but it can't
verify that the `Zeroizing` wrapper actually zeroes memory
on drop — miri doesn't inspect the post-drop memory state. The
test suite has no "read stack memory after token generation
and confirm it's zero" test either, because such a test
would itself be `unsafe` and defeat the purpose. The
zeroization is trusted by construction (the `zeroize` crate's
audit + the compiler's `write_volatile` preservation).

### Removed `Zeroize` import — minor follow-on observation

With the explicit `.zeroize()` call gone, `use
zeroize::Zeroize` is no longer needed in `token.rs`. Removed
to keep imports minimal. `Zeroizing` stays. Clippy would have
flagged the unused import eventually under a strict
`unused-imports` pass.

## Remaining items from note 0005 (status)

- **H3 (cosmetic, encrypt-error variant naming)**: not
  applied. Unreachable path in practice; either leave as-is
  or tidy later. No action this pass.
- **D1 (error-roundtrip fragility)**: not applied. Low-risk
  display-only concern; refactor to direct validation
  available if serde-json's error format ever drifts.
- **D2 (tenant status not checked during eval)**: Wave 1
  code, not introduced by Wave 2. Your call for Phase 8
  timing vs. now.
- **D3 (parse_permission_document direct unit test gap)**:
  low priority; covered indirectly via evaluator tests.

## Ready for 0.1.0

`philharmonic-policy` is now publish-ready from a Gate-2
perspective:

- Wave 2 crypto construction correct, vectors match external
  Python reference, side-channel opaque on decrypt.
- Wave 1 auth-boundary hardened (Findings #1 + #2 fixed).
- Zeroization tightened (H1 + H2 applied).
- Miri clean on nightly 9ec5d5f32.
- No `unsafe`, no `anyhow`, no `println!` in library code.
- Version still `0.0.0` in `Cargo.toml` — next commit wave
  (0.1.0 bump + CHANGELOG.md + release tag) is the publish
  step; waiting on your go/no-go.
