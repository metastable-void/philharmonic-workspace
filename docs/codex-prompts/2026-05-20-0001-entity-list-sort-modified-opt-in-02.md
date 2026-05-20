# Entity list sort: optional `?sort=modified_desc` mode, WebUI opts in (round 02)

**Date:** 2026-05-20 (JST)
**Slug:** `entity-list-sort-modified-opt-in`
**Round:** 02 — re-dispatch after round 01 died without doing
any work. Round 01's archive is at
[`2026-05-20-0001-entity-list-sort-modified-opt-in-01.md`](2026-05-20-0001-entity-list-sort-modified-opt-in-01.md);
its full preamble, references, hard constraints, per-file
scope, tests, verification, hand-off rules, and `<task>` block
are **all authoritative for this round**. Read round 01 in
full before starting; this file does not repeat the content.
**Subagent:** `codex:rescue`

## What's different in round 02

Nothing on the implementation contract. The prior dispatch
returned with the working tree clean (no edits, no
`task_complete` event in the rollout). This round is a clean
re-attempt of the same scope, against the same starting tree.

If you ran into something at runtime that prevented edits
last time, surface it in this round's session summary (or
codex-report) so Claude knows what to thread back to Yuka.

## Starting state

`./scripts/status.sh` should print `(clean)` for the parent
repo and every submodule. The only changes to land between
rounds 01 and 02 on origin are:

- `7ad827a` — `archive codex prompt: entity-list-sort-modified-opt-in round 01` (parent)
- This round-02 archive file + the round-01 outcome update,
  to be committed in a follow-up parent-only commit before
  re-dispatch.

If the starting tree is dirty for any other reason, **STOP
and report**.

## Outcome

Pending — will be updated after the Codex run.

---

<task>
Re-dispatch of round 01. Read the authoritative prompt in
full:

`/home/ubuntu/philharmonic-workspace/docs/codex-prompts/2026-05-20-0001-entity-list-sort-modified-opt-in-01.md`

The entire preamble (Motivation, References, Hard constraints,
Per-file scope, Shape (locked), Tests, Verification, Hand-off
shape, Codex report) plus the round-01 `<task>` block apply
verbatim to this round. Nothing is changed in the scope or
constraints.

If you encountered a runtime issue last round that prevented
any edits, mention it in your session summary so Claude can
diagnose. Otherwise proceed exactly per round 01's
specification.

Verification gate before declaring done (unchanged from
round 01):

- `./scripts/pre-landing.sh` clean (`=== pre-landing: all
  checks passed ===`).
- `./scripts/webui-build.sh --production` exit 0 (pre-
  existing webpack size warnings OK; no new TS errors).

Update the `## Outcome` section of **this** file (round 02)
before declaring done — not round 01's outcome, which is
already filled in with the "died without doing anything"
record.

Return the structured-output contract from round 01's
`<structured_output_contract>` block. Pay particular
attention to (3) cursor-shape diff, (5) the
latest_revision_timestamps SQL, (7) the paginate_items
consolidation outcome, (10) test coverage by name, (12)
`dist/` regeneration confirmation, and (17) the
`## Outcome` paragraph ready for Claude to paste into
this round-02 archive.
</task>
