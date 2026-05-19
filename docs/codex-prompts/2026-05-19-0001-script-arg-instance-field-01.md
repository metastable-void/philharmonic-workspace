# Add `instance: { id, step }` to the workflow script argument

**Date:** 2026-05-19 (JST)
**Slug:** `script-arg-instance-field`
**Round:** 01 — initial dispatch. One crate (`philharmonic-workflow`)
plus three doc files; no other submodules touched.
**Subagent:** `codex:rescue`

## Motivation

Workflow JS scripts currently receive
`{context, args, input, subject, data}`
([`philharmonic-workflow/src/engine.rs:243-249`](../../philharmonic-workflow/src/engine.rs#L243-L249)).
The workflow `context` field is the caller-mutable state slot — **not**
the instance UUID — so there is no way for a script to know which
`WorkflowInstance` it is running inside, nor which step seq it
occupies. Yuka has decided to surface this as a new top-level field
with shape `instance: { id, step }`.

This is a small, well-scoped feature that touches one engine site,
the script-arg JSON contract (JS-observable but additive, so
non-breaking from a script-author perspective), three design / guide
docs, and the `philharmonic-workflow` CHANGELOG `[Unreleased]`
section. Non-crypto, non-breaking from a JS author's perspective
(new field, ignorable; the JSON canonicalisation used to hash inputs
applies to the `input` slot only, not the assembled `script_arg`, so
no stored hashes shift). **No publish, no version bump** — the
crate's `Cargo.toml` stays at `0.1.6`; the change lands in the
`[Unreleased]` section of `CHANGELOG.md` and ships with whatever
version Yuka cuts later.

## References (authoritative if anything in this prompt contradicts them)

1. [`docs/design/07-workflow-orchestration.md`](../design/07-workflow-orchestration.md)
   — current spec lists `{context, args, input, subject, data}` at
   lines 241-242, 282-285, 353, and the historical-expansion bullet
   at 492-493. Will become
   `{context, args, input, subject, data, instance}`.
2. [`docs/design/06-execution-substrate.md`](../design/06-execution-substrate.md)
   — script signature example at line 104; statelessness paragraph
   at line 324 currently says the executor receives "no instance ID,
   no correlation" — that line needs careful rework (the executor
   itself remains stateless across calls; the *script arg* now
   carries the instance UUID as an input, not as executor-held
   state).
3. [`docs/guide/workflow-authoring.md`](../guide/workflow-authoring.md)
   §"Script argument" (lines ~717-727).
4. [`CONTRIBUTING.md`](../../CONTRIBUTING.md):
   - **§4** Git workflow — `./scripts/commit-all.sh` only;
     **you do not commit** (see Hand-off shape below).
   - **§5** Script wrappers — every cargo call routes via the
     wrappers (which set `CARGO_TARGET_DIR=target-main`).
   - **§10.3** No panics in library `src/` — no `.unwrap()` /
     `.expect()` on `Result` / `Option`, no `panic!` /
     `unreachable!` / `todo!` / `unimplemented!` on reachable
     paths. Tests are exempt.
   - **§10.4** Library crates take bytes, not paths — file I/O
     belongs in bins. Not relevant to this dispatch directly
     (no new I/O), but keep the boundary intact.
   - **§11** Pre-landing checks — `./scripts/pre-landing.sh` is
     mandatory before declaring done.
   - **§14.6** English as the default for prose.

## Context files pointed at

**Engine site (the only `src/` edit):**

- [`philharmonic-workflow/src/engine.rs`](../../philharmonic-workflow/src/engine.rs)
  — `execute_step` builds `script_arg` at line 243. Add the new
  `instance` field there. The instance UUID is available via
  `instance_id.internal().as_uuid()`; the step is the
  locally-computed `step_seq` (line 150-153).

**Trait that stays put (do not change its signature):**

- [`philharmonic-workflow/src/executor.rs`](../../philharmonic-workflow/src/executor.rs)
  — `StepExecutor::execute(script, arg: &JsonValue, config)` keeps
  its current shape. The new field travels inside the existing
  `arg` parameter; no trait-level change.

**Tests:**

- [`philharmonic-workflow/tests/`](../../philharmonic-workflow/tests/)
  — add coverage that the `instance` field is present with the
  right shape. There are existing harnesses in the crate that
  exercise `execute_step`; mirror their shape if one fits.
  Otherwise the cleanest seam is a small mock `StepExecutor`
  whose `execute` method captures the `arg` parameter into a
  `Mutex<Option<JsonValue>>` so the test can assert on the
  exact assembled value. Implementing the mock in-test is
  preferred over reaching for the real Boa executor — this is a
  shape contract test, not an end-to-end test.

**Version + CHANGELOG:**

- [`philharmonic-workflow/Cargo.toml`](../../philharmonic-workflow/Cargo.toml)
  — **stays at `0.1.6`. Do not bump.** Per Yuka's explicit call
  2026-05-19: "unreleased; no version bump needed."
- [`philharmonic-workflow/CHANGELOG.md`](../../philharmonic-workflow/CHANGELOG.md)
  — there is already an empty `## [Unreleased]` section just below
  the header. Add a bullet under it describing the new field. Do
  NOT create a new dated `## [0.1.x]` heading.

**Docs (the three files):**

- `docs/design/07-workflow-orchestration.md` (parent repo).
- `docs/design/06-execution-substrate.md` (parent repo).
- `docs/guide/workflow-authoring.md` (parent repo).

## Shape (locked)

```json
{
  "context":  { ... },
  "args":     { ... },
  "input":    { ... },
  "subject":  { ... },
  "data":     { ... },
  "instance": { "id": "<uuid string, lowercased, hyphenated>",
                "step": <unsigned integer, the step_seq for the
                         step currently executing> }
}
```

- `id`: the `WorkflowInstance` entity UUID, serialised via the
  default `Uuid::to_string()` (lowercased, hyphenated 8-4-4-4-12).
- `step`: the `step_seq` value the engine computes at the top of
  `execute_step` (1-based; `latest.revision_seq.checked_add(1)`).
  Semantic: "the seq the engine is about to assign to the step
  record being created" — matches what the audit log records for
  this same step.

No other fields. Yuka explicitly approved `{ id, step }`; do NOT
add `status`, `template_id`, `tenant_id`, `correlation_id`, or
anything else — those can be added later if a use case appears.

## Hard requirements

- Place the `instance` key after `data` in the JSON object source
  ordering, for readability when humans skim engine code or
  log dumps. (The wire format goes through `serde_json` which
  preserves insertion order in `json!{...}`; canonical JSON is
  only applied at hash sites and is not relevant to `script_arg`
  itself.)
- The `id` field must serialise as a JSON **string**, not a
  nested `{uuid: ...}` object, not a byte array. JS authors will
  write `arg.instance.id` and expect `typeof === 'string'`.
- The `step` field must serialise as a JSON **number**, not a
  string. Use `step_seq`'s native unsigned-integer type; do not
  stringify.
- **Do not change `StepExecutor`'s trait signature.** Adding a
  field inside `arg: &JsonValue` is sufficient.
- **Do not change `StepRecordSubject` or the step-record subject
  persistence shape.** The audit confinement at
  [`engine.rs:233`](../../philharmonic-workflow/src/engine.rs#L233)
  must stay limited to `kind` + `id` + `authority_id`. The
  instance identity is already implicit at the step-record level
  (the step record's parent entity is the instance) and leaking
  it into the subject would muddle the design.
- **No panics in library `src/`** per CONTRIBUTING.md §10.3.
  `step_seq` is already produced via `checked_add(1)?` and the
  UUID is infallible to format — no new fallibility expected,
  but if you find yourself reaching for `unwrap()`, stop and
  rethink.
- **No version bump.** `philharmonic-workflow/Cargo.toml` stays
  `version = "0.1.6"`. Add the entry to the existing
  `## [Unreleased]` section in `CHANGELOG.md`.

## Tests

Add to `philharmonic-workflow/tests/` (or extend an existing test
file if one already exercises `execute_step` and admits a small
addition). Cover at minimum:

1. **Field presence & shape.** `arg.instance` exists; `arg.instance.id`
   is a string matching `latest_instance_revision`'s UUID;
   `arg.instance.step` is a number equal to `step_seq`.
2. **Step increments across calls.** Two consecutive
   `execute_step` calls on the same instance produce `step = 1`
   then `step = 2`. (Adjust starting seq depending on how the
   existing test harness creates the instance — the contract is
   "the step seq the engine is about to assign", whatever that
   number is for the harness's first call.)
3. **Consistency with the step record.** The `step_seq` value
   the engine assigns to the new `StepRecord` equals the
   `arg.instance.step` value the script sees in the same call.
   (If the harness already exposes the StepRecord that lands,
   compare against its `step_seq` field. Otherwise this is
   implicit in (1).)
4. **Other fields unchanged.** The five existing fields
   (`context`, `args`, `input`, `subject`, `data`) remain
   present and unchanged in shape. A quick assertion that
   `arg.get("context")` etc. are all `Some(_)` is enough — this
   guards against accidental removal during the edit.

If the cleanest test seam is a tiny capture-mock `StepExecutor`,
implement it inline in the test file:

```rust
struct CaptureExecutor {
    captured: Mutex<Option<JsonValue>>,
}

#[async_trait::async_trait]
impl StepExecutor for CaptureExecutor {
    async fn execute(
        &self,
        _script: &str,
        arg: &JsonValue,
        _config: &JsonValue,
    ) -> Result<JsonValue, StepExecutionError> {
        *self.captured.lock().unwrap() = Some(arg.clone());
        // Return a minimal valid result so execute_step succeeds
        // and the step record lands. Match whatever shape the
        // engine's `parse_executor_result` expects (see
        // engine.rs::execute_step).
        Ok(json!({ "output": {}, "context": {}, "done": false }))
    }
}
```

(`Mutex::lock().unwrap()` is fine in test code — §10.3's
no-panics rule scopes to library `src/`, not tests.)

## Doc updates

### `docs/design/07-workflow-orchestration.md`

- Lines 241-242: `{context, args, input, subject, data}` →
  `{context, args, input, subject, data, instance}` and a short
  parenthetical "`instance` carries `{id, step}` — the running
  workflow instance's UUID and the step seq currently
  executing."
- Line 282: "five-field argument" → "six-field argument".
- Line 285: the destructured signature example —
  `function main({context, args, input, subject, data, instance})`.
- Line 353: "Assemble executor arg: `{context, args, input,
  subject, data, instance}`."
- Lines 492-493: the historical-expansion bullet ends at
  "`subject`". Append a new bullet to the same list:
  "Then expanded again to add `instance: {id, step}` so scripts
  can know which workflow instance and step they are executing
  inside (2026-05-19)."

### `docs/design/06-execution-substrate.md`

- Line 104: the script signature example —
  `function main({context, args, input, subject, data, instance})`.
- Line 324: the "no instance ID, no correlation" line. Rework
  so the statelessness invariant stays accurate without
  contradicting the new field. Suggested replacement: "The
  executor itself remains stateless across calls — it receives
  a script + arg + config triple per invocation and retains
  nothing across invocations. The `arg.instance` field carries
  the workflow instance UUID and step seq as inputs to the
  script, but those are call-time identifiers, not
  executor-held state." Adapt to the surrounding paragraph's
  flow; don't paste verbatim if it reads awkward.

### `docs/guide/workflow-authoring.md`

- §"Script argument" block (around lines 717-727): extend the
  code block to include the `instance` field, and add a short
  prose line below the block:

  > `instance.id` is the `WorkflowInstance` UUID (string);
  > `instance.step` is the step seq (number, 1-based) currently
  > executing. Useful for log correlation and idempotency-key
  > construction.

  Match the file's existing voice (declarative, short
  sentences).

## Version policy

**No bump.** `philharmonic-workflow/Cargo.toml` stays `0.1.6`.
The change rides on the next published version (whenever Yuka
cuts it). Add the entry to the existing `## [Unreleased]`
section in `philharmonic-workflow/CHANGELOG.md`:

```markdown
## [Unreleased]

### Added
- `script_arg.instance: { id, step }` — workflow scripts now
  receive the running `WorkflowInstance` UUID (string) and the
  step seq (number, 1-based) currently executing. Non-breaking:
  scripts that don't read `arg.instance` are unaffected.
```

**Do NOT bump** any other crate; **do NOT publish**. Yuka
publishes via `./scripts/publish-crate.sh` after review.

## Verification (mandatory before declaring done)

Run, once, at the end:

```sh
./scripts/pre-landing.sh
```

Must print `=== pre-landing: all checks passed ===`. The script
auto-detects modified crates (`philharmonic-workflow` here) and
runs fmt + check + clippy `-D warnings` + rustdoc + workspace
test + per-crate `--ignored` test phase. No raw `cargo fmt` /
`cargo clippy` / `cargo test` — `pre-landing.sh` covers them with
the right `CARGO_TARGET_DIR`.

If the box becomes contended mid-pre-landing, check with
`./scripts/xtask.sh resource-pressure` and back off if
`load1/cpus` climbs well above 1.0.

## Hand-off shape: Codex does not commit

**Leave the working tree dirty.** Claude commits via
`./scripts/commit-all.sh` after reviewing the diff. The script
has a `codex-guard` (`scripts/lib/codex-guard.sh`) that walks
the ancestor process chain and aborts if any process is named
`*codex*`; calling `commit-all.sh` from inside a Codex run will
hard-fail. Do not work around the guard.

Specifically:

- Do **not** run `./scripts/commit-all.sh` (any flags, including
  `--dry-run`, `--parent-only`, `--exclude`).
- Do **not** run raw `git commit` / `git push` / `git add`. The
  pre-commit hooks enforce signoff + signature + `Audit-Info:`
  trailer; the codex-guard fires from those hooks too.
- Do **not** run `git commit --no-verify` / `--no-gpg-sign`.
- Do **not** run `git reset` / `git rebase` / `git amend`.
  History is append-only.
- Do **not** run `./scripts/push-all.sh`. Claude pushes after
  reviewing.
- Do **not** run `./scripts/publish-crate.sh`. Yuka publishes.
- Do **not** edit `HUMANS.md`. Agent-readable, agent-writable
  forbidden.

Edits land in the working tree across:

- `philharmonic-workflow/` submodule — `src/engine.rs`,
  `tests/<new-or-extended>`, `CHANGELOG.md`.
- Parent repo — `docs/design/06-execution-substrate.md`,
  `docs/design/07-workflow-orchestration.md`,
  `docs/guide/workflow-authoring.md`, and `Cargo.lock`
  (regenerated automatically if `cargo build` ran).

Codex's session summary should mention which submodules + the
parent have dirty trees so Claude knows where to look.

## Codex report (encouraged)

If anything non-obvious surfaced during this round — a design
call on UUID serialisation, a choice of test seam, an edge case
in `step_seq` semantics, an unexpected interaction with the
existing test harness — write a short report to
`docs/codex-reports/2026-05-19-0001-script-arg-instance-field.md`
per [`docs/codex-reports/README.md`](../codex-reports/README.md).
Routine specified-and-shipped work doesn't need one; the session
summary covers it. Leave the report **dirty** in the working
tree; Claude commits it alongside the implementation diff.

If you skip the report, say so in the session summary.

## Outcome

Pending — will be updated after Codex run.

---

<task>
Add a sixth field, `instance: { id, step }`, to the
`script_arg` JSON value assembled in
`philharmonic-workflow::WorkflowEngine::execute_step`
([`philharmonic-workflow/src/engine.rs:243-249`](philharmonic-workflow/src/engine.rs#L243-L249)).
`id` is the running `WorkflowInstance`'s UUID as a hyphenated
lowercase string (default `Uuid::to_string()` form); `step` is
the `step_seq` value the engine computes at the top of
`execute_step` (1-based, the seq the engine will assign to the
step record being created). The trait signature of
`StepExecutor::execute` does **not** change — the new field
travels inside the existing `arg: &JsonValue` parameter.

**Reference docs (authoritative if they contradict this prompt):**

1. `docs/design/07-workflow-orchestration.md` lines 241-242,
   282-285, 353, 492-493.
2. `docs/design/06-execution-substrate.md` lines 104, 324.
3. `docs/guide/workflow-authoring.md` §"Script argument".
4. `CONTRIBUTING.md` §§4, 5, 10.3, 11, 14.6 plus the banned-dep
   posture in `deny.toml` (no new deps expected — none of the
   listed banned crates are at risk here).
5. The full preamble above (this prompt's `## …` sections;
   especially "Shape (locked)", "Hard requirements", "Tests",
   "Doc updates", "Version policy" = NO BUMP, "Verification").

**Hard constraints (locked):**

- `instance.id` serialises as a JSON string (default Uuid
  formatting); `instance.step` serialises as a JSON number.
- `instance` is the sixth and only new field; do not add
  `status` / `template_id` / `tenant_id` / `correlation_id`.
- `StepExecutor` trait signature unchanged.
- `StepRecordSubject` persistence shape unchanged — audit
  confinement stays at `kind` + `id` + `authority_id` only.
- No panics in `philharmonic-workflow/src/` per
  CONTRIBUTING.md §10.3. Tests exempt.
- **No version bump.** `philharmonic-workflow/Cargo.toml` stays
  at `0.1.6`. CHANGELOG entry goes under the existing
  `## [Unreleased]` section, NOT a new dated heading.
- **No publish.** Yuka publishes after review.

**Per-file scope (the full set of edits):**

- `philharmonic-workflow/src/engine.rs` — add `"instance":
  json!({ "id": <uuid string>, "step": <step_seq number> })`
  to the `script_arg` `json!{...}` at line 243.
- `philharmonic-workflow/tests/<file>` — add shape coverage
  (field presence, step increment across two calls,
  consistency with the step record's `step_seq`, other five
  fields still present). Use a capture-mock `StepExecutor` if
  no existing harness fits.
- `philharmonic-workflow/CHANGELOG.md` — add a bullet under
  `## [Unreleased]`.
- `docs/design/07-workflow-orchestration.md` — update the four
  script-arg references (lines 241-242, 282-285, 353,
  492-493) per the preamble.
- `docs/design/06-execution-substrate.md` — update the
  signature example (line 104) and rework the statelessness
  paragraph (line 324) per the preamble.
- `docs/guide/workflow-authoring.md` — extend the §"Script
  argument" code block and add a one-paragraph description of
  the new field.

**Verification (must run + pass before declaring done):**

- `./scripts/pre-landing.sh` — clean.

That is the entire mandatory verification surface for this
scope.

<default_follow_through_policy>
Codex is expected to land the engine edit, tests, CHANGELOG,
and all three doc updates in this single round. "Engine done,
docs pending" or "engine + docs done, tests pending" is **not**
a complete result — keep going.

If a hard blocker surfaces (e.g. the existing test harness's
shape makes a clean capture-mock impossible without invasive
refactoring), **STOP and report the blocker before partial
landing**. A partial result that mixes the engine change with
unfinished tests is worse than a clean "blocker found, here's
what I'd recommend" report.

If `pre-landing.sh` fails on something orthogonal (a
pre-existing flake unrelated to this change), **fix forward**
only if the fix is mechanical and local. If it's structural,
**STOP and report**.
</default_follow_through_policy>

<completeness_contract>
"Complete" means:

1. `philharmonic-workflow/src/engine.rs::execute_step` builds
   `script_arg` with the new `instance` field present, shaped
   `{ id: <string>, step: <number> }`.
2. `philharmonic-workflow/tests/` covers field presence,
   step-seq increment across two calls, consistency with the
   step record's `step_seq`, and other-fields-still-present.
3. `philharmonic-workflow/CHANGELOG.md` has a new bullet under
   the existing `## [Unreleased]` section. **No new dated
   heading; no version bump.**
4. `philharmonic-workflow/Cargo.toml` is unchanged (`version =
   "0.1.6"` stays).
5. `docs/design/07-workflow-orchestration.md` has the four
   script-arg references updated.
6. `docs/design/06-execution-substrate.md` has the signature
   example + statelessness paragraph updated.
7. `docs/guide/workflow-authoring.md` §"Script argument" has
   the code block extended and a description added.
8. `./scripts/pre-landing.sh` passes.
9. Working tree left dirty across `philharmonic-workflow/` +
   parent (per "Hand-off shape" above). **No commits, no
   pushes** — Claude commits and pushes after reviewing the
   diff.
10. Session summary lists which submodule + the parent have
    dirty trees so Claude can scope the `commit-all.sh` run.
11. Outcome section of this prompt file updated with: (a)
    list of files touched, (b) the line in `engine.rs` where
    the field was added, (c) the test seam chosen
    (capture-mock vs existing-harness), (d) any blockers
    encountered, (e) residual risks, (f) submodule + parent
    head SHAs at hand-off.

If any of (1)–(10) is incomplete, the dispatch is INCOMPLETE.
Report INCOMPLETE clearly with what's done and what's left,
and STOP — don't synthesise a half-result.
</completeness_contract>

<verification_loop>
During implementation (between rounds of edits):

  CARGO_TARGET_DIR=target-main cargo check -p philharmonic-workflow --all-targets

Per-crate tests:

  CARGO_TARGET_DIR=target-main cargo test -p philharmonic-workflow --all-targets

Final, single run:

  ./scripts/pre-landing.sh

If `pre-landing.sh` fails, read the failure carefully:

1. If a clippy / doctest / test in `philharmonic-workflow`
   caused it, that's a local fix — make the fix, re-run
   pre-landing.
2. If it's a workspace-wide failure (e.g. a downstream crate
   no longer compiles because of an unintended trait/type
   change), back the change out of that crate, re-run.
   `philharmonic-workflow` is a published-crate dep; downstream
   `philharmonic-api` and the bins consume it.
3. If you're tight-looping pre-landing.sh on a slow box, run
   `./scripts/xtask.sh resource-pressure` first to confirm the
   host has headroom; back off if it doesn't.

Do not run raw `cargo fmt` / `cargo clippy` / `cargo test` —
`pre-landing.sh` covers them with the right `CARGO_TARGET_DIR`
and feature flags.

Do not run `cargo build --workspace` standalone as a "check" —
the per-crate `cargo check -p` covers it without the full
link-time cost.
</verification_loop>

<missing_context_gating>
Before you start editing, the workspace state must match the
prompt's claims:

  ./scripts/status.sh

Should print `(clean)` for the parent repo and all submodules.
If it doesn't, **STOP and report**. The prompt assumes a clean
starting tree — uncommitted changes in unrelated submodules
mean someone else is mid-edit; don't conflict.

If the `## [Unreleased]` heading in
`philharmonic-workflow/CHANGELOG.md` has already collected
unrelated bullets that don't appear in `git status`, treat them
as in-progress work and append your bullet beneath them without
disturbing them. If the section is empty (a single blank line
under the heading), add an `### Added` subheading then your
bullet, mirroring the file's previous-version subheading style.

If the existing test layout makes a capture-mock executor
genuinely impossible without restructuring the harness, **STOP
and report**. Don't refactor the existing test harness as part
of this dispatch — propose a follow-up shape instead.
</missing_context_gating>

<action_safety>
- **You do not commit.** Leave the working tree dirty across
  `philharmonic-workflow/` + parent. `./scripts/commit-all.sh`
  (any flags) and raw `git commit` / `git push` / `git add` /
  `git reset` / `git rebase` / `git amend` are all forbidden.
  The script's `codex-guard` will hard-abort if you try; the
  same guard fires from the pre-commit hooks. Claude commits +
  pushes after reviewing the diff.
- **Never** invoke `./scripts/push-all.sh`. Claude pushes.
- **Never** invoke `./scripts/publish-crate.sh`. Yuka publishes.
- **Never** edit `HUMANS.md`. Agent-readable, agent-writable
  forbidden.
- Every `cargo` invocation needs
  `CARGO_TARGET_DIR=target-main` (the wrappers in `scripts/`
  set this; if you call cargo directly, set it yourself).
- POSIX-ish host: no `bash`-only constructs in any shell you
  invoke. The wrappers are POSIX `#!/bin/sh`.
- The workspace's authoritative timezone is JST (Asia/Tokyo).
  Any wall-clock value you generate for the CHANGELOG or the
  codex-report belongs in JST; today is 2026-05-19 (Tue). The
  CHANGELOG entry goes under `## [Unreleased]` and does not
  carry a date.
</action_safety>

<structured_output_contract>
At the end of the dispatch, return:

1. **Summary** (2-3 sentences): the engine site touched, the
   test seam chosen, total new/changed lines split engine vs
   tests vs docs.
2. **Touched files**: full list, grouped by submodule + parent.
3. **`script_arg` diff**: paste the before/after of the
   `json!{...}` call at `engine.rs:243` so the reviewer can
   eyeball the field placement.
4. **Test coverage**: number of new test cases, what each one
   asserts. Note whether you used a capture-mock or extended
   an existing harness.
5. **Doc updates**: list each of the three doc files with the
   specific anchors / line ranges touched.
6. **CHANGELOG entry**: paste the new bullet verbatim so the
   reviewer can confirm placement under `[Unreleased]`.
7. **Verification results**:
   - `pre-landing.sh`: PASS / FAIL (with one-line summary if
     FAIL).
8. **Working-tree state at hand-off**:
   - List which submodule + parent have dirty trees.
   - No commits expected from you. Claude will commit + push
     after reviewing the diff.
9. **Codex report**: if you wrote
   `docs/codex-reports/2026-05-19-0001-script-arg-instance-field.md`,
   note its presence (dirty in working tree; Claude commits
   it). If you skipped, say so.
10. **Residual risks**: anything you'd flag for Claude or
    Yuka before publish (e.g. a downstream consumer that
    might want to start reading `arg.instance` immediately).
11. **Outcome paragraph** for the prompt-archive file: 4-6
    sentences summarising the round for posterity, ready to
    drop into `## Outcome` of this file.
</structured_output_contract>
</task>
