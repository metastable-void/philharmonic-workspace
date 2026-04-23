# Philharmonic Workspace

Personal development project for generic workflow orchestration
infrastructure. Rust crate family — most member crates are git
submodules, with an in-tree `xtask/` crate for workspace dev
tooling (never published).

Developer: Yuka MORI.

- Primary plan: see ROADMAP.md
- Design docs: see docs/design/ (authoritative for architecture)
- Crypto-sensitive paths require Yuka's personal review; see
  ROADMAP.md §5.
- **POSIX-ish host required.** This workspace assumes a
  POSIX-ish development host: GNU/Linux (incl. WSL2 on
  Windows), macOS (Darwin), BSDs (FreeBSD/OpenBSD/NetBSD/
  DragonFly), illumos/Solaris, and musl distros (Alpine).
  Before running any script, spawning Codex, or attempting a
  Git state change, check the environment block's `Platform:`
  field. If it reports `Platform: linux` / `darwin` /
  `freebsd` / `openbsd` / `netbsd` / etc., proceed. If it
  reports `Platform: win32` (raw Microsoft Windows), **STOP
  IMMEDIATELY** — surface the mismatch in your first message
  and instruct the human to switch to WSL2. Do not run
  scripts, do not commit, do not spawn Codex. There's no
  runtime gate inside the scripts because raw Windows can't
  execute `#!/bin/sh` in the first place; the gate lives here,
  in the docs, for the agent's benefit. On Git Bash / MSYS /
  Cygwin (POSIX-compat layers): proceed with caution; flag any
  submodule / signing / permission anomaly before continuing.
  See docs/design/13-conventions.md §Development environment.
- Submodule discipline: commit inside submodule first, push, then
  bump parent pointer. See ROADMAP.md §2.
- ROADMAP.md is living. When a phase/task completes or plans
  change, update ROADMAP.md in the same commit as the work — not
  as a follow-up. A stale roadmap is worse than none. See
  docs/design/13-conventions.md §ROADMAP maintenance.
- **Write workspace-scoped conventions into the repo, not into
  machine-local memory.** When you learn a rule that applies to
  *this project* ("in this workspace, prefer X over Y", "every
  crate must do Z"), its durable home is the repo — this file,
  `AGENTS.md`, or `docs/design/13-conventions.md` — not your
  per-agent-install memory store. Memory is per-machine and
  doesn't travel with the repo to other clones, other
  contributors, or fresh WSL boxes; the repo does. Reserve
  machine-local memory for genuinely machine-local facts (e.g.
  "on this box, rustup/gh were installed on <date>"). Recurrent
  feedback that names the project or its conventions belongs in
  the repo, full stop — otherwise the same lesson has to be
  re-learned on every new machine.
- **Prefer `scripts/*.sh` wrappers over raw `cargo`.** Same rule
  as the git-workflow one below, for the same reason: the
  wrappers encode flag choices, auto-install, workspace-cd, and
  POSIX-compatibility guards that ad-hoc `cargo` invocations
  skip. Contributor-vs-CI parity is only guaranteed *because*
  the scripts are the single source of truth. Inventory (see
  README.md §"The workspace scripts" for the full list with
  flags):
  - `./scripts/pre-landing.sh` — `fmt --check` + `check` +
    `clippy -D warnings` + `test`, auto-detecting modified crates.
    Run before every commit that touches Rust.
  - `./scripts/rust-lint.sh [<crate>]` — fmt + check + clippy,
    workspace or per-crate.
  - `./scripts/rust-test.sh [--include-ignored|--ignored] [<crate>]`
    — cargo test with ignored-test control.
  - `./scripts/miri-test.sh --workspace | <crate>...` — routine
    `cargo +nightly miri test` for UB checks (uninitialized
    memory, OOB pointer arithmetic, data races, stacked borrows).
    Requires nightly + miri (installed by `setup.sh`, probed by
    `check-toolchain.sh`). Slow — not in `pre-landing.sh`; run
    manually pre-publish and on a schedule.
  - `./scripts/cargo-audit.sh` — RustSec advisories.
  - `./scripts/check-api-breakage.sh <crate> [<baseline-version>]`
    — cargo-semver-checks, per-crate, crates.io baseline.
  - `./scripts/publish-crate.sh [--dry-run] <crate>` — publish +
    signed tag.
  - `./scripts/verify-tag.sh <crate> [<tag>]` — three-way check
    that a release tag is locally present, signed, and pushed to
    origin at the same commit. Run after publish + push.
  - `./scripts/crate-version.sh <crate> | --all` — local version
    from Cargo.toml.
  - `./scripts/xtask.sh crates-io-versions -- <crate>` —
    published versions from crates.io sparse index (Rust bin in
    `xtask/`; replaces the former shell script, which depended
    on `jq` + `web-fetch.sh`).
  - `./scripts/check-toolchain.sh [--update]` — rust toolchain
    version + update probe.
  If no wrapper covers your case, extend a script or write a new
  one (see the "Extract routines into scripts" bullet below)
  rather than reaching for raw `cargo`. **Exempt** category:
  read-only cargo queries (`cargo tree`, `cargo metadata`,
  `cargo --version`) — nothing wraps these and they're cheap.
  See docs/design/13-conventions.md §Script wrappers for the
  authoritative statement and rationale.
- **Don't reach for `python`, `perl`, `ruby`, `node`, or any
  other non-baseline scripting language for workspace tooling.
  If you're tempted, write a Rust bin in `xtask/` instead.**
  Well-written POSIX shell (with `awk`, `sed`, `grep`, `cut`,
  `tr`, standard SUSv4 pipelines) remains the right home for
  simple orchestration, git workflow, cargo wrappers,
  filesystem glue — keep those in `scripts/*.sh`. The rule
  targets ad-hoc `python3 -c "..."` / `perl -e "..."` creep and
  non-POSIX / non-baseline tools (`jq`, `curl`, `wget`) — if
  you'd reach for `jq`, that's the signal to add a Rust bin in
  `xtask/` using `serde_json` instead. HTTP fetching already
  lives in `xtask/src/bin/web-fetch.rs` for the same reason. `xtask/` is an in-tree (non-submodule)
  member crate at the workspace root, multi-bin layout
  (`src/bin/*.rs`, one bin per tool), `publish = false`.
  Invoke via `./scripts/xtask.sh <tool> -- <args>` — the
  wrapper provides `--list`, `--help`, and a mandatory `--`
  separator before bin args. Don't call `cargo run -p xtask
  --bin <tool>` directly at call sites; `xtask.sh` is the
  single invocation surface. Current bins: `gen-uuid` (every
  stable wire-format UUID — entity `KIND` constants, algorithm
  identifiers, anything that once committed must never change)
  and `crates-io-versions` (list published versions of a
  crate). See docs/design/13-conventions.md §In-tree workspace
  tooling and §KIND UUID generation.
- **Every stable UUID used as a wire-format constant is
  generated via `./scripts/xtask.sh gen-uuid -- --v4`.** Not
  `python3 -c "import uuid"`, not `uuidgen`, not an online
  generator, and not a direct `cargo run` invocation. One
  canonical source — `xtask.sh` — so nobody accidentally
  commits a value they generated ad-hoc and meant to throw
  away.
- **No panics in library code.** This is systems-programming
  infrastructure — request handlers, long-lived services, crypto,
  storage — and a panicking task is user-visible failure with
  hard-to-diagnose blast radius. `src/**` in every published
  crate treats panics as bugs: no `.unwrap()` / `.expect()` on
  `Result`/`Option`, no `panic!` / `unreachable!` / `todo!` /
  `unimplemented!` on reachable paths, no unbounded indexing
  (`slice[i]`, `slice[a..b]`, `map[&k]` — use `.get(...)` →
  `Option` and propagate), no unchecked integer arithmetic
  (use `checked_*` / `saturating_*` / `wrapping_*` to declare
  intent), no lossy `as` casts on untrusted widths (use
  `TryFrom`). Narrow exceptions require an inline justification
  (unrecoverable OS failure like `OsRng` entropy exhaustion —
  the one approved pattern at
  `philharmonic-policy/src/sck.rs`; compile-time-validated
  literals like `uuid!(...)`; type-witness unreachability that
  the compiler can't express). Tests / `dev-dependencies` /
  `xtask/` bins can `.unwrap()` freely; production crate `src/`
  cannot. See docs/design/13-conventions.md §Panics and
  undefined behavior for the full rule with examples and the
  list of relevant future-Clippy lints.
- **Library crates take bytes, not file paths.** A library's
  public API accepts data (byte slices, pre-parsed structs,
  `Zeroizing<[u8; N]>` for private-key material) — never a
  `&Path`, filename, environment-variable name, or config-file
  path. File I/O, file-permission checks, environment lookup,
  and config-file parsing belong in the bin crate that holds
  the library's runtime context. The rule applies across the
  workspace and is a Gate-1 smell for crypto-adjacent APIs in
  particular. See docs/design/13-conventions.md §Library crate
  boundaries for the rule, rationale, and concrete examples
  (secret keys, trust stores, TLS certs, config structs).
- **Never recall a Rust crate's published version from memory
  — always look it up via
  `./scripts/xtask.sh crates-io-versions -- <crate>`.**
  Applies to every question about "what's the latest version of
  X on crates.io?", "has Y.Z.W been published yet?", or "what
  versions exist for this crate?" — whether it's a third-party
  crate or one of this workspace's own crates. Published
  versions drift constantly (new releases, yanks, pre-releases),
  model training data is months to years stale, and prior-
  session memory is frozen in time. Echoing a remembered number
  — even one that was right last week — is how wrong pins and
  wrong changelogs get written. The wrapper hits the crates.io
  sparse index and is authoritative. The only exception is the
  version declared *in this workspace's own `Cargo.toml`*, which
  `./scripts/crate-version.sh <crate>` reports — that's
  "what we're about to publish", not "what's on crates.io",
  and the two can legitimately differ. See
  docs/design/13-conventions.md §Crate version lookup.
- **Same rule for `mktemp`, `curl`, `wget`, and other external
  non-Rust tools.** Never call `mktemp`, `curl`, or `wget`
  directly from a workspace script. Use the wrappers:
  - `./scripts/mktemp.sh [<slug>]` — creates a temp file under
    `$TMPDIR` (or `/tmp`), falls back to a `/dev/urandom`-based
    suffix when `mktemp(1)` isn't installed. **Pair with a
    `trap 'rm -f "$tmp"' EXIT INT HUP TERM` in the caller** —
    the wrapper doesn't clean up for you.
  - `./scripts/web-fetch.sh <URL> [<outfile>]` — HTTP GET via
    whichever of `curl`/`wget`/`fetch`/`ftp` is present. All
    backends fail on HTTP 4xx/5xx (curl is invoked with `-f`).
    User-Agent override via `WEB_FETCH_UA`. Use `... || :` at
    the call site if you want to tolerate HTTP errors (see
    `print-audit-info.sh` for the idiom).
  The rule exists because these tools vary across minimal
  environments (Alpine busybox, FreeBSD, OpenBSD, macOS, WSL).
  The wrappers encode the portable choice once. New scripts
  that need temp files or HTTP must call the wrapper; if the
  wrapper doesn't do what you need, extend it — don't reach
  around it. See docs/design/13-conventions.md §External tool
  wrappers.
- **Notes to humans.** When you tell Yuka anything significant
  (verification results with informative "why", platform
  caveats, audit findings, mid-implementation design calls,
  non-obvious failure modes, anything flagged as "note this"),
  also write it to `docs/notes-to-humans/YYYY-MM-DD-NNNN-<slug>[-NN].md`
  and commit via `./scripts/commit-all.sh --parent-only` in the
  same session. Session-only output is not enough for
  substantial notes. See docs/design/13-conventions.md §Notes to
  humans and §Journal-like files (filename format).
- **HUMANS.md is read-only for agents.** `HUMANS.md` at the repo
  root is a human-authored note-to-self. You MAY read it freely
  for context on what Yuka is thinking; you MUST NOT edit, append
  to, or auto-generate content in it. `./scripts/commit-all.sh`
  automatically sweeps in any pending human edits via
  `git add -A` — that's fine. See docs/instructions/README.md
  §HUMANS.md for the full rule.
- **Terminology and language.** Prose you author — docs, code
  comments, commit messages and commit-message trailers,
  notes-to-humans entries, PR descriptions — follows the
  inclusive-neutral + FSF-preferred rules at
  [README.md §Terminology and language](README.md). Short form:
  no `master`/`slave` for technical relationships (use
  `primary`/`replica`, `leader`/`follower`, `parent`/`child`,
  etc.); no gendered defaults (prefer singular "they", avoid
  "guys"/"man" as generics); prefer `allowlist`/`denylist` over
  whitelist/blacklist; prefer `stub`/`placeholder`/`fake` over
  "dummy"; GNU/Linux (OS) vs. Linux kernel — don't collapse;
  `Microsoft Windows`/`Windows` in prose, never `win*`
  shorthand; prefer "free software" / "FLOSS" over standalone
  "open-source" unless quoting external conventions. Technical
  accuracy overrides aesthetic neutrality — use literal
  external identifiers (HTTP `Authorization`, `Win32`,
  `x86_64-pc-windows-msvc`, `uname -s`'s `Linux` string) as
  they ship. Authoritative statement with full anti-pattern
  list and exceptions: docs/design/13-conventions.md §Naming
  and terminology.
- Git workflow: all Git operations go through `scripts/*.sh`
  (`status.sh`, `pull-all.sh`, `commit-all.sh`, `push-all.sh`).
  If a script doesn't cover what you need, extend the script
  first rather than running ad-hoc git commands. Every commit is
  DCO-signed off (`-s`) and cryptographically signed (`-S`, GPG
  or SSH); `commit-all.sh` enforces both and verifies the
  signature post-commit. Tracked Git hooks under `.githooks/`
  (wired up by `setup.sh` via `core.hooksPath` on the parent and
  every submodule) also enforce these: `.githooks/pre-commit`
  rejects any commit that didn't come through `commit-all.sh`,
  `.githooks/commit-msg` rejects any message without a matching
  `Signed-off-by:` trailer, and `.githooks/post-commit` rolls
  back any unsigned commit that slipped through.
  **Git history is append-only** — no `git commit --amend`, no
  `git rebase`, no `git reset --hard`, no `git push --force`, no
  other history rewriting. Two authorized exceptions, both
  script-enforced and both bounded to local not-yet-pushed
  commits: (1) the unsigned-commit rollback in `post-commit` /
  `commit-all.sh`, and (2) the `--rebase` in `pull-all.sh`
  (parent `git pull --rebase` + `git submodule update --remote
  --rebase`; replays local-only commits when upstream moved).
  Mistakes otherwise ship as new commits; pushed mistakes get
  `git revert`ed. See docs/design/13-conventions.md §Git
  workflow.
- Shell scripts are **POSIX sh** (`#!/bin/sh`), not bash. No
  bashisms; explicit deviations (e.g. `ps -o rss=`) are tracked in
  docs/design/13-conventions.md §Shell scripts. Validate with
  `./scripts/test-scripts.sh` (mandatory after any script change;
  CI runs the same check).
- **Extract routines into scripts, not ad-hoc commands.** When
  you find yourself running the same command sequence more than
  once or twice — especially multi-line flows with flags,
  `git submodule foreach` loops, or POSIX-compatibility guards —
  stop and turn it into a `scripts/*.sh` file. Scripts are
  reviewable, testable, discoverable, and they capture flag
  choices that otherwise drift. The bar is low; a one-liner
  becomes a two-line script. After extracting: validate with
  `./scripts/test-scripts.sh`, update the scripts list in
  README.md + the `git-workflow` skill (if git-related), and
  document any associated rule here or in the conventions doc.
  See docs/design/13-conventions.md §Shell scripts.
- Codex has its own instruction file: `AGENTS.md` at the repo
  root. It's auto-loaded by Codex when it runs, and mirrors the
  Claude-vs-Codex division (Claude designs/commits; Codex
  implements; Codex doesn't run git). `.codex/config.toml` holds
  project-local Codex CLI settings (activated via
  `CODEX_HOME=.codex`). Don't edit AGENTS.md casually — it's the
  contract Codex sees every run.
- Fresh clone: run `scripts/setup.sh` once — it initializes all
  submodules recursively, installs the tracked Git hooks under
  `.githooks/` (via `core.hooksPath`), configures
  `commit.gpgsign=true` / `tag.gpgsign=true` on the parent and
  every submodule, sets `push.recurseSubmodules=check`, and
  warns if the Rust toolchain is missing. Idempotent.
- Claude-side procedures live as skills under `.claude/skills/`:
  `git-workflow` (the scripts-only rule above),
  `codex-prompt-archive` (how to hand off to Codex), and
  `crypto-review-protocol` (Yuka's two-gate review on
  crypto-sensitive paths). Consult them when their triggers fire.

## Claude vs. Codex division of labor

In this project Claude Code is the designer, reviewer, and
workspace caretaker; Codex is the implementation partner for
substantive coding.

- **Claude Code does**: architecture, API shape, design docs,
  ROADMAP updates, code review, workspace/repo management
  (scripts, Cargo.toml plumbing, submodule wrangling, doc
  reconciliation, small fixes that are really housekeeping).
  Coding that's maintenance rather than feature work stays with
  Claude — no Codex hand-off needed.
- **Codex does (spawned via plugins)**: non-trivial concrete
  coding — a crate's actual implementation, an algorithm, a
  connector impl, test suites of real size, anything where the
  work is "sit down and write real Rust." For those, Claude
  writes the prompt, spawns Codex through the `codex:` plugin,
  and reviews what comes back.

Rule of thumb: if the question is "what should this look like?"
Claude answers. If the question is "now write the thing," Claude
hands off to Codex unless it's plumbing/housekeeping that
doesn't warrant the round-trip.

**Archive every Codex prompt.** Before spawning Codex, write the
prompt to `docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>[-NN].md` and
commit it (via `scripts/commit-all.sh`). No Codex invocation is
ephemeral — the prompt is part of the project record. See
docs/design/13-conventions.md §Codex prompt archive (and
§Journal-like files for the filename format).