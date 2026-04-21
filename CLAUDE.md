# Philharmonic Workspace

Personal development project for generic workflow orchestration
infrastructure. Rust crate family, submodule-based workspace.

Developer: Yuka MORI.

- Primary plan: see ROADMAP.md
- Design docs: see docs/design/ (authoritative for architecture)
- Crypto-sensitive paths require Yuka's personal review; see
  ROADMAP.md §5.
- Submodule discipline: commit inside submodule first, push, then
  bump parent pointer. See ROADMAP.md §2.
- ROADMAP.md is living. When a phase/task completes or plans
  change, update ROADMAP.md in the same commit as the work — not
  as a follow-up. A stale roadmap is worse than none. See
  docs/design/13-conventions.md §ROADMAP maintenance.
- Pre-landing checks: before committing Rust changes, run
  `./scripts/rust-lint.sh` (fmt + check + clippy -D warnings)
  and `./scripts/rust-test.sh` (workspace tests, skip
  `#[ignore]`). Plus, for **every** crate modified in the
  commit, run `./scripts/rust-test.sh --ignored <crate>` to
  exercise its integration tests. Don't run raw `cargo
  fmt/check/clippy/test` when the scripts cover the case. See
  docs/design/13-conventions.md §Pre-landing checks.
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
- Git workflow: all Git operations go through `scripts/*.sh`
  (`status.sh`, `pull-all.sh`, `commit-all.sh`, `push-all.sh`).
  If a script doesn't cover what you need, extend the script
  first rather than running ad-hoc git commands. Every commit is
  DCO-signed off (`-s`) and cryptographically signed (`-S`, GPG
  or SSH); `commit-all.sh` enforces both and verifies the
  signature post-commit. See docs/design/13-conventions.md §Git
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
  submodules recursively and warns if the Rust toolchain is
  missing.
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