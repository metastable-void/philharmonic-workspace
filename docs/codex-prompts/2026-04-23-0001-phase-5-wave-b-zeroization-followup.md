# Phase 5 Wave B — zeroization follow-up + dead-code sweep

**Date:** 2026-04-23
**Slug:** `phase-5-wave-b-zeroization-followup`
**Round:** 01
**Subagent:** `codex:codex-rescue` (or direct companion
`--background` dispatch — see the workspace `.claude/skills/
codex-prompt-archive/SKILL.md` and the 2026-04-23 dispatch
mishap note at
`docs/notes-to-humans/2026-04-23-0001-phase-5-wave-b-codex-dispatch-complete.md`).

## Motivation

Claude's pre-Gate-2 read of the Wave B implementation (see the
note above, §"My review") flagged three nits. Yuka has reviewed
those flags and directed two of them to be acted on now:

1. **Fix** — `aead_key_bytes` stack array is not zeroized before
   being moved into `SecretBox<Box<[u8; 32]>>`. The raw AEAD
   key sits on the stack unwrapped from
   `Hkdf::expand`'s output slice through the
   `SecretBox::new(Box::new(_))` call, with no zeroization on
   the stack slot. Rest of the file wraps every key in
   `Zeroizing` / `SecretBox`; this slot stands out.
2. **Explore, then act** — Dead `prk_bytes` zeroizing buffer in
   the HKDF-extract block. Codex round-01 allocated
   `Zeroizing<[u8; 32]>`, copied the HKDF PRK into it, and
   never read it again. The actual HKDF expansion goes through
   the `hkdf` context object, not `prk_bytes`. Evaluate
   whether the buffer has any purpose Claude missed; if not,
   remove it; if it does, leave it and explain in the
   `## Outcome` follow-up below.

The third nit — `hkdf = "0.13"` vs. Gate-1 proposal's `"0.12"`
pin with a "check compat before pinning to 0.13" caveat — is
**not** a change request. Yuka's direction: the newer pin is
acceptable **if** `0.13` is still the latest on crates.io and
`cargo audit` is clean. Treat as a verification step, not a
code change.

## Scope

### In scope — fix

- `philharmonic-connector-client/src/encrypt.rs` at line ~183
  (`let aead_key = SecretBox::new(Box::new(aead_key_bytes));`).
- `philharmonic-connector-service/src/decrypt.rs` at line ~82
  (same pattern).
- Same-commit consequences: imports, `Cargo.toml` (if a new
  trait surface is used — e.g. `secrecy` features), `src/lib.rs`
  re-exports, `CHANGELOG.md` [Unreleased] entry (note the
  zeroization-tightening and dead-code removal).

Acceptable fixes include:

- Construct the key as `Zeroizing<[u8; 32]>` from the start and
  pass it into `SecretBox` via an owned move.
- Use `SecretBox::new(Box::new(Zeroizing::new(aead_key_bytes)))`
  or the `Zeroize::zeroize(&mut aead_key_bytes)` call
  immediately after the last read of the stack slot.
- Or any other concrete shape that leaves **no** unzeroized key
  material on the stack. The observable contract is: after
  `build_encrypt0` / `decrypt_payload` returns, no 32-byte copy
  of the derived AEAD key is recoverable from this function's
  stack frame.

Do **not** change the derivation itself (HKDF-SHA256 IKM order,
info string, output length) — only the storage/lifetime of the
derived key.

### In scope — explore

- `philharmonic-connector-client/src/encrypt.rs` lines ~180-181
  (`let mut prk_bytes = ...; prk_bytes.copy_from_slice(prk.as_ref());`).
- `philharmonic-connector-service/src/decrypt.rs` lines ~79-80
  (same).

Grep the surrounding code and tests to confirm `prk_bytes` is
never read. If it genuinely is dead code, remove it plus the
`prk` binding from the `Hkdf::<sha2::Sha256>::extract` tuple
(use `let (_, hkdf) = ...`). If there's a reason to keep it
that Claude missed, leave it and explain in `## Outcome`.

### Out of scope

- **Primitives**: no crate swaps, no version-pin changes, no
  alg changes. `ml-kem`, `x25519-dalek`, `aes-gcm`, `hkdf`,
  `sha2`, `ed25519-dalek`, `zeroize`, `secrecy` — all stay at
  their current pins.
- **Construction**: HKDF IKM order, HKDF `info`, AEAD nonce
  scheme, AAD shape, COSE alg id, custom header labels — all
  frozen. Gate-1 approved; do not revisit.
- **Test vectors**: `docs/crypto-vectors/wave-a/` and
  `docs/crypto-vectors/wave-b/` are read-only; the
  `encryption_vectors.rs`, `decryption_vectors.rs`,
  `e2e_roundtrip.rs`, and `verify_vectors.rs` tests must keep
  passing byte-for-byte against the committed hex. Any test
  you add is an *additional* test alongside the existing ones.
- **Router**: `philharmonic-connector-router` has no crypto;
  this dispatch doesn't touch it.
- **Publish**: no `cargo publish`, no `--dry-run` publish, no
  version bump.

## Verification steps (mandatory; report each verdict)

### Code-path
- `./scripts/pre-landing.sh` — must be green. fmt + check +
  clippy (`-D warnings`) + workspace test + `--ignored` per
  modified crate. Compares against the committed Wave A + Wave B
  vectors; a derivation change of any kind (even accidental)
  will fail these tests.
- Explicit re-run of the KAT vector tests per crate:
  ```sh
  ./scripts/rust-test.sh philharmonic-connector-client
  ./scripts/rust-test.sh philharmonic-connector-service
  ```
  Report the pass counts verbatim (should be `1 passed` for
  encryption_vectors, `16 passed` for decryption_vectors,
  `1 passed` for e2e_roundtrip, `11 passed` for verify_vectors).

### hkdf version sanity (for Yuka's Gate-2 sign-off on the pin)
- `./scripts/xtask.sh crates-io-versions -- hkdf` — list
  published versions. Report the latest non-yanked; confirm
  it's still `0.13.x`.
- `./scripts/cargo-audit.sh` — run workspace audit. Report
  verdict (`clean` or the exact advisories printed). If
  `cargo-audit` isn't pre-installed the script will install it
  on first run — that's expected.

### Memory-safety discipline
- No `unsafe` anywhere. Keep it zero.
- No new `.unwrap()` / `.expect()` / `panic!` /
  `unreachable!` / `todo!` on reachable paths. Tests may use
  these freely (§10.3 exemption).
- `grep -rn "\.unwrap()\|\.expect(\|panic!\|unreachable!\|todo!\|unsafe " philharmonic-connector-client/src philharmonic-connector-service/src`
  must return zero hits in library `src/` — report the output.

## Workspace conventions (authoritative: `CONTRIBUTING.md`)

- **No git state changes.** No `git add`, no `git commit`, no
  `git push`, no `cargo publish`. Leave the working tree dirty
  in `philharmonic-connector-client` and
  `philharmonic-connector-service`. Claude runs
  `scripts/commit-all.sh` and `scripts/push-all.sh` after
  auditing the result.
- **No history rewrites, no `git revert`.** This workspace is
  append-only; §4.4 forbids `revert` along with amend/rebase/
  reset. If anything is off in an earlier commit, flag it; do
  not attempt to rewrite.
- **Invoke cargo via the wrappers** — `./scripts/rust-lint.sh`,
  `./scripts/rust-test.sh`, `./scripts/pre-landing.sh`,
  `./scripts/xtask.sh`, `./scripts/cargo-audit.sh`. Direct
  `cargo fmt/check/clippy/test` is redundant with these.
  Read-only cargo queries (`cargo tree`, `cargo metadata`) are
  fine raw.
- **Keep the crypto-review-protocol posture.** If you discover
  anything that looks like a construction-level defect (wrong
  IKM order, wrong AAD binding, a signature/MAC over
  non-authenticated input, a key-material leak the stack-array
  fix doesn't cover), **stop and flag** in `## Outcome`. Do
  not silently fix construction in this round.

## Zeroization intent (from the Gate-1 proposal, for reference)

From `docs/design/crypto-proposals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md`
§"Zeroization points" — every in-memory key-material byte is
wrapped in `Zeroizing` or `SecretBox` and dropped explicitly
before the function returns. The AEAD key is the last
key-material derivation in each function and deserves the
same discipline as `kem_ss`, `ecdh_ss`, and `ikm`. The fix is
tightening existing policy, not a new policy.

## Deliverables

1. Updated `encrypt.rs` with the `aead_key_bytes` stack slot
   zeroized / constructed-as-`Zeroizing` from the start.
2. Updated `decrypt.rs` with the same.
3. `prk_bytes` and the unused `prk` binding removed (or
   justified in `## Outcome`).
4. CHANGELOG.md [Unreleased] entries on both client and service
   noting the tightening + dead-code removal (one short bullet
   each is fine; this is not a public API change).
5. Pre-landing green, KAT vector counts unchanged and all
   passing byte-for-byte.
6. `cargo audit` verdict + `crates-io-versions hkdf` output
   recorded in `## Outcome`.

## Structured output contract

Return:

- **Summary** — one paragraph.
- **Touched files** — full list with line counts / approximate
  diff sizes.
- **Zeroization-fix shape** — describe the pattern you settled
  on (where `aead_key_bytes` lives now, how it gets into
  `SecretBox`, whether any stack copy survives).
- **`prk_bytes` disposition** — removed + rationale, or kept +
  rationale. If kept, a one-sentence justification Yuka can
  check at Gate-2.
- **Verification verdicts** — pre-landing (PASS/FAIL + summary),
  per-crate KAT pass counts, `grep` no-panic-no-unsafe output,
  `cargo audit` verdict, `hkdf` latest-version lookup.
- **Residual risks / flagged concerns** — anything surprising,
  anything you couldn't resolve, anything that looks like it
  might merit a separate Gate-1 conversation.
- **Git state** — confirm tree is dirty, no commits, no pushes,
  no publishes.

## Missing-context gating

Pressure points where STOP-and-ask is the right move:

- `secrecy::SecretBox`'s constructor signature in the version
  pinned by this workspace (`0.10.x`). If the idiomatic "no
  stack copy" pattern requires a feature / trait that isn't on
  by default, flag it rather than guessing.
- Whether `Zeroize` is already a trait bound on `[u8; 32]` in
  the `zeroize` crate version pinned here. If not, the fix
  might need `Zeroizing::<[u8; 32]>::new(_)` wrapping rather
  than `Zeroize::zeroize(&mut _)`. Flag if the choice seems
  unclear.

## Action safety

No destructive git. No cargo publish. No branch / tag
operations. No modification to files outside the two submodule
crates' `src/`, `tests/`, `Cargo.toml`, `CHANGELOG.md`. The
reference vectors in `docs/crypto-vectors/` are read-only; if
you think they're wrong, report; do not edit. This dispatch
fixes implementation nits, not the Gate-1-approved approach.

## Outcome

Pending — will be updated after the Codex run completes.
Expected content: the bullet list under "Structured output
contract" above, verbatim.
