# Contributing to Philharmonic

Development conventions for [metastable-void/philharmonic-workspace](https://github.com/metastable-void/philharmonic-workspace)
and every crate in its family. Applies to everyone — humans and
coding agents (Claude Code, Codex) — committing against any
repository under the `metastable-void/` org that's a member of
this workspace (parent workspace + every submodule).

This is the **single authoritative home** for workspace-level
conventions. When a convention is disputed or unclear, the rule
here wins; every other doc (per-repo `README.md`, `CLAUDE.md`,
`AGENTS.md`, design docs) links back here rather than restating.

Related authoritative docs that stay as their own homes:
- [`ROADMAP.md`](ROADMAP.md) — linear plan (what to work on next).
- [`docs/design/`](docs/design/) — architectural design docs
  (what Philharmonic *is*, not how to contribute to it).
- [`POSIX_CHECKLIST.md`](POSIX_CHECKLIST.md) — POSIX-shell
  portability reference.

Quick navigation (major sections):

- [1. Quick start](#1-quick-start)
- [2. Development environment](#2-development-environment)
- [3. Repository structure](#3-repository-structure)
- [4. Git workflow](#4-git-workflow)
- [5. Script wrappers over raw `cargo`](#5-script-wrappers-over-raw-cargo)
- [6. Shell script rules (POSIX sh)](#6-shell-script-rules-posix-sh)
- [7. External tool wrappers](#7-external-tool-wrappers)
- [8. In-tree workspace tooling (`xtask/`)](#8-in-tree-workspace-tooling-xtask)
- [9. KIND UUID generation](#9-kind-uuid-generation)
- [10. Rust code conventions](#10-rust-code-conventions)
- [11. Pre-landing checks](#11-pre-landing-checks)
- [12. Versioning and releases](#12-versioning-and-releases)
- [13. Licensing](#13-licensing)
- [14. Naming and terminology](#14-naming-and-terminology)
- [15. Journal-like files](#15-journal-like-files)
- [16. ROADMAP maintenance](#16-roadmap-maintenance)
- [17. Conventions-about-conventions](#17-conventions-about-conventions)

---

## 1. Quick start

Fresh clone (with submodules):

```sh
git clone --recurse-submodules https://github.com/metastable-void/philharmonic-workspace.git
cd philharmonic-workspace
./scripts/setup.sh
```

`setup.sh` is idempotent. It initialises every submodule
recursively and configures, on the parent and every submodule:

- `push.recurseSubmodules=check` — required for `push-all.sh`'s
  safety rail.
- `core.hooksPath` pointing at the repo-tracked [`.githooks/`](.githooks/)
  (absolute on the parent; relative inside each submodule,
  computed via `scripts/lib/relpath.sh`).
- `commit.gpgsign=true`, `tag.gpgsign=true`, `rebase.gpgsign=true`.

It also ensures a stable Rust toolchain + nightly + miri are
installed (via `rustup`), and warns otherwise.

**Sign-key prerequisite.** Git signing must work on your clone
before you can commit. Configure once:

```sh
git config --global user.signingkey <key>
# optional — for SSH-signing instead of GPG:
git config --global gpg.format ssh
```

The scripts will refuse to land unsigned commits; the hooks will
refuse to push unsigned ones. If a commit attempt fails with a
signing error, resolve that before trying again.

After setup, the typical flow for any change:

1. Edit files across whichever submodules the change spans.
   Leave the working tree dirty; don't manually `git add` /
   `git commit`.
2. `./scripts/pre-landing.sh` — fmt + check + clippy (`-D
   warnings`) + test on every modified crate.
3. `./scripts/commit-all.sh "message"` — submodules first, then
   the parent bumps pointers.
4. `./scripts/push-all.sh` — submodules first, then the parent.

Read §4 (Git workflow) before your first commit. Read §10 (Rust
code conventions) before your first Rust edit. Everything else
can be skimmed and referenced as needed.

---

## 2. Development environment

This workspace targets **POSIX-ish hosts** for development. The
`scripts/*.sh` dispatcher is POSIX sh (`#!/bin/sh`), depends on
SUSv4-baseline utilities (`awk`, `sed`, `grep`, `cut`, `tr`),
and assumes POSIX file permissions, signal semantics, and process
semantics. Individual rules (shell-script portability, the
`mktemp` / `web-fetch` wrappers, the `xtask` Rust-bin escape
hatch for `jq` / `curl`) are downstream consequences of this
single baseline assumption.

### Supported dev platforms

- **GNU/Linux**, any distribution, any arch (x86_64, aarch64,
  etc.) — the primary development target.
- **WSL2 on Microsoft Windows** — `uname -s` reports `Linux`;
  behaves as GNU/Linux from the workspace's perspective. The
  supported way to develop on Windows hardware.
- **macOS (Darwin)** — POSIX-certified; the scripts work. HTTP
  fetching and sparse-index querying moved to `xtask/` Rust bins
  so stripped macOS installs (which lack `curl` / `jq` by
  default) need no extra tooling.
- **BSD family** — FreeBSD, OpenBSD, NetBSD, DragonFlyBSD.
  Covered by the POSIX-sh discipline; explicit deviations are
  tracked in §6.
- **illumos / Solaris** — POSIX-ish; should work, less
  exercised.
- **Alpine and other musl-based distros** — supported (busybox
  `ps` / `sh` variants handled by §6).

### Unsupported dev platforms

- **Raw (non-WSL) Microsoft Windows.** No cmd.exe, no
  PowerShell. The scripts won't even execute — `#!/bin/sh` isn't
  honored there — and even if POSIX sh were bootstrapped in,
  submodule permission handling and the signing / audit-trailer
  tooling assume a POSIX host.
- **Git Bash / MSYS2 / Cygwin.** Read-only browsing may work;
  state-changing operations (submodule ordering,
  `commit-all.sh`, `push-all.sh`) are fragile. Use WSL2.

### Agent rule (Claude Code, Codex)

**The docs are the gate.** Raw Windows can't run `#!/bin/sh`, so
a runtime detection inside a script never fires on the platforms
it would gate. AI agents doing development in this repo MUST
verify the host is POSIX-ish **before** running scripts, spawning
sub-agents, or attempting Git state changes. The check is
trivial — the environment block surfaced at session start
reports `Platform: linux` / `darwin` / `freebsd` / etc. on
supported hosts, and `Platform: win32` on raw Windows.

- On a supported POSIX-ish host: proceed normally.
- On **raw Windows** (`Platform: win32`): stop immediately.
  Surface the mismatch to the human: "This workspace is
  POSIX-sh-based; raw Windows is not supported. Use WSL2 to
  develop in this repo." Do not attempt Git operations, do not
  run the scripts, do not spawn Codex.
- On Git Bash / MSYS / Cygwin: proceed with caution; flag any
  submodule, signing, or file-permission anomaly and escalate.

---

## 3. Repository structure

One-crate-per-repo under `github.com/metastable-void/*`. Each
published crate has its own issue tracker, CI, release cycle,
`README.md`, and `CHANGELOG.md`.

`philharmonic-workspace` is the parent (the "meta-repo"):

- `[workspace] members` in `Cargo.toml` listing every crate.
- `scripts/` — development scripts (authoritative wrappers; see §5, §6).
- `xtask/` — in-tree non-submodule Rust dev tooling (see §8).
- `.githooks/` — tracked Git hooks installed via `core.hooksPath` (see §4).
- `docs/` — design docs, journal directories (see §15).
- `CONTRIBUTING.md` (this file), `ROADMAP.md`, `README.md`,
  `CLAUDE.md`, `AGENTS.md`, `HUMANS.md`, `POSIX_CHECKLIST.md`.

Each submodule is a standalone single-crate repo. Inside a
submodule: `Cargo.toml`, `README.md`, `CHANGELOG.md`,
`LICENSE-APACHE`, `LICENSE-MPL`, `src/`, `tests/`. Cross-crate
refactors require coordinated commits across repos;
cornerstone-versioning discipline absorbs most of this.

---

## 4. Git workflow

**All Git state-changing operations go through `scripts/*.sh`,
not raw `git`.** The scripts encode submodule-first ordering,
mandatory `-s` sign-off, mandatory `-S` signature, detached-HEAD
guards, and the audit-trailer workflow. Ad-hoc commands drift
from those defaults. Read-only git (`git log`, `git diff`,
`git show`, `git status`, `git blame`, `git rev-parse`) is fine
for history browsing; the prohibition is on state changes.

### 4.1 The scripts

- `setup.sh` — one-time (or post-fresh-clone) init.
  Initialises every submodule recursively; configures the parent
  and every submodule with `push.recurseSubmodules=check`,
  `core.hooksPath` pointing at `.githooks/` (relative path in
  submodules, computed via `scripts/lib/relpath.sh`),
  `commit.gpgsign=true`, `tag.gpgsign=true`, and
  `rebase.gpgsign=true`; warns if Rust isn't on PATH.
- `status.sh` — working-tree status of the parent + every
  submodule (clean submodules are hidden).
- `pull-all.sh` — rebase-pull the parent and update each
  submodule to the tip of its tracked remote branch. Does *not*
  commit bumped pointers. See §4.4 for the rebase-on-pull
  exception.
- `commit-all.sh [--anonymize] [--parent-only] [message]` —
  commit pending changes. Walks each dirty submodule first
  (committing with `-s -S`), then the parent. Default message
  is `"updates"`. `--parent-only` skips the submodule walk for
  parent-only work (docs, scripts, ROADMAP).
- `push-all.sh` — push each submodule's current branch, then
  the parent. Aborts before the parent push if any submodule
  push fails, so origin never sees an unresolvable parent
  pointer.
- `heads.sh` — short-SHA / `%G?` / subject for parent +
  every submodule in one pass. Canonical replacement for raw
  `git log -n 1` across repos.
- `check-detached.sh` — fails non-zero if any submodule is in
  detached HEAD. Pre-flight for `commit-all.sh`.
- `show-dirty.sh` — one-per-line list of dirty submodule names.

**Invoke by path, not by interpreter.** Run
`./scripts/commit-all.sh "msg"`, never `bash scripts/commit-all.sh`
or `sh scripts/commit-all.sh`. Prefixing `bash` silently forces
bash and hides any bashism that would break on Alpine / FreeBSD
/ macOS. The shebang is the whole point of the POSIX rule.

### 4.2 `commit-all.sh` sweeps all dirty parent files

Internally it runs `git add -A` before `git commit`, so
pre-staging a subset with `git add` does not scope the commit —
selective staging is meaningless against this script. When the
parent has unrelated dirty files you want to keep out of the
commit you're about to make, **clean them out of the tree first**:
move them to `/tmp` and restore after, or commit them
separately in a prior `--parent-only` invocation. The motivation
is that the script's contract is "commit everything dirty,
correctly, with every required invariant" — a selective path
would need different tooling, and if you find yourself wanting
it often, extend the script rather than working around it.

### 4.3 Every commit is signed off *and* cryptographically signed

The scripts pass both `-s` (DCO signoff, adds `Signed-off-by:`
trailer) and `-S` (GPG or SSH signature) to `git commit`.
`setup.sh` additionally sets `commit.gpgsign=true`,
`tag.gpgsign=true`, and `rebase.gpgsign=true` on the parent and
every submodule, so signing is picked up even when somebody
reaches around the wrapper, and rebase-replayed commits
(§4.4 exception) sign too. `commit-all.sh` verifies the
resulting HEAD with `git log --format=%G?` after every commit
it makes — if the commit somehow lacks a signature, it is
rolled back with `git reset --soft HEAD~1` and the script
aborts.

Why `rebase.gpgsign` separately from `commit.gpgsign`: `git
rebase` does not honour `commit.gpgsign` for commits it replays;
the documented control for rebase-replayed signing is
`rebase.gpgsign`. Without it, `pull-all.sh`'s rebase-on-pull
would produce unsigned commits that `.githooks/post-commit`
would roll back mid-rebase, breaking the exception path. Setting
both keys is what makes the exception usable.

### 4.4 No history modification

**Git history in this workspace is append-only.** No
`git commit --amend`, no `git rebase` (interactive or
otherwise), no `git reset --hard`, no `git reset --mixed` /
`--soft` except the narrow exceptions below, no `git push
--force` / `--force-with-lease`, no `git filter-branch` /
`git-filter-repo`, no `git cherry-pick` that drops intervening
commits, and no branch resets that discard unique commits. The
rule applies to the parent repo and every submodule, to
published and unpublished branches alike.

**Two narrow exceptions**, both script-enforced and both bounded
to local not-yet-pushed commits:

1. **Unsigned-commit rollback.** The `git reset --soft HEAD~1`
   that `.githooks/post-commit` and `commit-all.sh` both
   perform when a just-recorded commit violates the signature
   invariant. Both preserve the working tree (staged changes
   kept); both only touch the immediately-preceding commit on
   the local branch; both only fire while that commit has not
   been pushed.

2. **Rebase-on-pull in `scripts/pull-all.sh`.** `git pull
   --rebase` on the parent and `git submodule update --remote
   --rebase --recursive` on every submodule. In the typical
   case (no local commits ahead of upstream) this is a no-op
   or fast-forward; in the uncommon case (local commits ahead
   *and* upstream moved forward), it replays the local commits
   on top of the new upstream tip. Only local, not-yet-pushed
   commits are affected; `commit.gpgsign=true` and
   `rebase.gpgsign=true` mean replayed commits re-sign; the
   commit messages (including `Audit-Info:` and
   `Signed-off-by:` trailers) are preserved verbatim;
   author-date is preserved, committer-date is refreshed. The
   alternatives examined — `--ff-only` (fails in the uncommon
   case, no wrapper-friendly recovery path), default merge
   (produces merge commits, requires pre-commit hook wiring),
   default submodule checkout (detaches HEAD, breaks
   `commit-all.sh`'s detached-HEAD guard) — each violate other
   workspace invariants. Do not run `git pull --rebase` or
   `git rebase` outside `pull-all.sh`; the exception is for the
   script, not for the subcommand.

Nothing outside these two cases is authorised to rewrite history
— not amend, not rebase, not reset, not anything else,
regardless of intent. A future need for another script-enforced
history operation has to live in a wrapper script and be
documented here; ad-hoc history edits are not how new
exceptions get introduced.

**Why append-only:**

- Every commit carries a GPG/SSH signature; amending rewrites
  the commit object and the original signature becomes
  meaningless. The signed audit trail stops being an audit
  trail.
- Every commit carries an `Audit-Info:` trailer recording the
  environment that produced it. Amend would rewrite the trailer
  in place; the whole point is that it cannot be.
- Force-pushing a rewritten history through ~23 submodules
  means every other clone has to untangle itself; the cost is
  not yours alone.
- Mistakes ship as **new commits** — a fix-forward, or a
  `git revert` (which itself creates a new commit). History
  stays honest.

**If you need a mistake undone:**

- *Uncommitted working-tree changes*: edit and make a new
  commit via `commit-all.sh`.
- *Most recent commit, not yet pushed, and you want to tweak
  the code*: commit the tweak as the next commit. Do **not**
  amend.
- *Most recent commit, not yet pushed, got rolled back by
  post-commit or commit-all.sh*: the working tree is preserved;
  rerun `./scripts/commit-all.sh "msg"`. The hook's abort
  message spells out the exact retry path.
- *Any commit that has been pushed, regardless of how recent*:
  `git revert <sha>` produces a new commit that undoes it.
  That's the only supported undo for a commit that's reached
  origin.

If you find yourself reaching for amend / rebase / reset for a
"legitimate" reason that isn't in the list above, stop and
surface it. This rule has no quiet exceptions.

### 4.5 Tracked Git hooks

`setup.sh` points `core.hooksPath` at [`.githooks/`](.githooks/)
on the parent and at the relative-from-the-submodule equivalent
in every submodule, so the same four hooks run everywhere:

- **`.githooks/pre-commit`** — refuses any commit whose
  invocation didn't set `WORKSPACE_GIT_WRAPPER=1`. Only
  `commit-all.sh` exports that env var. The hook message points
  offenders at the wrapper and documents the `--no-verify`
  escape hatch for legitimate one-off cases.
- **`.githooks/commit-msg`** — refuses any commit whose message
  doesn't carry `Signed-off-by: <name> <email>` matching the
  committer identity. Merges, reverts, fixups, and empty
  messages are exempt. `commit-all.sh` always passes `-s`; this
  hook catches stray `git commit -m ...` invocations that
  bypassed the wrapper.
- **`.githooks/post-commit`** — if the just-recorded commit has
  no valid GPG/SSH signature (`%G?` ∉ `{G, U}`), rolls back
  with `git reset --soft HEAD~1`, preserves the staged tree,
  and saves the original message to `.git/UNSIGNED_COMMIT_MSG`
  so the user can re-commit cleanly. Defence-in-depth for the
  case where signing got bypassed explicitly (e.g.
  `--no-gpg-sign`). The abort message points at
  `scripts/commit-all.sh`, with a raw-git retry path documented
  as a **fresh** commit (not an amend — §4.4 forbids amend).
- **`.githooks/pre-push`** — final backstop before commits
  leave the machine. Walks every ref-update in the push and,
  for each new commit, rejects it if the GPG/SSH signature
  status is not `G`/`U`, or if the message lacks a plausible
  `Signed-off-by:` trailer (matching `commit-msg`'s
  `Merge`/`fixup!`/`squash!`/`Revert` exemptions). Redundant
  with `commit-msg` + `post-commit` for the normal
  `commit-all.sh` flow; its value is catching commits that
  bypassed those (`--no-verify` at commit time, imports via
  `cherry-pick`/`merge`, tool-produced commits). The abort
  message does **not** offer amend or interactive-rebase fixes
  (§4.4). A narrow escape hatch — `git config
  hooks.allowUnsignedPush true`, push, unset — is preserved for
  genuine emergencies.

Together: pre-commit says "go through the wrapper", commit-msg
says "carry a sign-off", post-commit says "rollback if
unsigned", pre-push says "don't let anything that bypassed the
first three leave the machine" — all four are workspace-wide
invariants that used to live only in the wrapper scripts. Don't
edit these hooks ad hoc; if a new invariant needs to be
enforced, change the `.githooks/*` file in a normal
`commit-all.sh` commit and it lands for every contributor on
their next `setup.sh` run.

### 4.6 Audit-Info trailer

Every commit produced by `commit-all.sh` carries an
`Audit-Info:` trailer (alongside `Signed-off-by:` and the
GPG/SSH signature) recording the environment that produced it —
timestamp, hostname, user/uid, group/gid, public IPv4+v6 with
geolocation (queried once per invocation from
`ipv4.icanhazip.com` / `ipv6.icanhazip.com`, not per submodule),
kernel/release, arch, OS. Produced by
`scripts/print-audit-info.sh`, parsed as a standard git trailer
(`git log --format='%(trailers)'`).

Pass `--anonymize` to `commit-all.sh` to replace the IPv4 and
IPv6 fields with `hidden/ZZ` while keeping the rest. Host, user,
kernel, and OS are always recorded — the audit line's purpose
is cross-checking "which machine produced this" against a
local-state map. It's not a substitute for the DCO or the
signature.

### 4.7 Other git rules

- **Don't invoke `git log -n 1`** to list HEAD state across the
  workspace — use `./scripts/heads.sh`. Raw `git log` remains
  fine for history browsing (`git log <path>`, `git log
  --oneline`, etc.); the rule targets specifically the "show
  current HEAD commit on each repo" pattern.
- **If a script doesn't cover your case, extend the script**
  (and update this file) rather than reaching for raw git. This
  is the whole point of the wrapper-only rule.

---

## 5. Script wrappers over raw `cargo`

**Rule: every `cargo` invocation with a `scripts/*.sh` wrapper
goes through the wrapper, not raw `cargo`.** Same principle as
the `scripts/*`-only git workflow: the wrappers are the single
source of truth for flag choices, ordering, install of optional
tools, and workspace-cd. Contributor-vs-CI parity is guaranteed
only *because* the wrappers are authoritative. Ad-hoc `cargo
<subcommand>` invocations drift quietly — a missing
`-D warnings`, a forgotten `--all-targets`, a workspace that
isn't at the expected CWD — and drift shows up as a CI failure
that a local run missed.

The inventory:

| Wrapper | Wraps | Notes |
|---|---|---|
| `./scripts/pre-landing.sh [<crate>...] [--no-ignored]` | `cargo fmt --check` + `cargo check` + `cargo clippy --all-targets -- -D warnings` + `cargo test --workspace` + `cargo test --ignored -p <crate>` per modified crate | The canonical pre-commit flow. Auto-detects modified crates via `show-dirty.sh`. CI runs the same script. See §11. |
| `./scripts/rust-lint.sh [<crate>]` | `cargo fmt --check` + `cargo check` + `cargo clippy --all-targets -- -D warnings` | Workspace (no arg) or per-crate. |
| `./scripts/rust-test.sh [--include-ignored\|--ignored] [<crate>]` | `cargo test` with ignored-test control | `--ignored` runs *only* `#[ignore]`-gated; `--include-ignored` runs everything. |
| `./scripts/miri-test.sh --workspace \| <crate>...` | `cargo +nightly miri test` | Slow; not in `pre-landing.sh`. See §10.7. |
| `./scripts/cargo-audit.sh [...]` | `cargo audit` | Auto-installs `cargo-audit` on first run. |
| `./scripts/check-api-breakage.sh <crate> [<baseline>]` | `cargo semver-checks check-release -p <crate> --baseline-version <ver>` | Per-crate; crates.io baseline (default: newest published). See §12.3. |
| `./scripts/publish-crate.sh [--dry-run] <crate>` | `cargo publish -p <crate>` + signed release tag | Enforces clean tree, branch-HEAD, no-existing-tag invariants. Tag created only on publish success. |
| `./scripts/crate-version.sh <crate> \| --all` | Parses `version = "..."` from `<crate>/Cargo.toml` | Single-crate for programmatic use; `--all` prints every workspace member's version. |
| `./scripts/xtask.sh crates-io-versions -- <crate>` | crates.io sparse-index query | Lists non-yanked published versions. Rust bin in `xtask/`. |
| `./scripts/xtask.sh <tool> -- <args>` | wrapper for in-tree Rust bins | Canonical invocation for any `xtask/` bin; mandatory `--` separates wrapper-level flags from bin args. |
| `./scripts/check-toolchain.sh [--update]` | `rustup check` / `rustup update` + version print | Step 0 of `pre-landing.sh`. |

**Exempt**: read-only cargo queries have no wrapper and don't
need one — `cargo tree`, `cargo metadata`, `cargo --version`,
`cargo search` are fine to run raw.

**If no wrapper fits**: extend one, or add a new `scripts/*.sh`
(see §6). Validate with `./scripts/test-scripts.sh`. Then use
the new wrapper — don't fall back to raw cargo because the
wrapper doesn't exist yet.

### 5.1 Crate version lookup

**Rule: never recall a crate's published version from memory.**
Every question about "what's on crates.io for crate X?" is
answered by:

```sh
./scripts/xtask.sh crates-io-versions -- <crate>
```

The wrapper queries the crates.io sparse index in-process (via
the `xtask/crates-io-versions` bin) and prints every non-yanked
published version. Applies to third-party crates (checking
whether a new `tokio` / `serde` / `sqlx` release exists before
bumping) *and* this workspace's own crates (confirming whether
`philharmonic-types 0.3.4` is already out before cutting
`0.3.5`).

Why the rule exists:

- **Model memory is stale.** Claude's and Codex's training data
  is months to years behind. A version remembered from training
  is almost certainly wrong for anything that released in the
  last year, and wrong in a way that's hard to notice — the
  remembered number *sounds* right.
- **Session memory is frozen in time.** A version confirmed
  three sessions ago may have been superseded, yanked, or
  replaced by a security patch since.
- **Echoing a remembered number is how wrong pins land.** Pins
  are checked into `Cargo.toml` and `Cargo.lock`; a pin to a
  non-existent version fails only at resolve time — after the
  commit lands and CI tries to build it.
- **The lookup is cheap.** One HTTP round-trip to the sparse
  index, no auth required, sub-second.

**Local version declarations (the "what we're about to publish"
number)** are separate and have their own wrapper:

```sh
./scripts/crate-version.sh <crate>       # single-crate (for scripting)
./scripts/crate-version.sh --all         # every workspace member
```

`crate-version.sh` parses the local `Cargo.toml`;
`crates-io-versions` queries crates.io. The two can legitimately
disagree (we declare `0.3.5` locally while crates.io has only up
to `0.3.4` because we're mid-release). Pick the wrapper that
matches the question. Neither answer should come from memory.

Applies to agents and humans equally. Humans forget versions
too, and the wrapper costs the same for either.

### 5.2 Extract routines into scripts

When you find yourself running the same command or command
sequence more than once or twice — especially multi-line
sequences with flags, `git submodule foreach` invocations, or
POSIX-compatibility guards — extract it into a `scripts/*.sh`
file. Rationale:

- Scripts are reviewable: the logic lands in diffs; ad-hoc
  commands live in chat scrollback and evaporate.
- Scripts are testable: runnable by humans, CI, and future
  sessions. Invariants encoded once stay encoded.
- Scripts are discoverable: newcomers and future collaborators
  see them in `scripts/`, not scattered across READMEs.
- Scripts capture flag choices (`-D warnings`, `--all-targets`,
  `--follow-tags`, `-S`) that otherwise drift between
  invocations.

The bar is low. A one-liner becomes a two-line script. Don't
let "it's just a small thing" justify keeping a recurring
pattern ad-hoc. After extracting: validate with
`./scripts/test-scripts.sh`, add it to the scripts list in
[`README.md`](README.md) and the `git-workflow` skill (if
git-related), and document any associated rule here or in
`CLAUDE.md`.

This rule applies primarily to Claude Code (orchestrating
across tasks, noticing repetition). Codex receives discrete
tasks and doesn't typically make extraction decisions — but if
Codex notices a pattern in its prompts that warrants a script,
flag it in the final summary so Claude can extract.

---

## 6. Shell script rules (POSIX sh)

All shell scripts in this workspace are **POSIX sh**
(`#!/bin/sh`), not bash. No bashisms.

- `set -eu`, not `set -euo pipefail`. `pipefail` isn't POSIX.
  Structure pipelines so a silent left-side failure isn't
  possible — e.g. `cmd | grep pat || true` instead of relying
  on pipefail to catch a broken `cmd`.
- No arrays (`arr=(a b c)`, `"${arr[@]}"`). Use newline- or
  space-separated strings and iterate with `for x in $var`.
- No `[[ ... ]]`, `=~`, `BASH_REMATCH`. Use `[ ... ]`, `case`,
  or pipe through `sed`/`awk`/`grep` for regex.
- No `<<<` herestrings, no `<(...)` process substitution, no
  `mapfile`/`readarray`. Use heredocs, temp files, or `while
  read` loops.
- No `${var:offset:length}` substring expansion. Use `printf
  '%.Ns'`, `cut -c`, or `expr substr`.
- No `${BASH_SOURCE[0]}`. Use `$0`; don't source these scripts.
- No `$'...'` ANSI-C quoting. Build escapes with `printf`
  (e.g. `BOLD=$(printf '\033[1m')`).
- No `local`. Namespace function-locals with a prefix
  (`_myfunc_pid`) if shadowing matters.
- No `pgrep`, no `/proc/$pid`, no `column -t`. Snapshot `ps
  -Aww -o ...` once and drive everything from the snapshot;
  print columns with `printf '%-Ns ...'`.

**Why.** The workspace is expected to work on Linux
distributions without bash installed (some minimal containers,
Alpine without `bash` in the image), and on FreeBSD/macOS where
`/proc` and procps-style utilities aren't guaranteed. Sticking
to POSIX sh + POSIX utilities means every script runs on every
platform without a "this one needs bash" asterisk.

**Busybox caveat.** Alpine-class busybox installations are
supported. *Extremely stripped* busybox builds (e.g. Ubuntu's
`/usr/bin/busybox` from `busybox-static`, which lacks `etime`,
`time`, `-p`) are out of scope — rescue/initramfs images aren't
a real target. When picking a `ps` field or flag, prefer ones
the Alpine build supports.

**Reference checklist**: see [`POSIX_CHECKLIST.md`](POSIX_CHECKLIST.md)
at the repo root for a detailed inventory of non-POSIX
constructs / utilities / flags to avoid (and the narrow set
that *are* in POSIX.1-2024 Issue 8).

### 6.1 Validate with `test-scripts.sh`

**Mandatory after any change under `scripts/` or `.githooks/`.**

```sh
./scripts/test-scripts.sh
```

Runs `dash -n` against every `.sh` under `scripts/` *and*
`scripts/lib/` (sourced helpers), falling back to `sh -n` if
dash isn't installed. CI runs the same script, so drift between
contributor and CI behaviour is impossible. Actual execution
under dash (the default `/bin/sh` on Debian/Ubuntu) is the
other half of the check — worth doing manually for scripts that
run state-changing logic.

### 6.2 Workspace-root resolution

Scripts that need to operate at the workspace root source
`scripts/lib/workspace-cd.sh`, which resolves the root with a
three-tier fallback — superproject of the current submodule,
else git toplevel, else the script's own `$0`-relative path —
and `cd`s there. Works whether the script is invoked from the
workspace root, from inside a submodule, or from outside any
git repo entirely. New scripts needing this behaviour should
source the helper rather than reimplementing the resolution
inline.

### 6.3 Explicit deviations from strict POSIX

Allowed and tracked here — add to the list when a new one is
introduced:

- **`ps -o rss=`** (`scripts/codex-status.sh`). POSIX mandates
  `vsz` but not `rss`. `rss` is supported identically on Linux
  procps, FreeBSD base ps, macOS BSD ps, and Alpine busybox,
  and matches what the user expects to see for process-memory
  summaries. Kept for output fidelity.

### 6.4 Noteworthy field choices (within POSIX)

- **`ps -o time=`, not `pcpu=`** (`scripts/codex-status.sh`).
  `time` (cumulative CPU time) is POSIX-mandated and present
  everywhere including Alpine busybox. `pcpu` / `%CPU` is not
  in busybox ps.
- **No `-w`/`-ww`.** Busybox ps rejects `-w`. macOS/BSD ps may
  truncate `args` without it, but `codex-status.sh` already
  truncates to 80 chars downstream.

---

## 7. External tool wrappers

**Rule: never call `mktemp`, `curl`, or `wget` directly from a
workspace script. Use the wrappers.** Portability: these tools
vary across Alpine/busybox, FreeBSD, OpenBSD, macOS, WSL, and
their flag surfaces aren't consistent (busybox wget rejects
`--show-progress`, busybox mktemp lacks some templates, OpenBSD
`ftp` speaks HTTP but has different flags). The wrappers encode
the portable choice once.

| Wrapper | Replaces | Notes |
|---|---|---|
| `./scripts/mktemp.sh [<slug>]` | `mktemp` | Delegates to `mktemp(1)` when present; falls back to a 10-char `[A-Za-z0-9]` suffix from `/dev/urandom` + `touch`. Fallback does **not** set 0600 perms (`chmod` after creation for confidential content). Caller **must** register cleanup: `trap 'rm -f "$tmp"' EXIT INT HUP TERM`. |
| `./scripts/web-fetch.sh <URL> [<outfile>]` | `curl`, `wget` | Thin shim that `exec`s `./scripts/xtask.sh web-fetch -- "$@"`; the real implementation is `xtask/src/bin/web-fetch.rs` using `ureq` + `rustls`, so there's no dependency on `curl` / `wget` / `fetch` / `ftp` being on `PATH`. UA override via `WEB_FETCH_UA` (default `philharmonic-dev-agent/1.0`). HTTP 4xx/5xx fails the fetch (exit 2). Callers that want to continue regardless of HTTP status use `./scripts/web-fetch.sh ... || :` at the call site. |

**When the wrapper's semantics don't match your need, extend
it.** Don't reach around to raw `curl -fsSL` / `mktemp
--suffix=...` / etc.

---

## 8. In-tree workspace tooling (`xtask/`)

**Rule: never invoke `python`, `perl`, `ruby`, `node`, or any
other non-baseline scripting language from workspace tooling.
If you're tempted, write a Rust bin in `xtask/` instead.**

Well-written POSIX shell (with `awk`, `sed`, `grep`, `cut`,
`tr`, standard text pipelines) stays where it is — shell is
right for orchestration, git workflow, cargo wrappers,
filesystem glue, and simple data pipelines. The rule targets
ad-hoc `python3 -c "..."` / `perl -e "..."` creep, not the
existing `scripts/*.sh`.

**`jq` is not POSIX and is not on every baseline** (not shipped
by default on macOS, not in Alpine base, not in stripped Debian
minimal). If you find yourself reaching for `jq`, that's a Rust
trigger — add a bin under `xtask/` using `serde_json`. Same for
`curl` / `wget` — the `xtask/` port of `web-fetch` uses `ureq`
+ `rustls` in-process. The POSIX-shell data-manipulation tools
considered baseline-safe are the ones in SUSv4: `awk`, `sed`,
`grep`, `cut`, `tr`, `sort`, `uniq`, `head`, `tail`, `wc`.

Decision table:

| Category | Example | Home |
|---|---|---|
| Ad-hoc one-off in a terminal session | "generate a UUID for this constant" | Rust bin (`./scripts/xtask.sh gen-uuid -- --v4`) — **never** `python3 -c "import uuid"` |
| Non-baseline language reach | "parse YAML, walk DOM, emit Rust" | Rust bin in `xtask/` |
| POSIX shell orchestration | "commit across submodules then push" | `scripts/*.sh` |
| POSIX shell with `awk` / `sed` (baseline-present) | "enumerate workspace members from Cargo.toml" | `scripts/*.sh` (e.g. `lib/workspace-members.sh`) |
| Depends on a non-POSIX / non-baseline tool (`jq`, `curl`, `wget`) | "list non-yanked versions from crates.io", "HTTP GET a URL" | Rust bin in `xtask/` |
| Trivial cargo wrapper | "fmt + check + clippy + test" | `scripts/*.sh` |
| Non-trivial parsing / cross-file validation / stateful check | "verify no two entity KINDs collide across the workspace" | Rust bin in `xtask/` |

`xtask/` is an **in-tree (non-submodule) member crate** at the
workspace root. It lives alongside the submodule-backed crates
in `[workspace] members`, but its files are tracked directly by
the parent repo. `publish = false` — it's dev tooling only.

Multi-bin layout:

```
xtask/
├── Cargo.toml                    # publish = false, name = "xtask"
└── src/
    └── bin/
        ├── gen-uuid.rs           # one tool per file
        ├── crates-io-versions.rs # crates.io sparse-index query
        └── web-fetch.rs          # in-process HTTP GET (ureq + rustls)
```

Each bin is invoked via the `xtask.sh` wrapper:

```sh
./scripts/xtask.sh --list                   # list available tools
./scripts/xtask.sh --help                   # wrapper help
./scripts/xtask.sh <tool>                   # run with no args
./scripts/xtask.sh <tool> -- <args>         # run with args (note the `--`)
```

The mandatory `--` separator exists so future wrapper-level
flags (e.g. `--release`) can't collide with a bin's own flag of
the same name. Don't call `cargo run -p xtask --bin <name> --`
directly at call sites — `xtask.sh` is the single invocation
surface.

### 8.1 Non-submodule member plumbing

The scripts that walk "workspace members" (`show-dirty.sh`,
`crate-version.sh --all`) enumerate members from the root
`Cargo.toml` via `scripts/lib/workspace-members.sh`, so in-tree
members like `xtask` are covered uniformly alongside submodule-
backed ones. Scripts that need to distinguish the two use
`-f <member>/.git` as the classifier: submodules carry a `.git`
pointer file at their root, in-tree directories don't.

---

## 9. KIND UUID generation

**Every stable wire-format UUID — entity `KIND` constants,
algorithm identifiers, key IDs, anything that once committed
must never change — is generated via:**

```sh
./scripts/xtask.sh gen-uuid -- --v4
```

Not `python3 -c "import uuid"`, not `uuidgen`, not online
generators. The rule has one reason: **one canonical source of
randomness across sessions, machines, and contributors.** Ad-hoc
UUID generation tools scattered across shell history make it
too easy to accidentally commit a value you meant to throw away,
or for two contributors to mint UUIDs from imperceptibly
different RNG sources.

`--v4` is mandatory on the CLI — every KIND we mint today is v4
random. Making the version-flag explicit means a future shift
to v5/v7 is a deliberate CLI change at each call site rather
than a silent default swap.

Usage in practice: when authoring a new entity kind (e.g.
`TenantEndpointConfig`), run `gen-uuid -- --v4` once, paste the
result into the Rust source as `const KIND: Uuid = uuid!("…")`,
and commit. Never regenerate.

---

## 10. Rust code conventions

### 10.1 Edition and MSRV

- **Edition 2024.**
- **MSRV 1.88.**

Documented in each `Cargo.toml`:

```toml
edition = "2024"
rust-version = "1.88"
```

MSRV bumps happen in coordinated minor releases across the
workspace.

### 10.2 Build targets

Production: `x86_64-unknown-linux-musl` (static linking).

Library crates build for consumer-chosen targets. Binary crates
(`mechanics`, future API and connector binaries) ship as
statically-linked musl binaries for minimal containers.

Constraint: C library dependencies must be statically linkable
or vendored. Most pure-Rust crates are fine; special attention
for crates wrapping native libraries.

### 10.3 Panics and undefined behavior

Philharmonic is systems-programming infrastructure: long-running
services, request-handling paths, cryptography, storage. A
panicking thread ends the task it was running and can
destabilise neighbouring tasks on the same worker; an unchecked
integer overflow changes behaviour silently between debug and
release; an out-of-bounds index read is an invariant violation
the compiler cannot catch. None of those failure modes are
acceptable surface behaviour for a crate meant to be trusted in
production.

**Principle: library code surfaces failures as typed `Result`s,
not panics.** Bugs and genuine unrecoverable conditions are
narrow exceptions.

**Banned in library code** (`src/**/*.rs`, excluding
`#[cfg(test)]` modules):

- **`.unwrap()` and `.expect()` on `Result` / `Option`** — use
  `?` with a typed error variant, or `.ok_or_else(...)` /
  `.map_err(...)`.
- **`panic!`, `unreachable!`, `todo!`, `unimplemented!` on
  reachable paths** — model unreachability at the type level
  (newtype, `NonZero<T>`, sealed enums, typestates) so the
  compiler proves the invariant.
- **Unbounded indexing** — `slice[i]`, `slice[a..b]`,
  `HashMap[&k]` all panic on absent/OOB access. Use `.get(i)`
  / `.get(a..b)` / `.get(&k)` and propagate the `Option`.
  Iterator-based access (`.iter().nth(i)`, `.windows(n)`,
  `.chunks(n)`) is fine — none of those panic.
- **Unchecked integer arithmetic** — `a + b`, `a - b`, etc.
  panic on overflow in debug and silently wrap/trap in release.
  Use `checked_*` / `saturating_*` / `wrapping_*` to declare
  intent at the call site. Plain `+` / `-` is fine for
  constants or cases the compiler can prove (e.g. `usize_len +
  1` for a bounded `Vec`).
- **Lossy `as` casts when the input can exceed the target
  type's range** — `n as u32` silently truncates when `n: u64 >
  u32::MAX`. Use `u32::try_from(n)`. Plain `as` is fine for
  provably-lossless casts (`u16 as u32`).
- **`debug_assert!` / `assert!` on data from outside the
  crate.** `debug_assert!` is compiled out in release; `assert!`
  panics. For internal consistency checks the crate controls
  end-to-end, `debug_assert!` is acceptable; for external inputs
  validate with a `Result`-returning helper.
- **`unsafe` blocks** — separately banned workspace-wide in
  crypto-sensitive crates (see `ROADMAP.md §5` and
  `docs/design/11-security-and-cryptography.md`). No library
  crate takes `unsafe` dependencies on invariants the type
  system can't express.

**Narrow exceptions — allowed with an inline justification:**

- **Unrecoverable OS / hardware failure.** `SysRng.try_fill_bytes(...)
  .expect("OS RNG failure — system entropy unavailable")` is
  the one pattern already approved (see
  `philharmonic-policy/src/sck.rs`). On a system that can't
  produce entropy, no cryptographic work is possible; no
  caller can recover. Comment the reason.
- **Build-time-validated constants.** `uuid!("literal")` is
  compile-time validated and has no runtime panic path.
- **Type-witness unreachability.** If you've exhausted a sealed
  enum or matched on a newtype whose constructor rules out a
  variant, `unreachable!()` is still wrong — change the match
  or the type. If the compiler is the only one that can't see
  unreachability (slice-pattern exhaustion that `match` can't
  express), prefer `.expect()` with a message naming the
  type-level reason.

**Where panics are fine**:

- `#[cfg(test)]` modules, `tests/*.rs` integration tests,
  `dev-dependencies`. Panicking is the mechanism of signalling
  test failure.
- `xtask/` bins: dev tooling, run from the contributor's shell.
  Typed errors for user-surface failures (bad CLI args, network
  failures) but `.unwrap()` / `.expect()` on invariants is fine.
- Reasonable `.expect()` at binary startup (e.g. parsing config
  at `main()`). In-service panic sources (request handlers,
  connection pools, task loops) still follow the library rules.

**Enforcement.** There is no automated lint yet. Reviewers audit
at PR time. The crypto-review protocol
(`.claude/skills/crypto-review-protocol/`) includes an explicit
panic-site pass over any crypto-sensitive diff. When Clippy adds
reliable lints for the patterns above (`clippy::indexing_slicing`,
`clippy::arithmetic_side_effects`, `clippy::unwrap_used`,
`clippy::expect_used`, `clippy::integer_arithmetic`), adopt them
per-crate with deny-level.

### 10.4 Library crate boundaries

Library crates expose data-taking APIs, not path-taking APIs.
When a secret or a config input is needed, the library accepts
the **bytes** (or a pre-parsed struct) — not a `&Path`, not a
filename, not an environment-variable name, not a config-file
path. File I/O, file-permission checks, environment lookup,
config-file parsing, and CLI-argument handling belong in the
bin crate that holds the library's runtime context.

Reasoning:

- **Testability.** A lib that takes bytes is unit-tested with
  fixture bytes — no tempfile, no filesystem permissions, no
  racing tests for a shared file.
- **Portability.** File-permission semantics differ across
  Unix, Microsoft Windows, and WASI. A lib that reads files
  carries that portability burden; a lib that takes bytes
  doesn't.
- **Composability.** Consumers that fetch secrets from a key
  manager, a KMS, or an environment variable can use the lib
  without pretending the bytes came from a file.
- **Config-surface discipline.** Config files are an
  application concern — serialization format, schema versioning,
  backward-compat story. A library that reads config files
  ships opinions the downstream binary may not share.

Concretely:

- **Secret keys.** Libraries take a pre-read byte slice (or
  `Zeroizing<[u8; N]>` for private-key material). The bin
  does the file read and any file-permission check.
- **Public-key registries, trust stores.** Libraries expose
  programmatic insertion (`insert(kid, entry)`) rather than
  `load_from_config_file(path)`. The bin parses whatever
  config format it chose and calls insert.
- **TLS certs, CA bundles.** Libraries accept bytes or
  pre-parsed types, not paths.
- **Runtime config structs.** Libraries accept a populated
  `Config { ... }` value. The bin decides whether that value
  came from a TOML file, CLI flags, or the environment.

Exception: bin crates may (and should) layer a thin
configuration surface over a library. The rule is about which
crate owns file I/O, not about banning config files in the
workspace. It also does **not** apply to `dev-dependencies` or
in-`tests/` helpers — a test may freely read a fixture file.

The crypto-review skill's Gate-1 checks treat any file-path-
taking crypto library API as a smell to flag explicitly.

### 10.5 Trait crate vs. implementation crate split

When a concern has a trait surface and one or more
implementations, they live in separate crates rather than being
feature-gated within one.

Reasoning:

- **Dependency hygiene**: trait crate minimal; implementations
  carry their own dependencies.
- **Independent versioning**: bug fix in one implementation
  doesn't require trait crate release.
- **Discoverability**: implementations are separate crates on
  crates.io with their own pages.
- **No feature flag combinatorics**: each crate tested in
  isolation.

Example: `philharmonic-store` (traits) +
`philharmonic-store-sqlx-mysql` (SQL impl); not
`philharmonic-store` with `mysql` feature.

### 10.6 Re-export discipline

Crates re-export types from their direct dependencies that
appear in their own public APIs. Consumers get a flat
namespace.

```rust
// philharmonic-store re-exports Uuid, Sha256, EntityId, etc.
// from philharmonic-types.
use philharmonic_store::{ContentStore, Uuid, EntityId};  // works
```

Rules:

- Re-export what appears in the crate's own public API.
- Don't re-export transitive dependencies.
- Don't re-export types the crate doesn't itself use.

### 10.7 Error types

Errors use `thiserror` for display and source-chain. Partition
by what the caller does with them (semantic violations,
concurrency outcomes, backend failures). Methods like
`is_retryable()` give uniform checks.

Don't use `anyhow` in library crates — callers can't match on
specific failure modes. Use `anyhow` in application binaries
where appropriate.

### 10.8 Async runtime

`tokio` is the workspace default. Avoid `async-std` or other
runtimes for consistency. Use `tokio::sync` primitives where
appropriate.

`async-trait` is used where trait objects need to be
dyn-compatible (current Rust stable async-in-traits support is
insufficient for trait-object use).

### 10.9 Testing

Unit tests colocated with source (`#[cfg(test)] mod tests` or
`mod.rs` tests). Integration tests in `tests/`. Real-
infrastructure tests (testcontainers, network) gated behind
features when appropriate.

Test helpers factored into a shared module within the tests
directory; no separate testing crates unless the helpers are
genuinely shared across crates.

### 10.10 Miri

Run `cargo +nightly miri test` routinely, via
`./scripts/miri-test.sh`. Miri is an interpreter for Rust's
mid-level IR that catches UB classes regular `cargo test`
doesn't: uninitialised-memory reads, out-of-bounds pointer
arithmetic, invalid `mem::transmute` / `mem::uninitialized`,
data races, type-layout confusion, and stacked-borrows
violations in `unsafe` code paths (including in dependencies).

This workspace bans `unsafe` in library code, but miri is cheap
insurance against UB smuggled in through:
- Dependency updates that quietly add or widen `unsafe` blocks.
- Test harnesses that bypass the library rules.
- `std` / core behaviour that's UB-adjacent on specific
  targets.

Invocation:
```sh
./scripts/miri-test.sh <crate> [<crate>...]   # per-crate
./scripts/miri-test.sh --workspace            # whole workspace
MIRIFLAGS="-Zmiri-disable-isolation" ./scripts/miri-test.sh <crate>
```

Setup (one-time, handled by `setup.sh`):
- `rustup toolchain install nightly --profile minimal`
- `rustup component add miri --toolchain nightly`

Scope caveats:
- Miri cannot exercise FFI, inline assembly, or most syscalls.
  Crates that depend on real sockets, DB drivers (sqlx),
  testcontainers, or other I/O won't run under miri — scope
  invocations to in-memory crates (`philharmonic-types`,
  `mechanics-config`, `philharmonic-policy`'s crypto paths)
  rather than `--workspace` blindly. Use
  `MIRIFLAGS=-Zmiri-disable-isolation` if you need filesystem
  / env access.
- Miri is slow (10–50× cargo-test). Don't put it in
  `pre-landing.sh`; run manually before publishing a crate and
  on a periodic schedule (weekly / pre-milestone).

Crypto paths (`philharmonic-policy` SCK / `pht_`) are the
highest-value miri targets — AES / SHA-2 implementations in
the `aes-gcm` / `sha2` crates use `unsafe` for SIMD intrinsics,
and miri exercises the slow no-SIMD reference paths.

---

## 11. Pre-landing checks

**Mandatory before every commit that touches Rust code.** One
command covers the full flow:

```sh
./scripts/pre-landing.sh
```

Auto-detects modified crates (submodules with a dirty working
tree) and runs, in order:

1. `./scripts/rust-lint.sh` — fmt-check + check + clippy
   (`-D warnings`).
2. `./scripts/rust-test.sh` — `cargo test --workspace` (skips
   `#[ignore]`-gated tests).
3. `./scripts/rust-test.sh --ignored <crate>` for every modified
   crate — exercises integration tests for what you actually
   changed.

Pass crate names to `pre-landing.sh` to override auto-detection;
pass `--no-ignored` to skip step 3 (rare, for fast iteration
when you're certain the slow tests aren't affected).

GitHub CI runs the same script on a clean checkout (no dirty
submodules → step 3 naturally empty) so contributor and CI
behaviour don't drift.

**Why split the test run.** Integration tests that need real
infrastructure carry `#[ignore]` so the default workspace run
stays fast. A single `cargo test --workspace` with everything
enabled would run for many minutes on unchanged crates. The
split is workspace-level skip-ignored for regression coverage
and per-touched-crate `--ignored` for your actual changes.
Don't run `--ignored` against untouched crates; it's waste.

### 11.1 Don't go raw

**Do not run raw `cargo fmt`, `cargo check`, `cargo clippy`, or
`cargo test`** when the scripts above cover the job. Bespoke
cargo invocations (e.g. `cargo test some_specific_test` for
focused debugging) remain fine — the rule targets the canonical
pre-landing flow, not exceptional cases.

Doc-only commits (markdown, scripts, `Cargo.toml` metadata that
doesn't affect code) may skip these. Anything that could affect
a `.rs` file's compilation or test outcome — including
dependency bumps — must run all three phases.

These rules apply equally to humans and AI agents.

### 11.2 CI

Each crate's CI runs at minimum:

- `cargo build` against MSRV.
- `cargo test` against current stable.
- `cargo clippy --all-targets` with warnings as errors.
- `cargo fmt --check`.
- `cargo doc --no-deps`.

CI mirrors the local pre-landing checks. If CI fails on a check
that passed locally, the cause is almost always a dirty working
tree or MSRV drift — investigate before forcing through.

Integration tests requiring external infrastructure are gated
behind features.

---

## 12. Versioning and releases

### 12.1 Semantic versioning (pre-1.0)

- **Patch (0.x.y → 0.x.(y+1))** — additive changes, bug fixes,
  docs.
- **Minor (0.x.y → 0.(x+1).0)** — changes to existing APIs.
  Pre-1.0 breakage signal.
- **Major (0.x.y → 1.0.0+)** — stability boundary.

The cornerstone (`philharmonic-types`) is on the strict end:
many dependents, so breaking changes are painful. Bundled,
announced.

Downstream crates pin the cornerstone to minor version
(`philharmonic-types = "0.3"`) to pick up patches automatically
while protecting against minor-version breakage.

Peer crates within the workspace pin loosely to each other
(`philharmonic-store = "0.1"`) — the workspace evolves together.

**Pinning to a patch version.** When a crate relies on a feature
or fix introduced in a specific patch, pin to that exact patch
(`philharmonic-types = "0.3.4"`) so `cargo` refuses to resolve
against an older patch that lacks the required feature. This is
the *only* reason to tighten beyond a minor pin; don't pin to a
patch for hygiene or habit. When the dependency publishes a
further patch whose feature set you start using, bump the pin.

### 12.2 Crate name claims

Defensive early publishing: reserve crate names on crates.io
before implementations are complete. Use `0.0.0` placeholders
with empty or minimal content.

This prevents name squatting and ensures the namespace is
predictable. When implementation begins, the crate transitions
from placeholder to real content via normal version bumps.

### 12.3 API breakage detection

`cargo-semver-checks` compares the public API surface of one
workspace crate against a crates.io baseline and flags
semver-incompatible changes (removed items, tightened trait
bounds, signature changes, etc.):

```sh
./scripts/check-api-breakage.sh <crate> [<baseline-version>]
```

Without `<baseline-version>`, `cargo-semver-checks` queries
crates.io for the newest published version of `<crate>` and
uses that as the baseline. Pass an explicit version (e.g.
`0.2.2`) to compare against a specific earlier release. The
script installs `cargo-semver-checks` via `cargo install
--locked` on first use.

**Per-crate, not workspace-wide.** An earlier version used
`--workspace --baseline-rev <rev>`. That doesn't work here: the
parent is a virtual workspace (no `[package]` table), and
`cargo-semver-checks` resolves `--baseline-rev` by `git clone`
at that rev — which doesn't recurse into submodules, so
workspace members can't be found at the baseline root. Per-crate
against a crates.io baseline sidesteps the problem entirely.
See
[`docs/notes-to-humans/2026-04-21-0006-mechanics-core-semver-checks-finding.md`](docs/notes-to-humans/2026-04-21-0006-mechanics-core-semver-checks-finding.md)
for the history.

**When to run:** before preparing a crate release, as part of
the pre-release review checklist. Not part of the default
pre-landing trio (fmt/clippy/test) because it's slower and the
signal is per-release rather than per-commit.

### 12.4 Release tagging

Every crate release is tagged. The tag:

- **Lives in the crate's own repo (the submodule), not the
  parent workspace.** Each submodule is a single-crate repo, so
  the tag `v<version>` is unambiguous. No per-crate prefixing
  is needed.
- **Is created only by `./scripts/publish-crate.sh`.** Running
  `git tag` by hand during a publish is the same class of
  mistake as ad-hoc `git commit`.
- **Is annotated and cryptographically signed** (`git tag -s`).
  Matches the "every commit is signed" rule.
- **Is created after `cargo publish` succeeds, not before.** A
  failed publish must not leave a dangling tag. If
  `publish-crate.sh` fails between `cargo publish` and `git
  tag`, the crate is on crates.io without a tag — recover by
  running `git tag -s v<version>` manually in the submodule and
  then `./scripts/push-all.sh`.
- **Is pushed by `./scripts/push-all.sh`** via `--follow-tags`.
  Only tags pointing at pushed commits go up, so stray local
  tags never leak.

Why: crates.io holds the published tarball, but the exact git
state that was published is only trivially recoverable if a tag
marks it. Tags also give `cargo-semver-checks` a clean baseline
reference for release-to-release API-breakage checks.

**Post-release verification.** `./scripts/verify-tag.sh <crate>
[<tag>]` confirms the tag landed cleanly end-to-end: local
presence, signature verifying with the local keyring, and
origin having the same tag pointing at the same commit. Run it
after `publish-crate.sh` + `push-all.sh`.

---

## 13. Licensing

All crates: `Apache-2.0 OR MPL-2.0`.

Both license files at the crate root (`LICENSE-APACHE`,
`LICENSE-MPL`). `Cargo.toml`:

```toml
license = "Apache-2.0 OR MPL-2.0"
```

Rationale: Apache-2.0 for permissive use with patent grants;
MPL-2.0 for file-level copyleft with GPL compatibility via the
secondary-license clause. Covers more deployment scenarios than
common `Apache-2.0 OR MIT` without compromising openness.

---

## 14. Naming and terminology

Documentation, comments, commit messages, and any other
workspace-authored prose follow two overlapping conventions:
inclusive/neutral/technically-accurate language, and
FSF-preferred framing for free-software terminology. Both are
soft rules — readability trumps dogma — but the anti-patterns
below have specific reasons behind them.

### 14.1 Crate naming

Pattern: `<subsystem>-<concern>[-<implementation>]`

- Subsystem prefix identifies the project: `philharmonic-`,
  `mechanics-`.
- Concern after prefix names what the crate is for: `-types`,
  `-store`, `-workflow`, `-connector`.
- Implementation suffix for multiple implementations of one
  concern: `philharmonic-store-sqlx-mysql` is "the storage
  trait, implemented via sqlx for MySQL."
- Meta-crate unsuffixed: `philharmonic`, `mechanics`.

Readers can infer relationships from names.
`philharmonic-store` is the trait; `philharmonic-store-sqlx-
mysql` is an implementation.

### 14.2 Inclusive, neutral, technically accurate language

- **No charged master/slave metaphors** for technical
  relationships. Use what the parts actually do: `primary` /
  `replica`, `leader` / `follower`, `parent` / `child`,
  `controller` / `agent`, `main` / `workers`. This workspace's
  default git branch is `main`, not `master`.
- **No gendered defaults.** Prefer the singular "they" when the
  referent's gender is unknown or irrelevant; avoid "he",
  "he/she", "(s)he", "the user … his …". Avoid "guys" / "man"
  as colloquial generics — write "folks", "everyone", "people",
  or the role itself ("developers", "operators", "reviewers").
- **Name what the thing does, not who's allowed to use it.**
  Prefer `allowlist` / `denylist` (or "permitted" /
  "disallowed") over `whitelist` / `blacklist`.
- **"Dummy" / "sanity check" / "crazy"-adjacent wording** has
  less charged technical equivalents — `stub`, `placeholder`,
  `fake`, "smoke test", "quick check", "verify", "unusual",
  "unexpected". Use them when they fit.
- **Technical accuracy overrides aesthetic neutrality.** When
  a protocol, library, or external project ships a term
  literally (HTTP `Authorization` header; the `master` branch
  of an external repo you're referencing), use the literal
  name. The rule targets prose we author, not identifiers
  other projects defined.

### 14.3 Operating systems and kernels

- **GNU/Linux** for the GNU-userspace-plus-Linux-kernel
  operating system, not just "Linux." Calling the whole system
  "Linux" credits the kernel alone for userspace that's
  largely GNU.
- **Linux kernel** (or "the kernel of Linux") when referring
  specifically to the kernel. Don't use "Linux" as a shorthand
  for the kernel when the kernel is what you mean.
- **Non-GNU Linux-based systems** are named explicitly, not
  collapsed into "Linux." Alpine is musl-based; Android is
  Linux-based but distinct from GNU/Linux; BusyBox environments
  are their own thing. Saying "works on Linux" papers over a
  distribution family that isn't uniform.
- **`uname -s` string matches** are a pragmatic exception. When
  shell code matches the literal kernel-identifier string
  `Linux`, writing `Linux` is accurate — that IS what `uname`
  returns. The rule targets human-facing prose, not
  kernel-interface string literals.

### 14.4 Microsoft Windows

- Write the full name — "Microsoft Windows" or just "Windows"
  — in neutral prose. Don't abbreviate to `Win`, `win32`,
  `win64`, or `WIN_` as a freeform shorthand; the abbreviation
  reads as Microsoft "winning" against competing systems. The
  exception is established technical identifiers that ship
  that way (the Windows API is literally the `Win32` API;
  package identifiers like `x86_64-pc-windows-msvc` have
  `windows` in them). Don't fight those; don't invent new
  `win*`-prefixed abbreviations.

### 14.5 Free software vs. "open-source"

- **"Free software"** (free as in freedom) when framing a
  software-freedom position or classifying a license.
- **"FLOSS"** (Free/Libre/Open-Source Software) as an
  inclusive umbrella when both the free-software and
  open-source communities are equally in scope.
- Avoid **"open-source"** as a standalone phrase when the
  intent is user freedom. "Open-source" is the marketing
  framing that intentionally sets the freedom argument aside.
  Use it when quoting external conventions (the "Open Source
  Initiative" is a proper noun; an "open-source license" is
  how the OSI describes licenses on their approved list), not
  as the default neutral term.

### 14.6 Enforcement

Enforcement is by review, not tooling — the workspace has no
linter for prose conventions. Sweeps happen opportunistically
when editing an affected file; a dedicated cleanup pass is
unnecessary unless a pattern is repeated enough to warrant
campaigning.

---

## 15. Journal-like files

Every journal-like file generated under this workspace follows
one filename shape:

```
YYYY-MM-DD-NNNN-<slug>[-NN].md
```

- **`YYYY-MM-DD`** — the date the file was created.
- **`NNNN`** (required) — four-digit daily sequence,
  zero-padded, counted **per-directory**. The first file
  written in a given journal directory on a given day is
  `0001`, the second `0002`, etc. `docs/codex-prompts/` and
  `docs/notes-to-humans/` each count independently.
- **`<slug>`** — short kebab-case task name. Example:
  `phase-1-mechanics-config-extraction`. Keep it ≤ ~60
  characters.
- **`[-NN]`** (optional) — two-digit round suffix for
  multi-part work on one logical entry. Use when a Codex
  session hit a limit mid-task and you're resuming, or a
  notes-to-humans entry genuinely needs a follow-up round.
  Omit when the entry stands alone.

Journal directories currently governed by this format:

- `docs/codex-prompts/` — §15.2.
- `docs/codex-reports/` — §15.3.
- `docs/notes-to-humans/` — §15.1.

Any future journal directory adopts the same format by default.
Don't invent a variant.

### 15.1 Notes to humans

[`docs/notes-to-humans/`](docs/notes-to-humans/) is where Claude
writes significant findings, observations, and decisions that
would otherwise live only in session scrollback. **When Claude
tells Yuka anything substantial, the note must also be written
to a file here and committed.** Session-only output is not
enough.

**What counts as "significant" (write a note):**

- Verification results where the *why* is informative ("your
  version bump is defensible for these reasons; CHANGELOG
  missed the behaviour change").
- Platform-specific caveats surfaced during investigation
  ("Alpine busybox ps has no `pcpu`; we switched to `time`").
- Audit results: what's been touched, what hasn't, what's
  load-bearing, what's dead.
- Design calls made mid-implementation that the prompt didn't
  spell out.
- Non-obvious failure modes surfaced during testing.
- Anything Yuka says to "note", "remember", or "flag".

**What does not (session-only is fine):**

- Routine acknowledgments ("done", "committed", "pushed").
- Echoes of commands run.
- Plan summaries before acting.
- Trivially-obvious diffs.

**Contents.** Prose-focused, one logical observation per file.
Enough context that a future reader can act without re-running
the investigation: file paths, commit SHAs, the why behind the
finding. If two unrelated observations are worth saving, write
two files (different daily sequences).

**Commit.** Through `./scripts/commit-all.sh --parent-only` in
the same session — the note is parent-level regardless of
whether the underlying work was in a submodule.

**Don't write to humans like a self-reminder.** The audience is
Yuka (and future collaborators reading the repo), not
Claude-next-session. Complete sentences, concrete references,
no in-jokes.

### 15.2 Codex prompt archive

Claude hands substantive coding to Codex (see `CLAUDE.md`).
Every prompt Claude writes for Codex is archived and committed
— there are no ephemeral Codex invocations.

**Location.** `docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>[-NN].md`.
`<slug>` names the task (`auth-middleware-rewrite`,
`sqlx-mysql-store-skeleton`). One file per prompt.

**Contents.** The full prompt text Claude sent to Codex,
verbatim, plus a short preamble with:

- The task's motivation (one or two sentences).
- Links to the relevant design-doc sections or ROADMAP
  entries.
- Any context files Claude pointed Codex at.

**Commit cadence.** Commit the prompt file *before* spawning
Codex, via `scripts/commit-all.sh`. The resulting code changes
land in a subsequent commit (or commits). Ordering the prompt
first means the archive is complete even if the Codex run is
abandoned partway through.

**Why.** The prompts are where design intent gets translated
into implementation instructions; they're the most useful
artefact for reconstructing why a chunk of code looks the way
it does, and they're where Claude/Codex collaboration mistakes
become visible in review. Losing them to chat history defeats
the reviewability of the workflow.

### 15.3 Codex reports

Parallel to `docs/codex-prompts/` (Claude → Codex) and
`docs/notes-to-humans/` (Claude → Yuka), `docs/codex-reports/`
is **Codex → the repo**: Codex-authored reports that capture
findings, design rationale, or implementation details worth
preserving past the session-summary text Codex writes back to
Claude.

**Location.** `docs/codex-reports/YYYY-MM-DD-NNNN-<slug>[-NN].md`.
`<slug>` usually mirrors the prompt's slug so the two files pair
by name.

**When Codex writes a report:**

- The prompt explicitly asks for one.
- The work involved a non-obvious design call that the prompt
  didn't spell out.
- Substantial findings surfaced during implementation that
  don't fit in the session-summary.
- Items flagged per a flag-vs-fix policy (crypto-review,
  zeroization gaps, `unsafe` in neighbouring code) that Codex
  observed but didn't fix.

**When session-summary alone is fine:**

- Routine implementation of a well-specified prompt with no
  surprises.
- Acknowledgement-style updates ("done, tests green").
- Diffs that are self-explanatory from the code alone.

**Contents.** Prose-focused, one logical report per file.
Audience is future-Claude and Yuka. Complete sentences,
concrete file paths, no session-specific in-jokes.
Cross-reference the prompt file with a short header:

```markdown
# <title>

**Date:** YYYY-MM-DD
**Prompt:** docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>.md
```

**Commit cadence.** Claude drives Git. Codex leaves the report
file in the working tree; Claude reviews and commits it
alongside the implementation diff.

**Codex does not commit.** Leave the working tree dirty.
Mention the report path in the final summary so Claude picks
it up on review.

---

## 16. ROADMAP maintenance

[`ROADMAP.md`](ROADMAP.md) at the repo root is the linear plan:
where the project is, what's next, what's blocked on what. Its
audience is Claude Code working sessions (and anyone
re-orienting to the project after a break). It's not a
historical diary or a wish list — it's the document that tells
you what to do next.

- **Update in the same commit as the work.** A commit that
  moves a phase forward should also mark the relevant tasks
  done (or partially done) in ROADMAP.md. Splitting "do the
  work" and "update ROADMAP" into separate commits lets the
  roadmap drift out of sync with reality; coupling them keeps
  the plan honest.
- **When plans change, update the plan.** If implementation
  reveals the planned approach is wrong, update ROADMAP.md
  (and the relevant design doc) with the new approach and a
  short line explaining *why* — future readers need the "why"
  more than the "what".
- **Don't paper over an unclear roadmap with code.** If the
  next step isn't clear from ROADMAP.md, stop and propose a
  roadmap update first. Implementing against a guess creates
  churn.

---

## 17. Conventions-about-conventions

### 17.1 When conventions should change

Conventions aren't immutable. Signs one should be revisited:

- **Repeated workarounds**: multiple crates work around the
  same convention.
- **Friction for new crates**: adding a new crate requires
  significant ceremony.
- **Ecosystem drift**: broader Rust conventions change.

Convention changes are workspace events: announced, applied in
coordinated releases, documented here.

### 17.2 Workspace inspirations

Conventions draw from:

- **Tokio's crate split**: `tokio`, `tokio-util`, `tokio-
  stream`. Separate crates for separate concerns with
  consistent naming.
- **sqlx's trait/backend pattern**: one trait surface, multiple
  backends. Philharmonic chose separate crates over features
  for the reasons in §10.5.
- **The Rust API Guidelines**: followed where applicable.

### 17.3 Claude Code skills

Claude-specific operational procedures live as skills under
[`.claude/skills/`](.claude/skills/):

- `git-workflow` — the scripts-only rule above, agent-facing.
- `codex-prompt-archive` — the archive-before-spawn rule above.
- `crypto-review-protocol` — Yuka's two-gate crypto review
  (Gate 1 design approval before code, Gate 2 code review
  before publish). Triggers on any crypto-sensitive path (SCK,
  COSE_Sign1, COSE_Encrypt0, hybrid KEM, payload-hash, `pht_`
  tokens).

Skills are part of the enforcement surface: Claude is expected
to invoke the relevant skill when its trigger fires.

### 17.4 Agent-specific files

- [`CLAUDE.md`](CLAUDE.md) — Claude Code's role, context,
  divisions of labour with Codex, executive summary of rules
  that matter most to Claude (with pointers here).
- [`AGENTS.md`](AGENTS.md) — Codex's role, what Codex does and
  doesn't do in this workspace, executive summary of rules
  that matter most to Codex (with pointers here).
- [`HUMANS.md`](HUMANS.md) — Yuka's note-to-self. Agent-readable,
  agent-writable is **forbidden**. `commit-all.sh` sweeps her
  pending edits into whatever commit is being made; that's the
  only way `HUMANS.md` changes reach the repo.
- [`docs/instructions/`](docs/instructions/) — human-authored
  rules specifically targeted at agents (rules about how to
  behave around particular artefacts like `HUMANS.md`). Not
  the general-purpose conventions home — this file is.

Those files should carry executive summaries + pointers, not
restate the rules. This file is the authoritative home; the
others are contextualised orientations.
