# Post-vacation status snapshot and next-steps — 2026-05-10

## Context

You were on Golden Week 2026-04-29 → 2026-05-06 (祝日 5/4 みどりの日,
5/5 こどもの日, 5/6 振替休日) plus a personal vacation 2026-05-07 / 05-08
(Thu / Fri). First regular working day back is **Mon 2026-05-11**.

This note summarises what landed in your absence, flags the only
critical-path human gate (Gate 1 review), and recommends the
order of next-actions for the return-from-vacation week. Written
on Sun 2026-05-10 (off-hours JST) at your explicit request to
revise the ROADMAP and document the next moves.

The `Current state` block in [`docs/ROADMAP.md`](../ROADMAP.md)
and the §9 sequencing have been updated in the same commit;
[`README.md`](../../README.md)'s Status section is also
refreshed. This note is the verbose companion.

## What landed since 2026-05-02

### 2026-05-02 (Fri) — Post-v1 quick wins, commit `ee2bd61`

D1, D2, and D10 from ROADMAP §9 landed in a single Codex
dispatch (Codex round 01 of prompt
`docs/codex-prompts/2026-05-02-0002-...`):

- **D1** — `philharmonic-store-sqlx-mysql 0.1.2 → 0.1.3`.
  `content.content_bytes` MEDIUMBLOB → LONGBLOB on fresh
  schemas; idempotent `ALTER MODIFY COLUMN` migration on
  existing deployments via the new `COLUMN_MIGRATIONS` const;
  integration test extended to assert post-migrate column
  type.
- **D2** — `mechanics-core 0.3.1 → 0.3.2`. Optional
  `MechanicsJob::run_timeout` override (`Option<Duration>`)
  with `with_run_timeout` builder + accessor + serde
  back-compat; `MechanicsPool::run` and
  `run_nonblocking_enqueue` resolve
  `job.run_timeout().unwrap_or(self.run_timeout)` at the
  deadline-from-timeout step. Four pool tests on the
  synthetic-worker pattern from `queue.rs` + three
  serde/builder tests.
- **D10** — `philharmonic` WebUI. Eight `<textarea>` JSON/JS
  editors across six pages (Templates, TemplateDetail,
  Endpoints, EndpointDetail, Instances, InstanceDetail)
  replaced with a 95-LOC CodeEditor.tsx wrapper around
  CodeMirror 6 (modular `@codemirror/*` + meta-package).
  `main.js` 91.5 KB → 253.4 KB gzipped; `dist/` regenerated.

Post-landing context: the introduction of CodeMirror also
adds a maintained editor dependency that D6 (embedding-datasets
WebUI) will reuse for its structured editors.

### 2026-05-04 (Mon) — Gate-1 proposal, commit `e2baa69`

Claude wrote
[`docs/crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md`](../crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md),
recommending **Approach B**: synthesize a non-persisted
instance UUID at the API server's lowerer for the embedding
job's per-request scope, instead of adding a `LowerScope`
enum to `philharmonic-workflow`'s public lowerer trait
(Approach A). Approach B leaves the workflow public surface
and the COSE_Sign1 / COSE_Encrypt0 shapes untouched — only
the API server lowerer changes. Cross-linked from
[`docs/design/16-embedding-datasets.md`](../design/16-embedding-datasets.md).

**Status: PENDING your review.** This is the only outstanding
Gate-1 obligation, and it blocks D4 and D5. Please read the
proposal and either sign off (per the §5 crypto-review
protocol) or push back with revisions.

### 2026-05-10 (Sun, today) — Workspace-tooling hygiene

Opportunistic infrastructure work, not on the dispatch plan:

- New [`xtask/src/bin/tar-concatenate.rs`](../../xtask/src/bin/tar-concatenate.rs)
  and [`scripts/archive-all.sh`](../../scripts/archive-all.sh):
  HEAD-pinned workspace bundles → `archives/...tar.zst`.
- [`CLAUDE.md`](../../CLAUDE.md) `calendar-jp` rule tightening
  + project-level PostToolUse hook
  ([`.claude/hooks/calendar-jp-grounding.sh`](../../.claude/hooks/calendar-jp-grounding.sh))
  that auto-injects calendar-jp output after every
  `commit-all.sh` / `push-all.sh` / `publish-crate.sh`. This
  was triggered by my own drift — I had failed to ground time
  for the entire Sunday session before you pointed it out.
  Going forward, the discipline is machine-enforced rather
  than agent-discipline.
- `cargo update` refresh of `Cargo.lock` (patch/minor bumps).
  One advisory remains: `rsa 0.9.10` RUSTSEC-2023-0071
  ("Marvin Attack" timing sidechannel). 0.9.10 IS the latest
  published version — no upstream fix yet — so `cargo update`
  cannot clear it. GitHub dependabot still shows
  `1 moderate, 1 low` after push.
- `Code-stats:` backfill: pre-trailer commits no longer show
  `-` in `stats-log.sh` or as gaps in `docs/stats.svg`.
  [`scripts/backfill-stats.sh`](../../scripts/backfill-stats.sh)
  reconstructs each pre-trailer commit's tree via
  `git archive` (parent + every gitlink-pinned submodule) into
  /tmp scratch, runs `tokei`, and appends a row to
  [`docs/stats-cache.tsv`](../stats-cache.tsv). Includes a
  rename heuristic that handled the
  `philharmonic-connector-impl-vector-searc` →
  `…-vector-search` typo correction cleanly. SVG now plots
  all 503 commits (vs 39 before).

## Pending decisions / blockers

### Critical path

**Gate-1 review.** Read
[`docs/crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md`](../crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md)
and decide between Approaches A (LowerScope enum) and B
(synthesized UUID, recommended). Approval lands as
`docs/crypto/approvals/...md` per the §5 protocol; refusal /
revision becomes a fresh proposal.

### Non-blocking but worth deciding this week

**`rsa 0.9.10` advisory (RUSTSEC-2023-0071).** No upstream
fix yet. Options, in order of effort:

1. Wait for upstream to ship the timing-sidechannel fix
   and re-run `cargo update` then.
2. Suppress in `.github/dependabot.yml` if you accept the
   risk (treat as "known, tracked, no action").
3. `cargo tree -i rsa` to identify the consumer; consider
   whether the dependency is replaceable.
4. Vendor-patch via `[patch.crates-io]` — heavy maintenance,
   only worth it if there's a real exploitation path in
   our usage.

Today's audit run is in commit `fcad83f`'s message; nothing
urgent.

## Suggested next-actions, in order

1. **Gate-1 review** — highest leverage; unblocks D4 and D5.
2. **D3 dispatch** — embedding-datasets backend (policy entity
   + `embed_dataset:*` permission atoms, API CRUD + source-items
   + corpus endpoints, `WorkflowTemplate.data_config` slot,
   workflow-engine `data` assembly in `execute_step`).
   Cross-crate but cohesive feature surface; one Codex dispatch.
   Independent of Gate 1 — can dispatch immediately.
3. **D6 dispatch** — embedding-datasets WebUI (structured
   table editor for source items, Import modal for CSV/JSON,
   collapsed-by-default vector view, i18n for `en.ts` and
   `ja.ts`). Depends on D3's API endpoints; can run in
   parallel with D4 / D5 once Gate 1 clears.
4. **D7 / D8 / D9 dispatches** — Tier 2/3 connectors:
   `philharmonic-connector-impl-email-smtp` (Tier 2),
   `…-llm-anthropic` (Tier 3),
   `…-llm-gemini` (Tier 3). Independent of one another and
   of section A; safe to dispatch in parallel when bandwidth
   allows. None touch the crypto path.
5. **D11 dispatch** — workflow authoring guide rewrite.
   Anytime; independent of everything else.
6. **`docs-jp/` exec summary** — eligible for an update if
   you'd like one for the May-2-to-now arc; the docs-jp
   skill is reserved to Claude (not Codex). Easiest to
   bundle with the next milestone landing rather than now.

## What is NOT awaiting you

- The reference deployment continues to run; no regression
  reports.
- No CI failures on `origin/main`.
- All 25 crate names reserved on crates.io; no publish
  cooldowns expiring this week.
- `pre-landing.sh` (workspace + `--xtask`) clean as of
  commit `fcad83f` (post-`cargo update`).
- No crypto path changed since the May 2 dispatch — the
  miri checkpoint at `philharmonic-policy` /
  `philharmonic-connector-{client,service,common}` /
  `philharmonic-types` is current.

## A small flag

I drifted on the `calendar-jp` grounding rule for the entire
Sunday session before you flagged it. Today's commits include
CLAUDE.md tightening + a mechanical PostToolUse hook that
auto-injects fresh JST time after every state-changing git
op, so the discipline is now machine-enforced and the same
drift should not recur. Flagging here so the choice is
visible in the durable journal and not just in chat
scrollback.
