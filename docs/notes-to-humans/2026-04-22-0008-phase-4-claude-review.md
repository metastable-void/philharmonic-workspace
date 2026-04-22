# Phase 4 review — `philharmonic-workflow` initial implementation

**Date:** 2026-04-22
**Landing commit:** `d4e8dba` (parent) / `a897790`
(philharmonic-workflow)
**Prompt:** `docs/codex-prompts/2026-04-22-0003-phase-4-workflow.md`
**Codex report:** `docs/codex-reports/2026-04-22-0002-phase-4-workflow.md`

Independent read-through of Codex's Phase 4 landing. Intermediate
state — nothing published yet. Flagging what I found so you
can adjudicate at Gate-2 before the `philharmonic-workflow
0.1.0` publish.

## Scope vs. spec

Every one of ROADMAP §Phase 4's seven tasks and the design doc
07 entity shapes is present:

- Three entity kinds declared in `src/entities.rs` with slot
  declarations matching doc 07 exactly: `WorkflowTemplate`
  (script + config content, tenant pinned, is_retired scalar);
  `WorkflowInstance` (context + args content, template + tenant
  pinned, status scalar); `StepRecord` (input + output + error +
  subject content, instance pinned, step_seq + outcome
  scalars).
- `SubjectContext` + `SubjectKind` in `src/subject.rs` reuse
  `philharmonic_policy::{Tenant, MintingAuthority}` as
  requested — no parallel marker types.
- `StepExecutor` + `ConfigLowerer` async traits use
  `#[async_trait]` as specified.
- `WorkflowEngine<S, E, L>` is the three-parameter generic from
  doc 07 with four async methods (`create_instance`,
  `execute_step`, `complete`, `cancel`).
- `InstanceStatus` is a `#[repr(i64)]` enum with `is_terminal`,
  `can_transition_to`, and `try_from_i64` methods. Exhaustive
  transition-matrix test in `src/status.rs`.
- Nine-step execution sequence implemented in
  `engine.rs::execute_step` — transport errors return early
  without records, script errors persist a failed step before
  transitioning the instance, malformed executor results fall
  through the script-error path.
- Subject-content audit discipline: see §Strongest design
  choice below.

## Strongest design choice

Codex enforced the audit-discipline invariant **architecturally**,
not just behaviorally. `src/subject.rs` defines two distinct
types:

- `SubjectContext` (full shape: kind, id, tenant_id,
  authority_id, claims) — used by the engine and the script
  argument.
- `StepRecordSubject` (persistence-safe: kind, id,
  authority_id only) — used only for step-record subject
  content.

The engine calls `subject.to_step_record_subject()` at the
persistence boundary, which drops `claims` AND `tenant_id` by
type construction. A future contributor cannot accidentally
leak claims into a step record by typoing a struct literal —
the `StepRecordSubject` type literally doesn't have the field.

The matching behavioral test
(`tests/engine_mock.rs::step_record_subject_never_persists_claims`)
seeds a distinctive claims marker, runs a step, reads the
persisted subject back from the content store, and asserts
both `claims` and `tenant_id` are absent from the serialized
object. Belt and suspenders on the most sensitive invariant
in Phase 4 — this is exactly the right discipline for the
single irrecoverable-if-wrong rule.

## Other invariants I verified

- **Step-record-before-instance-revision ordering** is pinned
  by `tests/engine_mock.rs::execute_step_writes_step_record_before_instance_revision`.
  The test records the `MockStore`'s call sequence during a
  step, finds both `AppendRevision { kind: StepRecord::KIND,
  revision_seq: 0, .. }` and `AppendRevision { kind:
  WorkflowInstance::KIND, revision_seq: 1, .. }`, and asserts
  the step-record index precedes the instance index. If
  someone inverts the order, this test fails immediately.
- **No panics in `src/`.** I greppped for `.unwrap()`,
  `.expect()`, `panic!`, `unreachable!`, `todo!`,
  `unimplemented!`. The only hits are one `.unwrap()` inside
  `#[cfg(test)] mod tests` in `status.rs` (fine per
  conventions). Production paths propagate with `?` through
  typed `WorkflowError` variants.
- **No unchecked arithmetic.** `step_seq = latest.revision_seq.checked_add(1)
  .ok_or(WorkflowError::IntegerOverflow { field: "step_seq" })`
  at `engine.rs:151` is the pattern I wanted to see for u64
  sequence counters.
- **No lossy `as` casts.** The only `as` in `src/` is
  `self as i64` in `InstanceStatus::as_i64`, which is
  provably-lossless because the enum is `#[repr(i64)]`.
- **KIND UUIDs** were generated via `./scripts/xtask.sh
  gen-uuid -- --v4` (Codex confirmed in its report); stored as
  `Uuid::from_u128(0x…)` literals with a generation-date
  comment on each.

## Non-obvious design call flagged

Codex added a `Pending → Failed` transition to the state
machine that the doc 07 transition diagram does not explicitly
list (it shows Pending → Running / Completed / Cancelled and
Running → Running / Completed / Failed / Cancelled). Codex's
justification: a first-step failure happens while the instance
is still `Pending` (revision 0), and the execution sequence
requires script errors to transition the instance to `Failed`.
Without `Pending → Failed`, first-step failures would violate
the state machine.

**My read:** Codex resolved a genuine inconsistency between
doc 07's §Status transitions diagram and §Execution sequence
step 8 ("Script error → record failed step, transition
instance to Failed"). The resolution is pragmatic and the
inline comment spells out why.

**Open question for you:** do we (a) update doc 07 to list
`Pending → Failed` explicitly, (b) rewrite the execution
sequence to transition Pending → Running before the step even
starts (so failure comes from Running), or (c) leave the
one-off justification in the code? My lean is (a) — the
existing doc is the one out of sync; the code now matches
what the spec clearly intends.

## Test suite

- Unit tests in `src/` — 3 (state-machine exhaustion and
  round-trip in `status.rs`).
- Tier-1 mock-substrate integration tests in
  `tests/engine_mock.rs` — 8 tests, all passing. Includes
  the two invariant tests above, plus the
  terminal-state-immutability cases, transport-failure
  short-circuit, `done: true` completion, and malformed-result
  handling.
- Tier-2 MySQL integration tests in `tests/engine_mysql.rs` —
  3 tests, `#[ignore]`'d. Exercise end-to-end
  create-then-N-steps against `philharmonic-store-sqlx-mysql`
  via testcontainers. Per Codex's report, they pass when run
  (`./scripts/rust-test.sh --ignored philharmonic-workflow`).

I did not run the testcontainers tier myself in this review
pass — trusting Codex's report plus the fact that tier-1 mock
coverage is thorough.

## Pre-landing status

Codex flagged that its full-workspace `pre-landing.sh
philharmonic-workflow` run failed on the `fmt --check` phase
because `xtask/src/bin/codex-fmt.rs` (Claude's newly-added
transcript viewer in the same session) was unformatted. Codex
correctly didn't fmt-fix a file outside its task's crate scope.

I've re-run `cargo fmt --all` after Codex handed off. Workspace
is now fmt-clean; clippy --workspace --all-targets -- -D
warnings is clean. The `cargo fmt` fix ships in the same
commit as this review.

The crate-scoped runs all passed per Codex's report:
- `rust-lint.sh philharmonic-workflow`
- `rust-test.sh philharmonic-workflow`
- `rust-test.sh --ignored philharmonic-workflow`

## Pre-publish checklist (still to do before 0.1.0)

- [ ] Full `pre-landing.sh` across the workspace, now that fmt
      drift is fixed.
- [ ] `miri-test.sh philharmonic-workflow`. The engine's
      storage interactions are async and could surface stacked-
      borrows issues the tier-1 mock doesn't; worth the run.
      Note that testcontainers tier-2 tests won't execute under
      miri (FFI + real I/O).
- [ ] Your Gate-2 review of the Pending → Failed transition
      resolution and any other design calls you want to audit.
- [ ] `CHANGELOG.md` initial entry (currently the crate has no
      CHANGELOG).
- [ ] `README.md` rewrite (same — crate has a minimal stub
      from Codex's pass; may want a longer usage example).
- [ ] Version bump `0.0.0 → 0.1.0` + `publish-crate.sh`.
- [ ] Post-publish `verify-tag.sh` + doc sweep marking Phase 4
      shipped (ROADMAP, 00-index, 03-crates-and-ownership,
      15-v1-scope, README).

## Minor cosmetic observations (not blockers)

- `tests/engine_mock.rs:36` has `ScalarValue::Bool(_) =>
  panic!("outcome must be i64")` — test-only panic; could be
  a `.unwrap_or_else(|| ...)` or a match-to-failure but not
  worth changing.
- Engine calls `json!({"context", "args", "input", "subject"})`
  to assemble the script argument — matches doc 07's four-field
  spec. Worth noting that `subject` here is the full
  `SubjectContext` JSON (with claims), distinct from the
  persisted `StepRecordSubject`. The script sees claims; the
  audit log doesn't. Correct per design doc 07 §"Script
  argument" and §"Step record" combined.

## Recommendation

Phase 4 is in good shape for Gate-2. The audit-discipline
invariant is enforced both architecturally and behaviorally;
the no-panic rule is honored; the spec-drift on Pending →
Failed is resolved pragmatically and flagged clearly. After
your Gate-2 sign-off on the spec-drift question and a clean
full-workspace pre-landing + miri pass, 0.1.0 can publish.

Not blocking for v1 and not urgent for the Golden Week window —
Phase 4 is the last thing on the runway I proposed to land
before the break. Phase 5 (connector triangle crypto) and
anything downstream stays post-break per your earlier decision.
