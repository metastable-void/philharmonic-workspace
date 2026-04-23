# Phase 5 Wave B Codex dispatch — complete, Gate-2 pending

**Date:** 2026-04-23
**Session:** 19737a1e-f157-4aee-ba7e-575a339459a9

## tl;dr

Codex finished the Wave B implementation run cleanly after a
botched first dispatch. Self-reported scope: full Wave B across
client/service/router, tests, docs, validation. I ran my own
pre-Gate-2 review pass (lint + tests + code read); everything
that can be mechanically checked is green, and the construction
aligns with the Gate-1-approved proposal. Three nit-level
implementation comments plus one version-pin deviation are
flagged below for your Gate-2 read. **Yuka Gate-2 review is
still required** — my pass is a scaffolding layer, not a
substitute. Per your instruction, the workspace is being
committed + pushed with an explicit "not yet Gate-2 reviewed"
notice so the append-only history reflects today's actual state.
I have not pulled the structured `codex:result` output yet — say
the word when you want me to and I'll reconstruct the archived
prompt's `## Outcome` section from it.

## What actually happened (not in git log)

### Dispatch-1 failed silently

First call went through the `codex:codex-rescue` subagent in
Claude-Code's background mode. The subagent ran
`codex-companion.mjs task` as a Claude-side background bash;
when the subagent's turn ended, the detached bash (and its
child `node` process) got reaped. Codex job log ended at
"Turn started" 14:32:17 at `phase: starting`, with no assistant
output ever generated. `./scripts/codex-status.sh` showed
nothing, which is how we caught it.

### Dispatch-2 worked

Cleaned up the stale `task-mob1oho0-wixrgy` via
`codex-companion.mjs cancel`, then called the companion script
**directly** with its own `--background` flag. `--background`
at the companion level uses `spawnDetachedTaskWorker` to
properly daemonize — the worker process survives any caller's
lifecycle. That job (`task-mob2cian-d255lb`, Codex session
`019db8e4-3905-7963-ada9-f99449d78f89`) ran 54m 6s and reported
completion.

**Takeaway for future dispatches:** do not hand a Codex task to
the `codex-rescue` subagent if you need it to outlive the
subagent's turn. Either invoke the companion directly with
`--background`, or extend the `codex-rescue` subagent's
behaviour to shell out via the companion's own `--background`
flag. The `run_in_background: true` flag on Claude-Code's Bash
tool is not sufficient — the detached shell is still a child of
the subagent, and the subagent dies when its turn ends.

## Design friction surfaced (worth fixing, not now)

`scripts/print-audit-info.sh` calls `cargo xtask web-fetch` for
IP geolocation. `cargo run` contends with any concurrent cargo
build on `target/debug/.cargo-lock`. Concretely: one of my
mid-Codex commits (`04ddfb4`) sat 6.5 minutes with commit-all.sh
blocked on the IPv6 lookup because Codex was mid-compile. I
SIGTERM'd the stuck cargo; the `|| :` best-effort pattern let
print-audit-info continue with the v6 IP field just missing
from the trailer.

This directly undercuts the "push early, push often" policy I
just documented in [`CONTRIBUTING.md §4.4`](../../CONTRIBUTING.md#44-no-history-modification)
— every mid-Codex commit will hit the same stall as long as we
keep going through `cargo run` for workspace-tool invocations.

Two candidate fixes (not implemented):

1. Use a prebuilt `target/release/web-fetch` binary directly
   in `print-audit-info.sh`, bypassing `cargo run`.
2. Give xtask builds their own target dir
   (`CARGO_TARGET_DIR=target-xtask` inside the xtask wrapper) so
   xtask and member-crate cargo builds don't share a lock.

Either is a small change; I didn't do it in the same turn
because it isn't what you asked for, and because the current
fix-by-kill works.

## What's on disk right now

- **Parent repo:** `04ddfb4` at origin/main. Clean working tree.
- **Submodules `philharmonic-connector-client`,
  `-router`, `-service`:** dirty — Codex's work-in-progress
  sits in each. Nothing committed, nothing pushed. Per the
  prompt's own `## Git` section this is intentional; Claude
  handles git after Gate-2.
- **Codex archive:**
  [`docs/codex-prompts/2026-04-22-0005-phase-5-wave-b-hybrid-kem-cose-encrypt0.md`](../codex-prompts/2026-04-22-0005-phase-5-wave-b-hybrid-kem-cose-encrypt0.md).
  `## Outcome` section still reads *"Pending — will be updated
  after the Codex run completes."* I'll fill it in as part of
  the Gate-2 follow-up.

## My review (not a substitute for Gate-2)

Read of:
[philharmonic-connector-client/src/encrypt.rs](../../philharmonic-connector-client/src/encrypt.rs),
[philharmonic-connector-client/src/error.rs](../../philharmonic-connector-client/src/error.rs),
[philharmonic-connector-client/tests/encryption_vectors.rs](../../philharmonic-connector-client/tests/encryption_vectors.rs),
[philharmonic-connector-service/src/decrypt.rs](../../philharmonic-connector-service/src/decrypt.rs),
[philharmonic-connector-service/src/error.rs](../../philharmonic-connector-service/src/error.rs),
[philharmonic-connector-service/src/verify.rs](../../philharmonic-connector-service/src/verify.rs),
[philharmonic-connector-service/src/realm_keys.rs](../../philharmonic-connector-service/src/realm_keys.rs),
[philharmonic-connector-service/tests/decryption_vectors.rs](../../philharmonic-connector-service/tests/decryption_vectors.rs),
[philharmonic-connector-service/tests/e2e_roundtrip.rs](../../philharmonic-connector-service/tests/e2e_roundtrip.rs),
[philharmonic-connector-router/src/dispatch.rs](../../philharmonic-connector-router/src/dispatch.rs),
[philharmonic-connector-router/src/config.rs](../../philharmonic-connector-router/src/config.rs),
plus all three `Cargo.toml`s and CHANGELOGs.

### Mechanical verdicts

- `./scripts/pre-landing.sh` — **passed** (fmt, check, clippy
  with `-D warnings`, workspace `cargo test`, `--ignored` per
  modified crate).
- Test counts confirmed via
  `./scripts/rust-test.sh philharmonic-connector-service`:
  decryption_vectors 16/16, e2e_roundtrip 1/1, verify_vectors
  (Wave A) 11/11 — all pass. encryption_vectors 1/1 on the
  client side also ran green in the workspace sweep.

### Construction alignment with Gate-1 proposal

All approved-at-Gate-1 choices are implemented verbatim:

- HKDF IKM order: `kem_ss` (32 B) first, then `ecdh_ss` (32 B) —
  matches Gate-1 answer #1.
- HKDF info string: `b"philharmonic/wave-b/hybrid-kem/v1/aead-key"`,
  byte-identical between encrypt and decrypt sides.
- AEAD nonce: 12 random bytes per encryption (answer #5).
- AAD: named-field CBOR map → SHA-256, passed as the
  `external_aad` to `coset::CoseEncrypt0::try_create_ciphertext` /
  `decrypt_ciphertext` (answer #4).
- COSE `alg = A256GCM` (IANA #3) + custom text-keyed
  `kem_ct` / `ecdh_eph_pk` headers (answer #7).
- Zeroization policy: `Zeroizing<[u8; …]>` on `kem_ss`, `ecdh_ss`,
  HKDF `ikm`, plaintext; `SecretBox` on the AEAD key; ML-KEM raw
  decap key bytes stored `Zeroizing<[u8; 2400]>` (answer #2).

### Verification order (service side)

[philharmonic-connector-service/src/verify.rs](../../philharmonic-connector-service/src/verify.rs)
carries the 11 Wave A steps numbered `// 1.` through `// 11.`
(parse → alg pin → kid lookup → window → size cap → signature →
claims decode → kid cross-check → expiry → constant-time
payload-hash compare → realm binding). `verify_and_decrypt`
chains into
[philharmonic-connector-service/src/decrypt.rs](../../philharmonic-connector-service/src/decrypt.rs)
for steps 12 (realm-key lookup by kid), 12a (realm-key window),
13 (KEM decap + ECDH + HKDF + AEAD decrypt), 14 (inner-realm
JSON parse), 15 (inner-realm ≡ `claims.realm`). `ct_eq` from
`subtle` is used for the payload-hash compare — no timing
sidechannel.

### Test hygiene

- All reference vectors loaded via `include_str!` from
  `docs/crypto-vectors/wave-b/*.hex` and
  `docs/crypto-vectors/wave-a/wave_a_composition_*.hex`.
  Compile-time, so a misnamed vector file or truncated hex would
  fail to build, not silently skip.
- Intermediate-value assertions throughout `encryption_vectors.rs`:
  `kem_ct`, `kem_ss`, `ecdh_ss`, `hkdf_ikm`, `aead_key`,
  `external_aad`, `cose_encrypt0` (whole envelope),
  `enc_structure`, `ciphertext_and_tag`, `payload_hash`. This is
  the "intermediate-value vector, not just final ciphertext"
  discipline the `crypto-review-protocol` skill calls for.
- `decryption_vectors.rs` covers the full malformation surface:
  truncated envelope, wrong alg, nonempty unprotected header,
  short `kem_ct` / `ecdh_eph_pk` / IV, unknown custom label,
  unknown realm kid, realm-key window, realm-key realm
  mismatch, tag tamper, `kem_ct` tamper, `ecdh_eph_pk` tamper,
  AAD mismatch, inner-realm mismatch. Each has its own `#[test]`.
- `e2e_roundtrip.rs` runs the full mint (Wave A) + encrypt
  (Wave B) → verify + decrypt chain against the pre-committed
  composition vectors and asserts byte-for-byte equality of
  intermediates plus a final plaintext equality.

### Safety discipline

- `grep -rn "\.unwrap()\|\.expect(\|panic!\|unreachable!\|todo!\|unimplemented!\|unsafe " <crate>/src/`
  across all three crates: clean. Only `.expect(...)` calls are
  inside [philharmonic-connector-router/src/dispatch.rs](../../philharmonic-connector-router/src/dispatch.rs)
  at line 194+, inside `#[cfg(test)] mod tests` — §10.3-exempt.
- No `unsafe` blocks anywhere.
- All error paths route through the declared `TokenVerifyError` /
  `EncryptError` variants; no panics on attacker-controlled input.
- Realm-key realm binding is explicit: decrypt rejects with
  `RealmKeyRealmMismatch` if the registered key's `realm` field
  differs from the service's own `service_realm` argument.
  Defence against a realm's key material being used to decrypt
  traffic targeted at a different realm.

### Nits flagged for your Gate-2 read

These are polish-level — I would not block on them, but they are
worth your eye:

1. **`aead_key_bytes` stack array is not zeroized** before being
   copied into `SecretBox<Box<[u8; 32]>>`. Happens on both sides:
   [philharmonic-connector-client/src/encrypt.rs:183](../../philharmonic-connector-client/src/encrypt.rs#L183)
   and
   [philharmonic-connector-service/src/decrypt.rs:82](../../philharmonic-connector-service/src/decrypt.rs#L82).
   The stack local holds the 32-byte AEAD key unwrapped from
   function entry through the `SecretBox::new(Box::new(_))` call;
   the stack copy is only dropped at end-of-scope with no
   zeroization. Tiny window, but the rest of the file wraps
   everything in `Zeroizing` so this stands out. Fix: construct
   directly as `Zeroizing<[u8; 32]>` and move-into-Box via an
   intermediate `Zeroizing`-aware adapter, or convert to
   `SecretBox<Zeroizing<[u8; 32]>>`.

2. **Dead `prk_bytes` buffer** —
   [philharmonic-connector-client/src/encrypt.rs:180-181](../../philharmonic-connector-client/src/encrypt.rs#L180-L181)
   and
   [philharmonic-connector-service/src/decrypt.rs:79-80](../../philharmonic-connector-service/src/decrypt.rs#L79-L80):
   both sides allocate a `Zeroizing<[u8; 32]>`, copy the HKDF
   PRK into it, and never read the buffer again (HKDF expansion
   uses the `hkdf` context object, not `prk_bytes`). Clippy
   missed it because it's assigned-to, not just bound. Looks
   like scaffolding from an intermediate refactor that's never
   removed. Harmless but worth deleting.

3. **`hkdf` crate pinned at `0.13`**, but the Gate-1 proposal
   (`docs/design/crypto-proposals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md`
   §"Primitives and library versions", line 216) specified
   `hkdf = "0.12"` with the note *"latest 0.13.0 (new major;
   check compatibility before pinning to "0.13")"*. Codex took
   the newer pin, apparently after verifying compatibility
   (the tests pass against the committed vectors, so the HKDF
   output bytes match byte-for-byte). Not a construction
   change — HKDF-SHA256 outputs are algorithmically identical
   across the crate's 0.12 → 0.13 major. But the
   `crypto-review-protocol` skill says *"Explicitly prohibit
   changing any primitive or construction choice without
   flagging."* Codex didn't flag; I'm flagging now. Your call
   whether to accept the bump or have Codex re-pin to 0.12.

### Router sanity

[philharmonic-connector-router](../../philharmonic-connector-router/)
is a plain HTTP dispatcher on axum + hyper + tower — no crypto
deps, no key material. Config splits into
[`DispatchConfig`](../../philharmonic-connector-router/src/config.rs)
(host + per-realm upstream map with validation) and a
[`Forwarder`](../../philharmonic-connector-router/src/dispatch.rs)
trait that the production `HyperForwarder` implements, and that
the single mock test substitutes for capturing the forwarded
request. Matches the "minimal HTTP dispatcher (no crypto)" scope
in the prompt.

## What I need from you

1. Say whether to pull the structured output now or wait. If
   yes, I'll run `/codex:result task-mob2cian-d255lb` (or the
   companion equivalent), paste the relevant deliverables into
   a Gate-2 review note, and update the archived prompt's
   `## Outcome` section.
2. Gate-2 review of the code itself is yours per the
   `crypto-review-protocol` skill. I can pre-digest the changes
   (touched files, test-vector match status, zeroization audit,
   `unsafe` grep) to give you a scaffold, but the line-by-line
   read is yours.
3. Once Gate-2 clears, the workspace commits are:
   `commit-all.sh` across the three dirty submodules + parent
   (picks up the Cargo.lock cascade too), then
   `push-all.sh`. No `cargo publish` this run — publication
   for the triangle crates is a separate decision after
   Gate-2.

## Other ground covered this session

For completeness; these all landed via normal commits:

- `0f23170` — `scripts/project-status.sh` + xtask
  `openai-chat` bin; LLM-generated workspace-status archive
  under `docs/project-status-reports/`.
- `dca510f` — `CONTRIBUTING.md §4.7` + `README.md`: documented
  the GitHub `Safety rules` ruleset (parent repo only,
  `required_signatures` / `non_fast_forward` / `deletion`,
  no bypass actors). Renumbered §4.7 "Other git rules" →
  §4.8.
- `04ddfb4` — `CONTRIBUTING.md §4.4` + `CLAUDE` / `AGENTS` /
  `README` / `pre-push`: `git revert` now forbidden (the "undo"
  framing clutters the log); push-early-push-often is explicit
  policy because append-only means unpushed commits can't be
  recovered.
