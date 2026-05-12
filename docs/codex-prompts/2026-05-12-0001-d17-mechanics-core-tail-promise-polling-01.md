# D17 — `mechanics-core` tail-promise polling (initial dispatch)

**Date:** 2026-05-12
**Slug:** `d17-mechanics-core-tail-promise-polling`
**Round:** 01 (initial dispatch — D17, ROADMAP §3.E, single
crate `mechanics-core`)
**Subagent:** `codex:codex-rescue`

## Motivation

Today the worker's run-job response is held open until every
queued promise / timer / async job inside the Boa realm drains
to quiescence. An unawaited `mechanics:endpoint(...)`, a live
`setTimeout`, or a fire-and-forget `Promise.then(...)` keeps
the step open until the per-job `max_execution_time` deadline
trips — the script's `return` is not the response fence;
quiescence is.

D17 inverts the response fence. As soon as the script's
top-level resolves (sync return, or an awaited promise
fulfilled), `mechanics-core` serialises the result and sends
the run-job response back. Pending promises continue to be
polled in the background on the same worker thread until they
settle or the deadline expires — their side effects (real HTTP
calls to connector services, real timer callbacks) still
complete, but no longer hold the response open.

Design is fully settled (2026-05-12) — see references below.
No open product questions; Codex implements the locked
behavior.

## References

- [`docs/ROADMAP.md` §3.E](../ROADMAP.md#e-execution-substrate-runtime-semantics-1-dispatch)
  — D17 entry with the resolved design-choices block.
- [`docs/design/06-execution-substrate.md` §Tail-promise
  polling](../design/06-execution-substrate.md#tail-promise-polling)
  — the authoritative behavior spec. If anything below
  contradicts that section, the design doc wins.
- Current `mechanics-core` 0.4.0 unhandled-rejection stance:
  [`mechanics-core/src/internal/runtime.rs:277-313`](../../mechanics-core/src/internal/runtime.rs#L277-L313).
  D17 preserves the "trust the script's own try/catch" stance
  — a tail-poll rejection never fails the response, only
  bumps the internal counter.

## Context files pointed at

- [`mechanics-core/src/internal/runtime.rs`](../../mechanics-core/src/internal/runtime.rs)
  — host hooks (`RuntimeHostHooks` lines 21-55, owns
  `pending_unhandled_rejections`), `RuntimeInternal`
  (line 122), `run_source_inner` (line 214), the call into
  `ctx.run_jobs()?` at line 275 that needs to become a
  partial-then-detached drain.
- [`mechanics-core/src/internal/executor.rs`](../../mechanics-core/src/internal/executor.rs)
  — `Queue` (line 18, implements Boa's `JobExecutor`),
  `run_jobs` sync bridge (line 202-206 via `tokio_local
  .block_on(&tokio_rt, …)`), `run_jobs_async` quiescence
  loop (line 209-302), `check_deadline` (line 65-73),
  `set_deadline`.
- [`mechanics-core/src/internal/pool/shared.rs`](../../mechanics-core/src/internal/pool/shared.rs)
  — worker thread (line 181-244). Each worker thread owns
  one `RuntimeInternal`, processes jobs serially, sends
  the result on `pool_job.reply_sender()` (line 230).
  Worker loop's structure constrains how the early-reply
  callback is wired.
- [`mechanics-core/src/internal/pool/worker.rs`](../../mechanics-core/src/internal/pool/worker.rs)
  — `PoolJob` (line 13-51), `reply_sender` (line 44),
  `send_result` (line 40). Reply channel is
  `crossbeam_channel::Sender<Result<Value, MechanicsError>>`.
- [`mechanics-core/src/internal/job.rs`](../../mechanics-core/src/internal/job.rs)
  — `MechanicsJob` (line 12), `MechanicsExecutionLimits`
  (`max_execution_time` at line 181 — the per-job
  deadline). No changes expected to this file.

## Outcome

Pending — will be updated after Codex run.

---

## STRUCTURED-OUTPUT-CONTRACT — READ THIS FIRST

Recent dispatches all honored the six-section structured-output
report emitted before `task_complete`, including the verbatim
`RUN STATUS: COMPLETE` token. **Maintain the streak.**

The contract is repeated at the end of the prompt.

---

## Shape (locked decisions)

All seven design choices come from [`docs/design/06-execution-substrate.md`
§Tail-promise polling](../design/06-execution-substrate.md#tail-promise-polling).
Implementation freedom is constrained to fitting the locked
behavior — don't relitigate the design.

### Behavior

1. **Response fence shifts.** `run_source` (the worker's
   blocking entry point) currently returns the final `Value`
   to its caller, which then ships it on the reply channel.
   The new shape: the response (main outcome) ships on the
   reply channel **as soon as main settles**, while `run_source`
   continues to drain the queue on the worker thread.
   `run_source` itself returns only when tail-poll has exited
   (quiescence or deadline).

2. **Partial-drain to main settlement.** Replace the
   unconditional `ctx.run_jobs()?` at
   `runtime.rs:275` with a partial-drain loop that runs one
   iteration of the executor at a time and checks `res.state()`
   after each iteration. Exit the loop when `res.state()` is
   `Fulfilled` / `Rejected`, or when the deadline trips
   (existing `Pending` → "Default export promise did not
   settle" error path preserved). Bound the loop by the
   same deadline check used in `run_jobs_async`.

3. **Early reply.** Once main has settled, dispatch the
   serialised result (the existing `js_value_to_serde_json` or
   equivalent path) to the reply channel via a callback
   supplied by the worker. After early reply, the response is
   no longer this function's concern — even if tail-poll
   subsequently fails, the response stays as sent.

4. **Tail-poll continuation.** After early reply, continue
   driving the executor (microtasks + async jobs + timeout
   jobs + generic jobs) using the same `run_jobs_async`
   quiescence loop already present in `executor.rs`. Stop
   on the first of: all queues empty (natural quiescence),
   or the deadline trips, or any internal executor error.

5. **Deadline mid-tail-poll.** When the deadline trips during
   tail-poll, drop the realm + in-flight futures and emit a
   single `tracing::warn!` line. Required fields: job ID (or a
   stable-per-job identifier the worker thread can supply),
   in-flight async-job count, and queued promise/timeout/generic
   job counts at abort time. Use whatever crate-level `tracing`
   imports are already in scope; if `tracing` isn't a dep yet,
   add it. Format example:

   ```
   tail_poll_aborted job_id=<id> in_flight=<n> queued=<m> reason="max_execution_time exceeded"
   ```

   The exact field shape is your call as long as the four
   facts (job_id, in-flight count, queued count, reason) are
   present in structured form. **Don't** emit any
   tracing/log line on normal tail-poll quiescence — that's
   the hot path, no per-job log spam.

6. **Realm + context lifetime.** The realm (`Context`) is
   `!Send` and lives on the worker thread; tail-poll continues
   in the same `tokio_local` LocalSet that drove main, on the
   same worker thread. Drop the realm at tail-poll exit
   (quiescence or deadline). Do not move the realm between
   threads.

7. **Unhandled rejection during tail-poll.** The existing
   `RuntimeHostHooks::promise_rejection_tracker` already
   increments `pending_unhandled_rejections` on `Reject` and
   decrements on `Handle` — that's the only sink today. Make
   sure tail-poll continues to feed that counter (no special
   handling needed if the host hook stays installed throughout
   tail-poll). Do **not** fail the response on a tail rejection
   — main has already responded; the post-script
   `has_unhandled_rejections()` check at `runtime.rs:277-308`
   is intentionally non-fatal post-main and stays that way.

### Hardcoded default; no knob

- No new `MechanicsJob` field. No per-job opt-in/opt-out.
- No new `MechanicsExecutionLimits` field. No per-worker
  opt-in/opt-out.
- No `mechanics-config` schema change.
- No `mechanics` HTTP-service wire-format change.

The new behavior is universal. Every script execution gets it.

### What stays the same

- The module-evaluation-time `has_unhandled_rejections()` check
  at `runtime.rs:261-265` stays strict (top-level awaits in
  modules are rare; module-load failure is a different class).
- The "Default export promise did not settle" error
  (`runtime.rs:310-313`) stays — it's the main-execution
  timeout path. If `res.state()` is still `Pending` after the
  partial-drain hits the deadline, return this error and don't
  send anything on the reply channel (let the worker layer
  send it on the reply, same as today).
- `RuntimeInternal::set_execution_limits` semantics. The
  `max_execution_time` budget is shared between main and tail
  (no separate tail-budget field).
- `mechanics-core`'s public API surface. `Runtime`,
  `RuntimePool`, `MechanicsJob`, `MechanicsConfig`,
  `MechanicsExecutionLimits` — all stable.
- `mechanics` (HTTP service) wire format. Request:
  `{module_source, arg, config}`. Response: JSON value or
  error. **No change.**

### Suggested implementation sketch (your call; don't follow blindly)

The worker loop at `pool/shared.rs:209-244` currently does:

```rust
let reply = pool_job.reply_sender();
let job = pool_job.into_job();
let result = std::panic::catch_unwind(... || runtime.run_source(job));
match result {
    Ok(result) => { let _ = reply.send(result); }
    Err(_) => { ... worker_panic ... }
}
```

A reasonable shape: extend `RuntimeInternal::run_source` (or
add a sibling `run_source_with_early_reply`) to take a
closure / `Sender` for the early reply. Inside, after the
partial-drain settles main, invoke the closure with the
serialised result. Then proceed to tail-poll. Return from
`run_source` only after tail-poll exits; the returned value
becomes either `Ok(())` (response already sent — worker
ignores) or a `MechanicsError` that the worker sends as a
fallback if early-reply never fired.

Two reasonable sub-shapes for the partial drain:

- **Sub-shape A**: parameterise `run_jobs_async` with an
  early-exit predicate `|| !main_promise.state().is_pending()`,
  and have it return as soon as the predicate trips. Reuse the
  same async loop for tail-poll, no early-exit.
- **Sub-shape B**: introduce a dedicated `run_jobs_until_main_settled`
  function that mirrors `run_jobs_async` but exits on the
  predicate, leaving `run_jobs_async` untouched for tail-poll.

Pick whichever produces cleaner code. **Document the choice in
residual risks.**

### Logging dep

If `mechanics-core` doesn't already pull `tracing` in,
add `tracing = "0.1"` (workspace-pinned version if one
exists; otherwise the latest 0.1.x). Don't add `tracing-subscriber`
— that's the consumer's job. If `log` is already a dep instead,
use `log::warn!` and note the choice in residual risks; either
crate satisfies the design's "emit one warn line" requirement.

### Version bump

Patch bump: `0.4.0 → 0.4.1`. The change is internal-behavior;
public API surface is preserved. Pre-1.0 SemVer allows either
patch or minor for a behavior change of this scope; patch is
the right call here since:

- No public signature changes.
- The wire-format response from the `mechanics` HTTP service
  is identical.
- The change is observable only via timing of the response and
  the new tail-poll lifecycle.

Update `CHANGELOG.md` with a `[0.4.1] - 2026-05-12` entry
above `[0.4.0] - 2026-05-11`. Single bullet — name the
behavior change, link to ROADMAP D17.

## Tests

Required, in this order:

1. **Sync return, no pending work** (existing semantics
   preserved): script `export default function() { return
   {ok: 1}; }`. Assert: response value is `{ok: 1}`, no
   warn line emitted, `run_source` returns promptly.

2. **Async return, all awaited** (existing semantics
   preserved): script awaits a single
   `mechanics:endpoint(...)` (test stub responding
   synchronously) and returns its result. Assert: response
   value matches expected, no warn line.

3. **Fire-and-forget endpoint (NEW)**: script kicks off
   `mechanics:endpoint(...)` **without** await, then returns
   `{ok: 2}` synchronously. The endpoint stub records a
   call counter on every invocation. Assert:
   - Response value is `{ok: 2}`.
   - After waiting for tail-poll to complete (use a small
     sync helper or test the runtime's job-completed signal),
     the endpoint counter is `1` — proves the side effect
     completed despite not being awaited.
   - No warn line.

4. **`setTimeout` without await (NEW)**: script calls
   `setTimeout(() => globalThis.__seen = true, 50)` and
   returns immediately. After tail-poll completes, assert
   that the timer fired (some test-observable signal — e.g.
   the endpoint stub being called inside the timer body, or
   a state hook). No warn line.

5. **Deadline mid-tail-poll (NEW)**: configure the
   `MechanicsExecutionLimits.max_execution_time` to a small
   value (e.g. 200ms). Script kicks off a
   `setTimeout(... , 10_000)` and returns. Assert:
   - Response is sent promptly (well before 200ms).
   - After 200ms+, `run_source` returns.
   - A `tracing::warn!` line was emitted naming the abort
     and including the in-flight + queued counts. Use
     `tracing_test` or a custom subscriber to capture
     lines; if neither is convenient, use a feature-gated
     test hook in `runtime.rs` that records the warn-line
     fields to a `Cell<Option<_>>` on `RuntimeHostHooks` —
     that's acceptable. Pick whichever is cleaner.

6. **Unhandled rejection during tail-poll (NEW)**: script
   does `Promise.reject(new Error("boom"))` without
   catching, then returns `{ok: 3}`. Assert:
   - Response value is `{ok: 3}`.
   - `pending_unhandled_rejections` counter ends > 0
     (read via a test-only accessor; add one if it
     doesn't exist).
   - No response failure.
   - No warn line (tail-rejection isn't the deadline abort).

7. **Main never settles (existing semantics preserved)**:
   script returns an unresolved `Promise` from an external
   resolver that's never called. Assert: deadline trips,
   `run_source` returns
   `Err(MechanicsError::runtime_limit("Default export
   promise did not settle"))` — the original error path.

Existing `mechanics-core` tests must remain green
byte-for-byte; if any of them implicitly depended on
quiescence-before-response timing, surface in residual
risks and propose a fix in the same dispatch.

## Verification flow

```sh
./scripts/pre-landing.sh
```

Runs fmt + check + clippy (-D warnings) + rustdoc + test
across the workspace. Slow — ~minutes — run once before
final commit, not in a tight edit/run loop.

Also run:

```sh
./scripts/check-api-breakage.sh mechanics-core 0.4.0
```

`cargo-semver-checks` against the 0.4.0 crates.io baseline.
A patch bump on internal-behavior is fine pre-1.0; the tool
may flag nothing or may flag the `RuntimeInternal` surface
if anything `pub(crate)`-ish changed in a way it doesn't
like. Surface the output in residual risks.

If `mechanics-core` has `--ignored` tests (real-network or
timing-sensitive ones), pre-landing already covers them
per the script's design. No extra invocation needed.

Skip:

- No publish — Claude reviews and decides post-Codex.
- No edits to `mechanics` (HTTP service) — its wire format
  is preserved. If you find yourself wanting to touch it,
  **stop** and surface in residuals.

## Prompt (verbatim)

<task>
Ship D17: introduce tail-promise polling to `mechanics-core`.
After this dispatch, the worker's run-job response returns to
its caller as soon as the script's top-level resolves, and
the realm continues polling pending promises in the
background on the same worker thread until quiescence or
the per-job `max_execution_time` deadline.

Single crate. No public-trait change. No `mechanics` HTTP
wire-format change. No `mechanics-config` schema change. No
crypto path touched.

Deliverables (in order):

1. **Partial-drain to main settlement.** In
   `mechanics-core/src/internal/runtime.rs`, replace the
   unconditional `ctx.run_jobs()?` at line 275 with a loop
   that drives the executor one iteration at a time and
   exits when the wrapped `JsPromise` for main has settled
   (`Fulfilled` / `Rejected`) **or** the deadline trips.
   Reuse the executor's existing drain machinery — pick
   sub-shape A or B per the prompt's "Suggested
   implementation sketch" section. Document the choice in
   residual risks. The existing `Pending` → "Default
   export promise did not settle" error path
   (lines 310-313) stays — that fires when the partial
   drain hits the deadline with main still pending.

2. **Early reply plumbing.** Extend
   `RuntimeInternal::run_source` (or add a sibling
   method — your call; whichever is cleaner) to accept a
   reply-channel `Sender` or a closure that the function
   invokes with the serialised main outcome the moment
   main settles. After early reply, `run_source` continues
   into tail-poll (step 3) and only returns when tail-poll
   exits. Update the worker loop at
   `mechanics-core/src/internal/pool/shared.rs` (line 209-
   244) to pass `pool_job.reply_sender()` into the new
   entry point. The worker still owns the panic-catch
   layer; on a panic mid-execution it sends the
   `worker_panic` error on the reply channel if the early
   reply hasn't already fired.

3. **Tail-poll continuation.** After early reply, drive
   the executor's existing `run_jobs_async` quiescence
   loop (or equivalent) until the queues are empty or the
   deadline trips. Reuse the existing
   `Queue::run_jobs_async` if it's a clean fit;
   factor out a helper if not. Side effects during
   tail-poll (HTTP calls to connectors, timer
   callbacks) must continue to complete normally — the
   realm and host hooks stay alive throughout.

4. **Deadline-abort warn line.** When the deadline trips
   during tail-poll, drop the realm + in-flight futures
   and emit one `tracing::warn!` line with at least these
   fields:
   - `job_id` or a stable per-job identifier (use whatever
     `MechanicsJob` carries today; if there's no
     intrinsic ID, derive one from a monotonic counter on
     the worker — your call, document the choice).
   - In-flight async-job count at abort time.
   - Queued promise/timeout/generic job counts.
   - Reason string (`"max_execution_time exceeded"`).
   Don't emit any log line on normal tail-poll quiescence.
   If `tracing` isn't already a `mechanics-core` dep, add
   `tracing = "0.1"`. Don't add `tracing-subscriber`.

5. **Test suite.** Add the seven tests listed in the
   prompt's "Tests" section. Existing
   `mechanics-core` tests must remain green
   byte-for-byte.

6. **Version + changelog.** Bump `Cargo.toml` from
   `0.4.0` to `0.4.1`. Add a `[0.4.1] - 2026-05-12`
   entry to `CHANGELOG.md` referencing ROADMAP D17.

7. **Verification.** Run `./scripts/pre-landing.sh` and
   `./scripts/check-api-breakage.sh mechanics-core 0.4.0`.
   Both outputs go in the structured-output report.

8. **No publish.** Claude reviews and decides post-Codex.

## Hard constraints

- **Pre-D17 behavior preserved on the failure path.** A
  main-execution timeout (deadline trips before main
  settles) returns the same "Default export promise did
  not settle" error today; D17 keeps that exact error and
  exact code path. The change is only on the success path
  (early reply + tail continuation).
- **No public-API change.** `Runtime`, `RuntimePool`,
  `MechanicsJob`, `MechanicsConfig`,
  `MechanicsExecutionLimits` — all signatures stable.
  `run_source` may take an additional **internal** /
  `pub(crate)` argument; the existing public entry stays
  unchanged in name and signature, or there's a new
  internal method called from the worker and the existing
  one delegates with a no-op early-reply callback for any
  remaining direct callers.
- **No HTTP-service wire change.** The `mechanics` HTTP
  service's request/response shape is preserved. End
  users see the same JSON come back — just faster.
- **No `mechanics-config` schema change.** No new fields.
- **No crypto path touched.** AAD / AEAD / COSE / SCK
  paths are untouched and don't appear in this dispatch.
- **No `unsafe` blocks.** No panicking in lib `src/`
  (no `.unwrap()` / `.expect()` on `Result`/`Option`,
  no `panic!` / `unreachable!` / `todo!` /
  `unimplemented!` on reachable paths). Tests are exempt.
- **Realm stays on its worker thread.** The Boa `Context`
  is `!Send`. Tail-poll continues on the same
  `tokio_local` LocalSet that drove main, on the same
  worker thread. No `tokio::spawn` (which requires
  `Send`); use `tokio::task::spawn_local` if you need it,
  or just keep tail-poll inline on the same `block_on`.

<structured_output_contract>
**Critical: emit this report before `task_complete`.**

Six sections, in this order:

1. **Summary** — one paragraph: what landed, sub-shape
   chosen (A or B), tail-warn dep choice (`tracing` vs.
   `log`), version bump applied, semver-checks outcome.
   Include the verbatim string `RUN STATUS: COMPLETE` or
   `RUN STATUS: PARTIAL — <reason>` for grep.

2. **Touched files** — exhaustive list with
   `(new|edited|deleted) <path> — <one-line note>`.

3. **Verification results** — exact commands + outcomes:
   - `./scripts/pre-landing.sh` — pass/fail/exit code.
   - `./scripts/check-api-breakage.sh mechanics-core
     0.4.0` — pass/fail/output excerpt.

4. **Residual risks / known issues** — including:
   - Sub-shape choice (A or B) and why.
   - Logging dep choice (`tracing` vs. `log`) and why.
   - Any timing-sensitive tests that needed special
     handling (e.g. tail-completion wait helper, deadline
     accuracy).
   - Job-ID source: intrinsic to `MechanicsJob` or
     derived per-worker; how stable it is.
   - Any pre-existing test that started failing and how
     you addressed it.
   - semver-checks output for `mechanics-core`.

5. **Git state** — current `HEAD` SHA in the parent
   workspace repo and in the `mechanics-core` submodule.
   Confirm no commits made.

6. **Open questions** — questions for Yuka or Claude:
   - Whether `tracing` is the right call vs. `log` (if
     you chose `tracing` and it's the first such use in
     the workspace).
   - Whether a future minor bump (0.4.1 → 0.5.0) is
     warranted to flag the behavior change more
     prominently in version metadata — Yuka's call, not
     yours.
   - Whether `mechanics` (HTTP service) should grow a
     `Connection: close` or similar response header now
     that responses can return while the worker is still
     busy with tail-poll — out of scope for this
     dispatch but worth flagging if you see a reason.
</structured_output_contract>

<default_follow_through_policy>
- Implement in the order listed: partial-drain → early
  reply → tail continuation → warn line → tests →
  version + changelog → verification.
- Run `cargo test -p mechanics-core` directly for fast
  iteration before invoking the heavier pre-landing
  pipeline.
- Prefer reusing `run_jobs_async` for both phases
  (parameterised with an early-exit predicate, sub-shape
  A) unless the predicate makes the loop awkward —
  document the choice either way.
- The new warn line is structured and one-shot per
  aborted job. No log spam on the hot path (normal
  quiescence).
- No edits outside `mechanics-core`. If you find
  yourself wanting to touch `mechanics`,
  `mechanics-config`, `philharmonic-*`, or any other
  workspace member, **stop** and surface in residual
  risks.
- If `cargo build` seems stuck for minutes, run
  `./scripts/build-status.sh` (or watch it) before
  declaring a hang — this workspace's `aws-lc-rs` C
  builds + Boa take real minutes on first build.
</default_follow_through_policy>

<completeness_contract>
"Done" means:

- Partial-drain replaces the unconditional `run_jobs()` at
  `runtime.rs:275`.
- Worker loop passes the reply channel into the new entry
  point; early reply fires on main settlement.
- Tail-poll continuation drives the queues to quiescence
  or deadline; realm stays alive across the transition.
- Deadline-mid-tail-poll emits a structured `tracing::warn!`
  line with job_id + in-flight count + queued count +
  reason.
- All seven tests pass.
- Existing `mechanics-core` tests still green
  byte-for-byte.
- `Cargo.toml` bumped 0.4.0 → 0.4.1; `CHANGELOG.md`
  entry added.
- `./scripts/pre-landing.sh` clean.
- `./scripts/check-api-breakage.sh mechanics-core 0.4.0`
  run and surfaced in residuals.
- Six-section structured-output report emitted before
  `task_complete`.

Partial completion is acceptable only if you hit a token
limit or a genuine blocker — say so explicitly with
`RUN STATUS: PARTIAL — <reason>`. A half-shipped state
machine where main responds early but tail-poll silently
swallows futures is worse than the pre-D17 status quo;
if you can't finish, leave `runtime.rs` reverted to its
pre-D17 shape so the workspace stays in a working state.

A run without the structured-output report is
**incomplete**, even if the code landed.
</completeness_contract>

<verification_loop>
1. Implement partial-drain + early-reply + tail-poll.
2. `cargo test -p mechanics-core` — green.
3. `CARGO_TARGET_DIR=target-main cargo check --workspace`
   — catches any downstream coupling.
4. Run `./scripts/pre-landing.sh` once.
5. Run `./scripts/check-api-breakage.sh mechanics-core
   0.4.0`.
6. Emit structured-output report.
7. `task_complete`.
</verification_loop>

<missing_context_gating>
If you find yourself needing information not in this prompt
or the cited authoritative sources (`docs/design/06` §Tail-
promise polling, ROADMAP §3.E), **stop** and report what's
missing in the structured output's "Open questions" section.

Specifically: do **not**:

- Touch any other crate (`mechanics`, `mechanics-config`,
  `philharmonic-*`, `inline-blob`, anything else).
- Add a new public surface to `mechanics-core` (a new
  `pub fn` / `pub struct` exposed via `lib.rs`). The
  change is internal-behavior; `pub(crate)` is fine.
- Add `tracing-subscriber`, `env_logger`, or any other
  log-consumer crate. Producer-side only.
- Add a new `MechanicsJob` field, a new
  `MechanicsExecutionLimits` field, or a new
  `MechanicsConfig` field.
- Change the HTTP wire format on the `mechanics`
  service.
- Touch any `.claude/`, `docs/`, `docs-jp/`,
  `HUMANS.md`, `CLAUDE.md`, `AGENTS.md`,
  `CONTRIBUTING.md`, or `scripts/` content.
- Publish to crates.io. No `cargo publish` even
  `--dry-run`. Claude reviews and decides post-Codex.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `mechanics-core/src/internal/runtime.rs`
  (edited — partial drain, early reply plumbing,
  tail-poll continuation).
- `mechanics-core/src/internal/executor.rs`
  (edited — likely refactor of `run_jobs_async` for
  early-exit predicate, or new sibling function).
- `mechanics-core/src/internal/pool/shared.rs`
  (edited — worker loop wires `pool_job.reply_sender()`
  into the new entry point).
- `mechanics-core/src/internal/runtime.rs` test module
  (or a new file under
  `mechanics-core/src/internal/runtime/` or
  `mechanics-core/tests/` — your call) for the seven
  required tests.
- `mechanics-core/Cargo.toml` (edited — version bump,
  possibly a `tracing` dep add).
- `mechanics-core/CHANGELOG.md` (edited — `[0.4.1]`
  entry).
- `Cargo.lock` (regenerates when cargo runs).

Files NOT to touch (flag if you find a reason to):

- Any file under `mechanics/`, `mechanics-config/`,
  `philharmonic*/`, `inline-blob/`, or any other
  workspace member.
- The workspace `Cargo.toml` `[patch.crates-io]` block.
- `.claude/`, `docs/`, `docs-jp/`, `HUMANS.md`,
  `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md`,
  `scripts/`.

Git rules: signed-off + signed commits via
`scripts/commit-all.sh` only. **Codex never runs
`commit-all.sh`** — Claude commits after reviewing your
work. No `cargo publish`. No raw `git commit` /
`git push` / `git add` / `git reset` / `git rebase` /
`git revert`. Read-only `git log` / `git diff` /
`git show` is fine.
</action_safety>
</task>
