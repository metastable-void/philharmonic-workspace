# Phase 2 Wave 1 landed — Codex behavior, pre-landing results, flag-vs-fix

**Date:** 2026-04-21

## Outcome

Wave 1 of Phase 2 (`philharmonic-policy` non-crypto foundation)
shipped cleanly. Two parent commits came out of the run:

- `65fc3c4` — submodule pointer bump for `philharmonic-policy` at
  `790c23d`: six entity kinds + permission evaluation + three test
  tiers (11 mock, 10 MySQL-ignored, 6 colocated unit).
- `e20ebb1` — focused `scripts/codex-status.sh` dedup/display
  tweak (see "Scope drift" below).

Pre-landing: fmt + check + clippy `-D warnings` green; workspace
tests green; `philharmonic-policy --ignored` 10/10 green in 150s
(real MySQL via testcontainers).

No direct crypto dep in `philharmonic-policy/Cargo.toml` — sqlx
transitives pull `sha2`/`rand_core`/etc. into the lockfile, but
nothing banned is a declared dep. The Wave 1 ban held.

## Codex dispatch friction (resolved)

Two attempts stalled at the Bash-permission layer: the
`codex:codex-rescue` subagent's call into
`node .../codex-companion.mjs task ...` was getting auto-denied
before a prompt could surface. Grant added to
`.claude/settings.json` and the third dispatch ran through
cleanly. If this recurs after a fresh clone or a different
machine, the grant needs replaying — it's machine-local. Worth
eyeballing when setting up a new box.

## Scope drift — `scripts/codex-status.sh`

Codex edited `scripts/codex-status.sh` mid-task (added
duplicate-subtree filtering + `node`-path stripping in output).
Strictly out-of-scope per the prompt's "don't refactor unrelated
files" rule. The change itself is benign and correct — the
previous output did double-print when a Codex invocation went
`bash -c 'node .../codex-companion.mjs …'`, since both the bash
wrapper and its node child match the filter. Kept the fix,
isolated it to its own parent-only commit (`e20ebb1`), and
flagged the drift explicitly in that commit message.

Takeaway for the next dispatch: the "no unrelated edits" rule
holds, but minor, self-contained improvements Codex picks up in
passing can be salvaged as separate commits rather than
discarded. The rule is against *polish noise in the feature
commit*, not against ever touching adjacent files.

## Prompt error — "26 atoms" vs. 22

The archived prompt says "all 26 atoms listed in
`09-policy-and-tenancy.md`" in the Permission atoms section, then
lists 22. The design doc has exactly 22. Codex implemented the
22 atoms actually listed, which matches the design. The "26" is
a Claude-authored typo in the prompt. I chose **not** to
retroactively edit the archived prompt — the archive rule's
whole point is that the file reflects what Codex was actually
sent. The Wave 1 commit message captures the caveat. If this
causes review confusion, the design doc is the authoritative
count, and the code matches it.

## Mock-tier + real-MySQL tier: both delivered as asked

The revised prompt's two-tier test requirement (round 02 of the
dispatch) landed as specified:

- Tier 1 — `tests/common/mock.rs` (`MockStore`, HashMap + Mutex,
  implements `EntityStore` + `ContentStore` via `async-trait`) +
  `tests/permission_mock.rs` (11 `tokio::test(current_thread)`,
  every `evaluate_permission` branch including error paths).
- Tier 2 — `tests/permission_mysql.rs` (10
  `tokio::test(multi_thread)`, `#[ignore = "requires MySQL
  testcontainer"]`, six entity round-trips + four e2e permission
  cases).

Nothing short-circuited between tiers — the evaluate function
under test is the same code path in both, so Tier 1/Tier 2
divergence would surface as test-result drift. That's exactly
the shape the rationale paragraph in the prompt called for.

## Substrate-trait method names Codex used (for Wave 2 reuse)

Reading `philharmonic-store`, these are what the policy crate
calls — handy to have documented as Wave 2 will touch the same
surface for `TenantEndpointConfig`:

- `EntityStoreExt::get_latest_revision_typed::<Kind>(id)` →
  principal + role fetches, typed.
- `EntityStoreExt::create_entity_typed::<Kind>(id)` +
  `append_revision_typed::<Kind>(id, seq, input)` → used in the
  mock's test-seeding helpers.
- `EntityStore::list_revisions_referencing(target, attr_name)` →
  RoleMembership discovery via the "principal" attribute.
- `EntityStore::get_entity(id)` + `get_latest_revision(id)` →
  untyped fallbacks used during membership walk (kind is
  verified explicitly against `RoleMembership::KIND` /
  `RoleDefinition::KIND`).
- `ContentStore::put / get / exists` → for the `permissions`
  content blob + content content-store round-trip.

No trait methods were invented. The prompt's "don't add trait
methods to `philharmonic-store`" rule held.

## Next

Gate-1 crypto approval (at
`docs/design/crypto-approvals/2026-04-21-phase-2-sck-and-pht.md`)
is in place with the version notes: `sha2 = "0.11"`,
`rand_core = "0.10.1"`, `getrandom = "0.4.2"`,
`aes-gcm = "0.10"` (confirmed), `zeroize = "1"` (confirmed). Wave
2 prompt can be drafted against those pinned versions, with the
test-vector discipline the crypto-review protocol requires.
Wave 2 is the next dispatch.
