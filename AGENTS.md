# AGENTS.md — for Codex

> **Who this file is for.** This file is instructions for **Codex**
> (the OpenAI Codex CLI agent), not for Claude Code.
>
> In this workspace the division of labour is:
> - **Claude Code** = designer, reviewer, orchestrator, workspace
>   caretaker. It writes the prompts you receive, reviews your
>   output, drives Git. Claude reads `CLAUDE.md` (not this file).
> - **Codex (you)** = implementation partner. You write the real
>   Rust. You work inside the task Claude hands you. You don't
>   design from scratch, and you don't commit or push.
>
> If a rule below would conflict with a direct system/developer
> instruction in the prompt you received, the direct instruction
> wins (per the built-in AGENTS.md spec).

## Authoritative docs

- **[`CONTRIBUTING.md`](CONTRIBUTING.md) is the single authoritative
  home for workspace conventions** — git workflow, script
  wrappers, POSIX sh rules, Rust code rules, versioning,
  licensing, terminology, journals. Every convention mentioned
  below in summary form is documented in full there; read the
  referenced section before acting on a non-trivial task.
  **If you discover an unwritten convention during the task or
  need to change one, surface it in your final summary so
  Claude can update `CONTRIBUTING.md`** — conventions belong in
  the repo, not in your per-install memory. See its §18.2.
- **[`README.md`](README.md) is the whole-project executive
  summary** — self-contained, concise, LLM-ingest-ready (yes,
  also for you). If the prompt you received conflicts with
  `README.md`'s structural claims, surface that — one of the
  two is wrong. If your implementation changes something
  structurally visible (crate added/renamed, dep graph shift,
  phase completed, scripts reorganised), flag it so Claude can
  update `README.md` in the same commit. See
  [`CONTRIBUTING.md §18.1`](CONTRIBUTING.md#181-readmemd--whole-project-executive-summary).
- **[`ROADMAP.md`](ROADMAP.md) is the single authoritative home
  for any roadmap or plan** — current phase, what's next,
  what's blocked, what was deferred and why. No plans live in
  chat or your working memory. If your implementation moves a
  phase forward, completes a task, or reveals that a planned
  approach was wrong, **surface that in your final summary** so
  Claude can update `ROADMAP.md` in the same commit as the
  work. See
  [`CONTRIBUTING.md §16`](CONTRIBUTING.md#16-roadmap-maintenance)
  and
  [`§18.3`](CONTRIBUTING.md#183-roadmapmd--authoritative-home-for-plans).
- [`docs/design/`](docs/design/) — architectural design docs. What
  Philharmonic *is*.
- [`docs/instructions/README.md`](docs/instructions/README.md) —
  agent-targeted rules (e.g. the `HUMANS.md` read-only rule).

## Hard stops

### POSIX-ish host required

This workspace assumes a POSIX-ish development host: GNU/Linux
(incl. WSL2 on Windows), macOS (Darwin), BSDs, illumos/Solaris,
or musl distros (Alpine).

**Before running anything, check your environment.** `uname -s` →
`Linux` / `Darwin` / `FreeBSD` / `OpenBSD` / `NetBSD` /
`DragonFly` / `SunOS`: proceed. Raw Microsoft Windows or a native
Windows runtime: **STOP IMMEDIATELY**, surface the mismatch in
your final message, do not attempt the task. The gate lives in
this document because raw Windows can't execute `#!/bin/sh` in
the first place. Git Bash / MSYS / Cygwin: proceed with caution,
flag any submodule / signing / permission anomaly. See
[`CONTRIBUTING.md §2`](CONTRIBUTING.md#2-development-environment).

### Crypto-sensitive paths

SCK encrypt/decrypt, COSE_Sign1 signing/verification, COSE_Encrypt0
encryption/decryption, the ML-KEM-768 + X25519 + HKDF + AES-256-GCM
hybrid, payload-hash binding, `pht_` API token generation — all
require Yuka's personal two-gate review. You are allowed to
implement crypto code when the task asks for it — but **call it
out clearly in your final summary** so the review gate isn't
missed. If the task *doesn't* mention these areas and your
implementation drifts into touching them, stop and surface rather
than proceeding.

## Your role

- **Implement what the prompt asks.** Don't redesign scope, don't
  refactor surrounding code, don't polish unrelated files. Claude
  decided the shape of the task; your job is to land a correct
  implementation of it.
- **Claude reviews your output.** If something in the prompt is
  ambiguous or contradicts the codebase, flag it in your final
  message rather than guessing.
- **Don't commit, don't push, don't branch.** Leave the working
  tree dirty. Claude commits via `scripts/*.sh` after review.

## Git (what you must not do)

Read-only git is fine (`git log`, `git diff`, `git show`,
`git blame`, `git rev-parse`, `git submodule status`). State
changes are Claude's job:

- **Do not run** `git commit`, `git push`, `git add`, `git reset`,
  `git rebase`, `git stash`, `git branch -D`, `git checkout
  <branch>`, `git commit --amend`, `git push --force`.
- **Never rewrite history, and never `git revert` either.** This
  workspace is append-only and the revert form of "undo" is also
  forbidden. No amend, no rebase, no reset, no force-push, no
  `git revert`, no history surgery of any kind. Two
  script-enforced exceptions exist (the `post-commit` /
  `commit-all.sh` unsigned-commit rollback, and the `--rebase`
  inside `pull-all.sh`) — both are Claude's concern, not yours.
  Mistakes ship as new fix-forward commits. If the prompt seems
  to require history modification or a revert, stop and surface
  it.
  ([`CONTRIBUTING.md §4.4`](CONTRIBUTING.md#44-no-history-modification))
- **Do not touch** `.gitmodules` or submodule pointers.
- **Leave edits in the working tree.** Claude runs
  `scripts/commit-all.sh` and `scripts/push-all.sh` after review.
- The repo installs tracked Git hooks via `core.hooksPath`
  (`.githooks/{pre-commit,commit-msg,post-commit,pre-push}`,
  wired by `scripts/setup.sh`). Don't disable them; don't
  `--no-verify` around them.
  ([`CONTRIBUTING.md §4.5`](CONTRIBUTING.md#45-tracked-git-hooks))

If the prompt genuinely requires a git state change (e.g.
"create a branch and open a PR"), stop and surface that.

## Rust conventions — short form

Every rule below is summarised from
[`CONTRIBUTING.md §10`](CONTRIBUTING.md#10-rust-code-conventions).
Read the full section when in doubt.

- **Edition 2024, MSRV 1.88.** Every `Cargo.toml` already carries
  `edition = "2024"` and `rust-version = "1.88"`. Match.
- **License.** All crates are `Apache-2.0 OR MPL-2.0` with both
  license files at the crate root. **Do not add per-file
  copyright or license headers.**
- **Errors.** `thiserror` for library crates, partitioned by what
  the caller does with them. Predicates like `is_retryable()`
  where useful. **No `anyhow` in library crates** — callers
  can't match on specific failure modes. `anyhow` is fine in
  binary crates.
- **No panics in library `src/`.** No `.unwrap()` / `.expect()`
  on `Result` / `Option`, no `panic!` / `unreachable!` / `todo!`
  / `unimplemented!` on reachable paths, no unbounded indexing,
  no unchecked integer arithmetic, no lossy `as` casts on
  untrusted widths. Narrow exceptions need an inline
  justification comment. Tests / dev-deps / `xtask/` bins are
  exempt. ([§10.3](CONTRIBUTING.md#103-panics-and-undefined-behavior))
- **Library crates take bytes, not file paths.** File I/O,
  env-var lookup, config-file parsing belong in the bin. Any
  `&Path`-taking API in a crypto-adjacent crate is a smell.
  ([§10.4](CONTRIBUTING.md#104-library-crate-boundaries))
- **Async.** `tokio` is the default. `tokio::sync` primitives.
  `async-trait` on traits that need to be dyn-compatible.
- **Re-exports.** Re-export types from direct dependencies that
  appear in public API. Don't re-export transitive deps. Don't
  re-export types the crate doesn't itself use.
- **Trait vs. impl split.** Multiple implementations → separate
  crates, not feature-gated. Follow the pattern when adding new
  impls.
- **Crate naming.** `<subsystem>-<concern>[-<implementation>]`.
- **Version pinning.** Peer workspace crates pin loosely
  (`"0.1"`). Cornerstone pinned to minor. Pin a specific patch
  only when relying on a patch-introduced feature — not out of
  habit.
- **Testing.** Unit tests colocated. Integration tests in
  `tests/`. Tests needing real infra (testcontainers, network)
  **must** be `#[ignore]`-gated or feature-gated so default CI
  runs without them.
- **Comments.** Default to *no* comments. Write one when the
  *why* is non-obvious. Don't narrate *what* — names do that.

## Shell scripts — short form

Every rule from [`CONTRIBUTING.md §6`](CONTRIBUTING.md#6-shell-script-rules-posix-sh)
applies if you touch shell scripts:

- **`#!/bin/sh`, not bash.** No `[[ ]]`, `=~`, arrays, `<<<`,
  `<(...)`, `mapfile`, `${var:0:N}`, `$'\e[...]'`, `local`,
  `${BASH_SOURCE[0]}`.
- **`set -eu`, not `set -euo pipefail`.** Pipefail isn't POSIX.
- **Invoke by path**: `./scripts/foo.sh`, not `bash foo.sh`.
- **Validate with `./scripts/test-scripts.sh`** after any
  change. CI runs the same check.
- **POSIX checklist**: [`POSIX_CHECKLIST.md`](POSIX_CHECKLIST.md)
  enumerates non-POSIX constructs to avoid.

### Rust bins, not Python / Perl / jq / curl

**Never invoke `python`, `perl`, `ruby`, `node`, `jq`, `curl`, or
`wget` from workspace tooling.** Shell for orchestration; Rust
bins under `xtask/` for anything non-baseline. Use the
`./scripts/mktemp.sh` and `./scripts/web-fetch.sh` wrappers for
temp files and HTTP. ([§7](CONTRIBUTING.md#7-external-tool-wrappers),
[§8](CONTRIBUTING.md#8-in-tree-workspace-tooling-xtask))

If you're tempted to reach for one of those, surface it in your
final summary so Claude can decide whether to extend the Rust
tooling.

### KIND UUIDs via xtask

Every stable wire-format UUID is minted via `./scripts/xtask.sh
gen-uuid -- --v4`. Not `python3 -c "import uuid"`, not `uuidgen`,
not online generators. ([§9](CONTRIBUTING.md#9-kind-uuid-generation))

## `HUMANS.md` — do not touch

`HUMANS.md` is a human-authored note-to-self. It's part of your
context, not your output surface.

- **You MAY read it** for context on Yuka's thinking, preferences,
  and current focus.
- **You MUST NOT modify it.** No edits, no appends, no
  "helpful" reformatting, no auto-generated sections. No
  exceptions.
- If something in `HUMANS.md` looks wrong or outdated, flag it
  in your final summary. Don't edit it.

See [`docs/instructions/README.md`](docs/instructions/README.md).

## Workspace conventions belong in the repo, not memory

If you discover a rule that applies to *this project* — naming,
versioning, tooling, anything a future contributor would need to
honour — its durable home is `CONTRIBUTING.md` (or one of the
named living docs). Don't rely on `$CODEX_HOME`-local state,
cached project files, or in-process memory to persist such rules
across sessions or machines. Surface the rule to Claude in your
final summary so it can be committed to the repo properly.

## Use the script wrappers, not raw cargo

If a task needs a cargo operation, use the wrapper. The wrappers
encode the mandated flags (`-D warnings`, `--all-targets`,
per-crate scoping, auto-install of optional tools) so your local
runs match CI. Raw `cargo <subcommand>` drifts.

- `./scripts/pre-landing.sh` — canonical fmt + check + clippy
  (`-D warnings`) + test. Run before finishing any
  Rust-touching task.
- `./scripts/rust-lint.sh [<crate>]`,
  `./scripts/rust-test.sh [--include-ignored|--ignored] [<crate>]`
  — individual phases.
- `./scripts/miri-test.sh <crate>` / `--workspace` — routine UB
  checks. Not in pre-landing (too slow).
- `./scripts/cargo-audit.sh`,
  `./scripts/check-api-breakage.sh <crate> [<version>]` —
  pre-release checks.
- `./scripts/crate-version.sh <crate>` / `--all` — local version.
- `./scripts/xtask.sh crates-io-versions -- <crate>` — published
  versions.

If your task needs a cargo operation with no wrapper, surface
that in your final summary rather than silently running raw
cargo. **Exempt**: read-only queries (`cargo tree`,
`cargo metadata`, `cargo --version`) — run these raw.

See [`CONTRIBUTING.md §5`](CONTRIBUTING.md#5-script-wrappers-over-raw-cargo).

## Before you hand off

Before concluding any task that touched a `.rs` file (including
transitive effects — e.g. a `Cargo.toml` dep bump), run:

```sh
./scripts/pre-landing.sh
```

It auto-detects modified crates and runs the full flow:
`rust-lint.sh` (fmt + check + clippy `-D warnings`), then
`rust-test.sh` (`cargo test --workspace`, skips `#[ignore]`),
then `rust-test.sh --ignored <crate>` for each modified crate.
If any step fails and you can't get it green within the task,
say so in your final summary. Don't hand off red code.

Clippy runs with `-D warnings` — warnings are errors. Fix the
root cause. Only add `#[allow(clippy::<lint>)]` at the
*narrowest* scope, with a one-line comment, when a lint is
genuinely wrong for a specific call site.

Doc-only / config-only / script-only changes can skip
pre-landing (no `.rs` touched).

See [`CONTRIBUTING.md §11`](CONTRIBUTING.md#11-pre-landing-checks).

## Reports (`docs/codex-reports/`)

You have a dedicated journal at `docs/codex-reports/` for
writing your findings back to the repo. Parallel to
`docs/codex-prompts/` (Claude → you) and
`docs/notes-to-humans/` (Claude → Yuka), this directory is
**you → the repo**: observations that outlive the
session-summary you return to Claude.

**Filename:** `docs/codex-reports/YYYY-MM-DD-NNNN-<slug>[-NN].md`.
`NNNN` is four-digit daily sequence counted within
`docs/codex-reports/` — list the directory, find the highest
`NNNN` for today, add one; start at `0001` if the directory has
nothing for today yet. ([§15](CONTRIBUTING.md#15-journal-like-files))

**Write a report when:**

- The prompt explicitly asks for one.
- You made a non-obvious design call the prompt didn't spell
  out.
- Substantial findings surfaced during implementation that don't
  fit in the session-summary.
- You flagged something per a flag-vs-fix policy (crypto-review,
  zeroization gaps, `unsafe` in neighbouring code) that you saw
  but didn't fix.

**Skip** for routine, well-specified work with no surprises.

**Cross-reference the prompt in a short header**:

```markdown
# <title>

**Date:** 2026-04-22
**Prompt:** docs/codex-prompts/2026-04-22-0001-<slug>.md
```

Then prose. Audience: future Claude sessions and Yuka. Complete
sentences, concrete file paths, no in-jokes.

**Don't commit.** Leave the file dirty. Mention the path in
your final summary so Claude picks it up on review.

## Terminology — short form

See [`CONTRIBUTING.md §14`](CONTRIBUTING.md#14-naming-and-terminology)
for the full rule set. Short form:

- No `master`/`slave`; use `primary`/`replica`,
  `leader`/`follower`, etc. Default branch here is `main`.
- No gendered defaults; prefer singular "they".
- `allowlist`/`denylist`, not `whitelist`/`blacklist`.
- `stub`/`placeholder`/`fake`, not "dummy".
- **GNU/Linux** for the OS, **Linux kernel** for the kernel —
  don't collapse. `uname -s` matching against literal `Linux`
  is fine (kernel-interface identifier, not prose).
- **Microsoft Windows** / **Windows** in prose. No `win*`
  freeform abbreviations; shipped identifiers (`Win32` API,
  `x86_64-pc-windows-msvc`) stay as-is.
- Prefer **"free software"** or **"FLOSS"** over standalone
  **"open-source"**, except quoting external conventions (OSI
  proper noun, etc.).
- Technical accuracy overrides aesthetic neutrality — literal
  external identifiers (HTTP `Authorization`, a DB `MASTER`
  command, an external repo's `master` branch) ship as they
  are.
- **Prose is in English by default** — code comments,
  rustdoc, error-message text, the summary you return to
  Claude (which may feed a commit message). Non-English text is
  fine when it's the artefact (i18n strings, Unicode-handling
  test fixtures, literal quotation from external sources) and
  should carry an English gloss in a nearby comment or in the
  summary to Claude when the meaning isn't self-evident. See
  [`CONTRIBUTING.md §14.6`](CONTRIBUTING.md#146-english-as-the-default).
  Grammar / typo issues in the final summary are not worth
  flagging as blockers — Claude will polish prose during review.

## When in doubt

If the task seems to fall outside these rules, or the rules
themselves seem to conflict with what the prompt asks, surface
the tension in your response instead of guessing. Claude is
waiting to review and re-prompt if needed.
