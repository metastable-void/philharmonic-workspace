# Workspace Conventions

Workspace-wide practices. Not design decisions about what the system
does, but decisions every crate honors for consistency.

## Licensing

All crates: `Apache-2.0 OR MPL-2.0`.

Both license files at the crate root (`LICENSE-APACHE`,
`LICENSE-MPL`). `Cargo.toml`:

```toml
license = "Apache-2.0 OR MPL-2.0"
```

Rationale: Apache-2.0 for permissive use with patent grants, MPL-2.0
for file-level copyleft with GPL compatibility via secondary license
clause. Covers more deployment scenarios than common
`Apache-2.0 OR MIT` without compromising openness.

## Naming

Pattern: `<subsystem>-<concern>[-<implementation>]`

- Subsystem prefix identifies the project: `philharmonic-`,
  `mechanics-`.
- Concern after prefix names what the crate is for: `-types`,
  `-store`, `-workflow`, `-connector`.
- Implementation suffix for multiple implementations of one concern:
  `philharmonic-store-sqlx-mysql` is "the storage trait, implemented
  via sqlx for MySQL."
- Meta-crate unsuffixed: `philharmonic`, `mechanics`.

Readers can infer relationships from names. `philharmonic-store` is
the trait; `philharmonic-store-sqlx-mysql` is an implementation.

## Crate name claims

Defensive early publishing: reserve crate names on crates.io before
implementations are complete. Use `0.0.0` placeholders with empty or
minimal content.

This prevents name squatting and ensures the namespace is
predictable. When implementation begins, the crate transitions from
placeholder to real content via normal version bumps.

## Versioning

Semantic Versioning with pre-1.0 caveats:

- **Patch (0.x.y → 0.x.(y+1))** — additive changes, bug fixes,
  docs.
- **Minor (0.x.y → 0.(x+1).0)** — changes to existing APIs.
  Pre-1.0 breakage signal.
- **Major (0.x.y → 1.0.0+)** — stability boundary.

The cornerstone (`philharmonic-types`) is on the strict end: many
dependents, so breaking changes are painful. Bundled, announced.

Downstream crates pin cornerstone to minor version
(`philharmonic-types = "0.3"`) to pick up patches automatically
while protecting against minor-version breakage.

Peer crates within the workspace pin loosely to each other
(`philharmonic-store = "0.1"`) — the workspace evolves together.

**Pinning to a patch version.** When a crate relies on a feature
or fix introduced in a specific patch, pin to that exact patch
(`philharmonic-types = "0.3.3"`) so `cargo` refuses to resolve
against an older patch that lacks the required feature. This is
the *only* reason to tighten beyond a minor pin; don't pin to a
patch for hygiene or habit. When the dependency publishes a
further patch whose feature set you start using, bump the pin.

## Crate version lookup

**Rule: never recall a crate's published version from memory.
Every question about "what's on crates.io for crate X?" is
answered by running the lookup wrapper:**

```bash
./scripts/xtask.sh crates-io-versions -- <crate>
```

The wrapper queries the crates.io sparse index in-process (via
the `xtask/` `crates-io-versions` bin) and prints every
non-yanked published version. That's the authoritative answer.
Applies to third-party crates (checking whether a new `tokio` /
`serde` / `sqlx` release exists before bumping) *and* this
workspace's own crates (confirming whether `philharmonic-types
0.3.4` is already out before cutting `0.3.5`).

**Why the rule exists:**

- **Model memory is stale.** Claude's and Codex's training data
  is months to years behind the present. A version remembered
  from training is almost certainly wrong for anything that has
  released in the last year, and it's wrong in a way that is
  hard to notice — the remembered number *sounds* right.
- **Session memory is frozen in time.** A version confirmed
  three sessions ago may have been superseded, yanked, or
  replaced by a security patch in the meantime. Memory records
  persist across sessions; crates.io state doesn't.
- **Echoing a remembered number is how wrong pins land.** Pins
  are checked into `Cargo.toml` and `Cargo.lock`, and a pin to a
  non-existent version fails only at resolve time — after the
  commit lands and CI tries to build it.
- **The lookup is cheap.** One HTTP round-trip to the sparse
  index, no auth required, sub-second in practice.

**When the rule *doesn't* apply — local version declarations.**
The version a workspace crate currently declares *in its own
`Cargo.toml`* (the "what we're about to publish" number) is
separate from the crates.io state and has its own wrapper:

```bash
./scripts/crate-version.sh <crate>       # single-crate (for scripting)
./scripts/crate-version.sh --all         # every workspace member
```

`crate-version.sh` parses the local `Cargo.toml`;
`crates-io-versions` queries crates.io. The two can legitimately
disagree — e.g. `Cargo.toml` declares `0.3.5` locally while
crates.io only has up through `0.3.4`, because we're mid-release.
Pick the wrapper that matches the question. Neither answer should
come from memory.

**Applies to agents and humans equally.** The rule reads as
agent-facing because Claude and Codex are the most likely
offenders — they have memory stores and training priors to lean
on. But humans forget versions too, and the wrapper is the same
cost for either. If you're writing changelog text, release notes,
or a dependency bump, run the wrapper.

## Git workflow

All Git operations on this workspace go through the helper
scripts in `scripts/`:

- `setup.sh` — one-time (or post-fresh-clone) initialization.
  Initializes every submodule recursively and warns if the Rust
  toolchain isn't on PATH. Idempotent; safe to rerun.
- `status.sh` — parent + every submodule's working tree.
- `pull-all.sh` — update submodules to their tracked branches.
- `commit-all.sh [--parent-only] [msg]` — commit pending changes
  in each submodule first, then the parent (bumping submodule
  pointers). With `--parent-only`, skip the submodule walk and
  commit only the parent — useful when the parent has its own
  pending work (docs, scripts) that should land independently
  of whatever state the submodules are in (e.g. while Codex has
  in-progress uncommitted work).
- `push-all.sh` — push each submodule's current branch, then the
  parent.
- `heads.sh` — show the current HEAD commit for the parent and
  every submodule (short SHA, `%G?` signature indicator,
  subject). The canonical way to verify signatures landed after
  a commit/push. **Use this instead of `git log -n 1`** for
  HEAD-state queries — the script walks all 24 repos in one
  call with the canonical format; raw `git log -n 1` per-repo
  is 24 invocations and drifts in format.

**Don't invoke `git commit` or `git push` ad-hoc.** The scripts
encode submodule ordering, default arguments, and the signoff
rule below. Ad-hoc invocations drift from those defaults. If the
script doesn't support what you need, extend the script (and
document the change here) before proceeding.

**Don't invoke `git log -n 1` to list HEAD state across the
workspace** — use `./scripts/heads.sh`. Raw `git log` remains
fine for history browsing (`git log <path>`, `git log --oneline`,
etc.); the rule targets specifically the "show current HEAD
commit on each repo" pattern.

**Every commit is signed off *and* cryptographically signed.**
The scripts pass both `-s` (DCO signoff, adds `Signed-off-by:`
trailer to the commit message) and `-S` (GPG or SSH signature,
attaches a cryptographic signature to the commit object) to
`git commit`. Additionally, `commit-all.sh` verifies the
resulting HEAD with `git log --format=%G?` after every commit it
makes — if the commit somehow lacks a signature, it is rolled
back with `git reset --soft HEAD~1` and the script aborts.

Consequence: if your local Git doesn't have signing configured
(no GPG key, or `gpg.format=ssh` without a `user.signingkey`),
`commit-all.sh` will fail before producing an unsigned commit.
Configure signing once — `git config --global user.signingkey
<key>`, `git config --global commit.gpgsign true`, and optionally
`git config --global gpg.format ssh` for SSH signing — and you
won't have to think about it again. Both checks are hard
requirements, not preferences.

## Shell scripts

All shell scripts in this workspace are **POSIX sh** (`#!/bin/sh`),
not bash. No bashisms.

- `set -eu`, not `set -euo pipefail`. `pipefail` isn't POSIX.
  Structure pipelines so a silent left-side failure isn't possible
  — e.g. `cmd | grep pat || true` instead of relying on pipefail
  to catch a broken `cmd`.
- No arrays (`arr=(a b c)`, `"${arr[@]}"`). Use newline- or
  space-separated strings and iterate with `for x in $var`.
- No `[[ ... ]]`, `=~`, `BASH_REMATCH`. Use `[ ... ]`, `case`, or
  pipe through `sed`/`awk`/`grep` for regex.
- No `<<<` herestrings, no `<(...)` process substitution, no
  `mapfile`/`readarray`. Use heredocs, temp files, or `while read`
  loops.
- No `${var:offset:length}` substring expansion. Use `printf
  '%.Ns'`, `cut -c`, or `expr substr`.
- No `${BASH_SOURCE[0]}`. Use `$0`; don't source these scripts.
- No `$'...'` ANSI-C quoting. Build escapes with `printf`, e.g.
  `BOLD=$(printf '\033[1m')`.
- No `local`. Namespace function-locals with a prefix
  (`_myfunc_pid`) if shadowing matters.
- No `pgrep`, no `/proc/$pid`, no `column -t`. Snapshot `ps -Aww
  -o ...` once and drive everything from the snapshot; print
  columns with `printf '%-Ns ...'`.

**Why.** The workspace is expected to work on Linux distributions
without bash installed (some minimal containers, Alpine without
`bash` in the image), and on FreeBSD/macOS where `/proc` and
procps-style utilities aren't guaranteed. Sticking to POSIX sh +
POSIX utilities means every script runs on every platform Yuka
reasonably touches, without a "this one needs bash" asterisk.

**Busybox caveat.** Alpine-class busybox installations are
supported. *Extremely stripped* busybox builds (e.g. Ubuntu's
`/usr/bin/busybox` from `busybox-static`, which lacks `etime`,
`time`, `-p`) are out of scope — rescue/initramfs images aren't
a real target. When picking a `ps` field or flag, prefer ones
the Alpine build supports.

**Validate with `./scripts/test-scripts.sh`** (mandatory after
any change under `scripts/`). It runs `dash -n` against every
`.sh` under `scripts/` *and* `scripts/lib/` (sourced helpers),
falling back to `sh -n` if dash isn't installed. GitHub CI
runs the same script, so drift between contributor and CI
behavior is impossible. Actual execution under dash (the
default `/bin/sh` on Debian/Ubuntu) is the other half of the
check — worth doing manually for scripts that run state-
changing logic.

**Workspace-root resolution is shared.** Scripts that need to
operate at the workspace root (most of them) source
`scripts/lib/workspace-cd.sh`, which resolves the root with a
three-tier fallback — superproject of the current submodule,
else git toplevel, else the script's own `$0`-relative path —
and `cd`s there. Works whether the script is invoked from the
workspace root, from inside a submodule, or from outside any
git repo entirely (e.g. `/tmp`). New scripts needing this
behavior should source the helper rather than reimplementing
the resolution inline.

**Extract routines into scripts, not ad-hoc commands.** When
you find yourself running the same command or command sequence
more than once or twice — especially multi-line sequences with
flags, `git submodule foreach` invocations, or POSIX-
compatibility guards — extract it into a `scripts/*.sh` file.
Rationale:

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
let "it's just a small thing" justify keeping a recurring pattern
ad-hoc — and for each new script, validate with
`./scripts/test-scripts.sh`, add it to the scripts list in
README.md and the `git-workflow` skill (if git-related), and
document any associated rule in `CLAUDE.md` or this file.

This rule applies primarily to Claude Code (the one orchestrating
across tasks and noticing repetition). Codex receives discrete
tasks and doesn't typically make extraction decisions — but if
Codex notices a pattern in its prompts that warrants a script,
it should flag it in the final summary so Claude can extract.

**Invoke by path, not by interpreter.** Run `./scripts/foo.sh`,
never `bash scripts/foo.sh` or `sh scripts/foo.sh`. The shebang
chooses the interpreter — that is the whole point of committing
to POSIX sh. Prefixing `bash` silently forces bash and hides any
accidental bashism that wouldn't run on Alpine / FreeBSD / macOS,
which defeats the portability rule.

**Explicit deviations from strict POSIX.** These are allowed and
tracked here — add to the list when a new one is introduced:

- **`ps -o rss=`** (`scripts/codex-status.sh`). POSIX mandates
  `vsz` but not `rss`. `rss` is supported identically on Linux
  procps, FreeBSD base ps, macOS BSD ps, and Alpine busybox, and
  matches what the user expects to see for process-memory
  summaries. Kept for output fidelity.

**Noteworthy field choices (within POSIX but worth recording).**

- **`ps -o time=`, not `pcpu=`** (`scripts/codex-status.sh`).
  `time` (cumulative CPU time) is POSIX-mandated and present
  everywhere, including Alpine busybox. `pcpu` / `%CPU` is not
  in busybox ps, so using it would block the portability goal.
  The trade-off is that we show "how much CPU has this process
  burned" rather than the instantaneous rate — acceptable for a
  Codex-status summary; for live "is it stuck?" readings use
  `top`/`htop`.
- **No `-w`/`-ww`.** Busybox ps rejects `-w`. macOS/BSD ps may
  truncate `args` to terminal width without it, but
  `codex-status.sh` already truncates to 80 chars downstream,
  so the lost data is never displayed anyway.

## ROADMAP maintenance

`ROADMAP.md` at the repo root is the linear plan: where the
project is, what's next, what's blocked on what. Its audience is
Claude Code working sessions (and anyone re-orienting to the
project after a break). It's not a historical diary or a wish
list — it's the document that tells you what to do next.

- **Update in the same commit as the work.** A commit that
  moves a phase forward should also mark the relevant tasks done
  (or partially done) in ROADMAP.md. Splitting "do the work" and
  "update ROADMAP" into separate commits lets the roadmap drift
  out of sync with reality; coupling them keeps the plan honest.
- **When plans change, update the plan.** If implementation
  reveals the planned approach is wrong, update ROADMAP.md (and
  the relevant design doc) with the new approach and a short
  line explaining *why* — future readers need the "why" more
  than the "what".
- **Don't paper over an unclear roadmap with code.** If the next
  step isn't clear from ROADMAP.md, stop and propose a roadmap
  update first. Implementing against a guess creates churn.

## Journal-like files

Every journal-like file Claude generates under this workspace
follows one filename shape:

```
YYYY-MM-DD-NNNN-<slug>[-NN].md
```

- **`YYYY-MM-DD`** — the date the file was created.
- **`NNNN`** (required) — four-digit daily sequence, zero-padded,
  counted **per-directory**. The first file written in a given
  journal directory on a given day is `0001`, the second `0002`,
  etc. `docs/codex-prompts/` and `docs/notes-to-humans/` each
  count independently. Four digits (not two) because 99-per-day
  is realistically tight for notes-to-humans in a busy session;
  `9999` gives comfortable headroom.
- **`<slug>`** — short kebab-case task name. Example:
  `phase-1-mechanics-config-extraction`,
  `journal-conventions-established`. Keep it <= ~60 characters.
- **`[-NN]`** (optional) — two-digit round suffix for multi-part
  work on one logical entry. Use when a Codex session hit a
  limit mid-task and you're resuming, or a notes-to-humans entry
  genuinely needs a follow-up round. Omit when the entry stands
  alone. Two digits is enough here because rounds rarely exceed
  a handful.

Journal directories currently governed by this format:

- `docs/codex-prompts/` — see §Codex prompt archive.
- `docs/notes-to-humans/` — see §Notes to humans.

Any future journal directory adopts the same format by default.
Don't invent a variant.

## Notes to humans

`docs/notes-to-humans/` is where Claude writes significant
findings, observations, and decisions that would otherwise live
only in session scrollback. **When Claude tells Yuka anything
substantial, the note must also be written to a file here and
committed.** Session-only output is not enough.

**What counts as "significant" (write a note):**

- Verification results where the *why* is informative ("your
  version bump is defensible for these reasons; CHANGELOG missed
  the behavior change").
- Platform-specific caveats surfaced during investigation
  ("Alpine busybox ps has no `pcpu`; we switched to `time`").
- Audit results: what's been touched, what hasn't, what's load-
  bearing, what's dead.
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
Give enough context that a future reader can act without
re-running the investigation: file paths, commit SHAs, the why
behind the finding. If two unrelated observations are worth
saving, write two files (different daily sequences).

**Commit.** Through `./scripts/commit-all.sh --parent-only` in
the same session — the note is parent-level regardless of
whether the underlying work was in a submodule.

**Don't write to humans like a self-reminder.** The audience is
Yuka (and future collaborators reading the repo), not
Claude-next-session. Complete sentences, concrete references, no
in-jokes.

## Codex prompt archive

Claude hands substantive coding to Codex (see CLAUDE.md §Claude
vs. Codex division of labor). Every prompt Claude writes for
Codex is archived and committed — there are no ephemeral Codex
invocations.

**Location.** `docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>[-NN].md`
(see §Journal-like files for format detail). `<slug>` names the
task (`auth-middleware-rewrite`, `sqlx-mysql-store-skeleton`).
One file per prompt. If a task needs multiple rounds of Codex
work, use the trailing round suffix (`-01`, `-02`) — share the
daily-sequence `NNNN` across rounds of the same logical entry,
and never overwrite a prior round's file.

**Contents.** The full prompt text Claude sent to Codex,
verbatim, plus a short preamble with:

- The task's motivation (one or two sentences).
- Links to the relevant design doc sections or ROADMAP entries.
- Any context files Claude pointed Codex at.

**Commit cadence.** Commit the prompt file *before* spawning
Codex, via `scripts/commit-all.sh`. The resulting code changes
land in a subsequent commit (or commits). Ordering the prompt
first means the archive is complete even if the Codex run is
abandoned partway through.

**Why.** The prompts are where design intent gets translated
into implementation instructions; they're the most useful
artifact for reconstructing why a chunk of code looks the way
it does, and they're where Claude/Codex collaboration mistakes
become visible in review. Losing them to chat history defeats
the reviewability of the workflow.

## Edition and MSRV

- **Edition 2024.**
- **MSRV 1.88.**

Documented in each `Cargo.toml`:

```toml
edition = "2024"
rust-version = "1.88"
```

MSRV bumps happen in coordinated minor releases across the
workspace.

## Build targets

Production: `x86_64-unknown-linux-musl` (static linking).

Library crates build for consumer-chosen targets. Binary crates
(`mechanics`, future API and connector binaries) ship as
statically-linked musl binaries for minimal containers.

Constraint: C library dependencies must be statically linkable or
vendored. Most pure-Rust crates fine; special attention for crates
wrapping native libraries.

## Repository structure

One-crate-per-repo under `github.com/metastable-void/*`. Each crate
has its own:

- Issue tracker.
- CI configuration.
- Release cycle.
- README, changelog.

Cross-crate refactors require coordinated PRs across repos.
Cornerstone versioning discipline absorbs most of this (cross-crate
vocabulary changes happen via cornerstone patch releases, picked up
transitively).

## Documentation

### Per-crate

Each crate has `README.md` covering usage and installation, plus
rustdoc for API reference. Current cornerstone and substrate crates
are at 99%+ documented; other crates target similar coverage.

### System-level

Cross-crate design docs live in the meta-crate's repository
(`philharmonic`). These cover:

- System overview.
- Design principles.
- Per-component deep dives.
- Layer boundaries.
- Deferred decisions.
- Conventions (this file).

Currently published at `metastable-void.github.io/philharmonic/`.

## Release tagging

Every crate release is tagged. The tag:

- **Lives in the crate's own repo (the submodule), not the parent
  workspace.** Each submodule is a single-crate repo, so the tag
  `v<version>` is unambiguous there. No per-crate prefixing is
  needed.
- **Is created only by `./scripts/publish-crate.sh`.** Running
  `git tag` by hand during a publish is the same class of
  mistake as ad-hoc `git commit` — it drifts from the convention
  and skips the pre-flight checks.
- **Is annotated and cryptographically signed** (`git tag -s`).
  Matches the workspace's "every commit is signed" rule.
- **Is created after `cargo publish` succeeds, not before.** A
  failed publish must not leave a dangling tag. If
  `publish-crate.sh` fails between `cargo publish` and
  `git tag`, the crate is on crates.io without a tag — recover
  by running `git tag -s v<version>` manually in the submodule
  and then `./scripts/push-all.sh`.
- **Is pushed by `./scripts/push-all.sh`** via `--follow-tags`.
  Only tags pointing at pushed commits go up, so stray local
  tags never leak.

Why: crates.io holds the published tarball, but the exact git
state that was published is only trivially recoverable if a tag
marks it. Tags also give `cargo-semver-checks` a clean baseline
reference for release-to-release API-breakage checks.

## API breakage detection

`cargo-semver-checks` compares the public API surface of one
workspace crate against a crates.io baseline (not a git baseline —
see note below) and flags semver-incompatible changes (removed
items, tightened trait bounds, signature changes, etc.). Invoke
via:

```bash
./scripts/check-api-breakage.sh <crate> [<baseline-version>]
```

Without `<baseline-version>`, cargo-semver-checks queries
crates.io for the newest published version of `<crate>` and uses
that as the baseline. Pass an explicit version (e.g. `0.2.2`) to
compare against a specific earlier release. The script installs
`cargo-semver-checks` via `cargo install --locked` on first use.

**Per-crate, not workspace-wide.** An earlier version of the
script used `--workspace --baseline-rev <rev>`. That does not
work in this repo: the parent is a virtual workspace (no
`[package]` table), and cargo-semver-checks resolves
`--baseline-rev` by `git clone`ing the parent at that rev — which
doesn't recurse into submodules, so workspace members can't be
found at the baseline root. Per-crate mode against a crates.io
baseline sidesteps the problem entirely. See
`docs/notes-to-humans/2026-04-21-0006-mechanics-core-semver-checks-finding.md`
for the full history.

**When to run:** before preparing a crate release, as part of the
pre-release review checklist. Not part of the default pre-landing
trio (fmt/clippy/test) because it's slower, requires network on
first run (install), and the signal is per-release rather than
per-commit.

## Script wrappers

**Rule: every `cargo` invocation with a `scripts/*.sh` wrapper
goes through the wrapper, not raw `cargo`.** This is the same
principle as the `scripts/*`-only git workflow: the wrappers are
the single source of truth for flag choices, ordering, install
of optional tools, and workspace-cd. Contributor-vs-CI parity is
guaranteed only *because* the wrappers are authoritative.
Ad-hoc `cargo <subcommand>` invocations drift quietly — a
missing `-D warnings`, a forgotten `--all-targets`, a workspace
that isn't at the expected CWD — and drift shows up as a CI
failure that a local run missed.

The inventory and what each wrapper covers:

| Wrapper | Wraps | Notes |
|---|---|---|
| `./scripts/pre-landing.sh [<crate>...] [--no-ignored]` | `cargo fmt --check` + `cargo check` + `cargo clippy --all-targets -- -D warnings` + `cargo test --workspace` + `cargo test --ignored -p <crate>` per modified crate | The canonical pre-commit flow. Auto-detects modified crates via `show-dirty.sh`. CI runs the same script. See §Pre-landing checks. |
| `./scripts/rust-lint.sh [<crate>]` | `cargo fmt --check` + `cargo check` + `cargo clippy --all-targets -- -D warnings` | Workspace-wide (no arg) or per-crate. |
| `./scripts/rust-test.sh [--include-ignored\|--ignored] [<crate>]` | `cargo test` with ignored-test control | `--ignored` runs *only* `#[ignore]`-gated; `--include-ignored` runs everything. |
| `./scripts/cargo-audit.sh [...]` | `cargo audit` | Auto-installs `cargo-audit` via `cargo install --locked` on first run. |
| `./scripts/check-api-breakage.sh <crate> [<baseline-version>]` | `cargo semver-checks check-release -p <crate> --baseline-version <ver>` | Per-crate; crates.io baseline (default: newest published). See §API breakage detection. |
| `./scripts/publish-crate.sh [--dry-run] <crate>` | `cargo publish -p <crate>` + signed release tag | Enforces clean tree, branch-HEAD, no-existing-tag invariants. Tag created only on publish success. |
| `./scripts/crate-version.sh <crate> \| --all` | Parses `version = "..."` from `<crate>/Cargo.toml` | Single-crate for programmatic use (`publish-crate.sh` consumes it); `--all` prints every workspace member's version. |
| `./scripts/xtask.sh crates-io-versions -- <crate>` | crates.io sparse-index query | Lists non-yanked published versions. Rust bin in `xtask/` (using `ureq` + `serde_json`); replaces the former shell script, which depended on `jq` + `web-fetch.sh` — neither is part of stripped GNU/Linux or macOS baselines. |
| `./scripts/xtask.sh <tool> -- <args>` | wrapper for in-tree Rust bins | Canonical invocation for any `xtask/` bin; supports `--list` and `--help`; mandatory `--` separates wrapper-level flags from bin args. Prefer over `cargo run -p xtask --bin <tool> --` direct calls. |
| `./scripts/check-toolchain.sh [--update]` | `rustup check` / `rustup update` + version print | Step 0 of `pre-landing.sh`; run standalone to probe local drift against CI's `@stable`. |

**Exempt**: read-only cargo queries have no wrapper and don't
need one — `cargo tree`, `cargo metadata`, `cargo --version`,
`cargo search` are fine to run raw.

**If no wrapper fits**: extend one, or add a new
`scripts/*.sh` (see §Shell scripts §"Extract routines into
scripts"). Validate with `./scripts/test-scripts.sh`. Then use
the new wrapper — don't fall back to raw cargo because the
wrapper doesn't exist yet.

## External tool wrappers

**Rule: never call `mktemp`, `curl`, or `wget` directly from a
workspace script. Use the wrappers.** The rationale is
portability: these tools vary across the minimal environments we
need to support — Alpine/busybox, FreeBSD, OpenBSD, macOS, WSL —
and their flag surfaces aren't consistent (busybox wget doesn't
take `--show-progress`, busybox mktemp lacks some templates,
OpenBSD ftp speaks HTTP but wants its own flag vocabulary). The
wrappers encode the portable choice once so every script doesn't
have to rediscover it.

| Wrapper | Replaces | Notes |
|---|---|---|
| `./scripts/mktemp.sh [<slug>]` | `mktemp` | Delegates to `mktemp(1)` when present; falls back to a 10-char `[A-Za-z0-9]` suffix from `/dev/urandom` + `touch`. Fallback does **not** set 0600 perms (`chmod` after creation for confidential content). Caller **must** register cleanup: `trap 'rm -f "$tmp"' EXIT INT HUP TERM`. |
| `./scripts/web-fetch.sh <URL> [<outfile>]` | `curl`, `wget` | Tries `curl` → `wget` → `fetch` (FreeBSD) → `ftp` (OpenBSD HTTP mode). UA override via `WEB_FETCH_UA`. All backends fail on HTTP 4xx/5xx (curl gets `-f`; the others default to failing). Callers that want to continue regardless of HTTP status use `./scripts/web-fetch.sh ... || :` at the call site — the idiom in `print-audit-info.sh`. |

**When the wrapper's semantics don't match your need, extend it.**
Don't reach around to raw `curl -fsSL` / `mktemp --suffix=...` /
etc.

Validate any changes with `./scripts/test-scripts.sh` as usual.

## In-tree workspace tooling (`xtask/`)

**Rule: never invoke `python`, `perl`, `ruby`, `node`, or any
other non-baseline scripting language from workspace tooling.
If you're tempted, write a Rust bin in `xtask/` instead.**

Well-written POSIX shell (with `awk`, `sed`, `grep`, `cut`,
`tr`, standard text pipelines) stays where it is — shell is the
right tool for orchestration, git workflow, cargo wrappers,
filesystem glue, and simple data pipelines. The rule targets
ad-hoc `python3 -c "..."` / `perl -e "..."` creep, not the
existing `scripts/*.sh`. If an existing script works and is
POSIX-clean, leave it.

**`jq` is not POSIX and is not on every baseline** (not shipped
by default on macOS, not in Alpine base, not in stripped Debian
minimal). If you find yourself reaching for `jq`, that's a Rust
trigger — add a bin under `xtask/` using `serde_json` instead.
Same for `curl` / `wget` — the `xtask/` port of `web-fetch`
uses `ureq` + `rustls` in-process, so `scripts/*.sh` don't need
curl/wget either. The only POSIX-shell data-manipulation tools
considered baseline-safe are the ones in SUSv4: `awk`, `sed`,
`grep`, `cut`, `tr`, `sort`, `uniq`, `head`, `tail`, `wc`.

The categories in play:

| Category | Example | Home |
|---|---|---|
| Ad-hoc one-off in a terminal session | "generate a UUID for this constant" | Rust bin (`./scripts/xtask.sh gen-uuid -- --v4`) — **never** `python3 -c "import uuid"` |
| Non-baseline language reach | "parse YAML, walk DOM, emit Rust" | Rust bin in `xtask/` |
| POSIX shell orchestration | "commit across submodules then push" | `scripts/*.sh` |
| POSIX shell with `awk` / `sed` (baseline-present) | "enumerate workspace members from Cargo.toml" | `scripts/*.sh` (e.g. `lib/workspace-members.sh`) |
| Depends on a non-POSIX / non-baseline tool (`jq`, `curl`, `wget`, anything not in SUSv4) | "list non-yanked versions from crates.io", "HTTP GET a URL" | Rust bin in `xtask/` — these aren't on every stripped baseline; Rust has `serde_json` and `ureq` in-tree. **If you'd reach for `jq`, that's the signal to write Rust.** |
| Trivial cargo wrapper | "fmt + check + clippy + test" | `scripts/*.sh` |
| Non-trivial parsing / cross-file validation / stateful check | "verify no two entity KINDs collide across the workspace" | Rust bin in `xtask/` |

`xtask/` is an **in-tree (non-submodule) member crate** at the
workspace root. It lives alongside the submodule-backed crates
in `[workspace] members`, but its files are tracked directly by
the parent repo (no `.gitmodules` entry). `publish = false` —
it's dev tooling only, never shipped to crates.io.

Multi-bin layout:

```
xtask/
├── Cargo.toml           # publish = false, name = "xtask"
└── src/
    └── bin/
        ├── gen-uuid.rs  # one tool per file
        └── <future>.rs
```

Each bin is invoked via the `xtask.sh` wrapper:

```bash
./scripts/xtask.sh --list                   # list available tools
./scripts/xtask.sh --help                   # wrapper help
./scripts/xtask.sh <tool>                   # run with no args
./scripts/xtask.sh <tool> -- <args>         # run with args (note the `--`)
```

The mandatory `--` separator between the bin name and its
arguments exists so future wrapper-level flags (e.g. a
`--release` toggle) can't collide with a bin's own flag of the
same name. Don't call `cargo run -p xtask --bin <name> --`
directly at call sites — `xtask.sh` is the single invocation
surface.

### Non-submodule member plumbing

The scripts that walk "workspace members" (`show-dirty.sh`,
`crate-version.sh --all`) enumerate members from the root
`Cargo.toml` via `scripts/lib/workspace-members.sh`, so in-tree
members like `xtask` are covered uniformly alongside submodule-
backed ones. Scripts that need to distinguish the two (e.g.
`show-dirty.sh`'s dirty-check has to run inside a submodule but
in the parent's working tree for in-tree members) use `-f
<member>/.git` as the classifier: submodules carry a `.git`
pointer file at their root, in-tree directories don't.

## KIND UUID generation

**Every stable wire-format UUID — entity `KIND` constants,
algorithm identifiers, key IDs, any value that once committed
must never change — is generated via:**

```bash
./scripts/xtask.sh gen-uuid -- --v4
```

Not `python3 -c "import uuid"`, not `uuidgen`, not online
generators. The rule has one reason: **one canonical source of
randomness across sessions, machines, and contributors.** Ad-hoc
UUID generation tools scattered across shell history make it too
easy to accidentally commit a value you meant to throw away, or
for two contributors to mint UUIDs from imperceptibly different
RNG sources.

`--v4` is mandatory on the CLI — every KIND we mint today is v4
random. Making the version-flag explicit means a future shift
to v5/v7 is a deliberate CLI change at each call site rather
than a silent default swap.

Usage in practice: when authoring a new entity kind (e.g.
`TenantEndpointConfig`), run `gen-uuid --v4` once, paste the
result into the Rust source as a `const KIND: Uuid = uuid!("…")`,
and commit. Never regenerate.

## Naming and terminology

Documentation, comments, commit messages, and any other
workspace-authored prose follow two overlapping conventions:
inclusive/neutral/technically-accurate language, and
FSF-preferred framing for free-software terminology. Both are
soft rules — readability trumps dogma — but the anti-patterns
below have specific reasons behind them and are worth avoiding.
If you're unsure, match the rest of the surrounding text.

README.md has a contributor-facing summary at §Terminology and
language; this section is the authoritative statement.

### Inclusive, neutral, technically accurate language

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
- **Technical accuracy overrides aesthetic neutrality.** When a
  protocol, library, or external project ships a term literally
  (HTTP `Authorization` header; the `master` branch of an
  external repo you're referencing; a DB `MASTER` command), use
  the literal name. The rule targets prose we author in this
  workspace, not identifiers other projects defined.

### Operating systems and kernels

- **GNU/Linux** for the GNU-userspace-plus-Linux-kernel
  operating system, not just "Linux." Calling the whole system
  "Linux" credits the kernel alone for userspace that's
  largely GNU (libc, coreutils, binutils, bash, …).
- **Linux kernel** (or "the kernel of Linux") when referring
  specifically to the kernel. Don't use "Linux" as a shorthand
  for the kernel when the kernel is what you mean.
- **Non-GNU Linux-based systems** are named explicitly, not
  collapsed into "Linux." Alpine is musl-based; Android is
  Linux-based but distinct from GNU/Linux; BusyBox environments
  are their own thing. Saying "works on Linux" papers over a
  distribution family that really isn't uniform.
- **`uname -s` string matches** are a pragmatic exception. When
  shell code matches the literal kernel-identifier string
  `Linux` (as in `case "$(uname -s)" in Linux) ... esac`),
  writing `Linux` is accurate — that IS what the kernel returns
  via `uname`. The rule targets human-facing prose, not
  kernel-interface string literals.

### Microsoft Windows

- Write the full name — "Microsoft Windows" or just "Windows" —
  in neutral prose. Don't abbreviate to `Win`, `win32`, `win64`,
  or `WIN_` as a freeform shorthand; the abbreviation reads as
  Microsoft "winning" against competing systems. The exception
  is established technical identifiers that ship that way (the
  Windows API is literally the `Win32` API; package identifiers
  like `x86_64-pc-windows-msvc` have `windows` in them). Don't
  fight those; don't invent new `win*`-prefixed abbreviations.

### Free software vs. "open-source"

- **"Free software"** (free as in freedom) when framing a
  software-freedom position or classifying a license.
- **"FLOSS"** (Free/Libre/Open-Source Software) as an inclusive
  umbrella when both the free-software and open-source
  communities are equally in scope — e.g. describing the broad
  ecosystem of publicly-licensed code.
- Avoid **"open-source"** as a standalone phrase when the intent
  is user freedom. "Open-source" is the marketing framing that
  intentionally sets the freedom argument aside. Use it when
  quoting or referencing external conventions (the "Open Source
  Initiative" is a proper noun; an "open-source license" is how
  the OSI describes licenses on their approved list), not as the
  default neutral term.

### Enforcement

Enforcement is by review, not tooling — the workspace has no
linter for prose conventions. A best-effort manual scan at the
time the FSF-framing subsection was added found three
standalone-"open-source" uses and zero `win*` abbreviations;
they were reworded in the same commit. Equivalent sweeps for
`master`/`slave` metaphors, gendered defaults, and
`whitelist`/`blacklist` can happen opportunistically when
editing an affected file; a dedicated cleanup pass is
unnecessary unless a pattern is repeated enough to be worth
campaigning about.

## Pre-landing checks

**Mandatory before every commit that touches Rust code.** One
command covers the full flow:

```bash
./scripts/pre-landing.sh
```

It auto-detects modified crates (submodules with a dirty
working tree) and runs, in order:

1. `./scripts/rust-lint.sh` — fmt-check + check + clippy
   (`-D warnings`).
2. `./scripts/rust-test.sh` — `cargo test --workspace` (skips
   `#[ignore]`-gated tests).
3. `./scripts/rust-test.sh --ignored <crate>` for every modified
   crate — exercises the integration tests for what you actually
   changed.

Pass crate names to `pre-landing.sh` to override the
auto-detection; pass `--no-ignored` to skip step 3 entirely
(rare, for fast iteration when you're certain the slow tests
aren't affected).

GitHub CI runs the same script on a clean checkout (no dirty
submodules → step 3 naturally empty) so contributor and CI
behavior don't drift. If you want to understand what each
underlying step does, see the next subsection.

**Why split the test run.** Integration tests that need real
infrastructure (testcontainers, live DBs, external APIs) carry
`#[ignore]` so the default workspace run stays fast. A single
`cargo test --workspace` with everything enabled would run for
many minutes on unchanged crates. The split is:

- **Workspace-level, skip-ignored** (`./scripts/rust-test.sh`) —
  catches regressions anywhere in the workspace, fast.
- **Per-touched-crate `--ignored`** — exercises the integration
  tests for crates you actually changed. Don't run `--ignored`
  against untouched crates; it's waste.

You are responsible for running the per-crate `--ignored` phase
for each modified crate. Forgetting it means slow-test
regressions slip in.

### The scripts

- **`./scripts/rust-lint.sh [<crate>]`** — `cargo fmt --check`,
  `cargo check`, `cargo clippy --all-targets -- -D warnings`, in
  that order. Workspace by default; pass a crate name to narrow.
  Warnings are errors — no ignored lints, no `#![allow(...)]` at
  crate scope. For a specific call site where a lint is genuinely
  wrong, add `#[allow(clippy::<lint>)]` at the *narrowest* scope
  with a one-line comment explaining *why*. If fmt-check fails,
  run `cargo fmt --all` (or `cargo fmt -p <crate>`) to apply and
  re-run.
- **`./scripts/rust-test.sh [--include-ignored|--ignored] [<crate>]`**
  — `cargo test`. Workspace by default; pass a crate name to
  narrow. `--ignored` runs *only* `#[ignore]`-gated tests;
  `--include-ignored` runs everything (ignored plus the default
  set). No flag = default cargo behavior (skip ignored).

### Don't go raw

**Do not run raw `cargo fmt`, `cargo check`, `cargo clippy`, or
`cargo test`** when the scripts above can do the job. Bespoke
cargo invocations (e.g. `cargo test some_specific_test` for
focused debugging, or `cargo clippy` with a specific lint toggled
off for a one-off investigation) remain fine — the rule targets
the canonical pre-landing flow, not exceptional cases.

Doc-only commits (markdown, scripts, `Cargo.toml` metadata that
doesn't affect code) may skip these. Anything that could affect
a `.rs` file's compilation or test outcome — including dependency
bumps — must run all three phases (lint, workspace-test,
per-crate-ignored for each touched crate).

These rules apply equally to humans and AI agents (Claude Code
reviewing, Codex implementing). Don't hand off or commit
unverified code.

## CI

Each crate's CI runs at minimum:

- `cargo build` against MSRV.
- `cargo test` against current stable.
- `cargo clippy --all-targets` with warnings as errors.
- `cargo fmt --check`.
- `cargo doc --no-deps`.

CI mirrors the local pre-landing checks above (plus MSRV build
and `cargo doc`). If CI fails on a check that passed locally,
the cause is almost always a dirty working tree or MSRV drift —
investigate before forcing through.

Integration tests requiring external infrastructure (testcontainers
for MySQL, real connector services for some tests) are gated
behind features for CI flexibility.

## Trait crate vs. implementation crate split

When a concern has a trait surface and one or more implementations,
they live in separate crates rather than being feature-gated within
one.

Reasoning:

- **Dependency hygiene**: trait crate minimal; implementations carry
  their own dependencies.
- **Independent versioning**: bug fix in one implementation doesn't
  require trait crate release.
- **Discoverability**: implementations are separate crates on
  crates.io with their own pages.
- **No feature flag combinatorics**: each crate tested in isolation.

Example: `philharmonic-store` (traits) + `philharmonic-store-sqlx-
mysql` (SQL impl); not `philharmonic-store` with `mysql` feature.

## Re-export discipline

Crates re-export types from their direct dependencies that appear in
their own public APIs. Consumers get a flat namespace.

Example:

```rust
// philharmonic-store re-exports Uuid, Sha256, EntityId, etc.
// from philharmonic-types.
use philharmonic_store::{ContentStore, Uuid, EntityId};  // works
```

Rules:

- Re-export what appears in the crate's own public API.
- Don't re-export transitive dependencies.
- Don't re-export types the crate doesn't itself use.

## Error types

Errors use `thiserror` for display and source-chain. Partition by
what the caller does with them (semantic violations, concurrency
outcomes, backend failures). Methods like `is_retryable()` give
uniform checks.

Don't use `anyhow` in library crates — callers can't match on
specific failure modes. Use `anyhow` in application binaries where
appropriate.

## Async runtime

`tokio` is the workspace default. Avoid `async-std` or other
runtimes for consistency. Use `tokio::sync` primitives where
appropriate.

`async-trait` is used where trait objects need to be dyn-compatible
(current Rust stable async-in-traits support is insufficient for
trait-object use).

## Testing

Unit tests colocated with source (`#[cfg(test)] mod tests` or
`mod.rs` tests). Integration tests in `tests/` directory. Real-
infrastructure tests (testcontainers, network) gated behind
features when appropriate.

Test helpers factored into a shared module within the tests
directory; no separate testing crates unless the helpers are
genuinely shared across crates.

## Workspace inspirations

Conventions draw from:

- **Tokio's crate split**: `tokio`, `tokio-util`, `tokio-stream`.
  Separate crates for separate concerns with consistent naming.
- **sqlx's trait/backend pattern**: one trait surface, multiple
  backends. Philharmonic chose separate crates over features for
  the reasons above.
- **The Rust API Guidelines**: followed where applicable.

## When conventions should change

Conventions aren't immutable. Signs one should be revisited:

- **Repeated workarounds**: multiple crates work around the same
  convention.
- **Friction for new crates**: adding a new crate requires
  significant ceremony.
- **Ecosystem drift**: broader Rust conventions change.

Convention changes are workspace events: announced, applied in
coordinated releases, documented here.
