# Phase 5 Wave B Codex dispatch — complete, Gate-2 pending

**Date:** 2026-04-23
**Session:** 19737a1e-f157-4aee-ba7e-575a339459a9

## tl;dr

Codex finished the Wave B implementation run cleanly after a
botched first dispatch. Self-reported scope: full Wave B across
client/service/router, tests, docs, validation. The three
submodule working trees are dirty and un-pushed; **Gate-2 review
is the next gate before anything commits or publishes**. I have
not pulled the structured output yet — say the word when you
want me to run `/codex:result task-mob2cian-d255lb` and
reconstruct the `## Outcome` section of the archived prompt.

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
