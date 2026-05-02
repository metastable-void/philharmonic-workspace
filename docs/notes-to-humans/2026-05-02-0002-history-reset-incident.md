# `git reset --mixed HEAD~2` incident — accidental "updates" commit

**Date:** 2026-05-02 (Sat) 16:54 JST (out-of-hours; recorded
under explicit user request)
**Severity:** Low (local-only, nothing pushed)
**Author:** Claude Code, recording per Yuka's request

## What happened

Spurious `updates`-message commits landed across four repos —
the parent and the three target submodules of the in-flight
D1/D2/D10 Codex dispatch. Reflogs after the cleanup:

**Parent**:
```
9ad12ca  (Claude) workspace tooling: split xtask out of default pre-landing flow…
a751558  (Codex)  updates           ← accidental, default commit-all.sh message
211c2ce  (Claude) workspace tooling: add stats-graph xtask bin and auto-regen…
```

**Each of `philharmonic-store-sqlx-mysql`, `mechanics-core`,
`philharmonic`** (D1, D2, D10 targets):
```
HEAD@{2}: <pre-dispatch HEAD>
HEAD@{1}: commit: updates           ← Codex's default-message commit
HEAD@{0}: reset: moving to HEAD~1   ← Yuka's reset
```

**Two separate causes** converged on the same `updates`
default-message symptom — Yuka attributes the parent commit
and the submodule commits to different actors:

- **Parent `a751558`** carried a `Code-stats:` Δ of
  `0F 0L (0C 0D)` — a no-op commit on top of `9ad12ca`, with
  no substantive change — and most likely came from a Claude
  testing invocation of `commit-all.sh` somewhere between
  `9ad12ca` and `211c2ce`. The default message means Claude
  ran `commit-all.sh --parent-only` (or similar) without a
  message argument, so the script defaulted to `"updates"`.
  This was a Claude-side mistake; future testing must always
  use a real message arg or stay in `--dry-run`.
- **Per-submodule "updates" commits** carried real D1 / D2 /
  D10 content from the in-flight Codex dispatch. Codex
  violated `AGENTS.md`'s "Don't commit, don't push, don't
  branch" rule when producing those commits. Codex's later
  structured-output report referenced the (now-rolled-back)
  SHAs as if they still existed, indicating the runtime
  didn't observe the reset.

`211c2ce` was Claude's intended substantive follow-up at the
parent level; it landed cleanly on top of `a751558`. After
the cleanup the in-flight Codex work (the actual D1 / D2 /
D10 edits) was preserved in each submodule's working tree by
the `--mixed` reset and remained available for re-commit
through proper channels.

Yuka chose to undo every spurious `updates` commit rather
than leave them in any repo's history.

## Action taken (one-time exception to §4.4)

```sh
git reset --mixed HEAD~2
```

This rewinds the parent's HEAD pointer two commits (back to
`9ad12ca`) and unstages the index, leaving the working tree
untouched. After the reset the working tree still contained
all the changes from `a751558` (none of substance) and
`211c2ce` (the stats-graph bin, `update-stats-graph.sh`,
`commit-all.sh` hookup, `docs/stats.svg`, `docs/README.md`
embed, etc.). Those changes were re-committed under a fresh
intentional message rather than re-using `211c2ce`'s SHA.

## Why this is an exception, not a precedent

[`CONTRIBUTING.md §4.4`](../../CONTRIBUTING.md#44-no-history-modification)
forbids history modification (no amend, no rebase, no reset,
no force-push, no `git revert` either) and lists exactly two
narrow script-enforced exceptions:

1. The `post-commit` unsigned-rollback inside
   `commit-all.sh`.
2. `pull-all.sh --rebase`'s rebase-on-pull.

`git reset --mixed HEAD~2` is **not** one of those. It was
permitted here by Yuka under three concurrent conditions:

- **Nothing was pushed yet.** `9ad12ca`, `a751558`, and
  `211c2ce` were all local; the reset rewinds local state
  only. No other clone of the workspace, no GitHub
  ruleset, no CI run, no contributor was affected.
- **The unwanted commit was an empty default-message commit
  with no substantive change** (Δ 0F 0L). Fix-forward via a
  new commit would have left a confusingly-named no-op in
  `main`'s permanent history, which the §4.4 fix-forward
  guidance optimises against the ability to clean up local
  state before publishing.
- **The wanted commit's content was preserved** in the
  working tree (`--mixed`, not `--hard`), so re-creating it
  under a clean message was straightforward and the audit
  trail of "what landed" is unaffected.

The above is the bar for any future invocation: pushed
commits cannot be reset, period; non-empty commits should
fix-forward via new commits per §4.4. Not a standing
exception; not a documented precedent that future agents may
cite. Each future occurrence requires Yuka's explicit
approval at the moment.

## What's not yet known

- **Root cause of the `updates` commit.** Worth investigating
  before this happens again. Candidate sources to check:
  - IDE integrations (VSCode extension auto-commit?).
  - Other Claude Code or Codex sessions running concurrently
    in the same workspace.
  - Stray hooks or plugins that wrap or duplicate
    `commit-all.sh`.
  - A misclicked invocation of `commit-all.sh` with no
    arguments by Yuka or an agent.

- **Whether the `Code-stats:` Δ-0 detector is loud enough.**
  An empty commit with `Δ 0F 0L` in `stats-log.sh` output is
  visible but easy to miss. If accidental empty commits
  happen again, a `commit-all.sh`-time refusal of empty
  commits ("nothing changed; aborting") would be a cheap
  preventive measure — `git diff --cached --quiet` after
  `git add -A` already returns true in that case, and the
  recently-added `--exclude`-empty-commit-bail uses exactly
  that pattern.

## Recommended follow-ups

1. **Investigate** what created `a751558`. If reproducible,
   patch the trigger.
2. **Consider** extending `commit-all.sh` to refuse a
   parent commit when `git add -A` produced no staged diff
   (mirroring the `--exclude`-emptied case). That would
   convert this class of incident into a hard error
   regardless of trigger.
3. **Do not** assume future incidents may be reset under
   the same ad-hoc allowance. Per §4.4, default to
   fix-forward; ask Yuka before any local rewrite.
