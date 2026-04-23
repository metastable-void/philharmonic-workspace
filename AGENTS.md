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

## Development host must be POSIX-ish

This workspace assumes a POSIX-ish development host: GNU/Linux
(incl. WSL2 on Windows), macOS (Darwin), BSDs
(FreeBSD/OpenBSD/NetBSD/DragonFly), illumos/Solaris, or musl
distros (Alpine). Every script is POSIX sh (`#!/bin/sh`); file-
permission, signal, and submodule-ordering semantics assume a
POSIX host.

**Before running any script or touching files, check your
environment.** If `uname -s` returns `Linux` / `Darwin` /
`FreeBSD` / `OpenBSD` / `NetBSD` / `DragonFly` / `SunOS`,
proceed. If it returns something indicating raw Microsoft
Windows (unlikely — `#!/bin/sh` wouldn't get you this far), or
if your runtime reports you're on native Windows, **STOP
IMMEDIATELY** and surface the mismatch in your final message.
Do not attempt the task. There's no runtime gate inside the
scripts themselves (raw Windows can't execute `#!/bin/sh`, so
it'd never fire); the gate lives here, in this document. On
Git Bash / MSYS / Cygwin (POSIX-compat layers over Windows),
proceed with caution and flag any submodule / signing /
permission anomaly before continuing. See
`docs/design/13-conventions.md §Development environment`.

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
- The repo installs tracked Git hooks (`.githooks/pre-commit`,
  `.githooks/commit-msg`, `.githooks/post-commit`) via
  `core.hooksPath`, wired up by `scripts/setup.sh`. Pre-commit
  rejects any `git commit` that didn't come through
  `commit-all.sh`; commit-msg rejects any message without a
  matching `Signed-off-by:` trailer; post-commit rolls back any
  unsigned commit that slipped through. Don't disable them;
  don't `--no-verify` around them.

If the prompt genuinely requires a git state change (e.g. "create
a branch and open a PR"), stop and surface that — don't guess.

## Rust conventions

- **Workspace conventions live in the repo, not in any local
  memory you or your runtime may keep.** If you discover a rule
  that applies to *this project* — naming, versioning, tooling
  choices, anything a future contributor would need to honor —
  its durable home is this file, `CLAUDE.md`, or
  `docs/design/13-conventions.md`. Don't rely on
  `$CODEX_HOME`-local state, cached project files, or your own
  in-process memory to persist such rules across sessions or
  machines; those are per-install and don't follow the repo.
  Surface the rule to Claude in your final summary so it can be
  committed to the repo properly.
- **Use `scripts/*.sh` wrappers over raw `cargo` subcommands.**
  The wrappers encode mandated flags (`-D warnings`,
  `--all-targets`, per-crate scoping, auto-install of
  cargo-audit / cargo-semver-checks on first use) so your local
  runs behave identically to CI. Raw `cargo <subcommand>` drifts.
  The wrapper inventory:
  - `./scripts/pre-landing.sh` — canonical fmt+check+clippy+test
    flow, auto-detects modified crates. Run before finishing any
    Rust-touching task.
  - `./scripts/rust-lint.sh [<crate>]`,
    `./scripts/rust-test.sh [--include-ignored|--ignored] [<crate>]`
    — individual phases if you need them standalone.
  - `./scripts/miri-test.sh --workspace | <crate>...` —
    `cargo +nightly miri test` for routine UB checks. Not in
    pre-landing (too slow); used pre-publish and periodically.
    Requires nightly + miri, installed via `setup.sh`.
  - `./scripts/cargo-audit.sh`,
    `./scripts/check-api-breakage.sh <crate> [<version>]` —
    pre-release checks.
  - `./scripts/publish-crate.sh [--dry-run] <crate>` — publish +
    signed release tag (Claude runs this, not you; it commits).
  - `./scripts/verify-tag.sh <crate> [<tag>]` — verify a release
    tag is locally present, signed, and pushed to origin at the
    same commit. Claude runs this post-publish.
  - `./scripts/crate-version.sh <crate>` / `--all` — local
    version from Cargo.toml.
  - `./scripts/xtask.sh crates-io-versions -- <crate>` —
    published versions from crates.io (Rust bin; no external
    `jq` / curl dep).
  - `./scripts/check-toolchain.sh` — rust toolchain state.
  If your task needs a cargo operation with no wrapper, surface
  that in your final summary rather than silently running raw
  cargo — the workspace convention is to extend a script first.
  **Exempt**: read-only queries (`cargo tree`, `cargo metadata`,
  `cargo --version`). See `docs/design/13-conventions.md §Script
  wrappers`.
- **Edition 2024, MSRV 1.88.** Every `Cargo.toml` already carries
  `edition = "2024"` and `rust-version = "1.88"` — match.
- **License.** All crates are `Apache-2.0 OR MPL-2.0` with both
  license files at the crate root. **Do not add per-file copyright
  or license headers** — this workspace doesn't use them.
- **Errors.** Use `thiserror` for library crates. Partition errors
  by what the caller does with them (semantic violations,
  concurrency outcomes, backend failures). Expose predicates like
  `is_retryable()` where useful. **Do not use `anyhow` in library
  crates** — callers can't match on specific failure modes. Binary
  crates may use `anyhow`.
- **No panics in library code.** This is systems-programming
  infrastructure (request handlers, long-lived services, crypto,
  storage); a panicking task is user-visible failure. In any
  `src/**/*.rs` outside `#[cfg(test)]`:
  - No `.unwrap()` / `.expect()` on `Result` / `Option` — use
    `?` with a typed error variant, or `.ok_or_else(...)` /
    `.map_err(...)`.
  - No `panic!` / `unreachable!` / `todo!` / `unimplemented!` on
    reachable paths — model unreachability at the type level
    (newtypes, sealed enums, `NonZero<T>`, typestates).
  - **No unbounded indexing** — `slice[i]`, `slice[a..b]`,
    `map[&k]` all panic on absent/OOB access. Use
    `.get(...)` → `Option`, propagate.
  - **No unchecked integer arithmetic** — `+`, `-`, `*`, `/`, `%`
    (and `+=` / `-=` / etc.) panic on overflow in debug and
    either trap or wrap silently in release. Use `checked_*` →
    `Option` when the caller should handle overflow as an
    error, `saturating_*` when clamping is the intent (common
    for `usize` subtraction), `wrapping_*` when modular arith
    is the actual semantic (counters, hash mixing). Plain `+` /
    `-` is fine for constants and compiler-provable cases.
  - No lossy `as` casts when the input width can exceed the
    target's range — use `TryFrom::try_from` and propagate the
    `TryFromIntError`. Provably-lossless casts (`u16 as u32`)
    are fine.
  Narrow, justified exceptions (inline comment required at the
  call site): unrecoverable OS / hardware failure (the
  `OsRng.try_fill_bytes(...).expect("OS RNG failure ...")`
  pattern); build-time-validated constants (`uuid!("literal")`);
  type-witness unreachability the compiler can't express. Tests
  / dev-deps / `xtask/` bins can `.unwrap()` freely — panics in
  tests are the failure-signal mechanism. See
  `docs/design/13-conventions.md §Panics and undefined behavior`
  for the full rule and rationale.
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
  patch (`"0.3.4"`) only when relying on a feature introduced in
  that patch — not out of habit.
- **Testing.** Unit tests colocated (`#[cfg(test)] mod tests`).
  Integration tests in `tests/`. Tests that need real
  infrastructure (testcontainers, network, etc.) **must** be
  feature-gated so default CI runs without them.
- **Comments.** Default to *no* comments. Only write one when the
  *why* is non-obvious (hidden constraint, subtle invariant,
  workaround for a specific bug). Don't narrate *what* — names do
  that.
- **Terminology and language.** Prose you author — code
  comments, rustdoc, error-message text, the final summary you
  return to Claude (which may feed a commit message) — follows
  the workspace terminology conventions at
  `README.md §Terminology and language`. Short form:
  - No `master`/`slave` for technical relationships (use
    `primary`/`replica`, `leader`/`follower`, `parent`/`child`,
    `controller`/`agent`, `main`/`workers`). Default git branch
    here is `main`.
  - No gendered defaults — prefer singular "they"; avoid
    "he"/"he/she"/"(s)he" and "guys"/"man" as generics.
  - Prefer `allowlist`/`denylist` over `whitelist`/`blacklist`;
    `stub`/`placeholder`/`fake` over "dummy"; "smoke test" /
    "verify" over "sanity check".
  - **GNU/Linux** for the OS, **Linux kernel** for the kernel —
    don't collapse the two in prose. Matching `uname -s`
    against the literal string `Linux` is fine (that's the
    kernel-interface identifier, not prose).
  - **Microsoft Windows** or **Windows** in prose. No
    `win*`-style freeform abbreviations; established
    identifiers (`Win32` API, `x86_64-pc-windows-msvc`) ship
    that way and are left alone.
  - Prefer **"free software"** or **"FLOSS"** over standalone
    **"open-source"**, except when quoting external conventions
    (OSI proper noun, "open-source licenses" per OSI).
  - Technical accuracy overrides aesthetic neutrality — use
    literal external identifiers (HTTP `Authorization`, a DB
    `MASTER` command, an external repo's `master` branch) as
    they ship; the rule targets prose we author in this
    workspace.
  Full rule set with exceptions:
  `docs/design/13-conventions.md §Naming and terminology`.

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
- **Never invoke `python`, `perl`, `ruby`, `node`, or other
  non-baseline scripting languages from workspace tooling.** If
  a task would lean on one, write a Rust bin in `xtask/` (the
  in-tree, non-submodule, multi-bin crate at the workspace root,
  `publish = false`). Existing POSIX shell scripts — `awk`,
  `sed`, `grep`, `cut`, `tr`, standard SUSv4 pipelines — remain
  fine as-is; the rule is about ad-hoc `python3 -c "..."` creep
  and non-POSIX tools. **`jq`, `curl`, and `wget` also trigger
  the Rust-bin rule** — they're not on every stripped baseline
  (macOS ships none of them by default; Alpine base doesn't
  either). If you'd reach for `jq`, write a Rust bin using
  `serde_json`; HTTP fetching already lives in
  `xtask/src/bin/web-fetch.rs`. If you're tempted to run any of
  these from a new script, prompt for a Rust bin extraction in
  your final summary rather than introducing the dep.
- **UUID generation for stable wire-format constants always goes
  through `./scripts/xtask.sh gen-uuid -- --v4`.** Every
  `KIND: Uuid` constant, algorithm identifier, or any value that
  once committed must never change is minted through this tool.
  Never `python3 -c "import uuid"`, `uuidgen`, online generators,
  or direct `cargo run` — the canonical invocation path
  (`xtask.sh` wrapper) keeps randomness uniform across sessions
  and machines and leaves room to add pre-build caching later.
- **Use the wrapper scripts for `mktemp` / `curl` / `wget`, not
  the raw tools.** These tools vary across minimal environments
  (Alpine busybox, FreeBSD, OpenBSD, macOS, WSL); the wrappers
  encode the portable choice once so shell scripts don't have
  to rediscover it.
  - Temp files: `tmp=$("$(dirname "$0")"/mktemp.sh [<slug>])`,
    paired with `trap 'rm -f "$tmp"' EXIT INT HUP TERM` in the
    caller (the wrapper doesn't clean up for you).
  - HTTP GET: `"$(dirname "$0")"/web-fetch.sh <URL> [<outfile>]`.
    User-Agent overridable via `WEB_FETCH_UA`. All backends fail
    on HTTP 4xx/5xx (curl is passed `-f`); use `... || :` if
    you want to tolerate HTTP errors.
  If a wrapper doesn't cover your case, extend it. Don't reach
  around it to raw `mktemp`/`curl`/`wget`. See
  `docs/design/13-conventions.md §External tool wrappers`.

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

## Reports (`docs/codex-reports/`)

You — Codex — have a dedicated journal at
`docs/codex-reports/` for writing your own findings back to the
repo. Parallel to `docs/codex-prompts/` (Claude → you) and
`docs/notes-to-humans/` (Claude → Yuka), this directory is
**you → the repo**: observations that outlive the session-
summary you return to Claude.

**Filename format** (same as the other journal directories):

```
docs/codex-reports/YYYY-MM-DD-NNNN-<slug>[-NN].md
```

- `YYYY-MM-DD` — today's date.
- `NNNN` — four-digit daily sequence *within this directory*.
  List `docs/codex-reports/`, find the highest `NNNN` for
  today, add one. If the directory has nothing for today yet,
  start at `0001`. Independent from the sequences in
  `docs/codex-prompts/` and `docs/notes-to-humans/`.
- `<slug>` — short kebab-case. Usually mirrors the prompt's
  slug so the two files pair by name.
- `[-NN]` — optional two-digit round suffix for multi-round
  follow-ups. Omit for standalone entries.

**Write a report when:**

- The prompt explicitly asks for one.
- You made a non-obvious design call the prompt didn't spell
  out, and future sessions would have to re-derive it from the
  code alone without the written rationale.
- Substantial findings surfaced during implementation that
  don't fit in the session-summary (test-matrix results beyond
  the acceptance list, blocker-then-resolution sequences,
  cross-dependency version notes).
- You flagged something per a flag-vs-fix policy (crypto-review,
  zeroization gaps, `unsafe` in neighboring code) that you
  saw but didn't fix. The session-summary mentions these; the
  report documents them in enough detail for Yuka to act later
  without re-running your investigation.

**Skip the report when** the work was routine, well-specified,
and produced no surprises. Don't write a report for the sake
of writing one — session-summary is sufficient for
"done, tests green, acceptance criteria met".

**Start each report with a header that cross-references the
prompt**:

```markdown
# <title>

**Date:** 2026-04-22
**Prompt:** docs/codex-prompts/2026-04-22-0001-<slug>.md
```

Then prose. Audience is future-Claude sessions (so they can
pick up context without re-running the task) and Yuka (for
human review). Complete sentences, concrete file paths, no
session-specific in-jokes.

**Don't commit.** Same rule as every other file you write:
leave it dirty in the working tree. Mention the report's path
in your final summary so Claude picks it up on review and
commits it alongside the implementation.

See `docs/design/13-conventions.md §Codex reports` for the
authoritative rule.

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
