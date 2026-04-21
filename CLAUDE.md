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
  `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, and `cargo test --workspace`.
  All three must pass. No exceptions for "small" changes. See
  docs/design/13-conventions.md §Pre-landing checks.
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
  `dash -n scripts/*.sh` before landing.
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
prompt to `docs/codex-prompts/YYYY-MM-DD-<slug>.md` and commit
it (via `scripts/commit-all.sh`). No Codex invocation is
ephemeral — the prompt is part of the project record. See
docs/design/13-conventions.md §Codex prompt archive.