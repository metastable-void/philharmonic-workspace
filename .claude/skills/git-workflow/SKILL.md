---
name: git-workflow
description: Use whenever any Git operation is about to run against the philharmonic-workspace repo (status, pull/fetch, add, commit, push, submodule update). All Git work in this workspace MUST go through scripts/*.sh, not raw git, because the scripts encode submodule-first ordering and the mandatory signoff rule. Trigger on intent ("commit the docs", "push my changes", "pull latest", "what's dirty?") even before the user mentions git explicitly.
---

# Philharmonic workspace Git workflow

This workspace is a parent Git repo containing ~23 submodules, one per
crate. Wrong ordering (parent pushed before submodule) produces a
pointer origin can't resolve; missing `-s` signoff violates the DCO
rule every repo in the family enforces; an unsigned commit breaks the
GPG/SSH-signature requirement. The `scripts/*.sh` helpers exist so
none of those mistakes are possible. Use them.

Authoritative sources (read these if anything below is unclear):
- `CLAUDE.md` → "Git workflow" bullet.
- `ROADMAP.md` §2 "Submodule discipline", "Git via scripts",
  "Every commit is signed off".
- `docs/design/13-conventions.md` §"Git workflow".

## Rules (non-negotiable)

1. **Never run raw `git commit` or `git push`** in this workspace —
   parent or submodule. The scripts encode submodule-first ordering,
   signoff (`-s`), signing (`-S`), and detached-HEAD guards that
   ad-hoc commands skip.
2. **Every commit is signed off *and* signed** (`Signed-off-by:`
   trailer via `-s`, plus a GPG or SSH signature via `-S`). The
   scripts pass both flags and verify the signature with
   `git log --format=%G?` after each commit; an unsigned commit
   is rolled back with `git reset --soft HEAD~1` and the script
   aborts. Don't bypass.
3. **Submodule commits land before the parent bumps their pointer.**
   `commit-all.sh` and `push-all.sh` walk submodules first on purpose.
   Reversing the order produces an unresolvable parent pointer on
   origin.
4. **If a script doesn't cover your case, extend the script first**
   (and update `docs/design/13-conventions.md §Git workflow`) rather
   than reaching for raw git. This is the whole point of the rule.
5. **Read-only inspection is fine** with raw git for history
   browsing (`git log`, `git diff`, `git show`, `git rev-parse`,
   `git branch`, `git submodule status`). The prohibition is on
   state-changing operations (`commit`, `push`, `add` outside
   what scripts do, `reset`, `rebase`, etc.). **Exception:** do
   *not* use raw `git log -n 1` to check current HEAD state
   across the workspace — use `./scripts/heads.sh`. It walks all
   24 repos (parent + submodules) in one call with the canonical
   SHA-sig-subject format; raw `git log -n 1` would need 24
   invocations to match and drifts in format between them.

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
Pushes each submodule's current branch (`git push --follow-tags
origin <branch>`), then pushes the parent. Submodule push failures
abort before the parent is pushed, so origin never gets a parent
pointer referencing an unpushed submodule commit. Detached-HEAD
submodules are skipped with a warning (normal right after
`git submodule update`). `--follow-tags` means release tags
created by `publish-crate.sh` ship along with their branch commit.

### `scripts/test-scripts.sh`
POSIX-parse-checks every `scripts/*.sh` with `dash -n` (fallback
`sh -n`). **Mandatory after any change under `scripts/`** before
committing. GitHub CI runs the same check.

### `scripts/check-detached.sh`
Fails with a non-zero exit if any submodule is in detached HEAD,
listing the offenders. Useful pre-flight before `commit-all.sh`,
`publish-crate.sh`, or any multi-commit operation.

### `scripts/show-dirty.sh`
Prints dirty-submodule names, one per line; empty output when
nothing's dirty. Machine-readable counterpart to `status.sh`;
used internally by `pre-landing.sh`.

### `scripts/heads.sh`
Shows the current HEAD commit for the parent and every submodule,
with short SHA, `%G?` signature indicator, and subject. Use after
`commit-all.sh` / `push-all.sh` to sanity-check what landed and
to verify every commit carries a cryptographic signature (`G` =
good, `U` = good+untrusted; anything else on a pushed commit is a
problem). Canonical replacement for ad-hoc `git log -n 1` across
repos — do not invoke raw `git log -n 1` for HEAD-state queries.

### `scripts/check-api-breakage.sh <crate> [<baseline-version>]`
Runs `cargo semver-checks check-release -p <crate>` against a
crates.io baseline (latest non-yanked by default, or the version
you pass). Per-crate, not `--workspace`, because this workspace
is a virtual-workspace parent + submodules and the
git-clone-based baseline modes don't resolve submodule members.
Installs `cargo-semver-checks` on first use. Use before preparing
a crate release.

### `scripts/publish-crate.sh [--dry-run] <crate>`
Publishes one crate to crates.io (via `cargo publish`) and tags
the release `v<version>` inside the submodule repo. Tag is
signed, created only after a successful publish. The tag is then
pushed by the next `push-all.sh` via `--follow-tags`.

### `scripts/cargo-audit.sh [...]`
Runs `cargo audit` against the workspace `Cargo.lock`;
auto-installs `cargo-audit` via `cargo install --locked` on
first use. Run alongside `check-api-breakage.sh` when preparing
a release.

### `scripts/crate-version.sh <crate> | --all`
Prints a crate's version string parsed from its own `Cargo.toml`.
Internal helper; used by `publish-crate.sh`. Pass `--all` instead
of a crate name to list every submodule's name and version in one
aligned pass — handy when preparing a multi-crate release.

### `scripts/crates-io-versions.sh <crate>`
Lists the published (non-yanked) versions of a crate on crates.io
by querying the sparse index directly. Complements
`crate-version.sh`: local vs. already-published state.
**Requires `curl` and `jq`** — not part of the workspace baseline;
the script bails with a clear message if either is missing.

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
