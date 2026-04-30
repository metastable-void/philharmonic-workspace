# Incident: stray `target-main/` in submodule + accidental direct `sqlx` dependency

**Date**: 2026-04-30
**Author**: Claude Code
**Severity**: Low (caught before publish; no user-facing impact)

---

## What happened

During the Phase 9 task 4c Codex dispatch (`philharmonic-api`
bin target), two issues were introduced:

### 1. Stray `target-main/` inside `philharmonic/` submodule

Codex ran `CARGO_TARGET_DIR=target-main cargo build` from
inside `philharmonic/` (the submodule directory) rather than
from the workspace root. This created
`philharmonic/target-main/` — a build artifact directory
inside the submodule's tracked tree.

The submodule's `.gitignore` only listed `/target`, not
`/target-main`, so the stray directory would have been swept
into the next `commit-all.sh` invocation as tracked content
(~hundreds of MB of compiled artifacts).

**Detection**: Yuka spotted it during review.

**Fix**: Removed the directory, added `/target-main`,
`/target-xtask`, `/target-publish` to
`philharmonic/.gitignore`. Same entries should be added to
every submodule's `.gitignore` as a preventive measure.

### 2. Direct `sqlx` dependency in the meta-crate

The Codex prompt specified adding `sqlx` as a direct
dependency to `philharmonic/Cargo.toml` for
`MySqlPool::connect()`. This violates the layered-dependency
principle: the meta-crate's bin targets should use the store
backend's public API, not reach into the backend's internal
dependencies.

`philharmonic-store-sqlx-mysql` already depends on `sqlx`
and wraps it behind `SinglePool::new(pool)`, but didn't
expose a `connect(url)` convenience constructor — which is
why the prompt asked for a direct `sqlx` dep as a workaround.

**Detection**: Yuka flagged the `sqlx` dep during Codex's
run, before Claude committed the output.

**Fix**: Added `SinglePool::connect(url: &str)` to
`philharmonic-store-sqlx-mysql` and `#[derive(Clone)]` on
`SinglePool` (safe — `MySqlPool` is `Arc`-based). Removed
the direct `sqlx` dep from the meta-crate. Updated the bin
to use `SinglePool::connect()` instead of
`MySqlPool::connect()`.

---

## Root causes

1. **Codex working directory**: Codex's sandbox sometimes
   `cd`s into a submodule directory for `cargo` commands.
   `CARGO_TARGET_DIR=target-main` then creates the target
   dir relative to the submodule, not the workspace root.
   The `.gitignore` entries in submodules didn't anticipate
   this because normal development always runs from the
   workspace root.

2. **Prompt specified the wrong dependency**: The Codex
   prompt (written by Claude) explicitly told Codex to add
   `sqlx` as a direct dependency. The correct approach —
   extending the store crate's public API — wasn't
   considered at prompt-writing time because the gap in
   `SinglePool`'s API wasn't discovered until review.

---

## Preventive actions

### Done in this session

- Added `/target-main`, `/target-xtask`, `/target-publish`
  to `philharmonic/.gitignore`.
- Added `SinglePool::connect()` to `philharmonic-store-sqlx-mysql`
  so future bin targets don't need direct `sqlx`.

### TODO (future commits)

- **Add `target-main/` to every submodule's `.gitignore`.**
  A sweep across all 25+ submodules adding the three target-dir
  entries. Low urgency (only matters if Codex or a human runs
  cargo from inside a submodule), but cheap insurance.

- **Codex prompt template update**: Add a standard line to
  future Codex prompts: "Run all cargo commands from the
  workspace root (`/home/ubuntu/philharmonic-workspace`), not
  from inside a submodule directory. If you `cd` into a
  submodule, `cd` back before running cargo."

- **Review Codex prompts for layering violations before
  dispatch**: The `sqlx` dep was in the prompt itself. Claude
  should verify that prompted dependencies respect the crate
  layering before archiving the prompt — catching the gap at
  prompt-writing time would have avoided the issue entirely.
