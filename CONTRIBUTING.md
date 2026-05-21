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

**Three standing rules that shape how this repo is documented**
(spelled out in full in §18):

1. **[`README.md`](README.md) is the whole-project executive
   summary.** Self-contained, concise, up-to-date. It will be
   fed to coding sub-agents as the project's one-page mental
   model; broken or stale claims there are bugs. See §18.1.
2. **This file is the authoritative home for every convention.**
   When you change a convention in practice — add a new rule,
   change an existing one, retire an old one, or discover an
   unwritten rule that should be authoritative — **update
   `CONTRIBUTING.md` in the same commit**. See §18.2.
3. **[`ROADMAP.md`](docs/ROADMAP.md) is the authoritative home for
   every roadmap and plan.** Current phase, what's next, what's
   blocked on what, what was deferred and why — all live there.
   No parallel TODO lists, no plans-of-record in chat / notes /
   a person's head. **When plans change, update
   `docs/ROADMAP.md` in the same commit as the work that changes
   them.** See §16 for
   mechanics and §18.3 for scope.

Related authoritative docs that stay as their own homes:
- [`ROADMAP.md`](docs/ROADMAP.md) — linear plan (what to work on next).
- [`docs/design/`](docs/design/) — architectural design docs
  (what Philharmonic *is*, not how to contribute to it).
- [`POSIX_CHECKLIST.md`](docs/POSIX_CHECKLIST.md) — POSIX-shell
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
- [18. Documentation obligations](#18-documentation-obligations)

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
2. `./scripts/pre-landing.sh` — cargo-deny bans + fmt + check +
   clippy (`-D warnings`) + test on every modified crate.
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
- `CONTRIBUTING.md` (this file), `docs/ROADMAP.md`, `README.md`,
  `CLAUDE.md`, `AGENTS.md`, `HUMANS.md`, `docs/POSIX_CHECKLIST.md`.

Each submodule is a standalone single-crate repo. Inside a
submodule: `Cargo.toml`, `README.md`, `CHANGELOG.md`,
`LICENSE-APACHE`, `LICENSE-MPL`, `src/`, `tests/`. Cross-crate
refactors require coordinated commits across repos;
cornerstone-versioning discipline absorbs most of this.

### 3.1 Submodules must build independently

Every submodule-backed crate must build standalone — cloned
on its own, outside this workspace, with `cargo build` /
`cargo test` working against its own `Cargo.toml` and the
crate's own `[profile.dev]` / `[profile.release]` (and any
other profile customisations) applied.

This is not optional. Each submodule has its own crates.io
release cycle; a downstream consumer reaching the crate via
`cargo add <crate>` or `cargo install <crate>`, or via a
fresh `git clone` of the submodule's own repo, must get the
same compile / link / panic / opt settings the crate
expects, without depending on the parent workspace's
`Cargo.toml`. A submodule that only builds *inside* this
workspace is a bug.

#### `[profile.*]` blocks in member `Cargo.toml`s are load-bearing

Most submodule crates carry their own `[profile.dev]`,
`[profile.release]`, and similar blocks. When cargo loads
the parent workspace it ignores those blocks (the workspace
root's profiles win for in-workspace builds) and emits a
three-line warning per affected member:

```
warning: profiles for the non root package will be ignored, ...
package:   /<workspace>/<crate>/Cargo.toml
workspace: /<workspace>/Cargo.toml
```

The warning is **expected and harmless**. The blocks have to
stay — they're what makes the standalone build produce the
right binary. The warning simply fires on every workspace
build because the blocks exist.

`scripts/lib/cargo-noise-filter.sh` strips this noise from
the build / test / lint / publish scripts that invoke cargo
against the workspace (`rust-lint.sh`, `rust-test.sh`,
`miri-test.sh`, `release-build.sh`, `publish-crate.sh`,
`check-api-breakage.sh`, and the stderr-only variant in
`xtask.sh`). New scripts that invoke
cargo against the workspace should source the filter and
route cargo through `run_with_cargo_noise_filter` (or
`run_with_cargo_noise_filter_stderr` for tool wrappers whose
stdout is captured by callers).

#### Forbidden: deleting `[profile.*]` to silence the warning

Do not remove `[profile.*]` sections from a submodule's
`Cargo.toml` to make the warning go away. That silently
breaks the standalone-build contract: a downstream user
building the published crate (or a `git clone` of the
submodule on its own) then gets cargo's *defaults* —
different panic strategy, optimisation level, debug-info,
strip-symbols, codegen-units, LTO settings — than the crate
was tested with. The warning we strip exists because cargo
deliberately ignores the section *only in workspace mode*;
the section is still authoritative outside the workspace.

If a profile setting genuinely belongs at the workspace
level (applies uniformly across every member during
in-workspace builds), add it to the parent `Cargo.toml`'s
`[profile.*]` blocks *in addition to* the per-crate setting,
not in place of it. The two coexist by design: the workspace
block governs in-workspace builds, the per-crate block
governs standalone builds.

#### Per-setting rationale for `[profile.release]`

The workspace's canonical `[profile.release]` block — used
both at the workspace root and (verbatim, intentionally) in
every published submodule's `Cargo.toml` — is:

```toml
[profile.release]
opt-level = 3
lto = true
strip = true
codegen-units = 1
panic = "abort"
overflow-checks = true
```

Each setting is a deliberate workspace choice. **Do not
revert any of them silently** to "match cargo defaults" or
"speed up CI" — each was set for a reason that survives the
short-term win of relaxing it. If a future change genuinely
requires a different value, surface the reasoning in the
relevant PR + commit message and update this section.

- **`opt-level = 3`** — release builds want the maximum
  optimisation tier. Default. Just made explicit so the
  intent is grep-able.
- **`lto = true`** — full link-time optimisation across
  every crate boundary in the release artifact. Slower
  link, smaller and faster binary. Worth it for the
  release bins and for downstream users embedding the
  library crates with their own LTO discipline.
- **`strip = true`** — strip symbols from the release
  binary. Smaller artefact; loses debuginfo (which release
  builds don't need; sourcemaps via `panic = "abort"` are
  cheap to capture with `RUST_BACKTRACE=1` at run time).
- **`codegen-units = 1`** — single codegen unit means the
  optimiser sees the whole crate at once and can inline
  / dead-code-eliminate across function boundaries that
  the multi-unit default would block. Slower compile per
  release build, **noticeably faster runtime + slimmer
  binary**. This trade is intentional and has been
  ratified by the workspace's release-performance work;
  do not bump back to a higher count to shave compile
  time. The compile cost is paid once per release; the
  runtime + size win is paid every time the binary runs.
- **`panic = "abort"`** — release builds abort on panic
  rather than unwinding. Smaller binary (no unwind tables
  on most call frames), faster (no per-frame landing-pad
  cost), and removes a class of unwind-based attacks. The
  workspace's no-panic-in-`src/` rule
  ([§10.3](#103-panics-and-undefined-behavior)) means
  panics are bugs we want to surface as crashes, not
  recover from. `catch_unwind` is unavailable under abort
  semantics; that's a feature, not a regression. The dev
  profile uses default unwind so test failures are
  diagnosable.
- **`overflow-checks = true`** — keep integer-overflow
  checks on in release. The workspace's no-panic rule
  treats unchecked arithmetic as a bug (use
  `checked_*` / `saturating_*` / `wrapping_*` to declare
  intent explicitly); silent wrap in release would mask
  rule violations. Combined with `panic = "abort"`,
  unchecked overflow → abort, which is exactly what
  the no-panic rule wants surfaced.

When you add a new published crate to the workspace, copy
this block verbatim into its `Cargo.toml`. Same when
adopting an existing remote crate as a submodule
(`./scripts/new-submodule.sh --adopt-existing`); the
adopted Cargo.toml MUST end up with this block before the
adoption commit lands.

---

## 4. Git workflow

**All Git state-changing operations go through `scripts/*.sh`,
not raw `git`.** The scripts encode submodule-first ordering,
mandatory `-s` sign-off, mandatory `-S` signature, detached-HEAD
guards, and the audit-trailer workflow. Ad-hoc commands drift
from those defaults.

**Read-only git is also soft-banned for the workspace-wide
queries** — raw `git status` / `git log` / `git diff` see only
the current repo and miss the workspace's submodule layout.
Use the wrappers instead:

- `./scripts/status.sh` for `git status` (parent + every
  submodule; `--diff` adds the per-repo diff for dirty files).
- `./scripts/heads.sh` for `git log -n 1` (HEAD per repo, with
  signature char).
- `./scripts/log.sh` for general `git log` (three modes:
  history / audit / stats — see §4.1 below).

Bespoke `git show`, `git blame`, `git rev-parse` (and other
narrow queries that don't have a workspace-wide
counterpart) remain fine raw; the soft-ban targets the three
queries with workspace-aware wrappers.

### 4.1 The scripts

- `setup.sh` — one-time (or post-fresh-clone) init.
  Initialises every submodule recursively; configures the parent
  and every submodule with `push.recurseSubmodules=check`,
  `core.hooksPath` pointing at `.githooks/` (relative path in
  submodules, computed via `scripts/lib/relpath.sh`),
  `commit.gpgsign=true`, `tag.gpgsign=true`, and
  `rebase.gpgsign=true`; then attaches each submodule's HEAD to
  its tracked branch (`branch = ...` field in `.gitmodules`,
  default `main`) via `scripts/lib/attach-submodule-branch.sh`,
  because `git submodule update --init` checks out the recorded
  SHA as a raw commit and would otherwise leave every submodule
  detached on a fresh clone — which trips `check-detached.sh`
  and `commit-all.sh` on first contributor edit. Off-branch pins
  and unsafe attaches (would require dropping local commits) are
  warned and left detached. Warns if Rust isn't on PATH.
- `status.sh [--diff]` — working-tree status of the parent +
  every submodule (clean submodules are hidden). `--diff`
  appends a colored diff (unstaged + staged) per repo for
  every dirty file; replaces raw `git diff` for workspace-wide
  inspection.
- `pull-all.sh` — rebase-pull the parent and update each
  submodule to the tip of its tracked remote branch, then re-run
  the same `attach-submodule-branch` pass `setup.sh` uses
  (`git submodule update --remote --rebase` silently degrades
  to a plain checkout when HEAD was already detached, so it does
  not re-attach on its own). Does *not* commit bumped pointers.
  See §4.4 for the rebase-on-pull exception.
- `commit-all.sh [--anonymize] [--parent-only] [--dry-run] [--exclude <path>]... [message]` —
  commit pending changes. Walks each dirty submodule first
  (committing with `-s -S`), then the parent. Default message
  is `"updates"`. `--parent-only` skips the submodule walk for
  parent-only work (docs, scripts, ROADMAP). `--dry-run` walks
  each dirty repo and prints `git status --short` for it without
  staging, committing, signing, computing the audit trailer, or
  running the settings.json guard — a read-only preview of what
  the real invocation would sweep in. Recommended pre-flight
  whenever the working tree's contents are uncertain (e.g. after
  a Codex dispatch). `--exclude <path>` (repeatable) holds a
  workspace-root-relative parent-repo path back from the commit:
  `git add -A` runs as usual, then each excluded path is
  unstaged with `git reset HEAD -- <path>`, leaving the
  working-tree change dirty for a later commit. Applies only to
  the parent commit phase; the submodule walk ignores `--exclude`.
  Paths must be whitespace-free. Typical use: hold `Cargo.lock`
  back from a parent-only doc commit so it lands with the
  corresponding submodule version-bump commits later. Codex
  itself never runs `commit-all.sh`, including `--dry-run` — the
  codex-guard sources at the top of the script abort under any
  Codex ancestor process.
- `push-all.sh` — push each submodule's current branch, then
  the parent. Aborts before the parent push if any submodule
  push fails, so origin never sees an unresolvable parent
  pointer.
- `heads.sh` — short-SHA / `%G?` / subject for parent +
  every submodule in one pass. Canonical replacement for raw
  `git log -n 1` across repos.
- `log.sh [--history|--audit|--stats] [-n <N>|--count <N>] [<submodule-path>]` —
  unified pretty-printed git-log front-end, three modes
  (mutually exclusive; `--history` is the default). Replaces
  the retired `git-log.sh` / `audit-log.sh` / `stats-log.sh`.
  Default target is the parent workspace repo; pass a
  submodule path relative to the workspace root (e.g.
  `mechanics-core`, `philharmonic-types`) to inspect that
  submodule's own history.
  - `--history` (default; previously `git-log.sh`): columns
    `<sha> <date> [<%G?>] [<sign-off-label>] <author> |
    <subject>`. Default count: 500. Sign-off label matches
    `Signed-off-by:` trailers against author email (`%ae`)
    so imported patches, co-author-only sign-offs, and
    genuine DCO violations are distinguishable. Audit the
    sign-off + signature invariants from §4.3 with
    `./scripts/log.sh | grep -E '\[(N|NOT signed-off)\]'`
    for the parent; loop over submodule names (or
    `git submodule foreach 'cd $toplevel && ./scripts/log.sh
    "$name"'`) to cover the whole workspace.
  - `--audit` (previously `audit-log.sh`): columns
    `<sha> <ISO> [<%G?>] [<sign-off-label>] <author>
    | -<del> +<ins> | <Audit-Info trailer>`. Default count: 200.
  - `--stats` (previously `stats-log.sh`): columns
    `<sha> <ISO-with-tz> <author> | <files>F <lines>L
    (<code>C <docs>D) | Δ +<files>F +<lines>L (+<code>C
    +<docs>D)`. Default count: 200. Falls back to
    `docs/stats-cache.tsv` for parent-repo commits whose
    `Code-stats:` trailer is absent (pre-trailer-adoption
    history).
  Rejects paths that aren't a git-repo root, so subdirectories
  of the parent and the in-tree `xtask/` member (which share
  the parent's history) don't accidentally masquerade as
  submodules. Requires git ≥ 2.32.
- `check-detached.sh` — fails non-zero if any submodule is in
  detached HEAD. Pre-flight for `commit-all.sh`.
- `show-dirty.sh` — one-per-line list of dirty submodule names.
- `archive-all.sh` — bundle the parent + every submodule's
  `HEAD` tree into a single zstd-compressed tarball at
  `archives/philharmonic-workspace-<HEAD_SHA>.tar.zst`. Read-only
  with respect to git state — runs `git archive HEAD` for the
  parent and `git submodule foreach --recursive` for each
  submodule, prefixes every entry with `philharmonic-workspace-
  <HEAD_SHA>/<displaypath>/`, and concatenates the per-tree
  uncompressed tarballs into one zstd-compressed output via the
  `tar-concatenate` xtask bin. Tempfiles come from
  `scripts/mktemp.sh` and are removed on any exit path. Aborts
  early on uninitialized submodules (a partial bundle would be
  silently incomplete). HEAD-only — uncommitted working-tree
  changes are not captured. Output dir is tracked
  (`archives/README.md`); generated `*.tar.gz` and `*.tar.zst`
  inside it are git-ignored.
- `backfill-stats.sh [--allow-partial] [--dry-run] [-n <N>]` —
  one-shot backfill of `Code-stats:`-equivalent rows for parent
  commits older than the trailer's adoption (everything before
  `46a7d29`). Reads-only on the working tree: reconstructs each
  pre-trailer commit by `git archive`-ing the parent and every
  gitlink-pinned submodule SHA into a /tmp scratch dir, runs
  `tokei`, and appends a sha-keyed row to `docs/stats-cache.tsv`
  (the tracked sidecar that `log.sh --stats` consults as a
  fallback when a commit's trailer is absent). Includes a rename
  heuristic — if the recorded path is missing in the current
  workspace, scans every initialized submodule for the SHA and
  uses that submodule as the source while extracting under the
  historical path. Bails by default on truly unreproducible
  commits (zero or multiple matches, force-pushed-away SHAs,
  etc.) with a workaround pointer; `--allow-partial` records a
  comment-line marker and continues. Resumable via the cache:
  rows already present are skipped on rerun.

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

**Preview the sweep with `--dry-run` first when the tree's
contents are uncertain.** `./scripts/commit-all.sh --dry-run`
(optionally combined with `--parent-only`) walks each dirty
repo and prints `git status --short` for it. No staging, no
signing, no temp message file, no `Audit-Info` / `Code-stats`
computation, no settings.json guard — purely read-only. Use it
after a Codex dispatch (or any other batch of edits) so the
real `commit-all.sh` invocation doesn't sweep in unintended
files (a stray `Cargo.lock` regen, an unrelated doc edit, a
new untracked report, etc.). The submodule walk uses the same
`git status --short` shape per submodule, so the dry-run
output is the authoritative preview of what `git add -A` would
stage on the next pass.

**Hold specific parent-repo paths back with `--exclude`.** When
the dry-run reveals a file you want kept out of the commit
(typically `Cargo.lock` after a submodule version bump that
hasn't landed yet), pass `--exclude <path>` per file. The flag
is repeatable, the path is workspace-root-relative, and
whitespace in paths is rejected at parse time. Implementation:
`git add -A` runs as normal, then each excluded path is
unstaged with `git reset HEAD -- <path>`, leaving the working
tree dirty so the file is available for a follow-up commit.
This is the mechanized form of the "move them to /tmp and
restore after" pattern above; prefer it for routine cases like
`Cargo.lock`. The flag applies only to the parent commit phase
— the submodule walk doesn't honor it. If `--exclude` removes
every staged change so the parent commit would be empty, the
script aborts with an explicit error rather than producing an
empty commit attempt. Under `--dry-run`, the excluded paths
are listed beneath the would-commit status block so the
preview matches the planned scope.

**Codex itself never runs `commit-all.sh`**, including
`--dry-run`. `scripts/lib/codex-guard.sh` is sourced at the
top of the script and walks the ancestor process tree; if
any process name matches `*codex*` (case-insensitive), the
script aborts before doing anything. Codex's contract is
"leave the tree dirty for Claude" (AGENTS.md §Don't commit);
the dry-run preview is Claude's tool, not Codex's.

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
   historical note below explains why this script-owned rebase
   path exists. Do not run `git pull --rebase` or `git rebase`
   outside `pull-all.sh`; the exception is for the script, not
   for the subcommand.

   Historical note: the alternatives originally considered each
   broke another workspace invariant. `--ff-only` failed when
   local commits existed and gave the wrapper no recovery path;
   default merge produced merge commits and required pre-commit
   hook plumbing; default submodule checkout detached HEAD and
   broke `commit-all.sh`'s detached-HEAD guard.

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
- Force-pushing a rewritten history through ~24 submodules
  means every other clone has to untangle itself; the cost is
  not yours alone.
- Mistakes ship as **new commits** via fix-forward — a new
  commit that makes the state right. The imperfect earlier
  commit stays in the log; the fix lives on top.

**`git revert` is also forbidden.** Even though it creates a new
commit and so respects the letter of append-only, the "undo"
framing clutters the log and turns history review into
bookkeeping about which past commits are still live versus
which have been rescinded. Fix-forward with a real change
instead: make a commit that brings the code to the state it
should be in, and let the earlier imperfect commit stand as
part of the honest record. The log's job is to tell what
happened; revert commits pretend otherwise.

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
  fix-forward — make a new commit that brings the code to the
  state it should be in, and push that. No `git revert`, no
  amend, no rebase. Live with the imperfect earlier commit in
  the log.

If you find yourself reaching for amend / rebase / reset / revert
for a "legitimate" reason that isn't in the list above, stop and
surface it. This rule has no quiet exceptions.

**Push early, push often — mid-work pushes are encouraged.** The
local hook layer (§4.5) and the GitHub-side ruleset (§4.8) both
accept work-in-progress commits on `main` as long as they're
signed, signed-off, and not force-pushes. Do not hoard commits
locally out of a "wait until it's clean" habit — in an
append-only world a lost local commit has no amend/rebase
recovery path, so a disk failure or an accidental clone wipe
takes unique work with it. Push the imperfect commit, push the
fix-forward on top, push the next fix-forward. The log tells the
honest story and origin is the backup; that's the whole
contract.

**Per-step commit-and-push for AI agents.** Claude Code's
default cadence is: finish a discrete unit of work →
`./scripts/commit-all.sh "..."` → `./scripts/push-all.sh` →
start the next unit. Every sensible-sized step lands as its
own commit-and-push, not at the end of the session. "Sensible
size" is small enough that a reader can take it in at one
sitting (a doc reconciliation, a script fix, one cohesive
refactor) without being so small that a single coherent change
splinters across three commits. Do not batch unrelated topics
into one commit and do not let pushes queue up locally between
steps — a session that crashes mid-flight should leave a
clean origin trail of completed steps, not a pile of unpushed
work. The two narrow exceptions: (a) a sequence whose
intermediate states wouldn't compile or pass `pre-landing.sh`
— land the whole sequence as one commit; and (b) edits the
user is actively iterating on in conversation, where the next
turn might revise them — wait for the user to say "looks
good" or otherwise signal closure on that step before
committing. When in doubt, ask the user if a piece of work is
its own step or part of a larger one.

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
timestamp, working-directory absolute path, hostname,
user/uid, group/gid, public IPv4+v6 with geolocation (queried
once per invocation from `ipv4.icanhazip.com` /
`ipv6.icanhazip.com`, not per submodule), kernel/release,
arch, OS, virtualization id (`v=` field — output of the
`detect-virt` xtask bin; `kvm`, `docker`, `lxc`, `wsl`,
`none`, etc., matching systemd-detect-virt(1)'s vocabulary),
Rust toolchain version, CPU thread count, and
available/total memory bytes. Produced by
`scripts/print-audit-info.sh`, parsed as a standard git
trailer (`git log --format='%(trailers)'`).

Pass `--anonymize` to `commit-all.sh` to replace the IPv4 and
IPv6 fields with `hidden/ZZ` while keeping the rest. Host, user,
kernel, and OS are always recorded — the audit line's purpose
is cross-checking "which machine produced this" against a
local-state map. It's not a substitute for the DCO or the
signature.

Inspect with `./scripts/log.sh --audit` (one line per commit —
hash, timestamp, signature/sign-off verdicts, author, diffstat,
and the Audit-Info trailer).

### 4.7 Code-stats trailer

`commit-all.sh` also writes a `Code-stats:` trailer recording a
snapshot of workspace size at commit time:

```
Code-stats: 812 files, 135719 total lines (68821 code lines, 48917 docs lines)
```

Produced from `./scripts/stats.sh` (which parses the Total row
from `./scripts/tokei.sh`). Per-commit deltas are not stored —
each trailer is an absolute snapshot — but
`./scripts/log.sh --stats` walks the log and computes deltas
against each commit's predecessor for at-a-glance growth
review. Because `commit-all.sh` uses one shared commit-message
file for its submodule-first walk and the parent commit,
submodule commits made by that invocation carry the same
`Code-stats:` trailer too; the numbers are still a parent
workspace snapshot, not per-submodule counts.

Historical note: commits before the `Code-stats:` trailer was
adopted (everything before `46a7d29`) carry no in-commit stats.
Coverage is restored out-of-band by
[`./scripts/backfill-stats.sh`](scripts/backfill-stats.sh), which
reconstructs each pre-trailer commit's tree (parent + every
submodule's gitlink-pinned SHA) into a /tmp scratch dir, runs
`tokei`, and appends a sha-keyed row to the tracked sidecar
cache at [`docs/stats-cache.tsv`](docs/stats-cache.tsv).
`scripts/log.sh --stats` consults that cache as a fallback when a
commit lacks the trailer — so the SVG and the textual log carry
real numbers + deltas across the trailer-adoption boundary
without the append-only-history rule needing to bend.

### 4.8 GitHub-side ruleset (parent workspace repo only)

The local hooks described in §4.5 are defence-in-depth, but they
run on the contributor's machine and can be bypassed
(`--no-verify`, deleted hooks, a reach-around commit from a stale
clone). A server-side backstop closes that gap for anything that
actually reaches GitHub. Applies to the parent
[`metastable-void/philharmonic-workspace`](https://github.com/metastable-void/philharmonic-workspace)
repository only — submodule repositories do not currently carry
matching rulesets.

**Ruleset: `Safety rules`**, target **branch**, scope **`~ALL`**
(every branch, not just `main`), enforcement **active**,
**no bypass actors** (including repo admins — `current_user_can_bypass: "never"`).
Three GitHub-native rules are on:

- **`required_signatures`** — every commit pushed to the repo
  must carry a valid GPG/SSH signature GitHub can verify. This
  is the server-side mirror of §4.3 + `.githooks/pre-push`; it
  catches anything the local layer missed. Imports (cherry-pick,
  merge) that carry an unsigned commit are rejected at push
  time.
- **`non_fast_forward`** — prevents force-pushes and any push
  that would rewrite history on the receiving branch. Server-side
  mirror of §4.4. The append-only invariant is enforced at the
  ref-update layer — not just by the client's voluntary discipline.
- **`deletion`** — branches (including feature branches) cannot
  be deleted through the API or UI. Prevents accidental (or
  deliberate) removal of commit reachability.

**Intentionally not enforced server-side:**

- **DCO / `Signed-off-by:` trailer.** GitHub's native ruleset
  grammar has no DCO rule type. A GitHub Actions DCO check exists
  but is not installed — the workspace relies on `commit-msg` +
  `pre-push` for sign-off enforcement, and the trailer is verified
  post-landing via `scripts/log.sh`'s `[signed-off]` /
  `[NOT signed-off]` labelling (see §4.5 and the script's header).
  If sign-off enforcement ever needs a server-side backstop, it
  will be a workflow / app addition, not a ruleset rule.

**Submodule repos: not covered, on purpose for now.** Matching
rulesets on every submodule repo is manual one-off work and a
maintenance cost per new submodule. The expected defence for
submodules today is the local hook layer (§4.5) — every
submodule inherits `core.hooksPath` via `setup.sh`, so
`pre-push` runs for submodule pushes too. Extending the
server-side ruleset to submodules is a deliberate follow-up when
the submodule list stabilises; adding it now would need a
rollout strategy each time a new submodule is added.

**Changing the ruleset.** Modifications go through
`gh api --method PUT repos/metastable-void/philharmonic-workspace/rulesets/<id>`
(or the GitHub web UI) and are audited on the GitHub side. Mention
the change in the same commit that touches this section so the
in-repo documentation stays aligned with what's actually
configured. There is no CI check that cross-validates the two
today; drift is caught by reviewing this section against
`gh api repos/metastable-void/philharmonic-workspace/rulesets/<id>`
output when anything changes.

### 4.9 Other git rules

- **Don't invoke `git log -n 1`** to list HEAD state across the
  workspace — use `./scripts/heads.sh`. Raw `git log` remains
  fine for history browsing (`git log <path>`, `git log
  --oneline`, etc.); the rule targets specifically the "show
  current HEAD commit on each repo" pattern.
- **If a script doesn't cover your case, extend the script**
  (and update this file) rather than reaching for raw git. This
  is the whole point of the wrapper-only rule.

### 4.10 Commit message format

Standard git format. Mandatory for both human-authored and
agent-authored commits.

- **First line: concise executive summary, ≤ 72 characters.**
  Imperative voice (`add X`, `fix Y`, `update Z`), no trailing
  period. The subject is what shows up in
  `./scripts/heads.sh`, `git log --oneline`, GitHub PR titles,
  Slack webhook digests, and the LLM summarizers — long subject
  lines waste tokens and bury the headline. If you need more
  than 72 chars to express the change, split into subject +
  body.
- **Blank line separating subject from body.**
- **Body wraps at ≈ 72 columns.** No 1000-character paragraphs
  on a single physical line. Hard-wrap by hand or in your
  editor; the goal is readable `git log` / `git show` output
  in a standard 80-column terminal.
- **Body covers what's not in the subject** — per-file scope,
  rationale, threat model implications, residual risks, links
  to design docs / Codex prompt files / ROADMAP §s. Bullets
  are fine; prose is fine; the only fixed shape is the wrap
  width.

The `Signed-off-by:` and `Audit-Info:` trailers (added by the
hooks; see §4.3 + §4.6) live at the bottom, separated from
the body by a blank line. The `Code-stats:` trailer (§4.7)
sits next to the audit trailer.

Why this matters: the workspace's auto-grounding hook,
push-time LLM summarizers, and human reviewers all read
subjects first; long single-line bodies-as-subjects defeat
every one of them and burn context tokens unnecessarily.

#### Passing the message to `commit-all.sh`

**Always pass commit messages via `--message-file`, never as a
positional argument.** Two viable surfaces under the workspace's
`rexec`-mandated execution model:

- **Canonical: `--message-file <path>`.** Body on disk; the
  agent hands `rexec` a file path. Auditable, immune to
  outer-quote slip-ups, and the path can be re-read by the
  agent or by a reviewer.
- **Alternative: `--message-file -` with `rexec --read-stdin`**
  (rexec v0.1.1+; if `rexec --version` reports v0.1.0, fall
  back to the canonical form or ask Yuka to upgrade). Pipes
  the heredoc body through `rexec` to the script's stdin in
  one shot. Use this when the message is short and the
  tempfile is genuine ceremony.

Only Claude commits in this workspace. Codex never commits
(the `codex-guard` in `scripts/lib/codex-guard.sh` hard-aborts
`commit-all.sh` under any `*codex*` ancestor process —
[§4 Git workflow](#4-git-workflow)). All Claude command
execution routes through `rexec` ([Hard]; see
[CLAUDE.md §"Command execution via `rexec`"](CLAUDE.md); no
direct-terminal exception).

**Canonical recipe (Claude Code via `rexec`):**

1. Use the editor's `Write` tool to drop the message into a
   path under `/tmp` (the env block lists `/tmp` as an
   additional working directory, so writing there is
   allowed):
   ```
   Write file_path=/tmp/<slug>-commit-msg.txt content=<the body>
   ```
2. Invoke the commit through `rexec`:
   ```sh
   rexec --whoami <agent> --dir <workspace-root> -- \
       ./scripts/commit-all.sh --message-file /tmp/<slug>-commit-msg.txt
   ```
3. Clean up:
   ```sh
   rexec --whoami <agent> --dir <workspace-root> -- \
       rm -f /tmp/<slug>-commit-msg.txt
   ```

The message body lands on disk verbatim — Bash never parses
it as a shell argument or as command-substitution input, so
backticked `identifier` / `path/like/this` / `command(...)`
tokens and `$VAR` / `$(cmd)` references all survive as
literal text.

**Alternative recipe (`--read-stdin`, single invocation):**

```sh
rexec --whoami <agent> --dir <workspace-root> --read-stdin -- \
    ./scripts/commit-all.sh --message-file - <<'EOF'
subject line ≤ 72 chars

body paragraph hard-wrapped at ≈ 72 cols. Backticked
`tokens`, `$VAR` references, and `$(cmd)` substitutions
all survive because the single-quoted `<<'EOF'` delimiter
suppresses shell expansion inside the heredoc body.
EOF
```

Two load-bearing pieces:

1. `rexec --read-stdin` (v0.1.1+) reads the client's stdin
   to EOF and forwards it to the inner child. Without this
   flag, the inner child's stdin is the PTY slave and reads
   hang because nothing is written to it.
2. The single-quoted `<<'EOF'` heredoc delimiter suppresses
   shell expansion inside the heredoc body. A bare `<<EOF`
   would still expand backticks and `$VAR` inside the body
   itself; the single quotes are mandatory.

**Why the path form is still canonical:**
- It's auditable: the body lives on disk and can be
  re-read with `Read` or `cat` to confirm what landed in
  the commit.
- It survives the "one missing outer quote" failure mode
  that the legacy `"$(cat <<'EOF' ... EOF)"` form is
  vulnerable to — the path form has no quoting boundary at
  all between the agent and the script.
- It's the same shape whether or not `rexec` is in the
  middle, which makes it portable across execution surfaces.

The `--read-stdin` form is the better choice when the
message is short, the tempfile would just be ceremony, and
you're confident in the heredoc quoting. Both forms are
legitimate; pick by readability.

**Legacy positional form** (still accepted by the script, but
fragile; do not use):

```sh
./scripts/commit-all.sh "$(cat <<'EOF'
…
EOF
)"
```

This works in current bash but re-introduces a quoting
boundary — a missing outer `"`, a stray `"` inside the
body, or an unusual quote-removal semantics in some shell
could re-introduce the expansion failure. Both
`--message-file` surfaces above remove the boundary
entirely.

Why all this discipline: with the legacy positional form
(or any plain `"message"` argument), bash silently:

- runs backticked spans as command substitution (so an
  innocuous \`build = "build.rs"\` in the body becomes the
  empty string at best, or `command not found` stderr at
  worst);
- expands `$VAR` references to their host values;
- runs `$(...)` as command substitution.

All silent from the committer's perspective —
`commit-all.sh` exits 0 and the commit just loses the
expanded tokens. **Combined with the append-only history
rule (§4.4)**, a mangled message is unfixable except via a
fix-forward errata note. `--message-file` is the easy
mechanical guardrail; use it unconditionally.

Same rule applies to any other script that takes a free-form
message argument (today only `commit-all.sh`). If a future
wrapper script grows a message argument, give it a
`--message-file <path>` (with `-` for stdin) sibling and
recommend the path form as the canonical surface — it's the
one that survives `rexec` without ceremony.

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
| `./scripts/pre-landing.sh [<crate>...] [--no-ignored] [--dry-run] [-v\|--verbose]` | `cargo deny check bans` + `cargo fmt` + `cargo check` + `cargo clippy --fix --allow-dirty --allow-staged --all-targets -- -D warnings` + `cargo test --workspace` + `cargo test --ignored -p <crate>` per modified crate | The canonical pre-commit flow. Auto-detects modified crates via `show-dirty.sh`. Default lint step is fix mode (autofixes fmt + clippy in place against a dirty tree, see §11.0.2); pass `--dry-run` for legacy check-only behaviour. Default `cargo-deny` invocation hides the inclusion graph for less noise; pass `-v` / `--verbose` to print it. CI runs the same script. See §11. |
| `./scripts/rust-lint.sh [<crate>] [--fix] [--phase <p>] [--quiet] [--target <triple>]` | `cargo fmt [--check]` + `cargo check` + `cargo clippy --all-targets -- -D warnings` + `cargo doc --no-deps` (rustdoc with `RUSTDOCFLAGS=-D missing_docs`) | Workspace (no arg) or per-crate. `--phase fmt\|check\|clippy\|doc` runs one of the four phases instead of all. `--quiet` propagates `--quiet` to cargo check/clippy/doc (suppresses "Compiling X v0.1.0" chatter; errors still surface). `--target <triple>` cross-compiles via cargo check / clippy / doc (fmt is source-level and skipped); requires `rustup target add <triple>` first. Useful for surfacing cfg-gated dead-code warnings on alternate platforms (e.g. `x86_64-unknown-freebsd` when most probe paths are `cfg(target_os = "linux")`-gated). |
| `./scripts/rust-test.sh [--include-ignored\|--ignored] [<crate>] [--filter <pat>] [--features <list>] [--no-default-features\|--all-features] [--release] [--quiet]` | `cargo test` with ignored-test control, name filter, feature selection, and release-mode toggle | `--ignored` runs *only* `#[ignore]`-gated; `--include-ignored` runs everything. `--filter <pat>` is cargo's positional substring test-name filter. `--features` / `--no-default-features` require a positional crate or `--xtask`. |
| `./scripts/miri-test.sh --workspace \| <crate>...` | `cargo +nightly miri test` | Slow; not in `pre-landing.sh`. See §10.7. |
| `./scripts/cargo-audit.sh [...]` | `cargo audit` | Auto-installs `cargo-audit` on first run. |
| `./scripts/cargo-deny.sh [...]` | `cargo deny check bans` by default; pass through to other subcommands (`check all`, `check licenses`, …). Auto-installs `cargo-deny` on first run. Step 1 of `pre-landing.sh`. |
| `./scripts/check-api-breakage.sh <crate> [<baseline>]` | `cargo semver-checks check-release -p <crate> --baseline-version <ver>` | Per-crate; crates.io baseline (default: newest published). See §12.3. |
| `./scripts/publish-crate.sh [--dry-run] <crate>` | `cargo publish -p <crate>` + signed release tag | Enforces clean tree, branch-HEAD, no-existing-tag invariants. Tag created only on publish success. |
| `./scripts/verify-tag.sh <crate> [<tag>]` | Three-way check that a release tag is locally present, signed, and on origin at the same commit | Run after `publish-crate.sh` + `push-all.sh`. See §12.4. |
| `./scripts/crate-version.sh <crate> \| --all` | Parses `version = "..."` from `<crate>/Cargo.toml` | Single-crate for programmatic use; `--all` prints every workspace member's version. |
| `./scripts/xtask.sh crates-io-versions -- <crate>` | crates.io sparse-index query | Lists non-yanked published versions. Rust bin in `xtask/`. |
| `./scripts/xtask.sh <tool> -- <args>` | wrapper for in-tree Rust bins | Canonical invocation for any `xtask/` bin; mandatory `--` separates wrapper-level flags from bin args. |
| `./scripts/check-toolchain.sh [--update]` | `rustup check` / `rustup update` + version print | Step 0 of `pre-landing.sh`. |
| `./scripts/check-no-registry.sh` | Refuse `registry = "..."` directives in any tracked `Cargo.toml` | Pre-publish guard: registry-routing belongs in `.cargo/config.toml`, never in published manifests. Run in workspace-level `pre-landing.sh` flow. Exits 0 clean / 1 with offending lines. |
| `./scripts/cargo-install.sh <crate>[@<ver>]... \| --setup \| -h\|--help` | `cargo binstall` for workspace tooling | Workspace-wide front door for installing cargo binaries. Bootstraps `cargo-binstall` itself via `cargo install --locked cargo-binstall` on first use, then forwards subsequent calls to `cargo binstall`. `--setup` does the bootstrap only (used by `setup.sh`). `--help` prints the inline usage block. Read-only `CARGO_HOME` exits 0 with a warning rather than failing — CI legs that only sanity-check scripts can call it harmlessly. Callers: `cargo-audit.sh`, `cargo-deny.sh`, `check-api-breakage.sh`, `setup.sh`, `tokei.sh`, and `.github/workflows/ci.yml`. |
| `./scripts/mdbook-build.sh [--clean] [--check]` | `mdbook build` (or `mdbook test`) | Wraps mdbook against the workspace's `book.toml`. `--clean` removes `book/` first; `--check` runs `mdbook test` for link/anchor checks. mdbook is installed via `setup.sh`; the `book/` directory is committed so consumers can browse the rendered docs without an mdbook install. |
| `./scripts/find-cargo-registry.sh <crate-substr> [<glob>] \| --list \| --root` | `find ~/.cargo/registry/src` with the workspace's canonical depth + glob | Locates vendored crate source files for upstream-behaviour investigations (e.g. how `sqlx-postgres` parses a connection URL, whether `ring`'s error implements `Debug`). `--list` enumerates every vendored crate directory; `--root` prints the resolved `registry/src/<index>/` root. Output paths are `~`-prefixed for readability. |

**Target-dir split**: every cargo-touching wrapper sources
`scripts/lib/cargo-target-dir.sh`, which sets
`CARGO_TARGET_DIR=target-main` unless the caller already set
it. This keeps CLI / CI / Codex builds in `target-main/`,
separate from `rust-analyzer`'s default `target/`.
`xtask.sh` uses `target-xtask/`; `publish-crate.sh` uses
`target-publish/`. The split eliminates the "Blocking waiting
for file lock on build directory" stall that occurs when two
cargo processes share `target/`. **If you must run cargo
outside a wrapper, prefix the command with
`CARGO_TARGET_DIR=target-main`.**

**Tmpfs redirect for `target-main`**:
`cargo-target-dir.sh` also turns `target-main` into a
symlink pointing at `/tmp/philharmonic-$(id -u)-target-main`
on first run, so workspace builds land on the host's `/tmp`
filesystem (typically a large tmpfs on the dev box) rather
than the persistent project filesystem. The setup is
idempotent and per-uid scoped; on hosts where `/tmp` isn't
tmpfs the symlink still works correctly, just with no RAM
benefit. The redirect is invisible at the cargo level —
`CARGO_TARGET_DIR=target-main` resolves through the symlink
exactly like the bare directory it replaces. The `xtask`
and `publish` target dirs are not redirected; they're
typically small enough that the tmpfs win isn't worth the
extra symlink mechanism.

**Internal-only scripts** (not in the inventory above; brief
mentions for completeness — they exist for operator / report
purposes and aren't part of the day-to-day contributor flow):

- `./scripts/hf-fetch-embed-model.sh` — deployment-build-time
  fetch of an embedding-model ONNX + tokenizer bundle from
  HuggingFace, pinned by revision SHA. Thin wrapper around the
  `hf-fetch-embed-model` xtask bin (see
  `xtask/src/bin/hf-fetch-embed-model.rs` for full
  documentation). Not invoked at runtime; the connector that
  consumes the bytes (`philharmonic-connector-impl-embed`) has
  no network code.
- `./scripts/slack-webhook.sh` — posts a project-summary Slack
  message generated from `README.md` + `ROADMAP.md` + recent
  `git log` for the workspace-internal Slack channel.
  Operator-facing.

**Exempt**: read-only cargo queries have no wrapper and don't
need one — `cargo tree`, `cargo metadata`, `cargo --version`,
`cargo search` are fine to run raw.

**Specifically soft-banned (raw forms)**: `cargo check`,
`cargo test`, `cargo fmt`, `cargo clippy`, `cargo doc`. All
five are covered by `rust-lint.sh` (with `--phase` to scope
to one) and `rust-test.sh` (with `--filter` / `--features` /
`--release` for common bespoke cases). The previous
"bespoke invocations remain fine" carve-out is removed —
when the script doesn't cover your case, surface the gap as
a prompt-override and extend the script.

**If no wrapper fits**: extend one, or add a new `scripts/*.sh`
(see §6). Validate with `./scripts/test-scripts.sh`. Then use
the new wrapper — don't fall back to raw cargo because the
wrapper doesn't exist yet.

### 5.1 Build status monitoring

When a cargo build, test, or clippy run appears stuck (no
output for minutes), run `./scripts/build-status.sh` to see
what's actually happening. It scans running processes for
`rustc`, `rust-lld`, `clippy-driver`, `miri`, `rustfmt`, and
`rustdoc`, and reports which crate each is processing, with
PIDs and elapsed times.

```sh
./scripts/build-status.sh              # one-shot snapshot
watch -n 2 ./scripts/build-status.sh   # continuous polling
```

Long silences are normal for large crates (LTO, bge-m3 model
embedding, aws-lc-rs C compilation). `build-status.sh`
distinguishes "still compiling" from "actually stuck" so agents
don't abort builds prematurely.

### 5.2 Crate version lookup

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

### 5.3 Extract routines into scripts

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

### 5.4 Crate-version cooldown

The workspace's [`/.cargo/config.toml`](.cargo/config.toml)
points the resolver at a **3-day cooldown mirror** —
`https://index.crates.menhera.org/3d/` — so a brand-new
crates.io release isn't pickable as a dependency until it has
aged 3 days. The rationale is the standard one: defend against
fast-yanked releases, post-publish typos, and zero-day
supply-chain compromises by never being in the first wave of
consumers.

Effects:

- **Adding or bumping a third-party dep** must pick a version
  that's already past the 3-day window. Before committing the
  `Cargo.toml` change, sanity-check via `./scripts/xtask.sh
  crates-io-versions -- <crate>`; the tool prints a stderr
  warning of the form `!! crates-io-versions: <version> is
  <Nd|Nh> old (< 3d threshold)` for any version inside the
  window. If the latest is in-window, pin to the prior version
  or wait.
- **Publishing through `./scripts/publish-crate.sh` deliberately
  bypasses the cooldown** for its own verify-build step — it
  uses the `cargo pub-fresh` alias, which redirects the
  source-replacement to the same Menhera proxy's `/0d/`
  no-cooldown endpoint. This is what makes a same-day cascade
  possible (e.g., publishing crate B that depends on a
  workspace-internal crate A's brand-new version, on the same
  day A was published). Normal builds — including consumers
  who pull our just-published crates from crates.io — still go
  through the `/3d/` endpoint, so the 3-day window protects
  downstream while not blocking workspace cascades.

**First-party exception**: the 3-day cooldown does not apply
to crates *authored within this workspace*. The cooldown's
threat model is defending against unknown-third-party releases;
for our own workspace-internal crates (every member crate of
this `Cargo.toml`), the source is in-tree, the publishing flow
is the workspace's own `publish-crate.sh`, and there's no
"unknown surprise" risk to defend against. Pinning embed to a
2-hour-old `inline-blob 0.1.0` is fine when both crates are
ours; the same pin against a third-party crate isn't.

The `crates-io-versions` warning still fires for first-party
crates inside the 3-day window — that's correct (the tool
doesn't and shouldn't know which crates we authored). Treat
the warning as **informational, not blocking** for our own
crates; document the exception in the relevant prompt's
Outcome section so the deviation is durable.

### 5.5 Reclaiming tmpfs when `/tmp` fills up

`scripts/lib/cargo-target-dir.sh` redirects `target-main`
to a per-uid `/tmp/...` symlink (see the "Tmpfs redirect"
paragraph at the top of §5). On a long-running session
the debug build cache there can grow into the
many-gigabyte range — fast for cargo, but eventually
`/tmp` runs low and every cargo run starts erroring on
the next file write.

**Recovery: `./scripts/clean-target-debug.sh`.**
The script `rm -rf`s exactly `target-main/debug` —
the bulk of the cache. It deliberately spares:

- `target-main/release` (release builds you may want
  to keep around between sessions).
- `target-main/doc` (rustdoc output, slow to rebuild).
- Anything else cargo wrote under `target-main/` for
  a non-debug profile.
- `target-xtask/` (xtask's separate target dir; not
  affected by the `target-main` tmpfs symlink).
- The cargo registry / git cache under `$CARGO_HOME`.

The next debug build is a cold rebuild after this —
slow first compile, normal incrementals after. That's
the trade vs. a `df -h /tmp` exhaustion that would
break every subsequent cargo run.

Run before kicking off heavy work if `df -Pk /tmp` shows
the tmpfs capacity above ~80% — the script itself emits
that same `df -Pk /tmp` line before and after the
deletion so the win is visible. (`df -h` is more
human-readable but is a GNU/BSD extension; `df -Pk`
gives portable POSIX output. Run `df -h /tmp` yourself
interactively if you prefer the human-readable form;
the script sticks to `-Pk` to stay POSIX per §6.) The
resource-pressure xtask covers CPU / load / RAM / swap
but not filesystem fill, so `df` is the direct signal
for the tmpfs target dir.

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

**Reference checklist**: see [`POSIX_CHECKLIST.md`](docs/POSIX_CHECKLIST.md)
under `docs/` for a detailed inventory of non-POSIX
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

- **`gzip`** (`scripts/release-build.sh`). SUSv4 specifies
  `compress`, not `gzip`. However `gzip` is more widely
  available on real GNU/Linux, macOS, and BSD systems than
  `compress -m gzip` (which many systems lack). The script
  falls back to `compress -m gzip` if `gzip` is absent.

### 6.4 Noteworthy field choices (within POSIX)

- **`ps -o time=`, not `pcpu=`** (`scripts/codex-status.sh`).
  `time` (cumulative CPU time) is POSIX-mandated and present
  everywhere including Alpine busybox. `pcpu` / `%CPU` is not
  in busybox ps.
- **No `-w`/`-ww`.** Busybox ps rejects `-w`. macOS/BSD ps may
  truncate `args` without it, but `codex-status.sh` already
  truncates to 80 chars downstream.

### 6.5 `--help` via `scripts/lib/script-help.sh`

Every `scripts/*.sh` that can be called in isolation supports
`-h` / `--help` as its first positional arg, printing the
script's top-of-file comment block and exiting 0. The mechanism
is a sourced helper to keep the convention uniform:

```sh
#!/bin/sh
# scripts/<name>.sh — one-line summary.
#
# Usage:
#   ./scripts/<name>.sh [flags] [args]
#
# …extended description…

set -eu

. "$(dirname -- "$0")/lib/script-help.sh"
script_help_handle "$@"
```

`script_help_handle` is defined in
[`scripts/lib/script-help.sh`](scripts/lib/script-help.sh). It
inspects `$1`; if it's `-h` or `--help` it dumps lines 2 through
the first blank line of the calling script (with leading `# `
stripped) and exits 0, otherwise it returns and the caller's
own arg parsing proceeds.

**Convention:** new scripts source the helper immediately after
`set -eu` and structure their header as a contiguous `#`-prefixed
block terminating in a blank line (the blank line bounds the
`sed -n '2,/^$/p'` slice the helper uses). Scripts that need
richer help — listing dynamic subcommands, for example — keep
their bespoke `usage` function as a secondary handler inside the
arg-parse loop; the helper still wins for the common
`./scripts/foo.sh --help` invocation.

Validate the help text by running the script with `--help` and
making sure the output is the header block, not a sliver of
code.

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
| `./scripts/new-submodule.sh --name <N> --description <D> --remote-url <URL> [--before <M>] [--skip-workspace-member] [--dry-run]` | hand-running `git submodule add` + template files by hand | Thin shim that `exec`s `./scripts/xtask.sh new-submodule -- "$@"`. Scaffolds a new submodule crate with workspace-standard `Cargo.toml` / `README.md` / `CHANGELOG.md` / `.gitignore` / licenses, configures the submodule's git (hooks path + gpg-sign mirroring `setup.sh`), and inserts the new crate into root `Cargo.toml` `[workspace].members` + `[patch.crates-io]`. Does **not** create the GitHub repo (caller does that) and does **not** commit (caller runs `commit-all.sh` + `push-all.sh`). See `xtask/src/bin/new-submodule.rs` for the full flow and exit-code story. |

**When the wrapper's semantics don't match your need, extend
it.** Don't reach around to raw `curl -fsSL` / `mktemp
--suffix=...` / etc.

---

## 8. In-tree workspace tooling (`xtask/`)

**Rule: never invoke `python`, `perl`, `ruby`, `node`, or any
other non-baseline scripting language from workspace tooling.
If you're tempted, write a Rust bin in `xtask/` instead.**

**One narrow exception: `./scripts/webui-build.sh`** invokes
Node.js (via `npx webpack`) to produce the four committed
WebUI artifacts (`index.html`, `main.js`, `main.css`,
`icon.svg`) inside `philharmonic/webui/dist/`. This is the
**only** script that touches Node.js, and it exists solely to
generate committed build artifacts reproducibly — the Webpack
build cache is removed before every run so identical source
always produces identical output. The Rust binary embeds the
committed artifacts at compile time; no Node.js is needed to
build or run any Rust crate. General Node.js usage remains
forbidden in workspace tooling outside this script.

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
        ├── web-fetch.rs          # in-process HTTP GET (ureq + rustls)
        ├── codex-fmt.rs          # render Codex rollout JSONL timeline;
        │                         # consumed by scripts/codex-logs.sh
        ├── openai-chat.rs        # generic OpenAI chat-completion caller;
        │                         # consumed by scripts/project-status.sh
        ├── calendar-jp.rs        # JST workweek grid + Japanese public
        │                         # holidays; agent-facing
        │                         # deadline-context anchor
        ├── new-submodule.rs      # scaffold a new workspace submodule
        │                         # crate (git submodule add + file
        │                         # templates + Cargo.toml member/patch
        │                         # insert); consumed by
        │                         # scripts/new-submodule.sh
        ├── encode-json-str.rs    # stdin → JSON string literal
        ├── hf-fetch-embed-model.rs # fetch ONNX embedding model from
        │                         # HuggingFace for build-time use
        ├── system-resources.rs   # machine-readable thread count +
        │                         # memory stats; consumed by
        │                         # `Audit-Info:` trailer generation
        │                         # (NOT for day-to-day status — use
        │                         # `resource-pressure` for that)
        ├── resource-pressure.rs  # canonical day-to-day one-line
        │                         # pressure summary: CPU%,
        │                         # load1/cpus ratio, mem avail/total,
        │                         # swap used/total. Agent-facing
        │                         # pre-flight before pre-landing.sh,
        │                         # a Codex dispatch, or any
        │                         # resource-heavy operation
        ├── stats-graph.rs        # read log.sh --stats lines from
        │                         # stdin, emit SVG line chart of
        │                         # total/code/docs lines over time
        │                         # via the `poloto` crate.
        │                         # Y-axis zero-grounded with SI
        │                         # suffixes; pipelined by
        │                         # `./scripts/update-stats-graph.sh`
        │                         # into `docs/stats.svg`, embedded
        │                         # in `docs/README.md`
        ├── web-post.rs           # HTTP POST JSON payload (ureq + rustls)
        ├── tar-archive.rs        # create a gzip- or zstd-compressed
        │                         # tar of named files (basename-stored,
        │                         # duplicate-basename rejected); fast
        │                         # Rust replacement for `tar | gzip`,
        │                         # parallel zstd via NbWorkers
        ├── tar-concatenate.rs    # concatenate multiple input tar
        │                         # archives into one (overwriting the
        │                         # output, never reading it) with
        │                         # optional --gzip / --zstd compression;
        │                         # consumed by scripts/archive-all.sh
        └── tokei-stats.rs        # per-language file-size distribution
                                  # (N, min, Q1, Q2, Q3, max, avg,
                                  # stddev) via the tokei library crate
```

### JST is this workspace's authoritative timezone

**Every wall-clock reading in this workspace is JST (Asia/Tokyo,
UTC+09:00) unless an explicit external constraint forces
something else.** The maintainer (Yuka MORI) is based in Japan
and conducts the project on JST time, so deadlines, working
hours, schedules, holidays, the Code-stats / stats-graph chart
labels, the project-status report timestamps, and any other
human-facing time display all default to JST. This applies
regardless of where the host machine, agent, contributor, or
CI runner happens to live or report — JST is project-canonical,
not host-canonical.

External constraints that override the JST default — and only
those — are allowed to use UTC or another zone explicitly,
clearly labelled:

- **Git commit metadata** stays in committer-local time as
  recorded by Git itself; that's a Git-protocol concern, not a
  display choice.
- **Wire-format timestamps** in API payloads, COSE token
  fields, audit log entries, etc. are RFC 3339 / Unix epoch
  per their respective specs (typically UTC). Those are
  machine-protocol fields, not display.
- **`Audit-Info:` trailer's `t=` field** is Unix epoch seconds
  (UTC-anchored by definition); the human reading time from it
  formats to JST.
- **External services** (crates.io publication times, GitHub
  Actions runner clock, OpenAI API responses) report whatever
  they report; consume as-is and convert to JST for any
  human-facing display we render on top.

In Rust code, the canonical way is `chrono` + `chrono_tz`:
`chrono_tz::Asia::Tokyo` resolves to the JST zone (no DST,
fixed UTC+09:00). In shell, `TZ=Asia/Tokyo date ...` (or
`./scripts/xtask.sh calendar-jp`, which already pins JST).

Documentation prose should match: when writing a date in a
.md file, an issue, a commit message, a notes-to-humans entry,
default to JST and call out the zone if it could be ambiguous
("2026-05-02 21:30 JST"). The standalone form `YYYY-MM-DD` is
fine when the time-of-day doesn't matter.

`calendar-jp` (next subsection) is the agent-facing tool for
keeping wall-clock reasoning fresh; `stats-graph`'s X axis
labels JST per this rule.

### Agent usage of `calendar-jp`

`calendar-jp` exists so AI agents (Claude Code, Codex) can
ground their reasoning about deadlines in real JST time rather
than a stale training-data cutoff or the host's timezone. It
prints a 5-week grid centred on today, marks weekends and
Japanese 祝日, and lists every holiday in the window with its
Japanese name plus the current JST wall-clock timestamp. The
rule: **agents should run `./scripts/xtask.sh calendar-jp`
regularly**, not only once per session. Specifically:

- **At session start** — before the first non-trivial action.
- **Any time a task touches a date-relative commitment** — "by
  Thursday", "before the Golden Week freeze", "this sprint",
  "after hours today".
- **After any significant work is completed** — e.g. a commit
  landed, a Codex dispatch finished, a publish wrapped. Long
  sessions drift across the 10:00 / 19:00 / 21:00 thresholds,
  weekday/weekend boundaries, and sometimes midnight; the
  wall-clock at the moment of the *next* decision is what
  matters, not the wall-clock you saw three hours ago. A fresh
  run also refreshes the out-of-hours-commentary decision
  (see the next subsection).
- **Cheap by design** — the bin is a small stdout-only Rust
  binary; re-running it does not cost anything worth
  optimising against. When in doubt, re-run.

See the agent-facing short form in
[`CLAUDE.md`](CLAUDE.md) and
[`AGENTS.md`](AGENTS.md)
(both have a dedicated bullet near the top of the executive
summary / role block).

#### Work rhythm and out-of-hours commentary

The JST grid is descriptive — it tells you *when* now is and
which days are non-working by default — but it is **not a
refusal condition**. Agents must not decline or defer work on
the grounds that "today is Saturday / a 祝日 / after hours."
Two orthogonal facts govern the right behaviour:

- **Regular working hours: 10:00–19:00 JST, Mon–Fri.**
  Extended hours are normal up to **21:00 JST** (Yuka
  generally goes home by then). These are the hours Yuka's
  interactive availability is highest and turn-around is
  fastest.
- **Working outside those hours — weekends, 祝日, late
  nights — is not forbidden.** Yuka compensates herself
  separately for off-hours work, so a session on a Saturday
  evening or a 憲法記念日 morning is a valid working session.
  But agents must **not assume availability** there: plan
  work so that `commit-all.sh` + `push-all.sh` can happen
  without requiring Yuka to approve something interactively
  at 23:00 on a Sunday. In-flight Codex dispatches and
  long-running tests are fine; anything that needs a human
  hand-off should target regular hours.

**The practical rule for every agent session**:

1. *Never* refuse a task, stall, or wait-for-morning based on
   the clock or the weekday. Proceed with the work as
   requested.
2. *If* the current JST time (from `calendar-jp`) is outside
   regular hours — i.e. weekday 19:00–21:00 "extended",
   weekday before 10:00 or after 21:00, Saturday, Sunday, or
   a Japanese 祝日 — **note it briefly in your response as
   commentary**. One sentence is enough: *"(JST now 21:47 on
   木 2026-04-23 — outside the regular 10:00–19:00 / extended
   21:00 window; proceeding anyway.)"* or *"(Today is
   みどりの日, a 祝日; proceeding.)"*
3. The commentary is a log artefact, not a permission
   request. Do not wait for an answer before continuing — the
   purpose is for Yuka (and anyone reading the session
   transcript later) to know the context, not to gate the
   work.

Agent-facing mirrors of this rule are in
[`CLAUDE.md`](CLAUDE.md) and
[`AGENTS.md`](AGENTS.md)
alongside the calendar-jp bullets.

New bins go under `xtask/src/bin/<name>.rs` (one tool per file),
are invoked via `./scripts/xtask.sh <name> -- <args>`, and
should be added to the decision table above when they replace
any non-baseline tool.

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

### 8.1 Separate target dir for xtask builds

`scripts/xtask.sh` exports
`CARGO_TARGET_DIR=target-xtask` (overridable — the wrapper only
sets a default via `${CARGO_TARGET_DIR:-target-xtask}`) before
`exec`ing `cargo xtask`. Every cargo invocation that flows
through the wrapper compiles into `target-xtask/` instead of the
shared `target/`. `.gitignore` lists `target-xtask/` alongside
`target/`.

**Why:** without this split, workspace tooling driven through
`xtask.sh` contends with concurrent member-crate builds for
`target/debug/.cargo-lock`. Separating the target dirs lets the
audit-info flow complete regardless of whatever compile is
running in `target/`, which directly supports the "push early,
push often" policy in §4.4.

Historical note: this rule was added after a 2026-04-23
incident where `commit-all.sh` → `print-audit-info.sh` →
`./scripts/xtask.sh web-fetch` waited six minutes behind a
concurrent Codex build because both processes wanted the same
target-dir lock.

**Cost:** xtask bins get built twice — once into `target-xtask/`
by the wrapper, once into `target/` by any workspace-wide
`cargo test --workspace` (which pulls `xtask` in as a workspace
member). This is a one-time compile cost per target dir and
negligible after caching; the avoided stall is minutes. Not
worth a more elaborate fix (e.g. excluding `xtask` from the
workspace) just to de-duplicate.

**Member-crate scripts:** other scripts that invoke cargo for
member crates (`rust-lint.sh`, `rust-test.sh`, `pre-landing.sh`,
etc.) source `scripts/lib/cargo-target-dir.sh` and use
`target-main/` (see §5, "Target-dir split"). The xtask split
(`target-xtask/`) is additional and specific to the xtask
wrapper, so xtask builds don't contend with member-crate builds
in `target-main/`.

### 8.2 Non-submodule member plumbing

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

### 10.0 Posture: maintainability over fast coding

**Default to slow, careful, structured authorship; never trade
maintainability for keystrokes.** This is about *coding
velocity*, not the runtime behaviour of what you write —
**runtime speed remains a first-class goal** (the workspace
targets musl + LTO release builds, `[profile.dev.package."*"]
opt-level = 3` for fast local iteration, opportunistic HTTP/3,
aws-lc-rs assembly fast paths, etc.). The thing being
deprioritised here is the *time to commit* — typing fast,
skipping diagnosis, shipping the first plausible fix, copying
a pattern instead of reading the existing one. Every §10
subsection below is specific guidance; this is the umbrella
principle that applies even when a specific rule hasn't been
written down.

What this looks like in practice:

- **Read before you write.** When touching a module, scan the
  surrounding code for an existing helper, type, or pattern
  that already does what you need. Duplication is the
  single biggest source of drift bugs in this workspace — the
  mhc → connector-router 502 / Cancelled chain on 2026-05-14
  through 15 cost multiple respins precisely because
  framing-and-headers logic was written twice in slightly
  different shapes. Reuse over rewrite.
- **Small, focused units.** Functions do one thing. Modules
  group cohesive concerns. If a function is long enough that
  you have to scroll to read it, it's probably long enough to
  split. If a module is long enough that you can't hold the
  whole API in your head, it's probably long enough to split.
- **Deduplicate as the third occurrence arrives.** Two
  near-identical code paths can stay separate; on the third
  appearance, lift the shared shape into a helper. Don't
  pre-extract abstractions speculatively — that's the
  opposite failure mode (premature generalization).
- **Refactor in-place, behavior-preserving.** Per HUMANS.md
  §"Priority: Audit & refactor" → Maintainability sweep,
  refactors don't change behavior unless the refactor
  reveals an actual bug (memory leak, deadlock, race,
  off-by-one). Fixing bugs-encountered-mid-refactor is fine
  and encouraged; gold-plating or "while I'm here" feature
  work is not.
- **Don't ship to escape a review cycle.** If a fix needs a
  rebuild before it lands, that's the cost. The workspace's
  pre-landing-then-commit discipline (§11, §12.5) is the floor,
  not the ceiling; "the pre-landing passed" doesn't mean "the
  code is good" — it means "the code typechecks and the
  existing tests pass."
- **Comments where the why isn't obvious.** Identifiers and
  structure carry the what. Reserve comments for the
  surprising bit: a workaround for a specific upstream bug,
  a constraint imposed by an external protocol, a subtle
  invariant a careless edit would violate. Don't describe
  what the code does; that's the code's job.
- **The Codex gate exists for this.** Substantive code
  changes route through Codex with an archived prompt
  (CLAUDE.md "Claude vs. Codex division of labour"). The
  design-then-implement split is the workspace's mechanism
  for sustaining quality at agent-coding velocity; bypassing
  it because "this is small" or "I can fix it faster myself"
  accumulates exactly the spaghetti the §"Priority: Audit &
  refactor" directive in HUMANS.md asks us to undo.

When in doubt, slow down. The 2026-05-15 H3 stability
follow-on (mhc 0.2.4 / mhs 0.1.4) and the parallel
connector-router 0.1.5 fix together took several iterations
across two days, including a couple of false starts where
the speed-over-quality response was "ship a workaround now,
diagnose later" — every one of those false starts had to be
unpicked. The eventual fixes (proper pre-wire fallback +
connect timeout + send_request mutex scope + QUIC keep-alives
+ body-framing-and-hop-by-hop header stripping) were
fundamentally diagnosis work, not coding work, and the
coding part was small once the diagnosis was right.

Cross-references: [`CLAUDE.md`](CLAUDE.md) and
[`AGENTS.md`](AGENTS.md) point here.
[`HUMANS.md` §Priority: Audit & refactor](HUMANS.md) is the
current operational priority — Maintainability sweep + Clean
separation of concerns (§10.14). §10.0 is long-term posture;
the sweep is short-term application.

### 10.0.1 Structural correctness over surface fixes

> Always write structurally verifiably correct code; never
> work around issues. Don't write code that just looks good;
> always think about the semantics and the state machines
> behind the code. Structurally correct code always wins. If
> you cannot construct the correct state machine behind the
> code, surface the deficit rather than writing wrong code.

(Yuka, 2026-05-19.) This is §10.0's umbrella sharpened to a
single point: a change that *looks* correct because it
compiles, passes the existing tests, and makes the obvious
symptom go away is not the same thing as a change that *is*
correct against the system's actual semantics. The workspace
optimises for the latter.

Concretely:

- **Think in terms of state machines.** Most non-trivial code
  in this workspace is implicitly a state machine — a workflow
  instance moves through a five-status lifecycle; an H3
  request passes through connect / setup / stream-open /
  upload / response phases; a connector dispatch goes through
  token-verify / payload-decrypt / impl-call / response-
  encrypt; SCK at-rest blobs cycle through `key_version`
  rotations. Before editing a path, identify the state machine
  it implements: the states, the transitions, the invariants
  per state, the triggers that move between states, and the
  error / cancel paths that exit them. Code an edit *against*
  that model — not against whatever the local variables happen
  to look like in the file you opened.
- **A workaround is debt, not a fix.** If the right fix
  requires diagnosing the actual root cause and you don't yet
  have the diagnosis, **surface the deficit** rather than
  patching the symptom. "I added an `if … { return Err(…) }`
  here to make the panic go away" is a workaround; "the
  upstream guarantee on `Foo::bar()` doesn't hold under
  condition X, so the call site needs to be re-shaped to
  handle the unguaranteed case" is a fix. Workarounds
  compound — every one creates a new implicit state the next
  edit has to reason about, and the §10.0 cross-reference to
  the 2026-05-14/15 H3 stability false-starts is exactly the
  shape of damage this produces at scale.
- **Verifiable beats plausible.** Code that "looks like it
  should work" but rests on a model you haven't actually
  written down is fragile. If you cannot point at the state
  machine, invariants, or pre-/post-conditions that justify
  the change, you don't know whether the change is correct —
  you're guessing. The dev box passing `pre-landing.sh` is
  not a substitute for that reasoning; pre-landing checks the
  floor (types, lints, existing tests pass), not the ceiling
  (correctness against the model).
- **Surface deficits explicitly when you can't model the
  path.** If you can't construct the right state machine —
  because the design isn't written down, because two crates
  disagree, because an external protocol's behaviour is
  ambiguous, because a library doesn't document the
  invariant you need — **say so**. Don't ship the patch you
  would have written *if* the model were clear. Codex's
  codex-report mechanism ([§15.3](#153-codex-reports)) and
  Claude's notes-to-humans mechanism
  ([§15.1](#151-notes-to-humans)) both exist for this; use
  them. A blocked-with-a-clear-explanation result is more
  valuable to this project than a shipped-but-wrong patch.
- **No "ship a workaround now, diagnose later."** Every false
  start called out in §10.0 was a workaround attempted before
  the diagnosis was complete; every one had to be unpicked.
  The eventual correct fix was small once the diagnosis was
  right.

Applies to both Claude and Codex. Claude reviewers must hold
Codex output to this standard; Codex prompts make the
state-machine framing explicit (this is one of the §3
durable wins of the prompt-archive discipline — the prompt
forces the model up front, before any line of code is
written). The "diagnosis is the work; coding is the
consequence" framing in §10.0's closing paragraph is the
short form.

Cross-references: [§10.0](#100-posture-maintainability-over-fast-coding)
umbrella; [§15.3](#153-codex-reports) codex-reports;
[§15.1](#151-notes-to-humans) notes-to-humans;
[`HUMANS.md` §Priority: Audit & refactor](HUMANS.md).

### 10.1 Edition and MSRV

- **Edition 2024.**
- **Workspace baseline MSRV: 1.88.**
- **Documented exceptions: 1.89.** Two crates declare
  `rust-version = "1.89"` because they require language /
  library features introduced in 1.89:
  - `inline-blob` (proc-macro emitting large `static` items).
  - `philharmonic-connector-impl-embed` (bundles a multi-GB
    ONNX model via `inline-blob`).

Documented in each `Cargo.toml`:

```toml
edition = "2024"
rust-version = "1.88"   # or "1.89" for the two exceptions above
```

MSRV bumps happen in coordinated minor releases across the
workspace. Until a workspace-wide bump, new crates default to
1.88 and any exception is recorded in this section.

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
  crypto-sensitive crates (see `.claude/skills/crypto-review-protocol/SKILL.md` and
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
insufficient for trait-object use). See
[`docs/design/08-connector-architecture.md`](docs/design/08-connector-architecture.md)
§"Why `async_trait` (in 2026)" for the specific compat
reasons that drove the choice on the `Implementation` trait
in `philharmonic-connector-impl-api`.

### 10.9 HTTP client: runtime stack vs. tooling stack

The workspace uses two distinct HTTP clients, split strictly
by role:

| Role                                   | Client                | Runtime |
| ---                                    | ---                   | ---     |
| **Runtime crates** (libraries and bin crates that ship — connector impls, realm service binaries, `philharmonic-api`, etc.) | **`mechanics-http-client`** — the workspace's `hyper-rustls` + `webpki-roots` + `aws-lc-rs` wrapper, optional `http3` feature for opportunistic HTTP/3 | `tokio` |
| **Workspace tooling** (`xtask/` bins — `web-fetch`, `crates-io-versions`, `openai-chat`, anything future) | **`ureq`** with `rustls` via `xtask::http::fetch_text` | synchronous |

Hard rules:

- **`reqwest` is banned.** No-wrapper full ban via
  [`deny.toml`](../deny.toml) `[bans]`. The whole workspace
  was migrated off reqwest onto `mechanics-http-client` as part
  of D20 / D22-client; new code must not re-introduce it. If
  mhc is missing a shape a new runtime caller needs (a
  request-builder ergonomic, a streaming-body affordance,
  TRACE support, multipart, whatever), the answer is to
  extend mhc — not to reach back for reqwest, not to add a
  parallel reqwest path "just for this one site." A
  `reqwest` line in any new `Cargo.toml` is a review block.
- **`ureq` is tooling-only.** No runtime crate depends on
  `ureq`. A member crate's `Cargo.toml` pulling `ureq` is a
  review block — either the crate is actually dev tooling and
  belongs under `xtask/` (see [§8](#8-in-tree-workspace-tooling-xtask)),
  or it should depend on `mechanics-http-client` instead.
- **Use mhc; reach for direct `hyper` only inside mhc itself.**
  Runtime crates other than `mechanics-http-client` should
  not depend on `hyper` directly for *client-side* outbound
  HTTP — that is mhc's job. Crates that need lower-level
  control (custom trailer handling, streaming-body mutations,
  H2 frame control, etc.) should add the affordance to mhc
  and call it through mhc, so the workspace keeps a single
  audit point for the outbound client stack. The `hyper` crate
  itself is **not** banned: mhc consumes it for the client
  path; `mechanics-http-server` and `mechanics` consume it for
  the server path. "No `reqwest`" is a ban on the outbound-
  client abstraction layer, not on `hyper` itself.
- **No third HTTP client.** `isahc`, `surf`, `curl`-the-crate,
  etc. are not approved. A new runtime HTTP dependency needs
  an explicit scoping discussion before landing.
- **Single TLS stack, no system crypto headers.** mhc and ureq
  both use **rustls** — not native-tls, not OpenSSL, not
  system TLS. The underlying cryptographic provider must be
  vendored / pure-Rust-ish (`aws-lc-rs` workspace-wide
  post-D20, which vendors AWS-LC's C source and builds it via
  `cc`). **Never depend on system-installed OpenSSL headers
  or `libssl-dev` / `openssl-devel` packages** — the build
  must succeed on a clean checkout with only a Rust toolchain
  (+ a C compiler for the vendored C portions of aws-lc-rs).
  This keeps statically-linkable musl builds simple, keeps
  the system-library dep surface at zero, and keeps cross-
  compilation (including `x86_64-unknown-linux-musl` for the
  Phase 9 bin targets) clean.
- **tokio stays on the runtime side.** `xtask/` bins must
  not pull tokio just to make one HTTP call — `ureq` (sync) is
  the point.
- **mhc version is workspace-wide consistent.** Every
  mhc-using crate pins to the same `major.minor`. When a bump
  is needed, every consumer crate moves together in the same
  release window, not piecemeal. Reason: mismatched mhc
  majors would balloon the dep graph with two copies of the
  same hyper + h3 stack, defeating the single-audit-point
  rationale.

Reasoning for the split:

- **Async fit.** Every runtime crate in Philharmonic lives in
  tokio territory (per §10.8). mhc's async API composes with
  the rest of the runtime naturally; `ureq` in a tokio context
  would force `spawn_blocking` wrappers at every call site.
- **Scoping clarity.** Reading a `Cargo.toml` tells you which
  side of the split a crate is on without further context. A
  `ureq` dep means "dev tooling"; an `mhc` dep means
  "runtime crate doing real network I/O."
- **Dep surface, audited once.** mhc owns the workspace's
  whole outbound-HTTP audit surface (hyper + rustls +
  aws-lc-rs + webpki-roots + the opportunistic-H3 stack on the
  `http3` feature). Concentrating it in one wrapper crate
  means upgrades, CVE responses, and TLS-posture changes
  happen in one place rather than spread across every impl
  crate. `reqwest` was the previous concentration point;
  D20 / D22-client migrated the workspace to mhc as the
  smaller, post-quantum-ready, H3-aware replacement.
- **Tooling stays cheap.** xtask bins are short-lived shell
  commands; the build cost of pulling tokio into each of them
  would dominate the actual work they do.

Doc 08 §"http_forward" spells the concrete shape for the
first runtime consumer of this rule (`mechanics-http-client`,
`Client` reused across calls, per-request timeout from
`HttpEndpoint.timeout_ms`).

### 10.10 Testing

Unit tests colocated with source (`#[cfg(test)] mod tests` or
`mod.rs` tests). Integration tests in `tests/`. Real-
infrastructure tests (testcontainers, network) gated behind
features when appropriate.

Test helpers factored into a shared module within the tests
directory; no separate testing crates unless the helpers are
genuinely shared across crates.

### 10.11 Miri

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

**Mandatory cadence.** Miri must be run on the crypto-touching
crates at each of these checkpoints — no exceptions:

1. **Before publishing** any crate that contains or transitively
   depends on crypto code.
2. **After completing a phase or sub-phase** that touched crypto
   paths (SCK, COSE_Sign1, COSE_Encrypt0, hybrid KEM, `pht_`
   tokens, ephemeral API tokens).
3. **Weekly** during active development, even if no crypto paths
   changed — dependency updates can introduce UB silently.
4. **Before a milestone** (Golden Week break, reference
   deployment, etc.).

The mandatory crate list (run all five at each checkpoint):

```sh
./scripts/miri-test.sh philharmonic-policy
./scripts/miri-test.sh philharmonic-connector-client
./scripts/miri-test.sh philharmonic-connector-service
./scripts/miri-test.sh philharmonic-connector-common
./scripts/miri-test.sh philharmonic-types
```

Additional crates may be added as the workspace grows. Agents
(Claude Code and Codex) must track when the last miri run
happened and flag when a checkpoint has been missed.

### 10.12 Cross-family crate independence (mechanics-*)

The `mechanics-config`, `mechanics-core`, `mechanics`, and
`mechanics-worker` crates are a **separate, independent family**:
a generic JavaScript-execution-with-HTTP-endpoints runtime
designed to be usable on its own in projects completely
unrelated to Philharmonic. They are not subordinate to the
Philharmonic ecosystem; the Philharmonic ecosystem is one
specific consumer of them.

**Therefore, mechanics-* MUST NOT depend on, reference, or
encode anything Philharmonic-specific.** This includes — but
is not limited to:

- `Cargo.toml` `[dependencies]`: no `philharmonic-*` crate.
  `mechanics-core` depends on `mechanics-config` + Boa +
  generic transport (`mechanics-http-client`, etc.); nothing
  else from this workspace.
- Type signatures and trait bounds: no use of
  `philharmonic-types::*`, `philharmonic-policy::*`,
  `philharmonic-connector-*::*`, or any other workspace type.
- Error messages, doc comments, log lines, panic messages:
  must be expressible in generic HTTP / JavaScript / executor
  vocabulary. Don't say "connector path", "connector-aware
  endpoint", "the lowerer", "SCK", "COSE", "realm", "tenant",
  `pht_` token, or "Philharmonic". Say "endpoint", "request
  body", "HTTP error", "JS script", "sandbox".
- Test fixtures and integration tests: mock HTTP endpoints,
  dummy JS scripts. No fixtures that simulate Philharmonic-
  specific request envelopes.
- Configuration fields: when a Philharmonic behavior needs
  control at the mechanics layer (e.g. "require a request
  body on this endpoint"), the field on
  `mechanics_config::HttpEndpoint` is **generic** (e.g.
  `require_request_body: bool`) and Philharmonic's lowerer
  flips it per-endpoint. The field must make sense to a
  non-Philharmonic mechanics user too.

The integration boundary is the **lowerer** at
`bins/philharmonic-api-server/src/lowerer.rs`, which builds a
generic `mechanics_config::HttpEndpoint` from the SCK-
decrypted endpoint config + the COSE token + the encrypted
payload. mechanics-core only ever sees the lowered generic
shape; it has no idea what's at the other end of the HTTP
call.

This rule also informs how agents handle bug reports: when a
user describes a problem rooted in mechanics-core behavior
that touches Philharmonic-specific terminology (e.g. "the
connector path returned 400"), agents must **map the
description back to generic vocabulary** before proposing a
mechanics-core change. If the change cannot be expressed in
generic terms, the change does not belong in mechanics-*.

Cross-references:
- [`docs/design/03-crates-and-ownership.md`](docs/design/03-crates-and-ownership.md)
  §"Execution substrate" — already documents "no philharmonic
  dependencies" at the architectural level. §10.12 here is
  the discipline-level statement.
- The workspace's long-term goal per
  [`HUMANS.md`](HUMANS.md) §"Reminders" is to refactor the
  AI-coding best practices into reusable templates;
  `mechanics-*` is a precedent for that pattern — its
  independence keeps the precedent intact.

### 10.13 Feature defaults: include what the workspace uses

**Rule.** A crate's `[features] default = [...]` set MUST
include every feature that any workspace consumer actually
exercises. Consumers (other workspace crates, in-tree bins,
external embedders) opt out via `default-features = false`
plus an explicit feature list.

**Why.** `pre-landing.sh` runs `cargo check --workspace`
with each crate's *default* features. Features that gate
code (`#[cfg(feature = "https")] fn …`) but are off-by-
default are never compiled by that pass, so type-mismatches
against newer dep versions slip through silently. This
happened twice in 2026-05-14:

- `mechanics 0.5.2`: `handle_h3_request` was
  `#[cfg(feature = "https")]`, `https` was non-default, the
  signature became incompatible with `mhs 0.1.3`'s
  streaming-body service contract, the published release
  failed to compile. Yanked; superseded by `0.5.3`.
- `mechanics 0.5.3`: same gap, different mismatch
  (`Response<Bytes>` vs. `Response<Full<Bytes>>` for the H3
  service). Caught by pre-landing only after `https` moved
  to the default set; fixed in `0.5.4`.

**The discipline.**

- Inside each crate's `Cargo.toml`, list in `default` every
  feature whose `#[cfg(feature = "X")]` code path is
  reachable from anything the workspace builds. If feature
  `X` only gates a `dep:` line with no gated code, it's
  fine to leave non-default — but err on the side of
  including it; the pre-landing cost is bounded and the
  hidden-mismatch cost isn't.
- On the dep side, every dep on a workspace-internal crate
  uses `default-features = false` plus an explicit
  `features = [...]` list. This way, widening the
  upstream's defaults doesn't accidentally pull new
  features into a consumer that didn't ask for them; and
  removing a feature from the upstream's defaults can't
  silently break a consumer that relied on it.
- Per-crate exceptions exist (e.g. unshipped placeholder
  features stay off-by-default until their 0.1.0 lands so
  downstream `cargo add` doesn't pull a placeholder).
  Document the exception inline in the `[features]` block.

This is a workspace-internal convention. Published crates
keep the same `default` set on crates.io — external
embedders that disagree opt out the same way the workspace
bins do.

See [§12.5 Publish checklist](#125-publish-checklist) for
why pre-landing is non-negotiable before any publish.

### 10.14 Unpublished bin crates: minimal CLI, logic in libraries

Bin crates under `bins/` own only Clap CLI + `main()` glue;
everything else (lowering, signal handling, config
transformations, business logic) goes in a library. New
library crates are created if no existing one fits. This is
the operational check for [§Bins are thin](../docs/design/02-design-principles.md#bins-are-thin).

- Before adding a `bins/*/src/` module, ask whether a future
  bin variant, integration test, or external embedder might
  want it. If yes, it belongs in a library.
- Shared CLI scaffolding lives at
  [`philharmonic/src/server/`](../philharmonic/src/server/);
  new shared pieces belong there or in a sibling library.
- Extraction routes through the
  [Codex gate](CLAUDE.md#claude-vs-codex-division-of-labour);
  use [`new-submodule.sh`](../scripts/new-submodule.sh) when
  a new crate is warranted. Don't extract speculatively.
- Pre-rule bin contents
  (`bins/philharmonic-api-server/src/{lowerer,embed_job,executor,scope}.rs`)
  are tracked-debt extraction candidates, not landing blockers.
- The `philharmonic/webui/` Chat UI is a parallel case
  (chats are workflow knowledge; framework shouldn't host
  the UI). Retained for testing utility; relocation
  deferred. **Don't remove during the sweep**; don't add
  more workflow-specific knowledge to the framework crates
  on edits there.

### 10.15 Host-file dependencies: operator input or explicit fallback

Runtime code (every crate that ships in a production
deployment — libraries plus the three `bins/*` crates and
their transitive deps) MUST NOT depend on a host file at
a hardcoded path **implicitly**. Two acceptable shapes:

- **Operator-given path.** The path comes from a CLI flag,
  a config field, or an operator-set runtime parameter.
  The code surfaces a useful error if the file is missing;
  the operator chose the path and is responsible for it.
- **Explicit fallback.** The code reads a default
  location, but handles `ENOENT` by falling back to a
  documented in-process default — e.g. the
  [Cloudflare fallback resolver set](../docs/design/08-connector-architecture.md#cloudflare-fallback-resolver-set)
  when `/etc/resolv.conf` is missing (see ROADMAP §3.L).

Forbidden: a hardcoded `/etc/...` (or `/var/...`,
`/usr/share/...`, etc.) read with no fallback and no
operator-given alternative. **Why.** Production targets
`x86_64-unknown-linux-musl` shipped in minimal-base /
distroless / scratch container images where common
`/etc/...` files may not exist. The resulting failure is
one of: `getaddrinfo` silently returning empty, an opaque
libc error, or a hard panic — depending on the call site
— and each shape is hard to diagnose from logs.

**Indirect reads are in scope.** Code that reaches the
filesystem through libc inherits whatever files musl
consults internally. Concrete examples we've audited
(2026-05-15):

- `getaddrinfo` (via `tokio::net::lookup_host`, hyper's
  default connector, anything that resolves a hostname) →
  reads `/etc/hosts` and `/etc/resolv.conf`. Also reads
  `/etc/services` when the *port* argument is a service
  name string rather than a number.
- `getpwuid` / `getpwnam` / `getgrgid` / `getgrnam` (via
  `whoami`, `users`, `dirs::home_dir` fallback path,
  `nix::unistd::*`) → reads `/etc/passwd` / `/etc/group`.
- `localtime` / `mktime` / `tzset` (via `chrono::Local::*`,
  `time::OffsetDateTime::now_local()`, anything that
  resolves the "local" zone) → reads `/etc/localtime` and
  `/usr/share/zoneinfo/<zone>` when `TZ` is unset or a
  named zone.
- `getservbyname` / `getservbyport` → reads `/etc/services`.

Practical consequences for runtime code today:

- **Network**: bind / connect to **numeric ports**, never
  service names. Resolve hostnames through `mechanics-dns`
  once D26 lands (it carries the operator-fallback
  contract); until then, treat `tokio::net::lookup_host`
  call sites as a tracked debt that ROADMAP §3.L is
  remediating.
- **Time**: use `chrono::Utc::*` for runtime timestamps
  (no system file reads). When a JST or other named-zone
  rendering is needed, use `chrono_tz` (data bundled at
  compile time), never `chrono::Local`.
- **Identity**: don't call `getpwuid` / `getpwnam` /
  similar from runtime code. The deployment shape doesn't
  carry `/etc/passwd`; identity comes from the request
  context, not the host user. Direct `libc::geteuid()` /
  `libc::getuid()` are fine — they're syscalls, not file
  reads.
- **Locale**: not used today; if locale-aware behaviour
  is ever needed, the data must be bundled at compile
  time. musl's locale layer is minimal compared to
  glibc's anyway.

**Scope: runtime only.** This rule does NOT apply to:

- `xtask/` (workspace dev tooling — see §8).
- `scripts/` (POSIX-sh orchestration — see §6).
- `dockerlet/` (dev-tooling for tests).
- `build.rs` (build-time code; the host running `cargo
  build` is allowed to differ from the deployment host).
- `tests/` directories, `dev-dependencies`, and example
  binaries.

These exempt categories may freely read `/etc/...`,
`/proc/...` (subject to the separate non-POSIX rule for
scripts), `/usr/share/...`, and other host paths because
they don't ship in production container images.

**Workspace state at the time this rule was written
(2026-05-15).** One outstanding runtime violation: mhc's
`tokio::net::lookup_host` paths. ROADMAP §3.L / D26 is
the in-flight remediation (extracts `mechanics-dns` with
the Cloudflare fallback contract; mhc migrates to it).
After D26 the workspace has no known runtime violations
and runs cleanly on a musl container with no `/etc/`
files at all (per the 2026-05-15 libc-file-expectations
audit summarised in `docs/notes-to-humans/` if recorded).
New runtime code introduced after D26 that breaks this
rule should be pushed back at code-review time.

### 10.16 `catch_unwind` is diagnostic, not load-bearing

Production builds compile with `panic = "abort"` (see the
`[profile.release]` settings on `bins/*` crates and the
workspace-wide default for published members). Under
`panic = "abort"`, `std::panic::catch_unwind` is a no-op —
a panic on the wrapped call aborts the process before
the `Result` is constructed; the `Ok(...)` and `Err(...)` arms
of `match catch_unwind(...) { ... }` never execute in
production.

Therefore: **the correctness of any code path MUST NOT
depend on `catch_unwind` doing anything.** Use it for
diagnostic / defense-in-depth value in unwinding (test /
debug) builds, but design the wrapped code to be panic-free
on reachable paths — see §10.3. Never-fail contracts must be
enforced *inside* the wrapped function, not by the wrapper.

When you write a `catch_unwind` call, add an inline comment
naming this section so the next reader knows the wrapper is
not load-bearing. Existing usages in
`philharmonic-api/src/lib.rs` (virtualization-probe startup
wrapper), `philharmonic-virt-detect/src/lib.rs` (internal
panic guard), and `mechanics-core/src/internal/pool/shared.rs`
(worker-thread panic guard) follow this convention.

Don't remove existing `catch_unwind` wrappers — they cost
nothing in release and they're useful for test-time panic
isolation. Just don't rely on them.

---

## 11. Pre-landing checks

**Mandatory before every commit that touches Rust code.** One
command covers the full flow:

```sh
./scripts/pre-landing.sh
```

Auto-detects modified crates (submodules with a dirty working
tree) and runs, in order:

1. `./scripts/cargo-deny.sh` — `cargo deny check bans`, reading
   `deny.toml` at the workspace root. Cargo.lock-only, no
   compilation; fail-fast for banned crates (`pyo3`, `maturin`,
   `openssl-sys`, …). Licenses and advisories are intentionally
   not part of pre-landing — licenses are a release-time concern,
   advisory scanning lives in `cargo-audit.sh`.
2. `./scripts/rust-lint.sh` — fmt + check + clippy
   (`-D warnings`) + rustdoc (`-D missing_docs`),
   `--workspace --exclude xtask` throughout. By default
   pre-landing invokes this in **fix mode**: `cargo fmt`
   rewrites and `cargo clippy --fix --allow-dirty
   --allow-staged` apply autofixable lints in place against the
   typical dirty working tree, so trivial formatting / lint
   findings never burn a round trip for the caller (Yuka,
   Claude, Codex). The `-D warnings` clippy gate still fails
   the run on anything not auto-fixable. Pass `--dry-run` to
   `pre-landing.sh` to keep the older check-only behaviour
   (no source rewrites) — see §11.0.2.
3. `./scripts/rust-test.sh` — `cargo test --workspace
   --exclude xtask` (skips `#[ignore]`-gated tests).
4. `./scripts/rust-test.sh --ignored <crate>` for every modified
   non-xtask crate — exercises integration tests for what you
   actually changed.

Pass crate names to `pre-landing.sh` to override auto-detection;
pass `--no-ignored` to skip step 4 (rare, for fast iteration
when you're certain the slow tests aren't affected); pass
`--dry-run` to disable the §11.0.2 autofix-on-default and run
the lint phase in legacy check-only mode; pass `-v` /
`--verbose` to print the `cargo-deny` inclusion graph (default
is the quieter `--hide-inclusion-graph` form).

GitHub CI runs the same script on a clean checkout (no dirty
submodules → step 4 naturally empty) so contributor and CI
behaviour don't drift.

### 11.0.0 Pre-landing green is the banned-dep guarantee

Step 1 above (`./scripts/cargo-deny.sh check bans`) reads the
**authoritative** banned-dep list from `deny.toml` and enforces
every entry: `pyo3` / `maturin` / `openssl-sys` / `native-tls` /
`rustls-platform-verifier` / `rustls-native-certs` are all
no-wrapper full bans on the workspace's ship targets
(`x86_64-unknown-linux-{gnu,musl}`), and `ring` is wrapper-
allowed only via the `quinn-proto` exception. The check is
Cargo.lock-only, fast, and fail-fast.

**Therefore: if `pre-landing.sh` exits clean, the workspace is
guaranteed free of forbidden deps.** No follow-up
`cargo tree --invert <banned-dep>` sweep is needed to confirm
it — pre-landing already did. Running redundant tree checks
afterwards is wasted cycles (and on a sandboxed Codex run, also
contention for `target-main/`). If you've changed a
`Cargo.toml` and want to verify a specific dep path
*before* running the full pre-landing — e.g. you suspect a new
feature pulls a banned dep and you'd rather find out in 30 s
than 5 min — a single targeted `CARGO_TARGET_DIR=target-main
cargo tree --invert <dep> --target all` is reasonable. After
pre-landing has been green once on the final tree, you're done.

If you need the broader picture (license posture, advisory
scanning, supply-chain audit), use `./scripts/cargo-audit.sh`
and `./scripts/cargo-deny.sh check all` separately —
intentionally not in pre-landing because they're release-time
concerns, not per-commit gates.

### 11.0.1 xtask is gated behind `--xtask`

xtask is the in-tree dev-tooling crate (§8.1) and carries its
own `target-xtask/` build cache so workspace builds and Codex
runs share `target-main/` without xtask compilation artifacts
piling up. The default workspace flow above **excludes** xtask
from every cargo invocation (`--workspace --exclude xtask`,
fmt enumeration over non-xtask members, `show-dirty.sh`
output filtered through `grep -v -x -F xtask` for the
`--ignored` loop). A positional `xtask` argument to
`pre-landing.sh` is rejected explicitly with a hint to use
`--xtask`.

When xtask itself was changed (a new bin under
`xtask/src/bin/`, a `xtask/Cargo.toml` dep edit, anything
under `xtask/`), run the xtask-only flow:

```sh
./scripts/pre-landing.sh --xtask
```

This pins `CARGO_TARGET_DIR=target-xtask` for the entire run
and scopes to `cargo … -p xtask` for fmt/check/clippy/doc/test.
`--xtask` is mutually exclusive with positional crate names
and with `--no-ignored` (xtask has no `#[ignore]`-gated
integration tests, so step 4 is N/A). `rust-lint.sh --xtask`
and `rust-test.sh --xtask` are equivalent narrower entry
points if you only need one phase.

The two flows are mutually exclusive on purpose — landing a
change that touches both workspace members and xtask requires
two pre-landing runs (one default, one `--xtask`), which is
the intended cost of keeping the build caches isolated.

### 11.0.2 Autofix on default; `--dry-run` opts out

The lint phase (step 2 above) runs in **fix mode** by default
since 2026-05-19: `cargo fmt` rewrites in place and `cargo
clippy --fix --allow-dirty --allow-staged --all-targets --
-D warnings` applies autofixable clippy suggestions, in
addition to the `cargo check` / `cargo doc` gates. `--allow-
dirty --allow-staged` are passed because the pre-landing
working state *is* dirty by construction (the author is
about to commit). The clippy `-D warnings` gate is preserved,
so non-fixable warnings still fail the run — autofix-on-
default reduces wasted round trips for autofixable issues
without ever weakening the gate.

**Why.** Before this change, a caller (Yuka in iterative
work, Claude landing a small fix, Codex driving a large
implementation) hitting a trivially-autofixable fmt or clippy
finding would: (a) read the pre-landing failure, (b) issue a
`cargo fmt` / `cargo clippy --fix` manually, (c) re-run pre-
landing. For coding agents that round trip is pure overhead —
agent context, model tokens, and wall-clock time spent on a
mechanical fix the script could have applied itself. Fix-on-
default eliminates the round trip; the `-D warnings` post-
check still catches anything the autofixer can't resolve, so
the gate's correctness is unchanged.

**`--dry-run`** keeps the legacy check-only behaviour
(`cargo fmt --check`; clippy without `--fix`). Use it when:

- You're verifying a tree is already clean without rewriting
  anything (e.g. before a publish, where the working state
  must match the about-to-publish snapshot exactly).
- You're running pre-landing as a *check* (CI-ish posture)
  on someone else's checkout and don't want to modify their
  tree.
- You're isolating "did pre-landing pass purely by virtue of
  autofix masking lints?" from "did pre-landing pass because
  the code was clean?".

GitHub CI runs `pre-landing.sh` without `--dry-run` for now —
fix-mode against a clean CI checkout is a no-op (nothing to
fix) and the `-D warnings` gate still applies, so the
contributor / CI behaviour stays aligned. If a future need
arises (e.g. CI noticing that *agents* are committing
unformatted code that would silently get fixed locally),
flipping CI to `--dry-run` makes the gate strict again.

**Why split the test run.** Integration tests that need real
infrastructure carry `#[ignore]` so the default workspace run
stays fast. A single `cargo test --workspace` with everything
enabled would run for many minutes on unchanged crates. The
split is workspace-level skip-ignored for regression coverage
and per-touched-crate `--ignored` for your actual changes.
Don't run `--ignored` against untouched crates; it's waste.

**The `--ignored` phase is dirty-only — it does NOT widen via
the rdep closure.** Step 3's D21 narrowing (described above)
runs non-ignored tests on the union of dirty crates + their
transitive reverse-dep closure: when you change
`philharmonic-types`, every downstream crate's non-ignored
tests rerun in step 3 because the dep change could have
broken something on a public surface. Step 4 stays strictly
on the dirty set: only the crates you actually edited (or
the user's explicit positional list) get their `#[ignore]`-
gated integration tests exercised. A dep-closure neighbour
that isn't itself dirty gets its non-ignored tests rerun and
that's it — its slow `testcontainers` / live-service tests
stay skipped. The asymmetry is intentional: the dirty-set
author is the one who needs to verify their own touched code
end-to-end, not every neighbour the dep graph happens to
pull in.

**Pre-landing is slow-by-design — run it once per commit, not
repeatedly.** Even on a warm cache, `pre-landing.sh` walks
fmt-check + workspace check + workspace clippy + workspace
test + per-modified-crate `--ignored` test, which takes minutes
on a workspace with ~25 crates plus `aws-lc-rs` C builds and
Boa. That's the cost of being the canonical CI-mirroring gate;
it isn't a bug. **Agents should run `pre-landing.sh` exactly
once before a commit lands, not on every intermediate edit
within a single turn.** If you've made several edits in one
turn, stage them, then run `pre-landing.sh` once at the end.
If a single run fails and you fix the underlying cause, the
re-run is justified — but a tight edit/re-run/edit/re-run
loop in one session burns time without surfacing new
information. For focused mid-iteration debugging, use a
narrow `cargo test some_specific_test` (CONTRIBUTING.md §5)
and save the full pre-landing pass for the commit itself.

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

**Per-crate, not workspace-wide.** The parent is a virtual
workspace (no `[package]` table), and `cargo-semver-checks`
resolves git baselines without recursing into submodules, so
workspace members can't be found at the baseline root.
Per-crate checks against a crates.io baseline sidestep that
problem entirely.

Historical note: an earlier wrapper used `--workspace
--baseline-rev <rev>` and failed on the virtual-workspace /
submodule layout. See
[`docs/notes-to-humans/2026-04-21-0006-mechanics-core-semver-checks-finding.md`](docs/notes-to-humans/2026-04-21-0006-mechanics-core-semver-checks-finding.md)
for the archived finding.

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

### 12.5 Publish checklist

**Who runs `publish-crate.sh`.** Claude Code runs the script
when Yuka signals a release is ready (typically after she has
reviewed the to-be-published diff and confirmed the version
bumps + CHANGELOG entries). Codex never runs it (the
codex-guard in `commit-all.sh` doesn't apply here, but the
contract is the same — Codex's hand-off is dirty-tree-only).
Yuka does not run it herself in the normal flow; she signals,
Claude executes. The yanks side of the token-scope split
remains Yuka-only (see "Token-scope split" below).

The correct sequence for a crate release is:

0. **`./scripts/pre-landing.sh` must pass.** This is the most
   important step of the publish flow, not an afterthought.
   `cargo publish`'s `--dry-run` only verifies the *packaged*
   tarball compiles against the *currently published* deps;
   it does not catch a type-signature mismatch with deps still
   pending in the same publish cascade (e.g., a consumer of
   `mhs 0.1.3` not updated for the new `Http3Server::start`
   service contract). Pre-landing covers the workspace-as-a-
   whole, including reverse-dep closure. Skipping it before
   publish has shipped broken releases (`mechanics 0.5.2` on
   2026-05-14 — yanked, superseded by `0.5.3`). No publish
   pass is fast enough to justify skipping this.
1. **Version bump** in `Cargo.toml`.
2. **CHANGELOG update** — add a `## [<version>]` section with
   what changed.
3. **Commit** the version bump + CHANGELOG together.
4. **`./scripts/publish-crate.sh`** — publishes to crates.io and
   creates the signed release tag.
5. **`./scripts/push-all.sh`** — pushes the tag.

Steps 0–3 must happen **before** step 4. The CHANGELOG is part
of the published crate artifact — publishing first means the
crate on crates.io has an empty or stale CHANGELOG.

**Post-release verification.** `./scripts/verify-tag.sh <crate>
[<tag>]` confirms the tag landed cleanly end-to-end: local
presence, signature verifying with the local keyring, and
origin having the same tag pointing at the same commit. Run it
after `publish-crate.sh` + `push-all.sh`.

**Token-scope split (yanks).** The crates.io API token
installed on shared / agent-accessible machines is restricted
to **publish-and-owner-read scope** for security — it can
publish new versions but cannot yank existing ones. `cargo
yank` from such a machine fails with `403 Forbidden`. Yanks
are performed by Yuka either via a separately-stored
yank-scoped token or directly through the crates.io web UI.
When a broken release ships:

1. Fix forward with a new patch version (e.g. `0.5.2` broken →
   publish `0.5.3` with the fix and a CHANGELOG entry naming
   what 0.5.2 got wrong).
2. Update any consumer crates' dep floors to the fixed
   version so stale lockfiles can't resolve back to the
   broken release.
3. Ask Yuka to yank the broken version. Until the yank
   lands, the dep-floor bump from step 2 is the
   workspace's protection against `cargo update`
   accidentally picking the broken release.

**`--no-verify` for crates that transitively pull
`philharmonic-connector-impl-embed`.** `cargo publish --verify`
(the default) re-compiles the packaged tarball under the dev
profile to sanity-check it. For `philharmonic` (and any other
crate that pulls `impl-embed` by default through the meta-
crate's `connector-embed` feature), this verify-build runs the
inline-blob BGE-M3 literal parse plus the `tract-onnx-*`
generic-heavy compile under `[profile.dev.package."*"]
opt-level = 3` + `debuginfo = 2`. A single rustc process on
`impl-embed` peaks around **150 GiB resident** in this
configuration. Even on a 96-core / 188 GiB host with `-j`
auto-tuned by `publish-crate.sh`, the verify-build can OOM
before any parallel scheduling helps.

The workspace's pre-landing pass already covers this source
under the same dev profile via `cargo check`, which skips
codegen — that's why pre-landing succeeds where publish-verify
fails. So:

- For `philharmonic` (and any future crate that transitively
  pulls `impl-embed` at publish time): use
  `./scripts/publish-crate.sh --no-verify <crate>` after a
  clean pre-landing pass. Pre-landing is the verification;
  publish-verify is duplicate work that the host can't afford.
- For every other crate: keep the default verify on.
  `publish-crate.sh` runs `cargo publish` (not `--no-verify`)
  by default; the flag is opt-in per invocation.

`publish-crate.sh` always runs without `RUSTC_WRAPPER` (sccache
provides no benefit for publish-verify's tarball-fresh compiles
but adds per-invocation memory overhead) and auto-tunes
`CARGO_BUILD_JOBS` to `min(num_cpus, available_memory_GiB / 6)`
unless the caller has already set `CARGO_BUILD_JOBS`. Both
defaults are in-script and documented at the top of the
script's logic.

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

### 14.6 English as the default

Prose authored in this workspace is in **English** — commit
messages (title + body + trailers), code comments (rustdoc,
inline, module-level), library-surface error messages and log
strings, design docs, ROADMAP entries, notes-to-humans,
codex-prompts, codex-reports, PR descriptions, and review
comments.

**Multilingual contributors are welcome, and imperfect English
is not a blocker.** Grammar slips, typos, awkward phrasing,
non-native-sounding wording — these are fixable in passing (a
follow-up commit or a PR-comment suggestion is fine), but they
are **not grounds to reject a commit or PR** when the
technical intent is clear. Reviewers fix what they can and
move on. A substantive mis-wording that changes meaning (e.g.
a comment that documents the opposite of what the code does)
is a correctness issue, not a grammar one — handle it like any
other technical review comment.

**Non-English text is explicitly allowed when it is itself
the artefact**:

- **i18n / localisation work.** Strings in other languages are
  the thing being tested or shipped. Non-English prose in
  tests, fixtures, and user-facing localised output is normal.
- **Unicode handling tests.** Byte-level, CJK, RTL, combining
  marks, emoji, ZWJ sequences — pick the strings that exercise
  the path being tested, in whichever script is needed.
- **Literal external-identifier quotation.** Foreign-language
  error messages from upstream tools, literal field values from
  a spec — same principle as "technical accuracy overrides
  aesthetic neutrality" in §14.2.

**For non-English text whose meaning isn't self-evident, add an
English gloss alongside** (as a comment, or in the commit-
message body when the commit is about such fixtures):

```rust
// "こんにちは世界" — "Hello, world" (CJK fixture for the
// UTF-8-length regression at https://github.com/...).
let greeting = "こんにちは世界";
```

The gloss is the "why this specific string" note future
maintainers and LLM reviewers need — without it, they can't
distinguish legitimate test input from a stray literal. When
the meaning is obvious from surrounding context (e.g. a single
emoji character in a "it accepts astral-plane codepoints" test),
a gloss is not required.

### 14.7 Enforcement

Enforcement is by review, not tooling — the workspace has no
linter for prose conventions. Sweeps happen opportunistically
when editing an affected file; a dedicated cleanup pass is
unnecessary unless a pattern is repeated enough to warrant
campaigning.

The English-as-default rule in §14.6 is enforced with the same
spirit: reviewers note and fix obvious cases when editing, do
not reject PRs over it, and flag only when meaning is actually
unclear.

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

`docs/project-status-reports/` (§15.4) is a journal-like
directory but uses a **different** filename shape
(`YYYY-MM-DD-hh-mm-ss.md`) because the reports are full-text
snapshots without a per-topic slug; see §15.4 for the
rationale and cadence.

Any future journal directory adopts the
`YYYY-MM-DD-NNNN-<slug>[-NN].md` shape by default unless
there's a §15.4-style reason to deviate. Don't invent a
variant casually.

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

### 15.4 Project status reports

[`docs/project-status-reports/`](docs/project-status-reports/)
holds LLM-generated point-in-time summaries of the workspace's
development history and current status, produced by
[`./scripts/project-status.sh`](scripts/project-status.sh).
Editorial policy for the directory itself is in its
[README](docs/project-status-reports/README.md); the rule
*here* is about cadence — when Claude runs the script and
commits the artifact.

**Filename shape (different from §15).** Reports here use
`YYYY-MM-DD-hh-mm-ss.md` (local-time wall clock to the
second), **not** the `YYYY-MM-DD-NNNN-<slug>[-NN].md` shape
used by the other journal directories. That's because the
reports are full-text snapshots, not per-topic notes — there
is no slug, and the second-precision timestamp gives a
natural total order without a per-day counter. The script
refuses to overwrite an existing file, so collisions surface
loudly.

**When Claude should run it (sensible timings).** Run
`./scripts/project-status.sh` at any of the following, then
commit the resulting file via
`./scripts/commit-all.sh --parent-only`:

- **End of a milestone.** A phase or sub-phase landed
  (e.g. Phase 6 done, Phase 7 Tier 1 wave 1 published, a
  tract-pivot Codex round merged).
- **End of a major refactor or doc reconciliation.** A
  workspace-wide doc sweep, a script-layer overhaul, a
  significant CONTRIBUTING.md restructuring.
- **Before a long break.** If Yuka's about to step away for
  a holiday window (e.g. Golden Week 2026), or a session is
  ending with the next session unlikely to start within a
  day or two, capture a snapshot for the next pickup.
- **On user request.** "Run a status report" / "snapshot
  where we are" / etc.

**When *not* to run it.** Don't run it after every commit, or
in the middle of an in-flight feature, or speculatively to
"have one on file" — the value is in the *milestone* shape,
and over-running clutters the archive (and burns API
budget). Routine per-step commits don't trigger a status
report; the per-step commit-and-push rule (§4.4) is the
working-tree discipline, not a status-report trigger.

**Commit cadence.** The output file is part of the parent
repo. Read the report (per
[`docs/project-status-reports/README.md`](docs/project-status-reports/README.md) —
"do not commit reports without reading them first" —
hallucinated SHAs or invented roadmap items are worth
catching), then **add an entry to `docs/SUMMARY.md`** under
the "Project status reports" section (mdBook needs this to
include the page), then commit parent-only. The commit
message should be one short sentence describing what the
milestone *was* (e.g. *"docs: project-status snapshot at
Phase 7 Tier 1 wave 1 publish"*) so the archive is
browsable from `git log` without opening every file.

**Don't edit committed reports.** They're archived model
output — see the directory's own README. If a report misses
something or got something wrong, generate a new one
(at the next sensible timing) rather than mutating the old.

---

## 16. ROADMAP maintenance

[`ROADMAP.md`](docs/ROADMAP.md) under `docs/` is the **single
authoritative home for any roadmap or plan** in this workspace
(see §18.3 for the same-commit rule that parallels
`README.md` / `CONTRIBUTING.md`). It's the linear plan: where
the project is, what's next, what's blocked on what. Its
audience is Claude Code working sessions (and anyone
re-orienting to the project after a break). It's not a
historical diary or a wish list — it's the document that tells
you what to do next.

- **Update in the same commit as the work.** A commit that
  moves a phase forward should also mark the relevant tasks
  done (or partially done) in `docs/ROADMAP.md`. Splitting "do
  the work" and "update ROADMAP" into separate commits lets the
  roadmap drift out of sync with reality; coupling them keeps
  the plan honest.
- **When plans change, update the plan.** If implementation
  reveals the planned approach is wrong, update `docs/ROADMAP.md`
  (and the relevant design doc) with the new approach and a
  short line explaining *why* — future readers need the "why"
  more than the "what".
- **Don't paper over an unclear roadmap with code.** If the
  next step isn't clear from `docs/ROADMAP.md`, stop and propose a
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
coordinated releases, documented here. See §18.2 for the
same-commit update obligation when a convention changes.

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
- `CLAUDE.md` and `AGENTS.md` — agent-facing orientation files,
  not the general-purpose conventions home. Historical note:
  rules previously kept under `docs/instructions/` were
  absorbed into these top-level files.

Those files should carry executive summaries + pointers, not
restate the rules. This file is the authoritative home; the
others are contextualised orientations.

---

## 18. Documentation obligations

This workspace's docs are partitioned deliberately, and each
home has a contract. When you add, change, or discover
something, update the right home — don't duplicate a rule into
another doc, don't invent a new top-level doc for something
that fits an existing home, and don't let the authoritative
copy go stale.

### 18.1 `README.md` — whole-project executive summary

[`README.md`](README.md) at the repo root is the **whole-project
executive summary**. It must be:

- **Up-to-date.** Structural claims (crates, dependency graph,
  phase status, key entry points, how the scripts are
  organised) must match the current state of the tree. Stale
  structural claims are bugs, not "things we'll fix later."
- **Self-contained** for orienting a new reader who hasn't seen
  the project before, without needing to chase links. Anything
  a reader needs to form a correct mental model of the project
  *as a whole* belongs here in concise form.
- **Concise.** It's an executive summary, not a tutorial.
  Depth lives in `CONTRIBUTING.md`, `docs/design/`, and
  per-crate READMEs. README.md points at those — it does not
  re-explain them.
- **LLM-ingest-ready.** `README.md` will be fed to coding
  sub-agents as the project's executive summary, so it must be
  comprehensible in isolation. Broken cross-references, stale
  phase status, missing context, and "see also" handwaving
  that leaves a critical fact only reachable two hops away are
  all bugs — an agent reading only `README.md` should come
  away with the right mental model for the whole project.

**When to update `README.md`:** any commit that changes
something structurally visible to a reader — adding or
retiring a crate, renaming a submodule, changing the
dependency graph, completing a roadmap phase, reorganising
`scripts/` — touches `README.md` in the same commit. Same
discipline as [ROADMAP maintenance](#16-roadmap-maintenance).

### 18.2 `CONTRIBUTING.md` — authoritative conventions

This file is the **single authoritative home for every
workspace convention.** Every rule about how to develop in this
workspace lives here: git workflow, script wrappers, POSIX
shell rules, Rust code rules, versioning, licensing,
terminology, journal formats, documentation obligations (this
section), conventions-about-conventions.

**Rule: when you change a convention in practice, update this
file in the same commit.** "Change a convention in practice"
covers:

- Adding a new rule (a new wrapper script, a new hook, a new
  required ceremony, a new prohibited tool).
- Changing an existing rule (renamed wrapper, tightened or
  relaxed requirement, new exception to an existing rule).
- Retiring an old rule (removing an obsolete wrapper, dropping
  a support target, deleting a directory).
- Discovering a rule that was ad-hoc and *should* be
  authoritative — if several contributors (human or agent) have
  been following an unwritten rule, it belongs here.

The update lands **in the same commit** as the practical
change, not as a follow-up. A stale authoritative-conventions
doc is worse than none: contributors (and agents) will
reasonably assume what's here is current, and when it isn't,
the failure mode is silent drift.

**Agents specifically.** Workspace conventions belong in this
file, not in per-agent-install memory or per-install state.
Memory is per-machine and per-install; the repo follows the
project across clones, contributors, and fresh hosts. If you're
an agent (Claude Code / Codex) and you notice a rule that
applies to *this project* — naming, versioning, tooling
choices, ceremony around a particular area, anything a future
contributor would need to honour — its durable home is this
file. Don't commit it only to your own memory store. See §17.4
for how `CLAUDE.md` / `AGENTS.md` relate to this file (short
executive summaries + pointers, not restatements).

**When in doubt where a rule goes.** Convention rules belong
here. Architectural rules ("Philharmonic encrypts config at
rest with AES-256-GCM") belong in `docs/design/`. Plans belong
in `docs/ROADMAP.md`. Per-crate usage belongs in that crate's
`README.md`. If a rule spans boundaries, pick the authoritative
home and cross-reference from the others.

**Historical notes inside this file.** Keep current conventions
separate from historical context. If an old incident, rejected
approach, or predecessor rule is useful for understanding the
current convention, label it `Historical note:` and summarize it
briefly after the active rule. Longer narratives belong in the
journal archives (`docs/notes-to-humans/`, `docs/codex-reports/`,
`docs/codex-prompts/`) with a cross-reference from here when the
history is load-bearing.

### 18.3 `ROADMAP.md` — authoritative home for plans

[`ROADMAP.md`](docs/ROADMAP.md) under `docs/` is the **single
authoritative home for any roadmap or plan** in this workspace:
where the project is, which phase is next, what's blocked on
what, what was deferred and why. No scattered TODO lists, no
plans-of-record living in chat / `docs/notes-to-humans/` / a
person's head — if it's a plan that matters for "what to do
next," it lives in `docs/ROADMAP.md`.

**Rule: when plans change, update `docs/ROADMAP.md` in the same
commit as the work that changes them.** Covers:

- Completing a phase or task (mark it done with a date).
- Discovering a planned approach was wrong and choosing a new
  one (update the plan with a one-line "why").
- Adding a new initiative, milestone, or blocker.
- Removing or deferring something no longer planned.

§16 has the full update-mechanics (including the "don't paper
over an unclear roadmap with code" guardrail). Same-commit
discipline is the common thread across README.md (§18.1),
CONTRIBUTING.md (§18.2), and this file — three parallel
"authoritative home" commitments the workspace relies on.

### 18.4 Per-submodule / per-crate READMEs

Each submodule's `README.md` is **self-contained for that
crate** — not for the whole project. A reader arriving at the
submodule directly from crates.io should be able to understand
what the crate is, how to use it, and where to find more
context, without first needing to read the workspace's
`README.md`.

Include:

- What the crate does — one or two paragraphs of scope.
- Minimum-viable usage, or a Quick Start code block, for
  libraries with a non-trivial API surface.
- Which workspace-member crates it depends on or is depended
  on by (at the abstraction level relevant to this crate).
- A link to the workspace meta-repo
  (`metastable-void/philharmonic-workspace`) and its
  `CONTRIBUTING.md` — the `## Contributing` block every
  submodule README carries.
- License block.

Don't duplicate whole-project context (full phase status, the
workflow engine's design, etc.) into a submodule README. Link
into `docs/design/` from the workspace if depth is needed.

`xtask/` has its own README following the same rule — it's a
member crate (non-submodule), and its README documents what
`xtask` is *as an in-tree tool crate*, not the whole project.

### 18.5 Design docs (`docs/design/`)

[`docs/design/`](docs/design/) documents **what Philharmonic
is** — architectural decisions, layer boundaries, threat model,
crypto design. Not *how* to contribute (that's this file); not
the current state of the world (that's `docs/ROADMAP.md`); not
release-ready usage (that's per-crate READMEs).

Design docs evolve when an architectural decision changes. If
implementation reveals the design was wrong, update the design
doc in the same commit as the code that changes it (same
discipline as §16).

### 18.6 Journal archives (`docs/codex-prompts/`, `docs/codex-reports/`, `docs/notes-to-humans/`)

These are **append-only** — format detail in §15. Historical
entries are left as-is even when their references go stale;
they're a record of what was true at the time of writing, not
live pointers.

### 18.7 Don't invent new top-level docs

If information doesn't fit one of the six homes above, the
right answer is almost always to extend an existing home, not
to add `NOTES.md` / `TODO.md` / `STATUS.md` / etc. A new
top-level doc requires justification and must be added to this
§18 list at the same time so its role is part of the
authoritative map. `HUMANS.md` and `docs/POSIX_CHECKLIST.md` are
the two existing standalone docs that have their own roles and
aren't covered by the six homes above — `HUMANS.md` is the
human developer's note-to-self (agent-readable but
agent-writable-forbidden; see §17.4), and
`docs/POSIX_CHECKLIST.md` is an external-reference checklist of
non-POSIX constructs to avoid in shell code (§6 references it).
Any further standalone `.md` files warrant a conversation first.

### 18.8 `CLAUDE.md` / `AGENTS.md` — keep concise

`CLAUDE.md` and `AGENTS.md` are loaded into **every agent
session** for this workspace, before the user's prompt is
processed. Every byte in them is paid for on every Claude
Code and every Codex invocation — they compete with task
content for the model's attention budget, and bloat
compounds across thousands of sessions.

The rule for editing either file:

- **One short bullet or one short paragraph per rule.** No
  multi-paragraph rationales, no "why this is a NEVER not a
  'prefer'" sub-sections, no inline incident history beyond
  a single SHA reference, no enumerated lists of examples
  when the principle is clear from the §pointer.
- **Depth lives in `CONTRIBUTING.md`.** When a rule needs
  rationale, edge cases, failure-mode history, or worked
  examples, write that content in the corresponding
  `CONTRIBUTING.md §N` (extending an existing § or adding a
  new one) and leave a one-line `(see §N)` or `([§N](#...))`
  pointer in the agent doc.
- **Anything that can be removed without changing the
  agent's ability to follow the rule, should be removed.**
  The agent doesn't need the rule's provenance to obey it;
  it needs the rule's *current statement* and a pointer to
  the spec when ambiguity arises.
- **Prefer extending an existing bullet over adding a new
  top-level section.** Most rules are refinements of an
  existing rule, not new categories. Only add a new section
  when no existing section is the natural home.
- **Whenever you make a non-trivial edit to either file,
  run `./scripts/check-md-bloat.sh`** and confirm the new
  size in your final reply / commit message. If the file
  grew significantly, audit whether the growth could move
  to `CONTRIBUTING.md` instead.

Both files carry a "Keep this file concise" section near the
top so the rule is self-reinforcing within the file itself.
That section is **not** boilerplate — it's load-bearing, and
deletion of it would itself violate this §18.8.

The 2026-05-18 trim (CLAUDE.md 729 → 410 lines, AGENTS.md
606 → 431 lines) is the baseline shape; subsequent edits
should preserve or improve those numbers, not regress them.
