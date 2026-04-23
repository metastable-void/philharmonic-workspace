# Phase 5 Wave B zeroization follow-up — Claude's audit of Codex round 02

**Date:** 2026-04-23
**Preceding artefacts:**
- Archived prompt: [docs/codex-prompts/2026-04-23-0001-phase-5-wave-b-zeroization-followup.md](../codex-prompts/2026-04-23-0001-phase-5-wave-b-zeroization-followup.md)
- First audit note: [docs/notes-to-humans/2026-04-23-0001-phase-5-wave-b-codex-dispatch-complete.md](2026-04-23-0001-phase-5-wave-b-codex-dispatch-complete.md)
- Codex job: `task-mob67y42-1fo299`, session `019db947-7c2d-7291-a260-348c22747857`.

## tl;dr

Codex produced a minimal, correctly-shaped fix for both nits I
flagged: (a) `aead_key_bytes` stack slot now explicitly zeroed
via `Zeroize::zeroize()` right after the `SecretBox` wrap, and
(b) the dead `prk_bytes` buffer + unused `prk` binding removed
with `let (_, hkdf) = …`. Total change: 14 source lines across
two files, plus 5-line `[Unreleased]` CHANGELOG entries in each
crate. **No construction changes, no dependency changes, no test
vector changes.** Every Wave A + Wave B KAT still passes
byte-for-byte against the committed hex vectors.

I independently re-ran pre-landing, the per-suite KATs,
`cargo audit`, the `crates-io-versions hkdf` check, and the
panic/unsafe grep. Results below. Three pre-existing workspace
advisories surfaced in `cargo audit` — none of them on Wave B
crypto paths; each reaches the graph via non-Wave-B dependencies
(mysql, boa JS engine, HTTP tooling). Those are worth a separate
triage item; they are **not** blockers for Gate-2 approval of
this round's Wave B crypto code.

**My recommendation:** this round is safe to Gate-2 approve, and
the Wave B crypto code (as of `philharmonic-connector-client` +
`philharmonic-connector-service` with this follow-up applied) is
safe to Gate-2 approve as the crypto-review-protocol skill's
second gate. Yuka acts on this report; I do not stamp Gate-2.

## What Codex did (verified against actual diffs)

### Files touched

Exactly what the prompt scoped, nothing else:

- [philharmonic-connector-client/src/encrypt.rs](../../philharmonic-connector-client/src/encrypt.rs)
  — 3 hunks, net `+3/-4`: import `Zeroize`; replace
  `let (prk, hkdf) = …` + `prk_bytes` allocation+copy with
  `let (_, hkdf) = …`; insert `aead_key_bytes.zeroize();` on
  the line following the `SecretBox::new(Box::new(aead_key_bytes))`
  wrap.
- [philharmonic-connector-service/src/decrypt.rs](../../philharmonic-connector-service/src/decrypt.rs)
  — same three hunks, symmetric.
- [philharmonic-connector-client/CHANGELOG.md](../../philharmonic-connector-client/CHANGELOG.md)
  — new `### Changed` block under `## [Unreleased]` with two
  bullets (zeroization tightening + dead-code removal).
- [philharmonic-connector-service/CHANGELOG.md](../../philharmonic-connector-service/CHANGELOG.md)
  — identical bullets.

No touches to `Cargo.toml`, `src/lib.rs`, any other source file,
any test, or anything outside the two dirty submodules. `git
status` inside each submodule shows only the four files above.

### Zeroization-fix shape

Pattern settled on:

```rust
let mut aead_key_bytes = [0_u8; AEAD_KEY_LEN];
hkdf.expand(HKDF_INFO, &mut aead_key_bytes)
    .map_err(|_| TokenVerifyError::DecryptionFailed)?;

let aead_key = SecretBox::new(Box::new(aead_key_bytes));
aead_key_bytes.zeroize();
```

Analysis:

- `[u8; 32]` is `Copy`, so `Box::new(aead_key_bytes)` copies
  the 32 bytes into a fresh heap allocation and leaves the
  stack slot intact.
- `SecretBox<Box<[u8; 32]>>` zeroizes the heap allocation on
  drop (that's `secrecy`'s contract).
- `aead_key_bytes.zeroize()` uses `zeroize`'s volatile-write
  implementation to force-zero the 32-byte stack slot — the
  compiler cannot elide it, even with NLL / optimizations
  running.
- After this sequence, no unzeroized 32-byte copy of the AEAD
  key survives in the function stack frame. Drop of `aead_key`
  at end-of-scope handles the heap copy.

The implementation matches one of the three acceptable shapes I
listed in the prompt ("call `Zeroize::zeroize(&mut _)` on the
stack slot immediately after the last read"). It is not the
shape I would have picked first (I suggested constructing as
`Zeroizing<[u8; 32]>` upfront), but it is strictly equivalent in
effect — the stack slot is an unavoidable temporary because
`hkdf.expand` takes `&mut [u8]` into the caller's buffer, and
Codex's pattern keeps `secrecy::SecretBox` owning the heap copy
while the explicit `.zeroize()` call handles the stack copy.
Clean.

### `prk_bytes` disposition

Removed in both crates. The removal is correct: the value was
copied-into from `prk.as_ref()` and never read. HKDF expansion
goes through the `hkdf` context object (`Hkdf<…>` struct) that
retains its own PRK state internally; the stored `prk_bytes`
buffer never participated in expansion. Replacing
`let (prk, hkdf) = …` with `let (_, hkdf) = …` discards the
unused tuple element — the `prk` type already had `Drop` that
zeroizes internally (it's an `hkdf::Hkdf::Prk<…>` with
Zeroize-on-drop), so nothing of value is lost, and nothing
unzeroized is exposed.

No construction implication. HKDF-SHA256 extract+expand still
runs with identical inputs, identical intermediate state,
identical output bytes — which is why the Wave B KAT vectors
still match byte-for-byte (see below).

## Verification I ran myself

### `./scripts/pre-landing.sh` — **PASS**

Final line: `=== pre-landing: all checks passed ===`. Auto-detected
both modified crates (`philharmonic-connector-client`,
`philharmonic-connector-service`), ran lint + workspace test +
`--ignored` per crate. Exit 0.

### Per-crate KAT pass counts (`./scripts/rust-test.sh <crate>`)

- `philharmonic-connector-client`:
  - `tests/encryption_vectors.rs` — **1 passed** (Wave B KAT
    with all intermediate-value assertions).
  - `tests/signing_vectors.rs` — **2 passed** (Wave A, pre-existing).
- `philharmonic-connector-service`:
  - `tests/decryption_vectors.rs` — **16 passed** (1 positive + 15 negatives).
  - `tests/e2e_roundtrip.rs` — **1 passed** (Wave A × Wave B composition).
  - `tests/verify_vectors.rs` — **11 passed** (Wave A, pre-existing).

Same counts as before the follow-up. Byte-for-byte vector match
preserved — which confirms the HKDF expansion and AEAD
encrypt/decrypt paths produce identical output despite the
`prk_bytes` / `aead_key_bytes` lifecycle changes. That's the
strongest mechanical evidence that the change is behaviour-
preserving.

### Panic / unsafe grep

```
grep -rn "\.unwrap()\|\.expect(\|panic!\|unreachable!\|todo!\|unsafe " \
     philharmonic-connector-client/src philharmonic-connector-service/src
```

Zero hits. Library `src/` stays panic-free and `unsafe`-free.

### `./scripts/xtask.sh crates-io-versions -- hkdf`

Latest non-prerelease listed: `0.13.0`. (`0.13.0-pre.*` and
`0.13.0-rc.*` are pre-release identifiers, not stable.) The
workspace pin `hkdf = "0.13"` resolves to `0.13.0` via Cargo's
semver rules — still the latest stable release. Confirms your
instruction that `0.13` is acceptable provided it's latest.

### `./scripts/cargo-audit.sh` — **not clean**, but nothing on Wave B crypto paths

Three findings, all pre-existing in the workspace:

| ID | Crate | Severity | Introduces via |
|---|---|---|---|
| RUSTSEC-2023-0071 | `rsa` 0.9.10 | Vulnerability (Marvin Attack — timing sidechannel in RSA ops) | `philharmonic-store-sqlx-mysql` → `sqlx` → `sqlx-mysql` → `rsa`. MySQL auth path only; not Wave B. |
| RUSTSEC-2026-0104 | `rustls-webpki` | Vulnerability (reachable panic parsing CRLs) | `xtask` → `ureq` → `rustls` → `rustls-webpki`, and `testcontainers` → `bollard` → `reqwest` → `hyper-rustls` → `rustls` → `rustls-webpki`. Tooling / integration-test deps; not Wave B. |
| RUSTSEC-2024-0436 | `paste` | Warning (unmaintained) | `mechanics` / `mechanics-core` → `boa_engine` → `boa_string` / `boa_gc` / etc. → `paste`. JS-engine path for mechanics, not Wave B. |

None of `aes-gcm`, `ml-kem`, `x25519-dalek`, `hkdf`, `sha2`,
`ed25519-dalek`, `zeroize`, `secrecy`, `coset`, `ciborium`
pull any of the advisories in.

**My read:** these advisories predate this dispatch and are
orthogonal to the Wave B crypto work. They warrant a separate
triage conversation (likely outcomes: bump `sqlx` / `rustls-webpki`
for the two vulns, and either accept the `paste` unmaintained
warning or bump `boa_engine`). Not a Gate-2 blocker for Wave B
as Codex left it.

## What's on disk right now

- **Parent repo:** HEAD at `0bf4fc2` (the archived prompt from
  this round). Clean — I have not yet committed the audit note
  you're reading or the submodule changes.
- **`philharmonic-connector-client`:** dirty. `src/encrypt.rs` +
  `CHANGELOG.md`. Uncommitted.
- **`philharmonic-connector-service`:** dirty. `src/decrypt.rs` +
  `CHANGELOG.md`. Uncommitted.
- **`philharmonic-connector-router`:** clean (not touched this
  round).
- Codex job state: `task-mob67y42-1fo299` completed, `phase: done`.

## What I'm going to do next

Per your instruction in the previous turn — commit + push the
workspace with a "not yet Gate-2 approved" notice. Intended
sequence:

1. Update the archived prompt's `## Outcome` section with
   Codex's structured output (per the
   `codex-prompt-archive` skill).
2. `./scripts/commit-all.sh` (not `--parent-only` — sweeps both
   dirty submodules + the parent) with a message noting the
   two source-line changes, the KAT-vectors-still-pass
   verdict, my audit references, and the explicit "Gate-2
   still pending".
3. `./scripts/push-all.sh`.

I will wait for your explicit Gate-2 approval (in a new
conversation turn) before any subsequent publish-path work
begins.

## What I need from you

1. Gate-2 decision on the Wave B crypto code (as it stands at
   the next workspace HEAD after this round's commit). The code
   path:
   [philharmonic-connector-client/src/encrypt.rs](../../philharmonic-connector-client/src/encrypt.rs),
   [philharmonic-connector-service/src/decrypt.rs](../../philharmonic-connector-service/src/decrypt.rs),
   [philharmonic-connector-service/src/verify.rs](../../philharmonic-connector-service/src/verify.rs),
   [philharmonic-connector-service/src/realm_keys.rs](../../philharmonic-connector-service/src/realm_keys.rs),
   the three crates' `tests/*.rs`, and the frozen
   `docs/crypto-vectors/wave-b/*.hex`. Round-01 audit in
   [docs/notes-to-humans/2026-04-23-0001-phase-5-wave-b-codex-dispatch-complete.md](2026-04-23-0001-phase-5-wave-b-codex-dispatch-complete.md);
   round-02 delta audit in this file.
2. Separately, an explicit decision on the `cargo audit`
   findings: accept-for-now with a tracking issue, or triage
   now? They're workspace-wide, not Wave-B specific, and I did
   not block on them.
3. Once Gate-2 clears, I proceed to: bump
   `philharmonic-connector-client`, `-service`, and `-router`
   to `0.1.0` per the ROADMAP Phase-5-triangle-publish gate,
   run `./scripts/verify-tag.sh` per crate after publish, etc.
   None of that begins without your Gate-2 stamp.
