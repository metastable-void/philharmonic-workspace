# Post-v1 Quick Wins Prompt Audit

**Date:** 2026-05-02
**Prompt:** `docs/codex-prompts/2026-05-02-0002-post-v1-quick-wins-d1-d2-d10-01.md`

## Scope

I audited the new D1/D2/D10 Codex prompt against the current roadmap,
design 16, AGENTS.md constraints, and the implementation surfaces named
by the prompt. I did not implement the dispatch.

## Findings

### High: prompt requires Codex to commit, which conflicts with AGENTS.md

The prompt repeatedly requires Codex to create commits:

- `docs/codex-prompts/2026-05-02-0002-post-v1-quick-wins-d1-d2-d10-01.md:379-401`
  says to use `./scripts/commit-all.sh` for every commit and gives a
  three-commit sequence.
- `:466-490` makes committed submodule + parent commits part of the
  completeness contract.
- `:498-504` asks for `status.sh` clean and signed HEAD confirmation from
  `heads.sh`.
- The structured output contract asks for submodule and parent commit SHAs.

That directly conflicts with the current Codex rule in `AGENTS.md`: Codex
does not commit, push, branch, reset, rebase, stash, or otherwise change
Git state. The same rule is mirrored in `CONTRIBUTING.md` and in the role
split described by the workspace. If this prompt is handed to Codex as-is,
the agent either has to refuse the commit portions or violate its own
workspace instructions.

Suggested fix: rewrite the prompt so Codex leaves the working tree dirty,
runs the verification commands, and reports intended commit boundaries and
candidate commit messages. Claude should run `commit-all.sh` after review.
If Claude wants three logical chunks, the prompt can say "finish D1, report
ready for Claude commit; then continue only after Claude commits" or split
the dispatch into three prompts.

### Medium: stale `docs/instructions/README.md` reference

The prompt says the HUMANS.md rule is "enforced by
`docs/instructions/README.md`" at `:535-536`. That file no longer exists.
The current rule lives in `AGENTS.md`, `CLAUDE.md`, and `CONTRIBUTING.md`.

This is not just cosmetic because the same prompt also says not to write to
`docs/notes-to-humans/` or `docs/design/`, so it is trying to be precise
about ownership boundaries. A deleted enforcement path makes that section
look stale.

Suggested fix: replace the parenthetical with "as documented in
`AGENTS.md`, `CLAUDE.md`, and `CONTRIBUTING.md`."

### Medium: D10 omits an existing editable JSON textarea

D10 says it will retrofit existing JSON / JS textareas and lists the
affected sites at `:308-324`. The list covers:

- `Templates.tsx`
- `TemplateDetail.tsx`
- `Endpoints.tsx`
- `EndpointDetail.tsx`
- `Instances.tsx`

But the current WebUI also has an editable JSON textarea in
`philharmonic/webui/src/pages/InstanceDetail.tsx:146` /
`philharmonic/webui/src/pages/InstanceDetail.tsx:260-265`: the
`executeInput` field for manual instance execution. This is not a
read-only `JsonViewer`; it is a user-editable JSON input parsed by
`parseJson(executeInput)`.

Leaving it out means D10 would still leave one visible raw JSON textarea
after claiming to retrofit existing JSON / JS editors. It also means the
completeness contract's "all five page sites updated" check at `:479-482`
would pass while the user-facing editor work remains incomplete.

Suggested fix: add `src/pages/InstanceDetail.tsx` to the references,
context files, affected-sites list, completeness contract, and structured
output list, with `executeInput` as language `json`.

### Medium: D2 motivation conflates pool `run_timeout` with JavaScript execution limits

The prompt motivates D2 by saying embedding-dataset jobs will run for
minutes and need a longer per-job `run_timeout` (`:144-152`). It then asks
only for `MechanicsJob.run_timeout` to override the pool's deadline in
`MechanicsPool::run` / `run_nonblocking_enqueue` (`:187-202`).

In current code, that pool timeout is only the Rust-side deadline for how
long `run()` waits for enqueue/reply. The Boa runtime separately enforces
`MechanicsExecutionLimits.max_execution_time`: `RuntimeInternal::run_source_inner`
computes a deadline from `self.execution_limits.max_execution_time`, and
the default is 10 seconds. A job with `run_timeout = 30 minutes` can still
be killed by the runtime execution limit if the script runs longer than the
pool's configured `max_execution_time`.

The roadmap and design 16 currently name D2 as a per-job `run_timeout`
override, so the implementation request is consistent with the dispatch
label. The prompt should still clarify the limitation so Codex does not
believe D2 alone makes multi-minute embed scripts possible. Either:

- explicitly state that D2 only extends the pool wait deadline and that D5
  must handle mechanics execution-limit provisioning separately, or
- expand D2 to include a per-job execution-limit override if that is the
  actual intended prerequisite.

### Medium: D2 timeout test recipe is likely to produce the wrong failure

The prompt asks for `run_uses_per_job_timeout_when_set` using "a script
that sleeps via a loop" and expecting `MechanicsError::run_timeout` within
about one second (`:208-210`). Existing runtime tests show tight loops are
normally stopped by Boa/runtime execution limits, especially
`max_loop_iterations`, and report `MechanicsError::Execution`, not
`MechanicsError::RunTimeout`.

The current test suite's reliable `RunTimeout` examples are queue/reply
timeout tests in `mechanics-core/src/internal/pool/tests/queue.rs`, where
no worker reply arrives before the pool deadline. For a real executing
script, the runtime execution limit can win before the pool wait timeout,
which makes the proposed test flaky or simply wrong.

Suggested fix: for per-job pool timeout behavior, ask for tests that use
the synthetic queue/reply-timeout pattern, or a local endpoint server with
execution limits set high enough that the pool deadline is guaranteed to
fire first. Avoid describing the fixture as "sleep via a loop" unless the
expected error is an execution-limit error.

## Non-issues Checked

- The stated current versions for `philharmonic-store-sqlx-mysql` (`0.1.2`)
  and `mechanics-core` (`0.3.1`) match the local manifests.
- The D1 `CREATE_CONTENT` target is correct: current schema still declares
  `content_bytes MEDIUMBLOB NOT NULL`.
- The existing D1 index migration template and `is_duplicate_key_name`
  helper exist in `philharmonic-store-sqlx-mysql/src/schema.rs`.
- The named D10 textareas in `Templates`, `TemplateDetail`, `Endpoints`,
  `EndpointDetail`, and `Instances` exist and are editable.

## Recommendation

Do not dispatch the prompt unchanged. The Git-state conflict is the main
blocker. While editing it, also add the missing `InstanceDetail.tsx`
textarea, replace the stale `docs/instructions/README.md` reference, and
clarify the D2 timeout/execution-limit boundary before Codex starts work.
