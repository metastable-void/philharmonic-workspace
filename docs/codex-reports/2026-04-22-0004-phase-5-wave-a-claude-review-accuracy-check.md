# Phase 5 Wave A Claude Gate-2 note accuracy check

**Date:** 2026-04-22
**Prompt:** docs/codex-prompts/2026-04-22-0004-phase-5-wave-a-cose-sign1-tokens.md

## Scope

Reviewed for factual accuracy:

- `docs/notes-to-humans/2026-04-22-0011-phase-5-wave-a-claude-review.md`

Validated against:

- `philharmonic-connector-client/src/signing.rs`
- `philharmonic-connector-client/tests/signing_vectors.rs`
- `philharmonic-connector-service/src/verify.rs`
- `philharmonic-connector-service/src/error.rs`
- `philharmonic-connector-service/src/context.rs`
- `philharmonic-connector-service/src/registry.rs`
- `philharmonic-connector-service/tests/verify_vectors.rs`
- `philharmonic-connector-common/src/lib.rs`
- `philharmonic-connector-client/Cargo.toml`
- `philharmonic-connector-service/Cargo.toml`
- `docs/design/crypto-proposals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md`

Also checked commit/statements with:

- `git -C philharmonic-connector-client show --stat --oneline 9634f68`
- `git -C philharmonic-connector-service show --stat --oneline bcf8ea6`
- `git show --stat --oneline ac02232`
- `git -C philharmonic-connector-client branch -r --contains 9634f68`
- `git -C philharmonic-connector-service branch -r --contains bcf8ea6`
- `git branch -r --contains ac02232`

Reproduced test/lint/UB-check claims with wrappers:

- `./scripts/rust-lint.sh philharmonic-connector-client`
- `./scripts/rust-lint.sh philharmonic-connector-service`
- `./scripts/rust-test.sh philharmonic-connector-client`
- `./scripts/rust-test.sh philharmonic-connector-service`
- `./scripts/rust-test.sh --ignored philharmonic-connector-client`
- `./scripts/rust-test.sh --ignored philharmonic-connector-service`
- `./scripts/miri-test.sh philharmonic-connector-client`
- `./scripts/miri-test.sh philharmonic-connector-service`

All eight wrapper runs succeeded in this workspace on 2026-04-22.

## Verdict

The note is materially accurate. I found two precision-level issues:

1. The step table in the note says step 3 maps to `UnknownKid { kid }`, but step 3 in code can also return `Malformed` when protected-header `kid` bytes are not UTF-8 (`verify.rs`:52-53).
2. The note says `philharmonic-types = "0.3.5"` is "explicit, not caret." In Cargo semantics, `"0.3.5"` is still caret requirement syntax (equivalent to `^0.3.5`), though it does enforce a `>= 0.3.5` floor within `0.3.x`.

Neither issue changes the core Gate-2 conclusion.

## Claim-by-claim check summary

- Verification order and error taxonomy: accurate, with the step-3 UTF-8 nuance above.
- Constant-time payload-hash compare (`ct_eq` + `bool::from`): accurate.
- Library-bytes-only boundary (`from_seed`, registry insert; no file-path/file-I/O APIs in these libs): accurate.
- Zeroization strategy (`Zeroizing<[u8; 32]>`, per-call transient `SigningKey::from_bytes`): accurate.
- No-panic discipline in library `src/`: accurate for the pattern set quoted in the note.
- Known-answer vector parity assertions (claims/protected/Sig_structure1/signature/final COSE): accurate.
- 10 negative vector tests using exact-variant `assert_eq!`: accurate.
- Dependency/version statements (`version = "0.0.0"` for both crates, `philharmonic-types = "0.3.5"` present): accurate except wording nuance above.
- `issued_at` follow-up concern (`ConnectorCallContext.issued_at` currently set to `now` in `build_call_context`): accurate.
- Minor notes:
  - `MintingKeyRegistry::insert` replace semantics via `HashMap::insert`: accurate.
  - no `MintingKeyRegistry::remove`: accurate.
  - service crate re-exports `ConnectorCallContext` while context construction is internal helper: accurate.

