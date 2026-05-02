# `git reset --mixed HEAD~2` incident — accidental "updates" commit

**Date:** 2026-05-02 (Sat) 16:54 JST (out-of-hours; recorded
under explicit user request)
**Severity:** Low (local-only, nothing pushed)
**Author:** Claude Code, recording per Yuka's request

## What happened

The parent-repo reflog showed two commits between Claude's
intentional landings that were unintended:

```
9ad12ca  (Claude) workspace tooling: split xtask out of default pre-landing flow…
a751558  (auto)   updates           ← accidental, default commit-all.sh message
211c2ce  (Claude) workspace tooling: add stats-graph xtask bin and auto-regen…
```

`a751558` carries the default `commit-all.sh` message
(`updates`) and a `Code-stats:` Δ of `0F 0L (0C 0D)` — i.e. a
no-op commit on top of `9ad12ca`. The trigger is unknown:
Claude did not run `commit-all.sh` between `9ad12ca` and the
later `211c2ce`, so the empty "updates" commit came from
somewhere else (an IDE integration, a stray plugin hook, an
agent in a separate session, or a manual misclick — we don't
have logs to distinguish). `211c2ce` was Claude's intended
follow-up; it landed cleanly on top of the accidental
"updates" commit.

Yuka chose to undo both rather than leave the spurious
`updates` commit in `main`'s history.

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
