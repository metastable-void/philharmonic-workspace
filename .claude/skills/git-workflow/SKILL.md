---
name: git-workflow
description: Use whenever any Git operation is about to run against the philharmonic-workspace repo (status, pull/fetch, add, commit, push, submodule update). All Git work in this workspace MUST go through scripts/*.sh, not raw git, because the scripts encode submodule-first ordering and the mandatory signoff rule. Trigger on intent ("commit the docs", "push my changes", "pull latest", "what's dirty?") even before the user mentions git explicitly.
---

# Philharmonic workspace Git workflow

This workspace is a parent Git repo containing ~23 submodules, one per
crate. Wrong ordering (parent pushed before submodule) produces a
pointer origin can't resolve; missing `-s` signoff violates the DCO
rule every repo in the family enforces. The `scripts/*.sh` helpers
exist so neither mistake is possible. Use them.

Authoritative sources (read these if anything below is unclear):
- `CLAUDE.md` → "Git workflow" bullet.
- `ROADMAP.md` §2 "Submodule discipline", "Git via scripts",
  "Every commit is signed off".
- `docs/design/13-conventions.md` §"Git workflow".

## Rules (non-negotiable)

1. **Never run raw `git commit` or `git push`** in this workspace —
   parent or submodule. The scripts encode submodule-first ordering,
   signoff (`-s`), and detached-HEAD guards that ad-hoc commands skip.
2. **Every commit is signed off** (`Signed-off-by:` trailer). The
   scripts pass `-s`; don't bypass.
3. **Submodule commits land before the parent bumps their pointer.**
   `commit-all.sh` and `push-all.sh` walk submodules first on purpose.
   Reversing the order produces an unresolvable parent pointer on
   origin.
4. **If a script doesn't cover your case, extend the script first**
   (and update `docs/design/13-conventions.md §Git workflow`) rather
   than reaching for raw git. This is the whole point of the rule.
5. **Read-only inspection is fine** with raw git (`git log`,
   `git diff`, `git show`, `git rev-parse`, `git branch`,
   `git submodule status`). The prohibition is on state-changing
   operations (`commit`, `push`, `add` outside what scripts do,
   `reset`, `rebase`, etc.).

## The scripts

All live in `scripts/` at the workspace root. Run from anywhere —
each script `cd`s to the workspace top level itself.

**Invoke by path, not by interpreter.** Always run
`./scripts/commit-all.sh "msg"`, never `bash scripts/commit-all.sh
"msg"` or `sh scripts/commit-all.sh "msg"`. The scripts are
POSIX-sh with `#!/bin/sh` shebangs (see
docs/design/13-conventions.md §Shell scripts); prefixing `bash`
silently forces bash and makes any introduced bashism "work" on
your machine while breaking on Alpine / FreeBSD / macOS.
Honoring the shebang is the entire point of the POSIX rule — so
let the shebang do its job.

### `scripts/status.sh`
Shows working-tree state of the parent and every submodule, plus
ahead/behind vs. upstream, plus a detached-HEAD warning. Use this
before committing or pushing to sanity-check what's about to land.

### `scripts/pull-all.sh`
Fetches the parent and updates each submodule to the tip of its
tracked remote branch (`git submodule update --remote --recursive`).
Prints status at the end. **Does not** commit the bumped submodule
pointers — that's `commit-all.sh`'s job.

### `scripts/commit-all.sh [--parent-only] [message]`
The only supported way to create commits here. Walks each submodule,
commits any dirty tree with `-s`, then commits the parent (which now
includes the bumped pointers).

- Default message is `"updates"`. Pass a real one:
  `scripts/commit-all.sh "extract mechanics-config types"`.
- `--parent-only` skips the submodule walk. Use when the parent has
  its own pending work (docs, scripts, ROADMAP tweaks) that should
  land independently of whatever the submodules are doing — e.g.
  while Codex has in-progress uncommitted work in a submodule you
  don't want to commit yet.
- Refuses to commit in a submodule that's in detached HEAD with
  changes (that commit would be orphaned). If you hit this, checkout
  a branch inside the submodule and rerun.
- The message is passed via a tempfile so special characters are safe
  — no escaping gymnastics needed.

### `scripts/push-all.sh`
Pushes each submodule's current branch (`git push -u origin
<branch>`), then pushes the parent. Submodule push failures abort
before the parent is pushed, so origin never gets a parent pointer
referencing an unpushed submodule commit. Detached-HEAD submodules
are skipped with a warning (normal right after `git submodule
update`).

## Decision tree

```
Want to see state?               → scripts/status.sh
Want latest from origin?         → scripts/pull-all.sh
Have changes to commit?
  - submodules + parent together → scripts/commit-all.sh "msg"
  - parent only (docs, scripts)  → scripts/commit-all.sh --parent-only "msg"
Ready to share?                  → scripts/push-all.sh
Inspecting history?              → raw git log/diff/show is fine
Need something the scripts don't do?
                                 → extend the script, THEN use it
```

## Common failure modes

- **"I just need to amend quickly"** — no. Amend drops the signoff
  unless you pass `-s` again, and the scripts don't amend. Make a new
  commit via `commit-all.sh`; history stays honest.
- **"The parent push failed: some submodule commit is missing on
  remote"** — that's `push.recurseSubmodules=check` working as
  designed. Push the submodule (or rerun `push-all.sh` from scratch),
  then retry.
- **"`commit-all.sh` refused: detached HEAD"** — `cd` into the
  named submodule, `git checkout <branch>` (or create one), rerun.
- **"I want to commit only one submodule"** — run `commit-all.sh`
  with the other submodules clean. It only commits submodules that
  are dirty, so a targeted `git add` inside the one submodule
  followed by `commit-all.sh "msg"` does exactly that.

## Setup

If this is a fresh clone and scripts are failing because submodules
aren't initialized, run `scripts/setup.sh` once — it initializes all
submodules recursively and warns if the Rust toolchain is missing.
After that, the helpers above work.
