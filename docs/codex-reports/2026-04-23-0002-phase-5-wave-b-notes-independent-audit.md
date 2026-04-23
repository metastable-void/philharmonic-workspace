# Phase 5 Wave B notes-to-humans independent audit

**Date:** 2026-04-23
**Prompt:** docs/codex-prompts/2026-04-23-0001-phase-5-wave-b-zeroization-followup.md

## Scope

Audited:
- `docs/notes-to-humans/2026-04-23-0001-phase-5-wave-b-codex-dispatch-complete.md`
- `docs/notes-to-humans/2026-04-23-0002-phase-5-wave-b-zeroization-followup-audit.md`
- The Codex-produced code and commits those notes describe (`task-mob2cian-d255lb`, `task-mob67y42-1fo299`) across `philharmonic-connector-client`, `philharmonic-connector-service`, and `philharmonic-connector-router`.

## Findings (ordered)

1. **Medium:** one technical claim in note `0002` about HKDF PRK zeroization is inaccurate.
- Claim text: `docs/notes-to-humans/2026-04-23-0002-phase-5-wave-b-zeroization-followup-audit.md:107-109` says the discarded `prk` has zeroizing `Drop` and names it as `hkdf::Hkdf::Prk<...>`.
- Verified crate behavior (`hkdf 0.13.0` source): `extract` returns `(digest::Output<H>, GenericHkdf<H>)`, not a `Prk` wrapper type.
- Verified feature set (`cargo tree -p philharmonic-connector-client -e features -i digest@0.11.2`): digest 0.11.2 is enabled with `alloc`, `block-api`, `default`, `mac`, `oid`; **not** with `zeroize`.
- Impact: this is a report-accuracy issue, not a construction change in shipped code. The code changes themselves remain behavior-preserving.

2. **Low:** both notes have state snapshots that are now stale at current `HEAD` (expected for journal notes).
- Current parent `HEAD` is `da0dff5` with clean trees in parent and all three triangle submodules.
- `0001` “What’s on disk right now” (`...0001...md:85-97`) and `0002` snapshot (`...0002...md:182-193`) are historically consistent with commit chronology (`504977c` -> `2f9da51` -> `0bf4fc2` -> `da0dff5`).

3. **Low:** note `0002`’s `rustls-webpki` dependency-path summary is incomplete relative to current audit output.
- The note lists `xtask/ureq` and `testcontainers` routes (`...0002...md:168`).
- Current `./scripts/cargo-audit.sh` also reports a `mechanics-core -> reqwest -> rustls-platform-verifier -> rustls-webpki` route.
- Impact: categorization as non-Wave-B remains correct.

## Claim verification

### Note `2026-04-23-0001` (`phase-5-wave-b-codex-dispatch-complete`)

Verified or historically corroborated:
- Wave B implementation existed across client/service/router and matches described file surfaces.
- Mechanical test assertions are consistent with independent reruns (`pre-landing`, per-crate KAT counts).
- Gate-1 alignment items are present in code: HKDF IKM order, HKDF info string, A256GCM, custom `kem_ct`/`ecdh_eph_pk`, AAD digest binding, `ct_eq` payload-hash compare, realm-binding checks.
- Reported nits were real in the first Wave B commit state:
  - Missing stack zeroization for `aead_key_bytes` in both crates.
  - Dead `prk_bytes` scratch copy in both crates.
  - `hkdf = "0.13"` pin while proposal text listed `0.12` with explicit compatibility-check note.
- Operational claim that `print-audit-info` path contended on shared `target` is supported by the subsequent fix commit `e630cd6` introducing `CARGO_TARGET_DIR=target-xtask` in `scripts/xtask.sh`.

Not independently verifiable from repo artifacts alone:
- Dispatch-process lifecycle details for dispatch-1/dispatch-2 (`codex-rescue` reaping behavior, exact task runtime) and raw `codex-status` observations.

### Note `2026-04-23-0002` (`phase-5-wave-b-zeroization-followup-audit`)

Verified:
- Diff shape and scope are accurate for follow-up commits (`f821b77` client, `cad211e` service): only `src/encrypt.rs`/`src/decrypt.rs` plus both `CHANGELOG.md` files.
- Zeroization fix shape is present exactly as described: `aead_key_bytes.zeroize()` immediately after `SecretBox::new(Box::new(aead_key_bytes))`.
- Dead `prk_bytes` and named `prk` binding removed on both sides (`let (_, hkdf) = ...`).
- No `Cargo.toml`/construction/dependency/test-vector edits in the follow-up commit.
- Test-count claims match independent reruns:
  - client: `encryption_vectors` 1/1, `signing_vectors` 2/2.
  - service: `decryption_vectors` 16/16, `e2e_roundtrip` 1/1, `verify_vectors` 11/11.
- `hkdf` crates.io latest stable remains `0.13.0` (with prerelease/rc tags also listed).
- `cargo-audit` still reports the same three advisory IDs and none land on Wave B primitives.

Incorrect or overstated:
- The PRK-zeroizing-drop statement in lines 107-109 (see Finding #1).

## Independent audit of Codex-produced code

Wave B base implementation (Codex `task-mob2cian-d255lb`) and follow-up (Codex `task-mob67y42-1fo299`) were both audited directly via commit diffs and current source.

Confirmed properties:
- No `unsafe` in `philharmonic-connector-client`, `...-service`, or `...-router`.
- Library panic-pattern grep on client+service `src/` returns zero hits.
- Router `expect(...)` usage is confined to `#[cfg(test)]` section in `src/dispatch.rs`.
- Vector tests are compile-time `include_str!` backed and still passing.
- Follow-up fix did not alter cryptographic construction outputs (KATs remain byte-for-byte).

## Verification commands executed

Executed during this audit:
- `./scripts/pre-landing.sh`
- `./scripts/rust-test.sh philharmonic-connector-client`
- `./scripts/rust-test.sh philharmonic-connector-service`
- `grep -rn "\\.unwrap()\\|\\.expect(\\|panic!\\|unreachable!\\|todo!\\|unsafe " philharmonic-connector-client/src philharmonic-connector-service/src`
- `./scripts/xtask.sh crates-io-versions -- hkdf`
- `./scripts/cargo-audit.sh`
- Historical/structural verification via `git show`, `git log`, `git diff-tree`, and direct file reads.

## Final verdict

- The two notes are largely accurate and useful.
- One medium-severity correction is required in note `0002`: the PRK drop-zeroization rationale should be fixed.
- All substantive claims about the Codex code changes themselves (scope, fix shape, and verification outcomes) are corroborated.
