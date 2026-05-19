# Philharmonic Workspace — Claude Code briefing

Personal development project for generic workflow orchestration
infrastructure. Rust crate family — most member crates are git
submodules, with an in-tree `xtask/` crate for workspace dev
tooling (never published). Developer: Yuka MORI.

## Keep this file concise

This file is loaded into every Claude Code session for this
workspace and competes with task content for context budget.
**One short bullet or one short paragraph per rule** — no
multi-paragraph rationales, no "why this is a NEVER not a
'prefer'" sub-sections, no inline incident history beyond a
single SHA. Depth lives in `CONTRIBUTING.md`; this file is a
prompt, not a spec. When you edit this file, prefer compressing
existing bullets over adding new ones. See
[`CONTRIBUTING.md §18.8`](CONTRIBUTING.md#188-claudemd--agentsmd--keep-concise).

## Authoritative docs (read these, don't re-derive)

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — single authoritative
  home for workspace conventions. When you change a convention
  in practice, update it in the same commit
  ([§18.2](CONTRIBUTING.md#182-contributingmd--single-authoritative-home-for-conventions)).
- [`README.md`](README.md) — whole-project executive summary,
  fed to LLM sub-agents. Update in the same commit as any
  structurally visible change
  ([§18.1](CONTRIBUTING.md#181-readmemd--whole-project-executive-summary)).
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — single authoritative
  home for plans. Update in the same commit as work that
  changes them
  ([§16](CONTRIBUTING.md#16-roadmap-maintenance) /
  [§18.3](CONTRIBUTING.md#183-roadmapmd--authoritative-home-for-plans)).
- [`docs/design/`](docs/design/) — architectural design docs
  (what Philharmonic *is*).
- [`.claude/skills/`](.claude/skills/) — git-workflow,
  codex-prompt-archive, crypto-review-protocol. Invoke when
  their triggers fire.
- [`AGENTS.md`](AGENTS.md) — Codex's counterpart to this file.
- [`HUMANS.md`](HUMANS.md) — Yuka's note-to-self.
  **Agent-readable, agent-writable is forbidden.**
  `commit-all.sh` sweeps her pending edits into the commit
  being made; that's the only way changes reach the repo.

## Posture: maintainability over fast coding

Default to slow, careful authorship; never trade maintainability
for keystrokes. Runtime speed is still a first-class goal — what's
deprioritised is *coding velocity*. Reuse over rewrite; small
focused units; deduplicate at the third occurrence; route
substantive coding through the Codex gate. **Structural
correctness over surface fixes**: think in state machines and
invariants; never ship a workaround in place of a diagnosis; if
you can't construct the right model, surface the deficit (via a
codex-report / notes-to-humans entry) rather than ship
wrong-but-plausible code — see
[CONTRIBUTING §10.0.1](CONTRIBUTING.md#1001-structural-correctness-over-surface-fixes).
Operational priority lives in [`docs/ROADMAP.md`](docs/ROADMAP.md)
and [`HUMANS.md`](HUMANS.md); consult both at session start.
Umbrella:
[CONTRIBUTING §10.0](CONTRIBUTING.md#100-posture-maintainability-over-fast-coding).

## Hard stops before doing anything

- **POSIX-ish host required.** Check env block's `Platform:`
  field. `linux` / `darwin` / `freebsd` / `openbsd` / `netbsd`:
  proceed. `win32` (raw Windows): STOP, surface the mismatch,
  instruct the user to switch to WSL2. Git Bash / MSYS / Cygwin:
  proceed with caution. ([§2](CONTRIBUTING.md#2-development-environment))
- **Crypto-sensitive paths are gated.** SCK, COSE_Sign1,
  COSE_Encrypt0, hybrid KEM, payload-hash, `pht_` tokens — all
  trigger the two-gate review protocol. See the
  [`crypto-review-protocol`](.claude/skills/crypto-review-protocol/SKILL.md)
  skill (authoritative) and
  [`docs/ROADMAP.md §2`](docs/ROADMAP.md#2-crypto-review-protocol-pointer).

## Production is not this machine

Production Philharmonic runs on a separate host. When a runtime
symptom is reported from production, do **not** treat dev-box
observations as production state — `tcpdump` / `ss` / `lsof` /
`pstree` / `journalctl` / file-on-disk inspection here reflect
*this machine's* processes only; a local `cargo run` does not
carry the production worker's long-lived hyper TCP pool,
tail-promise queue, H3 negative cache, or accumulated state. A
"doesn't reproduce on the dev box" result does not falsify a
production-only symptom. Default to reasoning about long-lived
production process state; if on-production observation is
genuinely needed, say so explicitly rather than substituting
local equivalents as production evidence. Canonical example:
the 2026-05-18 mhc TCP-pool poisoning fix (no `lo` packets
after one soft-failed step — production, not the dev box).

## Working-directory discipline

`cd` into the workspace root once at session start, then call
workspace scripts as `./scripts/foo.sh`. **Never hardcode an
absolute path to the repo** in any script invocation, doc edit,
or note-to-humans — the workspace root varies across dev boxes.
Read it from the env block's `Primary working directory:` field.
This overrides the Bash tool's default "prefer absolute paths,
avoid `cd`" guidance for this workspace. The scripts themselves
handle any cwd via internal helpers; the discipline exists
because `./scripts/foo.sh` only resolves when the shell is
actually at the workspace root, and a drifted cwd tempts the
host-specific absolute form. Codex specifies each call's cwd by
design; this rule is Claude-only.

## Claude vs. Codex division of labour

- **Claude does:** architecture, API shape, design docs, ROADMAP
  updates, code review, workspace/repo management, small fixes
  that are really housekeeping.
- **Codex does:** non-trivial concrete coding — actual crate
  implementations, algorithms, connector impls, test suites of
  real size. Claude writes the prompt (archived first via the
  [`codex-prompt-archive`](.claude/skills/codex-prompt-archive/SKILL.md)
  skill), spawns Codex through the `codex:` plugin, reviews.

Rule of thumb: "what should this look like?" → Claude. "Now
write the thing" → Codex unless it's plumbing/housekeeping.

- **Human override.** If Yuka explicitly says a task goes to
  Codex, Claude MUST archive a prompt and dispatch regardless
  of scope. No pushback.
- **The Codex gate is mandatory for auditability.** Anything
  beyond mechanical `pub use` / `Cargo.toml` / config / doc
  changes goes through Codex with a prompt archived first.
  Borderline (~50–100 lines new logic) defaults to Codex.
- **Never assume Codex finished.** Subagent return ≠ done.
  Before touching any file Codex might be working on, verify
  both: (1) `./scripts/codex-logs.sh --no-tool-output | grep
  'task_complete'` shows the event, and (2) `pstree <codex-pid>`
  has no child processes (`bwrap`, `cargo`, `rustfmt`, etc.).
  If neither confirms, wait. Touching files while Codex runs
  has caused repeated incidents.
- **Once Codex is verifiably done, dry-run before committing.**
  Run `./scripts/commit-all.sh --dry-run` (combine with
  `--parent-only` to scope) to preview file scope, then run
  the real commit. If something should stay out, pass
  `--exclude <workspace-relative-path>` (repeatable). Codex
  itself never runs `commit-all.sh` (the codex-guard aborts
  under any Codex ancestor process).
- **Cargo appears stuck?** Run `./scripts/build-status.sh`
  (`watch -n 2` for continuous). Reference it in Codex prompts.
  ([§5.1](CONTRIBUTING.md#51-build-status-monitoring))
- **Check resource pressure before heavy work.** Run
  `./scripts/xtask.sh resource-pressure` (one-line CPU / load /
  memory / swap summary) before pre-landing, before dispatching
  Codex, before a full workspace test. `system-resources` is
  the audit-trailer feed, not a status check.
- **Codex monitoring scripts have a scope.** `codex-status.sh`
  / `codex-logs.sh` filter on `originator: Claude Code` —
  standalone `codex` runs (user-launched, VSCode extension)
  don't appear. If the user dispatched Codex separately, ask
  for completion confirmation before touching the tree.

## Executive summary of rules you'll trip over most

Every item below is the short form of something in
`CONTRIBUTING.md`. Read the referenced section before acting on
anything non-trivial — this summary is a prompt, not a spec.

- **JST is authoritative.** Every human-facing wall-clock
  reading defaults to JST (Asia/Tokyo, UTC+09:00). Wire-format
  fields stay in spec-mandated zones, formatted to JST for
  display. `chrono_tz::Asia::Tokyo` in Rust; `TZ=Asia/Tokyo`
  or `calendar-jp` in shell.
  ([§JST](CONTRIBUTING.md#jst-is-this-workspaces-authoritative-timezone))
- **Ground yourself in JST time — mechanically, not by judgment.**
  Run `./scripts/xtask.sh calendar-jp` (5-week grid, weekend /
  holiday markers, current JST timestamp) *before your next
  reply* after each of: session start; `commit-all.sh` /
  `push-all.sh` / `publish-crate.sh` success; Codex
  `task_complete`; or reasoning about a deadline / release
  window / off-hours hand-off. "Small commit" / "one-line edit"
  is not a reason to skip. If overdue, run now and add
  `(grounding time now — was overdue.)`. **Never pipe the
  output through `head` / `tail`** — every line matters.
  A PostToolUse hook in [`.claude/settings.json`](.claude/settings.json)
  pipes calendar-jp back after the three named scripts; the
  prose rule remains authoritative for session start, deadline
  reasoning, and Codex `task_complete`.
- **Work rhythm: never refuse on time; note out-of-hours.**
  Regular hours 10:00–19:00 JST Mon–Fri, extended to 21:00.
  Nights / weekends (土/日) / 祝日 allowed (Yuka compensates
  separately) but availability not assumed — don't queue work
  needing a 23:00 Sunday hand-off. Outside regular hours, add
  one-line context to the reply (*"(JST now 21:47 木 — outside
  regular hours; proceeding.)"*) — log artefact, not a
  permission request. Never stall on the clock.
  ([§work-rhythm](CONTRIBUTING.md#work-rhythm-and-out-of-hours-commentary))
- **All Git state changes via `scripts/*.sh`.** Never raw `git
  commit` / `git push` / `git add`. Every commit is `-s` +
  `-S` + `Audit-Info:` trailer (hooks enforce).
  ([§4](CONTRIBUTING.md#4-git-workflow))
- **Commit messages: subject ≤ 72, blank line, body wrapped
  at ≈ 72 cols.** Imperative subject; body covers per-file
  scope / rationale / residual risks. Hard-wrap the body by
  hand. ([§4.10](CONTRIBUTING.md#410-commit-message-format))
- **ALWAYS pass commit messages via `--message-file -` +
  single-quoted heredoc**, even for one-line commits:
  ```sh
  ./scripts/commit-all.sh --message-file - <<'EOF'
  subject ≤ 72 chars

  body wrapped at ≈ 72 cols. Backticked `tokens`, `$VAR`
  references, and `$(cmd)` substitutions land verbatim.
  EOF
  ```
  `--message-file -` reads stdin without a command-substitution
  boundary; `<<'EOF'` (single-quoted) suppresses expansion
  inside the heredoc body. The legacy `"$(cat <<'EOF' ... EOF
  )"` form is fragile — bash can still re-expand on a missing
  outer quote or quirky `"` placement. History is append-only,
  so a mangled message is unfixable except via a fix-forward
  note. (Incident: `a5833d5` lost ≈ 8 backticked tokens to the
  double-quoted-string form.)
  ([§4.10](CONTRIBUTING.md#410-commit-message-format))
- **Git history is append-only.** No amend, no rebase, no
  reset, no force-push, no `git revert`. Two narrow
  script-enforced exceptions: `post-commit` unsigned-rollback
  and `pull-all.sh --rebase`. Mistakes ship as fix-forward
  commits. ([§4.4](CONTRIBUTING.md#44-no-history-modification))
- **Push early, push often.** After each discrete unit of work:
  `commit-all.sh`, then `push-all.sh`, then next unit. Don't
  batch unrelated topics; don't queue local pushes; don't save
  for end-of-session. Narrow exceptions: sequences whose
  intermediate states wouldn't pass pre-landing (land as one
  commit); edits the user is actively iterating on (wait for
  closure). ([§4.4](CONTRIBUTING.md#44-no-history-modification))
- **Always use `scripts/*.sh` wrappers for cargo.** The wrappers
  set `CARGO_TARGET_DIR=target-main` so CLI/Codex builds don't
  fight `rust-analyzer`'s `target/` for the lock. `xtask.sh`
  uses `target-xtask/`; `publish-crate.sh` uses
  `target-publish/`. Read-only queries (`cargo tree`,
  `cargo metadata`) are fine raw; everything else must set
  `CARGO_TARGET_DIR` if you must bypass the wrapper.
  ([§5](CONTRIBUTING.md#5-script-wrappers-over-raw-cargo))
- **Run `./scripts/pre-landing.sh` before every Rust-touching
  commit.** cargo-deny bans + fmt + check + clippy
  (`-D warnings`) + rustdoc + test, auto-detects modified
  crates. xtask is gated behind `pre-landing.sh --xtask`
  (uses `target-xtask/`). When you've touched both, run twice.
  Slow-by-design — run once before the commit, not in a tight
  edit/re-run loop within a turn. Pre-landing green is the
  banned-dep guarantee — no need for separate `cargo tree
  --invert` sweeps after.
  ([§11](CONTRIBUTING.md#11-pre-landing-checks) /
  [§11.0.0](CONTRIBUTING.md#1100-pre-landing-green-is-the-banned-dep-guarantee))
- **Claude runs `publish-crate.sh` on Yuka's signal.**
  Publishing is Claude's job — the publish-and-owner-read
  token is on this machine for that reason. Flow: Yuka reviews
  the commit (version bump + CHANGELOG + cascade), signals
  "ready", Claude runs `./scripts/publish-crate.sh <crate>` in
  dep-order. A release-ready commit on `main` is not a signal
  by itself. ([§12.5](CONTRIBUTING.md#125-publish-checklist))
- **Pre-landing.sh before every publish is non-negotiable.**
  `cargo publish --dry-run` only verifies the tarball against
  *currently-published* deps; it misses workspace-internal dep
  mismatches being staged in the same cascade. Skipping has
  forced a yank (mechanics 0.5.2 → 0.5.3, 2026-05-14).
  ([§12.5](CONTRIBUTING.md#125-publish-checklist))
- **Yanks aren't Claude's job.** The token here is
  publish-and-owner-read scoped; `cargo yank` returns 403.
  Fix forward with a new patch + dep-floor bump on consumers,
  ask Yuka to yank from her separate token / web UI. Don't
  work around the 403.
  ([§12.5](CONTRIBUTING.md#125-publish-checklist))
- **Don't re-run a Rust-build-heavy script after losing
  context — re-read its captured output.** Every Bash
  invocation and `run_in_background` task writes full
  stdout+stderr to `/tmp/claude-*/.../tasks/<id>.output`. The
  heavy set: `pre-landing.sh`, `miri-test.sh`,
  `release-build.sh`, `check-api-breakage.sh`, any bare
  workspace `cargo build/check/test`, plus any background
  task that took > ~30 s. Top cost drivers: a full
  `cargo test --workspace`, and any
  `philharmonic-connector-impl-embed` compile (BGE-M3 ONNX
  bundling via inline-blob + tract). Light scripts
  (`webui-build.sh`, `cargo-audit.sh`, per-crate `cargo check
  -p <one>`) are fine to re-run.
- **Never pipe a Rust-build-heavy script through `head` /
  `tail`** — truncation happens before the Bash capture file
  is written, so the trimmed lines are gone and the next
  question forces a re-run. Redirect to a file or let Bash
  capture everything, then `grep` / `Read` with offsets.
  Cheap commands (`status.sh`, `heads.sh`, `git log`, single
  `cargo tree`, `webui-build.sh` tail) are fine through head/tail.
- **Run `./scripts/miri-test.sh` on the crypto crate set at
  every checkpoint** — before publishing crypto-touching
  crates, after a phase / sub-phase with crypto changes,
  weekly during active development, before milestones.
  Mandatory five: `philharmonic-policy`,
  `philharmonic-connector-client`,
  `philharmonic-connector-service`,
  `philharmonic-connector-common`, `philharmonic-types`. Track
  the last run; flag missed checkpoints.
  ([§10.11](CONTRIBUTING.md#1011-miri))
- **Track doc/code volume.** Run `./scripts/check-md-bloat.sh`
  and `./scripts/tokei.sh` after sub-phases, doc
  reconciliations, or volume-heavy sessions. Hygiene check, not
  a gate.
- **Never recall a crate version from memory.** Use
  `./scripts/xtask.sh crates-io-versions -- <crate>` for
  published versions, `./scripts/crate-version.sh` for local.
  ([§5.1](CONTRIBUTING.md#51-crate-version-lookup))
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
- **HTTP client split.** Runtime crates use
  **`mechanics-http-client`** (hyper-rustls + webpki-roots +
  aws-lc-rs; opt-in HTTP/3 via `http3`). **`reqwest` is
  banned** via `deny.toml`; extend mhc rather than reaching
  back for reqwest. xtask tooling uses **`ureq` + rustls** via
  `xtask::http::fetch_text`. `hyper` itself is **not** banned
  (mhc + server crates consume it); the ban scopes the
  outbound-client abstraction layer only. rustls everywhere;
  no native-tls, no OpenSSL.
  ([§10.9](CONTRIBUTING.md#109-http-client-runtime-stack-vs-tooling-stack))
- **Shell scripts are POSIX sh** (`#!/bin/sh`), not bash.
  Invoke by path (`./scripts/foo.sh`). Validate with
  `./scripts/test-scripts.sh` after any change.
  ([§6](CONTRIBUTING.md#6-shell-script-rules-posix-sh))
- **No `python` / `perl` / `ruby` / `node` / `jq` / `curl` /
  `wget` in workspace tooling.** Shell for orchestration; Rust
  bins under `xtask/` otherwise. Use `./scripts/mktemp.sh` and
  `./scripts/web-fetch.sh`. One narrow exception:
  `webui-build.sh` invokes Node.js (via `npx webpack`) to
  generate committed WebUI artefacts.
  ([§7](CONTRIBUTING.md#7-external-tool-wrappers) /
  [§8](CONTRIBUTING.md#8-in-tree-workspace-tooling-xtask))
- **Every stable UUID via `./scripts/xtask.sh gen-uuid --
  --v4`.** Not `uuidgen`, not online, not Python.
  ([§9](CONTRIBUTING.md#9-kind-uuid-generation))
- **Notes to humans.** Substantial things you tell Yuka also
  go in `docs/notes-to-humans/YYYY-MM-DD-NNNN-<slug>.md`,
  committed via `./scripts/commit-all.sh --parent-only`.
  ([§15.1](CONTRIBUTING.md#151-notes-to-humans))
- **Project status reports at milestones.** At inflection
  points (phase landed, refactor done, before a long break,
  user request): `./scripts/project-status.sh` → writes to
  `docs/project-status-reports/`; read it (model can
  hallucinate), add a `docs/SUMMARY.md` entry, commit
  parent-only. Not after every commit.
  ([§15.4](CONTRIBUTING.md#154-project-status-reports))
- **Japanese executive summary at milestones.** Same triggers
  as above — invoke the `docs-jp` skill to update
  `docs-jp/YYYY-MM-DD-開発サマリー.md`. Claude's task, not
  Codex's. Read `docs-jp/README.md` every time (authoritative
  spec).
- **Archive every Codex prompt** *before* spawning — write to
  `docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>.md` and commit.
  See the [`codex-prompt-archive`](.claude/skills/codex-prompt-archive/SKILL.md)
  skill. ([§15.2](CONTRIBUTING.md#152-codex-prompt-archive))
- **Terminology follows §14.** Inclusive / neutral /
  technically accurate, FSF-preferred for free-software
  framing. Literal external identifiers (HTTP `Authorization`,
  `Win32`, `x86_64-pc-windows-msvc`) stay as they ship.
- **Prose is English by default.** Commit messages, code
  comments, docs, notes-to-humans, PR/review text. Multilingual
  contributors' grammar/typo issues are fixed best-effort in
  review, never grounds to reject. Non-English text is
  allowed when it's the artefact (i18n strings, Unicode
  tests, external identifiers); add an English gloss when
  meaning isn't self-evident.
  ([§14.6](CONTRIBUTING.md#146-english-as-the-default))

## Memory / persistence policy

**NEVER save workspace knowledge to machine-local memory.**
Workspace knowledge — conventions, architectural rules, project
history, Yuka's preferences, decisions, crate-family boundaries,
anything you "learned" about how this project works — belongs in
the **repo**, never in your per-agent-install memory store.
Includes feedback / project / reference memories that mention
any file in this workspace.

Why this is a NEVER: machine-local memory is per-agent-install,
invisible to other developers / clones / machines / Codex / CI /
future sessions on different hosts. The repo is the canonical
source of truth; saving a workspace rule to memory is a stealth
fork. Multiple Claude installs would drift; the repo never
drifts from itself.

When you would have written a workspace-knowledge memory:
identify the right living doc (`CONTRIBUTING.md`, `CLAUDE.md`,
`AGENTS.md`, `docs/ROADMAP.md`, `docs/design/*.md`; never
`HUMANS.md`), edit it, commit via `scripts/commit-all.sh`. The
commit is the persistence mechanism.

Machine-local memory is reserved for narrowly machine-local
facts: "rustup/gh installed on this box on <date>"; "this is
the Yuka-home WSL"; "Codex CLI version is X". Nothing else.

## Fresh clone

```sh
git clone --recurse-submodules https://github.com/metastable-void/philharmonic-workspace.git
cd philharmonic-workspace
./scripts/setup.sh
```

`setup.sh` is idempotent: configures submodule init,
`push.recurseSubmodules=check`, `core.hooksPath=.githooks`,
`commit.gpgsign=true` / `tag.gpgsign=true` /
`rebase.gpgsign=true`, installs nightly+miri via rustup.
([§1](CONTRIBUTING.md#1-quick-start))
