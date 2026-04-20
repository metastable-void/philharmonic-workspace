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
- Git workflow: all Git operations go through `scripts/*.sh`
  (`status.sh`, `pull-all.sh`, `commit-all.sh`, `push-all.sh`).
  If a script doesn't cover what you need, extend the script
  first rather than running ad-hoc git commands. Every commit is
  signed off (`-s`). See docs/design/13-conventions.md §Git
  workflow.

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