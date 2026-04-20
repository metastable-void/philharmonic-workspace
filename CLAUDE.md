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