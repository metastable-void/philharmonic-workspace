# AGENTS.md — for Codex

> **Who this file is for.** This file is instructions for **Codex**
> (the OpenAI Codex CLI agent), not for Claude Code.
>
> In this workspace the division of labor is:
> - **Claude Code** = designer, reviewer, orchestrator, workspace
>   caretaker. It writes the prompts you receive, reviews your
>   output, and drives Git. Claude reads `CLAUDE.md` (not this file).
> - **Codex (you)** = implementation partner. You write the real
>   Rust. You work inside the task Claude hands you. You don't
>   design from scratch, and you don't commit or push.
>
> If a rule below would conflict with a direct system/developer
> instruction in the prompt you received, the direct instruction
> wins (per the built-in AGENTS.md spec).

Authoritative detail for every rule summarized here lives in
`docs/design/13-conventions.md`. Read it when a summary isn't
enough.

## Your role in this workspace

- **Implement what the prompt asks.** Don't redesign scope, don't
  refactor surrounding code, don't "polish" unrelated files.
  Claude already decided the shape of the task; your job is to
  land a correct implementation of it.
- **Claude reviews your output.** If something in the prompt is
  ambiguous or contradicts the codebase, flag it in your final
  message rather than guessing — Claude will adjudicate and
  re-prompt if needed.
- **Don't commit, don't push, don't branch.** Leave the working
  tree dirty. Claude commits via `scripts/*.sh` after review. See
  *Git* below.

## Git

Read-only git is fine (`git log`, `git diff`, `git show`,
`git blame`, `git rev-parse`, `git submodule status`). State-
changing git is Claude's job, not yours:

- **Do not run** `git commit`, `git push`, `git add`, `git reset`,
  `git rebase`, `git stash`, `git branch` (with `-D` etc.),
  `git checkout <branch>`.
- **Do not touch** `.gitmodules` or submodule pointers. The
  workspace has ~23 submodules with ordering rules encoded in
  `scripts/*.sh`; Claude drives those.
- When you finish, leave edits in the working tree. Claude runs
  `scripts/commit-all.sh` and `scripts/push-all.sh`.

If the prompt genuinely requires a git state change (e.g. "create
a branch and open a PR"), stop and surface that — don't guess.

## Rust conventions

- **Edition 2024, MSRV 1.85.** Every `Cargo.toml` already carries
  `edition = "2024"` and `rust-version = "1.85"` — match.
- **License.** All crates are `Apache-2.0 OR MPL-2.0` with both
  license files at the crate root. **Do not add per-file copyright
  or license headers** — this workspace doesn't use them.
- **Errors.** Use `thiserror` for library crates. Partition errors
  by what the caller does with them (semantic violations,
  concurrency outcomes, backend failures). Expose predicates like
  `is_retryable()` where useful. **Do not use `anyhow` in library
  crates** — callers can't match on specific failure modes. Binary
  crates may use `anyhow`.
- **Async.** `tokio` is the workspace default. Use `tokio::sync`
  primitives. Use `async-trait` on traits that need to be
  dyn-compatible.
- **Re-exports.** Re-export types from direct dependencies that
  appear in a crate's own public API. Don't re-export transitive
  dependencies. Don't re-export types the crate doesn't itself use.
- **Trait vs. impl split.** Concerns with multiple implementations
  live in separate crates (e.g. `philharmonic-store` traits,
  `philharmonic-store-sqlx-mysql` impl) — not feature-gated within
  one crate. Follow this pattern when adding new impls.
- **Crate-name pattern.** `<subsystem>-<concern>[-<implementation>]`,
  e.g. `philharmonic-connector-impl-sql-mysql`.
- **Version pinning.** Peer workspace crates pin loosely to each
  other (`philharmonic-store = "0.1"`). The cornerstone
  (`philharmonic-types`) is pinned to minor. Pin to a specific
  patch (`"0.3.3"`) only when relying on a feature introduced in
  that patch — not out of habit.
- **Testing.** Unit tests colocated (`#[cfg(test)] mod tests`).
  Integration tests in `tests/`. Tests that need real
  infrastructure (testcontainers, network, etc.) **must** be
  feature-gated so default CI runs without them.
- **Comments.** Default to *no* comments. Only write one when the
  *why* is non-obvious (hidden constraint, subtle invariant,
  workaround for a specific bug). Don't narrate *what* — names do
  that.

## Shell scripts

If the task has you writing or editing a shell script in this
workspace:

- **`#!/bin/sh`, not `#!/usr/bin/env bash`.** Scripts are POSIX
  sh. No bashisms (`[[ ]]`, `=~`, arrays, `<<<`, `<(...)`,
  `mapfile`, `${var:0:N}`, `$'\e[...]'`, `local`, `${BASH_SOURCE[0]}`).
- **`set -eu`, not `set -euo pipefail`** — `pipefail` isn't POSIX.
- **Invoke by path**: `./scripts/foo.sh`, not `bash scripts/foo.sh`.
  Prefixing `bash` hides bashisms that would break on Alpine /
  FreeBSD / macOS.
- **Validate with `./scripts/test-scripts.sh`** (runs `dash -n`
  against every `scripts/*.sh`, falling back to `sh -n`) before
  concluding. CI runs the same check.
- Explicit POSIX deviations (e.g. `ps -o rss=`) are tracked in
  `docs/design/13-conventions.md §Shell scripts`. Don't introduce
  new ones without a recorded reason.

## HUMANS.md (do not touch)

`HUMANS.md` at the repo root is a human-authored note-to-self.
It's part of your context, not your output surface.

- **You MAY read it** for context on Yuka's thinking, preferences,
  and current focus. Reading it is encouraged when it might
  inform the work you're doing.
- **You MUST NOT modify it.** No edits, no appends, no "helpful"
  reformatting, no auto-generated sections. No exceptions.
- You don't commit in this workspace anyway (see §Git), so the
  commit-side rules don't apply to you — just: read freely,
  never modify.

If something in `HUMANS.md` looks wrong, outdated, or contradicts
the code you're touching, flag it in your final summary so
Claude Code can bring it to Yuka's attention. Don't edit the
file to "fix" it.

For the full rule set, see `docs/instructions/README.md`
§HUMANS.md.

## Crypto-sensitive paths

The following areas require Yuka's personal review and must not
be altered silently:

- SCK encrypt/decrypt.
- COSE_Sign1 signing/verification; COSE_Encrypt0
  encryption/decryption.
- The ML-KEM-768 + X25519 + HKDF + AES-256-GCM hybrid construction.
- Payload-hash binding.
- `pht_` API token generation.

You are allowed to implement crypto code when the task asks for
it — but **call it out clearly in your final summary** so the
review gate isn't missed. If the task *doesn't* mention these
areas and your implementation drifts into touching them, stop
and surface that rather than proceeding.

## Before you hand off

Before concluding any task that touched a `.rs` file (including
transitive effects — e.g. a `Cargo.toml` dep bump), run:

```bash
./scripts/pre-landing.sh
```

It auto-detects modified crates and runs the full flow:
`rust-lint.sh` (fmt + check + clippy `-D warnings`), then
`rust-test.sh` (`cargo test --workspace`, skips `#[ignore]`),
then `rust-test.sh --ignored <crate>` for each modified crate
(exercises the `#[ignore]`-gated integration tests). If any
step fails and you can't get it green within the task, say so
in your final summary. Don't hand off red code.

- **Use the scripts**, not raw `cargo fmt/check/clippy/test`.
  The scripts encode the mandated flags (`-D warnings`,
  `--all-targets`, etc.) so you don't have to remember them.
  Bespoke `cargo test <pattern>` for focused debugging remains
  fine; the rule targets the canonical pre-landing flow.
- Clippy runs with `-D warnings` — warnings are errors. Fix the
  root cause. Only add `#[allow(clippy::<lint>)]` at the
  *narrowest* scope, with a one-line comment explaining why,
  when a lint is genuinely wrong for a specific call site.
- If the fmt-check step fails, run `cargo fmt --all` (or
  `cargo fmt -p <crate>`) to apply and re-run `pre-landing.sh`.
- `#[ignore]` is the project convention for tests that need real
  infrastructure (testcontainers, live services) and are slow.
  The workspace-level run skips them; the per-modified-crate
  `--ignored` phase exercises them for crates you touched.

Doc-only / config-only / script-only changes can skip these (no
`.rs` file touched).

## Documentation

When a change warrants a doc update (new trait, new error
variant, changed public API, new crate), update the matching
docs:

- Per-crate `README.md` for usage.
- Rustdoc for API reference (cornerstone crates target 99%+
  coverage).
- `docs/design/*.md` for architectural changes.

Don't create new top-level docs (`NOTES.md`, `TODO.md`, etc.)
unless the task asks. Work from the prompt; don't leave
scratch-files behind.

## When in doubt

If the task seems to fall outside these rules, or the rules
themselves seem to conflict with what the prompt asks, surface
the tension in your response instead of guessing. Claude is
waiting to review and re-prompt if needed.
