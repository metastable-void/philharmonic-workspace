# AGENTS.md — for Codex

> **Who this file is for.** Instructions for **Codex** (the
> OpenAI Codex CLI agent), not Claude Code. Division of labour:
> Claude designs / reviews / drives Git and reads `CLAUDE.md`;
> Codex (you) implements inside the task Claude hands you,
> doesn't design from scratch, doesn't commit or push.
>
> If a rule here conflicts with a direct system/developer
> instruction in your prompt, the prompt wins (per the AGENTS.md
> spec).

## Keep this file concise

This file is loaded into every Codex session for this workspace
and competes with task content for context budget. **One short
bullet or one short paragraph per rule** — no multi-paragraph
rationales, no inline incident history beyond a single SHA.
Depth lives in `CONTRIBUTING.md`. When you (or Claude) edit
this file, prefer compressing existing bullets over adding new
ones. See [`CONTRIBUTING.md §18.8`](CONTRIBUTING.md#188-claudemd--agentsmd--keep-concise).

## Authoritative docs

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — single authoritative
  home for workspace conventions. Read the referenced § before
  acting on anything non-trivial. If you discover an unwritten
  convention or need to change one, surface it in your final
  summary so Claude can update it ([§18.2](CONTRIBUTING.md#182-contributingmd--single-authoritative-home-for-conventions)).
- [`README.md`](README.md) — whole-project executive summary,
  LLM-ingest-ready (yes, for you too). If your implementation
  changes something structurally visible (crate added/renamed,
  dep-graph shift, phase complete, scripts reorganised),
  surface it so Claude updates README in the same commit
  ([§18.1](CONTRIBUTING.md#181-readmemd--whole-project-executive-summary)).
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — single authoritative
  home for plans. If your work moves a phase forward, completes
  a task, or reveals an approach was wrong, surface it for
  Claude to update in the same commit
  ([§16](CONTRIBUTING.md#16-roadmap-maintenance) /
  [§18.3](CONTRIBUTING.md#183-roadmapmd--authoritative-home-for-plans)).
- [`docs/design/`](docs/design/) — architectural design docs.
- [`CLAUDE.md`](CLAUDE.md) — Claude's counterpart to this file.

## Posture: maintainability over fast coding

Default to slow, careful authorship; never trade maintainability
for keystrokes. Runtime speed is still first-class — what's
deprioritised is *coding velocity*. Reuse over rewrite; small
focused units; deduplicate at the third occurrence; refactor
behaviour-preserving (fixing bugs encountered mid-task is fine,
gold-plating is not). **Structural correctness over surface
fixes**: think in state machines and invariants; never ship a
workaround in place of a diagnosis; if you cannot construct the
right model, surface the deficit in your codex-report rather
than ship wrong-but-plausible code
([§10.0.1](CONTRIBUTING.md#1001-structural-correctness-over-surface-fixes)).
Cross-cutting sub-directives: **bins are thin, logic in
libraries**
([design §02](docs/design/02-design-principles.md#bins-are-thin)
/ [§10.14](CONTRIBUTING.md#1014-unpublished-bin-crates-minimal-cli-logic-in-libraries)).
Umbrella: [§10.0](CONTRIBUTING.md#100-posture-maintainability-over-fast-coding).

## Hard stops

### POSIX-ish host required

`uname -s` → `Linux` / `Darwin` / `FreeBSD` / `OpenBSD` /
`NetBSD` / `DragonFly` / `SunOS`: proceed. Raw Microsoft
Windows: STOP, surface the mismatch, don't attempt the task.
Git Bash / MSYS / Cygwin: proceed with caution.
([§2](CONTRIBUTING.md#2-development-environment))

### Crypto-sensitive paths

SCK encrypt/decrypt, COSE_Sign1, COSE_Encrypt0, ML-KEM-768 +
X25519 + HKDF + AES-256-GCM hybrid, payload-hash binding,
`pht_` token generation — all require Yuka's two-gate review.
Implement when the task asks, but **call it out in your final
summary** so the gate isn't missed. If the task doesn't mention
these areas and your implementation drifts into them, stop and
surface rather than proceed.

### Production is not this sandbox

The dev sandbox is **not** the production Philharmonic host.
When the prompt cites a production runtime symptom, do not
assume local observations reflect production state — `tcpdump`,
`ss`, `pstree`, `cargo run` reproductions here cannot carry
the production worker's long-lived hyper TCP pool,
tail-promise queue, H3 negative cache, or accumulated state.
A symptom that doesn't reproduce on this sandbox is not
falsified — the sandbox just lacks the production host's
long-lived state. Default to reasoning about production
process state; if on-production observation is genuinely
needed, say so in your final message rather than substituting
local equivalents as production evidence. Canonical example:
2026-05-18 mhc TCP-pool poisoning fix (no `lo` packets after
one soft-failed step — production, not the sandbox).

## Your role

- **Implement what the prompt asks.** Don't redesign scope,
  refactor surrounding code, or polish unrelated files. Claude
  decided the shape; land a correct implementation of it.
- **Claude reviews your output.** If the prompt is ambiguous or
  contradicts the codebase, flag in your final message — don't
  guess.
- **Don't commit, don't push, don't branch.** Leave the working
  tree dirty. Claude commits via `scripts/*.sh` after review.
- **JST is authoritative.** Every human-facing wall-clock
  reading defaults to JST (Asia/Tokyo, UTC+09:00). Wire-format
  fields stay in spec-mandated zones, formatted to JST for
  display. `chrono_tz::Asia::Tokyo` in Rust; `TZ=Asia/Tokyo`
  or `calendar-jp` in shell.
  ([§JST](CONTRIBUTING.md#jst-is-this-workspaces-authoritative-timezone))
- **Ground in JST time regularly.** Run `./scripts/xtask.sh
  calendar-jp` at start, after any significant unit of work,
  and any time the prompt references a date-relative
  commitment. Long sessions drift across 10:00 / 19:00 / 21:00
  / midnight thresholds — a stale timestamp is the failure
  mode. Host TZ and your training-data cutoff are unreliable
  here; the bin is authoritative and cheap to re-run.
- **Never refuse on time grounds.** Regular hours 10:00–19:00
  JST Mon–Fri (extended to 21:00). Nights, weekends (土/日),
  and 祝日 are **allowed** — do the work. Outside regular
  hours, add a short note in your final message
  (*"(JST now 20:14 土 — out-of-hours session; proceeding.)"*).
  Log artefact, not a permission request.

## Git (what you must not do)

Read-only git is fine (`log`, `diff`, `show`, `blame`,
`rev-parse`, `submodule status`). State changes are Claude's.

- **Do not run** `git commit`, `push`, `add`, `reset`,
  `rebase`, `stash`, `branch -D`, `checkout <branch>`,
  `commit --amend`, `push --force`.
- **Never rewrite history; never `git revert`.** Append-only.
  Two script-enforced exceptions (`post-commit` /
  `commit-all.sh` unsigned rollback; `pull-all.sh --rebase`)
  are Claude's, not yours. Mistakes ship as fix-forward
  commits. If the prompt seems to require history modification
  or revert, stop and surface.
  ([§4.4](CONTRIBUTING.md#44-no-history-modification))
- **Do not touch** `.gitmodules` or submodule pointers.
- **Leave edits in the working tree.** Claude runs
  `commit-all.sh` / `push-all.sh` after review.
- Tracked Git hooks via `core.hooksPath` (`.githooks/...`,
  wired by `setup.sh`). Don't disable; don't `--no-verify`.
  ([§4.5](CONTRIBUTING.md#45-tracked-git-hooks))

If the prompt genuinely needs a git state change (branch,
PR), stop and surface.

## Rust conventions — short form

Every rule below is summarised from
[`§10`](CONTRIBUTING.md#10-rust-code-conventions). Read the
full section when in doubt.

- **Edition 2024, MSRV 1.88.** Every `Cargo.toml` already
  carries `edition = "2024"` and `rust-version = "1.88"`.
- **License.** `Apache-2.0 OR MPL-2.0` with both files at the
  crate root. **No per-file copyright or license headers.**
- **Errors.** `thiserror` in libraries, partitioned by what
  callers do with them. **No `anyhow` in library crates** —
  callers can't match. `anyhow` is fine in binaries.
- **No panics in library `src/`.** No `.unwrap()` / `.expect()`
  on `Result`/`Option`, no `panic!` / `unreachable!` / `todo!`
  / `unimplemented!` on reachable paths, no unbounded indexing,
  no unchecked arithmetic, no lossy `as` on untrusted widths.
  Narrow exceptions need an inline justification. Tests /
  dev-deps / `xtask/` bins exempt.
  ([§10.3](CONTRIBUTING.md#103-panics-and-undefined-behavior))
- **Library crates take bytes, not file paths.** File I/O,
  env-var lookup, config-file parsing belong in the bin.
  Crypto-adjacent especially.
  ([§10.4](CONTRIBUTING.md#104-library-crate-boundaries))
- **Async.** `tokio` default; `tokio::sync` primitives;
  `async-trait` on dyn-compatible traits (see doc 08 §"Why
  `async_trait` (in 2026)").
- **HTTP client split.** Runtime crates use
  **`mechanics-http-client`** (hyper-rustls + webpki-roots +
  aws-lc-rs; opt-in HTTP/3 via `http3`). **`reqwest` is
  banned** via `deny.toml`; if mhc lacks a shape you need,
  surface that in your final message rather than reach for
  reqwest. xtask bins use **`ureq` + rustls**. `hyper` itself
  is **not** banned (mhc + server crates consume it); the ban
  scopes the outbound-client abstraction layer only. rustls
  everywhere; no native-tls, no OpenSSL.
  ([§10.9](CONTRIBUTING.md#109-http-client-runtime-stack-vs-tooling-stack))
- **Re-exports.** Re-export types from direct deps that appear
  in public API; don't re-export transitive deps or unused
  types.
- **Trait vs. impl split.** Multiple implementations → separate
  crates, not feature-gated.
- **Crate naming.** `<subsystem>-<concern>[-<implementation>]`.
- **Version pinning.** Peer workspace crates pin loosely
  (`"0.1"`); cornerstone pinned to minor; pin a patch only
  when relying on a patch-introduced feature.
- **Testing.** Unit tests colocated. Integration in `tests/`.
  Real-infra tests (testcontainers, network) must be
  `#[ignore]`-gated or feature-gated.
- **Comments.** Default to *no* comments. Write one when the
  *why* is non-obvious. Don't narrate *what* — names do that.
- **Always use `scripts/*.sh` for cargo.** Wrappers set
  `CARGO_TARGET_DIR=target-main` so builds don't fight
  `rust-analyzer`'s `target/`. `xtask.sh` uses `target-xtask/`;
  `publish-crate.sh` uses `target-publish/`. Raw cargo
  (`cargo tree`, `cargo metadata`) is fine read-only; for
  anything that builds, prefix with `CARGO_TARGET_DIR=target-main`.
- **Track volume.** After a sub-phase or significant batch,
  run `./scripts/check-md-bloat.sh` + `./scripts/tokei.sh`,
  note the output in your final summary.
- **Check resource pressure before heavy work.**
  `./scripts/xtask.sh resource-pressure` (one-line CPU / load /
  memory / swap). Use before pre-landing, long `cargo test
  --workspace`, etc. If `load1/cpus` is well above 1.0 or swap
  is climbing, defer. `xtask.sh system-resources` is the
  audit-trailer feed, not a status check.

## Shell scripts — short form

If you touch shell scripts, every rule from
[`§6`](CONTRIBUTING.md#6-shell-script-rules-posix-sh) applies:

- **`#!/bin/sh`, not bash.** No `[[ ]]`, `=~`, arrays, `<<<`,
  `<(...)`, `mapfile`, `${var:0:N}`, `local`,
  `${BASH_SOURCE[0]}`.
- **`set -eu`**, not `set -euo pipefail` (pipefail isn't POSIX).
- **Invoke by path** (`./scripts/foo.sh`), not `bash foo.sh`.
- **Validate** with `./scripts/test-scripts.sh` after any
  change.
- POSIX checklist: [`docs/POSIX_CHECKLIST.md`](docs/POSIX_CHECKLIST.md).

### Rust bins, not Python / Perl / jq / curl

**Never invoke `python`, `perl`, `ruby`, `node`, `jq`, `curl`,
`wget` from workspace tooling.** Shell for orchestration; Rust
bins under `xtask/` otherwise. Use `./scripts/mktemp.sh` and
`./scripts/web-fetch.sh`. If tempted to reach for one, surface
in your final summary.
([§7](CONTRIBUTING.md#7-external-tool-wrappers) /
[§8](CONTRIBUTING.md#8-in-tree-workspace-tooling-xtask))

### KIND UUIDs via xtask

Every stable wire-format UUID via `./scripts/xtask.sh gen-uuid
-- --v4`. Not `python3 -c "import uuid"`, not `uuidgen`, not
online generators. ([§9](CONTRIBUTING.md#9-kind-uuid-generation))

## `HUMANS.md` — do not touch

Yuka's note-to-self. **You MAY read it** for context. **You
MUST NOT modify it** — no edits, no appends, no "helpful"
reformatting, no exceptions. If something looks wrong, flag in
your final summary.

## Workspace conventions belong in the repo, NEVER in memory

**NEVER persist workspace knowledge in machine-local storage** —
not `$CODEX_HOME`-local state, not cached project files, not
in-process memory, nothing outside the repo checkout. Workspace
knowledge means: coding conventions, architectural rules,
project history, Yuka's preferences, decisions, crate-family
boundaries, anything you learned about how this project works.

Machine-local state is per-install, invisible to other
developers / clones / machines / Claude / future Codex sessions.
The repo is the canonical source of truth; saving a workspace
rule to local state is a stealth fork.

If you discover a rule that applies to *this project*, surface
it in your structured-output final report (under residuals /
open questions). Claude edits `CONTRIBUTING.md` (or the
relevant agent-facing doc) and commits — the commit is the
persistence mechanism.

## Use the script wrappers, not raw cargo

Wrappers encode the mandated flags (`-D warnings`,
`--all-targets`, per-crate scoping, auto-install of optional
tools) so local runs match CI. Raw `cargo <subcommand>` drifts.

- **`./scripts/pre-landing.sh`** — canonical cargo-deny bans +
  fmt + check + clippy (`-D warnings`) + rustdoc + test. Run
  before finishing any Rust-touching task. Default flow runs
  `--workspace --exclude xtask`; xtask is gated behind
  `pre-landing.sh --xtask` (uses `target-xtask/` so xtask
  doesn't share the build cache with workspace or Codex). If
  your task touched both, run pre-landing twice. Slow-by-design
  (minutes per run on ~25 crates with `aws-lc-rs` C builds and
  Boa) — **run it once at the end, not repeatedly between
  edits**. For focused mid-iteration debugging use narrow
  `cargo test <name>`. ([§11](CONTRIBUTING.md#11-pre-landing-checks))

  **Lint phase autofixes by default (dirty trees OK).** Step 2
  invokes `rust-lint.sh --fix`, which runs `cargo fmt`
  (rewriting in place) and `cargo clippy --fix --allow-dirty
  --allow-staged --all-targets -- -D warnings`. Trivially-
  autofixable fmt / clippy findings are resolved without an
  extra "cargo fmt then re-run" round trip — important for
  agent contexts where the round trip costs tokens. Non-
  fixable warnings still fail the run via `-D warnings`, so
  the gate is unchanged. Pass `--dry-run` to opt out and run
  the lint phase in legacy check-only mode (no source
  rewrites; useful when verifying a tree is already clean,
  e.g. before publish).
  ([§11.0.2](CONTRIBUTING.md#1102-autofix-on-default---dry-run-opts-out))

  **Pre-landing green is the banned-dep guarantee.** Step 1
  (`cargo deny check bans`) reads `deny.toml` and enforces the
  full ban list (`pyo3` / `maturin` / `openssl-sys` /
  `native-tls` / `rustls-platform-verifier` /
  `rustls-native-certs` / `reqwest` no-wrapper full bans;
  `ring` wrapper-allowed only via `quinn-proto`). If pre-landing
  exits clean, the tree is forbidden-dep-free — do **not** run
  redundant `cargo tree --invert <banned>` sweeps afterwards.
  ([§11.0.0](CONTRIBUTING.md#1100-pre-landing-green-is-the-banned-dep-guarantee))

  **Don't re-run a Rust-build-heavy script after losing
  context — re-read its captured output.** Every Bash
  invocation and background task writes full stdout+stderr to
  `/tmp/.../tasks/<id>.output`. Heavy set: `pre-landing.sh`,
  `miri-test.sh`, `release-build.sh`, `check-api-breakage.sh`,
  bare `cargo {build,check,test} --workspace`, plus any
  background task that took > ~30 s. Top cost drivers: a full
  `cargo test --workspace`, and any
  `philharmonic-connector-impl-embed` compile (BGE-M3 ONNX
  bundling). Light scripts (`webui-build.sh` ~12 s,
  `cargo-audit.sh`, per-crate `cargo check -p <one>`) re-run
  cheaply.

  **Never pipe a Rust-build-heavy script through `head` /
  `tail`** — truncation happens before the capture file is
  written, so the trimmed lines are gone. Redirect to a file
  or let Bash capture everything, then `grep` / `Read` with
  offsets. Cheap commands are fine through head/tail.

- **`./scripts/rust-lint.sh [<crate>]`,
  `./scripts/rust-test.sh [--include-ignored|--ignored]
  [<crate>]`** — individual phases.
- **`./scripts/miri-test.sh <crate>` / `--workspace`** —
  routine UB checks. Not in pre-landing (too slow).
- **`./scripts/build-status.sh`** — shows what `rustc` /
  `rust-lld` / `clippy` / `miri` is currently processing.
  Use when cargo appears stuck. Long silences are normal for
  large crates — this distinguishes "still compiling" from
  "actually stuck". ([§5.1](CONTRIBUTING.md#51-build-status-monitoring))

  **Known limitation inside the Codex sandbox.** Codex's
  process namespace hides processes outside the sandbox from
  `ps -eo …`. A Codex-initiated `cargo build` typically runs
  in a separate group / namespace and **does not appear**.
  **An empty `build-status.sh` result is NOT evidence the
  build stalled — it's evidence you can't see your own builder
  from inside the sandbox.** Do not kill the build, abort,
  or retry on that signal. Use either the cargo invocation
  still emitting on its own pipe, or wall-clock elapsed
  against a prior baseline. If you need real visibility, ask
  the user / Claude to run it from outside.
- **`./scripts/cargo-audit.sh`,
  `./scripts/check-api-breakage.sh <crate> [<version>]`** —
  pre-release checks.
- **`./scripts/crate-version.sh <crate>` / `--all`** — local
  version.
- **`./scripts/xtask.sh crates-io-versions -- <crate>`** —
  published versions.

If a task needs a cargo operation with no wrapper, surface in
your final summary. Read-only queries (`cargo tree`,
`cargo metadata`, `cargo --version`) are exempt — run raw.

## Before you hand off

Before concluding any task that touched a `.rs` file (including
transitive — e.g. a `Cargo.toml` dep bump), run:

```sh
./scripts/pre-landing.sh
```

It auto-detects modified crates and runs the full flow
(lint → test → ignored-test for each modified crate). Clippy
is `-D warnings`. Fix root causes; `#[allow(clippy::<lint>)]`
only at the narrowest scope with a one-line comment. If any
step fails and you can't get it green within the task, say so
in your final summary — don't hand off red code.

Doc-only / config-only / script-only changes can skip
pre-landing (no `.rs` touched).
([§11](CONTRIBUTING.md#11-pre-landing-checks))

## Reports (`docs/codex-reports/`)

Your journal at `docs/codex-reports/` is **you → the repo**:
findings that outlive the session-summary you return to Claude.
Parallel to `docs/codex-prompts/` (Claude → you) and
`docs/notes-to-humans/` (Claude → Yuka).

- **Filename**: `YYYY-MM-DD-NNNN-<slug>[-NN].md`. `NNNN` is
  four-digit daily sequence within `docs/codex-reports/`; list
  the dir, take highest+1, start at `0001` if empty for today.
  ([§15](CONTRIBUTING.md#15-journal-like-files))
- **Write when:** the prompt asks; you made a non-obvious
  design call the prompt didn't spell out; substantial findings
  surfaced; you flagged something per flag-vs-fix (crypto,
  zeroization, `unsafe`) without fixing it.
- **Skip** for routine, well-specified work with no surprises.
- **Header**: `# <title>` / `**Date:** YYYY-MM-DD` / **Prompt:**
  pointer to the codex-prompts file. Then prose. Audience:
  future Claude sessions and Yuka. Concrete file paths, no
  in-jokes.
- **Don't commit.** Leave the file dirty; mention the path in
  your final summary.

## Terminology — short form

Full set: [`§14`](CONTRIBUTING.md#14-naming-and-terminology).
Short form:

- No `master`/`slave`; use `primary`/`replica`,
  `leader`/`follower`. Default branch is `main`.
- No gendered defaults; prefer singular "they".
- `allowlist`/`denylist`, not `whitelist`/`blacklist`.
- `stub`/`placeholder`/`fake`, not "dummy".
- **GNU/Linux** for the OS, **Linux kernel** for the kernel.
  `uname -s` matching against literal `Linux` is fine
  (kernel-interface identifier).
- **Microsoft Windows** / **Windows** in prose; shipped
  identifiers (`Win32`, `x86_64-pc-windows-msvc`) stay as-is.
- Prefer **"free software"** or **"FLOSS"** over standalone
  **"open-source"**, except quoting external conventions.
- Technical accuracy overrides aesthetic neutrality — literal
  external identifiers (HTTP `Authorization`, DB `MASTER`
  command, an external `master` branch) ship as-is.
- **Prose is English by default** — code comments, rustdoc,
  error-message text, the summary you return to Claude.
  Non-English text is fine when it's the artefact (i18n
  strings, Unicode tests, external quotation); add an
  English gloss when meaning isn't self-evident. Grammar /
  typos aren't blockers — Claude polishes in review.
  ([§14.6](CONTRIBUTING.md#146-english-as-the-default))

## When in doubt

If the task seems to fall outside these rules, or the rules
seem to conflict with what the prompt asks, surface the
tension in your response — don't guess. Claude is waiting to
review and re-prompt if needed.
