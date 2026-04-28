# Pre-GW target — full API + Web UI + bin targets end-to-end by Sat 2 May 2026

**Author:** Claude Code · **Audience:** Yuka · **Date:** 2026-04-28 (Tue) JST late morning

Yuka's call this morning, recorded for durability:

> Aim for full API + Web UI testing + bin target end-to-end testing before 2 May (Sat).

## What "the target" covers

By Saturday 2026-05-02 we aim to have:

1. **Phase 8 done end-to-end.** All sub-phases A→I landed,
   including:
   - A — skeleton (✅ landed 2026-04-28 morning, commit
     `51de953`).
   - B — auth (long-lived `pht_` + ephemeral COSE_Sign1).
     Crypto-touching; code-review gate fires.
   - C — authz + tenant-scope enforcement.
   - D — workflow management endpoints.
   - E — endpoint-config CRUD with SCK encrypt/decrypt.
     Crypto-touching; code-review gate fires.
   - F — principal/role/minting-authority CRUD + long-lived
     token generation.
   - G — token-minting endpoint with claim-clipping +
     `authority_epoch` binding. Crypto-touching;
     code-review gate fires.
   - H — audit + rate limit + tenant-admin + operator
     endpoints.
   - I — publish `philharmonic-api` 0.1.0 to crates.io.
2. **Phase 9 test WebUI + binary targets in shape.**
   In-tree, non-published. Per the existing ROADMAP §Phase 9
   task 2:
   - Bin crates that wire the framework library crates into
     runnable processes (at minimum: API host, mechanics
     worker, connector router, connector service).
   - A small WebUI that drives the API end-to-end (login
     via long-lived API token, workflow-template creation,
     instance creation, step execution, audit-log
     inspection).
3. **End-to-end integration scenario green.** The ROADMAP
   §Phase 9 task 1 testcontainers flow exercises every
   crate in one round-trip: tenant admin creates endpoint
   config → creates workflow template → tenant caller
   executes steps → instance reaches terminal state →
   audit log records all of it.

The chat-app-shaped ephemeral-token flow can be deferred
past 5/2 if scope pressure forces a cut — but the
testcontainers happy-path scenario is the minimum bar for
"end-to-end testing".

## Calendar reality

JST today: 2026-04-28 Tue late morning. Calendar window to
5/2:

```
Tue 4/28 — today, working day
Wed 4/29 — 昭和の日 holiday
Thu 4/30 — working day
Fri 5/1  — working day
Sat 5/2  — target completion date (out-of-hours if Yuka works)
```

Three normal working days + one weekend day for: 8 Codex
sub-phase rounds (B–I, with three crypto-review gates at
B/E/G), plus Phase 9 test WebUI + bin targets + the
end-to-end integration scenario.

This is **stretch territory**. The bottleneck isn't Codex
throughput — Codex rounds are ~25 minutes each, so all
8 sub-phases can dispatch back-to-back inside one Yuka-day.
The bottleneck is Yuka's review time, especially the three
crypto-review code gates (B / E / G) which can't be batched
or skipped.

## Working assumptions

1. **Codex rounds dispatch back-to-back where possible.**
   No artificial waits between sub-phases as long as the
   prior round's output is reviewed and merged. Sub-phase
   B's dispatch is happening today (immediately after this
   note + ROADMAP land).
2. **Crypto-review gates fire as scheduled** (B/E/G). They
   are not waivable. If the deadline pressure surfaces a
   reason to skip a gate, the answer is to push the
   deadline, not skip the gate.
3. **Out-of-scope drift gets captured but not implemented.**
   If a sub-phase produces a "would be nice to also fix
   X" finding, X gets noted in
   [`docs/design/12-deferred-decisions.md`](../design/12-deferred-decisions.md)
   or in
   [`docs/design/14-open-questions.md`](../design/14-open-questions.md)
   for a later sweep. Sub-phases stay tight to the
   sub-phase plan in
   [`ROADMAP.md` §Phase 8 §Sub-phase plan](../../ROADMAP.md).
4. **Phase 9 work parallelizable with later Phase 8
   sub-phases.** Once enough of Phase 8 is in for the
   Phase 9 bin targets to compile (probably after F), the
   bin/WebUI work can run alongside G/H/I rather than
   sequentially after I.
5. **Out-of-hours commentary applies.** GW days +
   weekends fall outside regular hours; Yuka may or may
   not work them. Codex rounds Claude dispatches during
   GW are fine — they sit unreviewed until Yuka picks
   them up. No human hand-offs queued for off-hours.

## What gets cut first if 5/2 slips

In rough order, if scope pressure forces a cut:

1. **Phase 9 chat-app-shaped ephemeral-token flow** —
   defer to post-5/2.
2. **Phase 9 binary-target completeness** — keep the API
   host + mechanics worker bins; defer the connector
   router + connector service bins until they're needed
   for a test scenario.
3. **Phase 8 sub-phase H rate limiting** — ship without,
   capture as deferred. Rate limiting can land in a 0.2.0
   bump.
4. **Phase 8 sub-phase H operator-endpoint completeness**
   — ship just `tenant create / suspend / unsuspend`.
5. **Phase 9 WebUI** — defer entirely past 5/2 if all
   other items are cut and we're still slipping. The
   WebUI is *demo* polish; the testcontainers scenario
   covers correctness.

What does NOT get cut:
- The three crypto-review gates (B/E/G).
- `philharmonic-api` 0.1.0 publish.
- The testcontainers end-to-end happy-path scenario.

## What's happening immediately after this note

1. ROADMAP §Phase 8 + §Phase 9 gain a "Target: end-to-end
   in shape by Sat 2026-05-02" header.
2. Sub-phase B Codex prompt drafted + archived per the
   `codex-prompt-archive` skill.
3. Sub-phase B Codex round dispatches today.
4. Codex churns; Yuka reviews B's output (crypto-review
   gate); B merges; Claude dispatches C; iterate.

## Where to read more

- [`ROADMAP.md` §Phase 8 §Sub-phase plan`](../../ROADMAP.md)
  — A→I task list + which sub-phases are crypto-touching.
- [`docs/notes-to-humans/2026-04-28-0001-phase-8-decisions-confirmed.md`](2026-04-28-0001-phase-8-decisions-confirmed.md)
  — confirmed decisions A/B/C/D + crypto-approach
  approval.
- [`docs/codex-prompts/2026-04-28-0001-phase-8-sub-phase-a-skeleton.md`](../codex-prompts/2026-04-28-0001-phase-8-sub-phase-a-skeleton.md)
  — sub-phase A's prompt + outcome (already landed).
