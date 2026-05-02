# Philharmonic Workspace — Claude Code briefing

Personal development project for generic workflow orchestration
infrastructure. Rust crate family — most member crates are git
submodules, with an in-tree `xtask/` crate for workspace dev
tooling (never published).

Developer: Yuka MORI.

## Authoritative docs (read these, don't re-derive)

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — **single authoritative home
  for workspace conventions.** Git workflow, script wrappers, POSIX
  shell rules, Rust code rules, versioning, licensing, terminology,
  journal formats, everything. When a rule seems to apply, read the
  relevant §here rather than restating. **When you change a
  convention in practice (new rule, changed rule, retired rule,
  or an ad-hoc rule that deserves to be authoritative), update
  `CONTRIBUTING.md` in the same commit** — see its §18.2.
- [`README.md`](README.md) — **whole-project executive summary.**
  Self-contained, concise, up-to-date. Will be fed to LLM
  sub-agents as the project's one-page mental model; structurally
  stale claims there are bugs. **When you change anything
  structurally visible (add/rename a crate, reshape the dep
  graph, complete a phase, reorganise `scripts/`), update
  `README.md` in the same commit** — see
  [`CONTRIBUTING.md §18.1`](CONTRIBUTING.md#181-readmemd--whole-project-executive-summary).
- [`ROADMAP.md`](docs/ROADMAP.md) — **single authoritative home for any
  roadmap or plan.** Current phase, what's next, what's blocked,
  what was deferred and why. No parallel TODO lists or
  plans-of-record elsewhere. **When plans change (phase done, new
  blocker, deferred task, approach changed), update `ROADMAP.md`
  in the same commit as the work that changes them** — see
  [`CONTRIBUTING.md §16`](CONTRIBUTING.md#16-roadmap-maintenance)
  and
  [`§18.3`](CONTRIBUTING.md#183-roadmapmd--authoritative-home-for-plans).
- [`docs/design/`](docs/design/) — architectural design docs (what
  Philharmonic *is*, not how to develop on it).
- [`.claude/skills/`](.claude/skills/) — git-workflow,
  codex-prompt-archive, crypto-review-protocol. Invoke when their
  triggers fire.
- [`AGENTS.md`](AGENTS.md) — Codex's counterpart to this file.
- [`HUMANS.md`](HUMANS.md) — Yuka's note-to-self. **Agent-readable,
  agent-writable is forbidden.** `commit-all.sh` sweeps her pending
  edits into whatever commit is being made; that's the only way
  `HUMANS.md` changes reach the repo.

## Hard stops before doing anything

- **POSIX-ish host required.** Before running any script, spawning
  Codex, or attempting a Git state change, check the environment
  block's `Platform:` field. `linux` / `darwin` / `freebsd` /
  `openbsd` / `netbsd` / etc. → proceed. `win32` (raw Microsoft
  Windows) → **STOP IMMEDIATELY**, surface the mismatch, instruct
  the human to switch to WSL2. Do not run scripts, commit, or
  spawn Codex. The gate lives here because raw Windows can't
  execute `#!/bin/sh` in the first place. Git Bash / MSYS / Cygwin
  → proceed with caution, flag any submodule / signing / permission
  anomaly. See [`CONTRIBUTING.md §2`](CONTRIBUTING.md#2-development-environment).
- **Crypto-sensitive paths are gated.** SCK, COSE_Sign1,
  COSE_Encrypt0, hybrid KEM, payload-hash, `pht_` tokens — all
  trigger Yuka's two-gate personal review protocol via the
  `crypto-review-protocol` skill. See
  [`ROADMAP.md §5`](docs/ROADMAP.md) and the skill.

## Claude vs. Codex division of labour

Claude Code is the designer, reviewer, and workspace caretaker.
Codex is the implementation partner for substantive coding.

- **Claude does:** architecture, API shape, design docs, ROADMAP
  updates, code review, workspace/repo management (scripts,
  `Cargo.toml` plumbing, submodule wrangling, doc reconciliation,
  small fixes that are really housekeeping). Maintenance coding
  stays with Claude — no Codex round-trip.
- **Codex does:** non-trivial concrete coding — a crate's actual
  implementation, an algorithm, a connector impl, test suites of
  real size. For those, Claude writes the prompt (archived first;
  see `codex-prompt-archive` skill), spawns Codex through the
  `codex:` plugin, and reviews what comes back.

Rule of thumb: "what should this look like?" → Claude. "Now
write the thing" → Codex, unless it's plumbing/housekeeping.

**Human override: if Yuka explicitly says a task should go to
Codex, Claude MUST archive a prompt and spawn Codex regardless
of the task's complexity or scope.** No pushback on "this is
too small for Codex" or "Claude can handle this" — the human
developer's explicit dispatch decision is final. The archival
discipline still applies (prompt committed before spawn).

**The Codex gate is mandatory for auditability, not optional
for convenience.** Any new module, feature implementation, or
file with non-trivial logic (roughly: more than mechanical
`pub use` / `Cargo.toml` / config / doc changes) goes through
Codex with a prompt archived beforehand. Borderline cases
(~50–100 lines of new logic) should default to Codex — the
archival overhead is low and the audit trail is valuable.
Claude doing substantial coding directly bypasses the
design-then-implement split that makes history reviewable.

**Never assume Codex finished.** Codex writes files
mid-run; seeing modified files in the workspace or
receiving a subagent return does NOT mean the Codex
process is done. Before touching any file Codex might be
working on, **verify completion** via both:
1. `./scripts/codex-logs.sh --no-tool-output | grep
   'task_complete'` — the event must be present.
2. Process tree check (`pstree <codex-pid>`) — no child
   processes (no `bwrap`, `cargo`, `rustfmt`, etc.).
If neither confirms, **wait**. Running `cargo build`,
`commit-all.sh`, or editing workspace files while Codex
is still running will silently kill it or produce broken
state from incomplete output. This has caused repeated
incidents.

**Once Codex is verifiably done, dry-run before
committing.** Codex may have touched files outside the
prompt's stated scope (a stray `Cargo.lock` regen, an
unintended doc edit, a new untracked report). Run
`./scripts/commit-all.sh --dry-run` (combine with
`--parent-only` to scope to the parent repo) to preview
the exact file list that `git add -A` would sweep into
each commit. Inspect the output, confirm the scope
matches what you expected from the dispatch, and only
then run the real `commit-all.sh`. Dry-run is read-only
— no staging, no signing, no temp message file, no
`.claude/settings.json` guard — purely a preview. If the
dry-run reveals a file you want kept out of the commit,
pass `--exclude <workspace-relative-path>` (repeatable)
to `commit-all.sh`; the flag unstages each named path
after `git add -A`, leaving its working-tree change
dirty for a follow-up commit. Typical use: hold
`Cargo.lock` back from a parent-only doc commit so it
lands with the corresponding submodule version-bump
commits later. **Codex itself never runs
`commit-all.sh`** (including `--dry-run` and
`--exclude`); the codex-guard in the script aborts
under any Codex ancestor process.

**When cargo appears stuck** (no output for minutes),
run `./scripts/build-status.sh` — it shows which crates
are being compiled/linked/tested, with PIDs and elapsed
times. Use `watch -n 2 ./scripts/build-status.sh` for
continuous monitoring. Reference it in Codex prompts to
prevent false "build is stuck" aborts.
([§5.1](CONTRIBUTING.md#51-build-status-monitoring))

**Check resource pressure before kicking off heavy
work.** `./scripts/xtask.sh resource-pressure` prints a
one-line summary: CPU%, `load_avg_1 / num_cpus` ratio,
available/total memory, used/total swap. Run it before
`pre-landing.sh`, before dispatching Codex, before a
`cargo test --workspace` pass, or any time you'd like
a fast read on whether the box has headroom. If
`load1/cpus` is well above 1.0 or swap usage is climbing,
something else is already saturating the host; queue
heavy work behind it instead of piling on. The companion
bin `system-resources` is reserved for machine-readable
audit-trailer generation — don't use it for status
checks; it doesn't sample CPU activity.

**Codex monitoring scripts have a scope.**
`./scripts/codex-status.sh` and `./scripts/codex-logs.sh`
both filter on `originator: Claude Code` — they only see
Codex sessions spawned via the Claude-Code `codex:` plugin
shim. **A standalone `codex` run launched independently
(e.g. by the user in another terminal, or the VSCode
extension's app-server) does not appear in either tool.**
A "No Codex process running" report from
`codex-status.sh` only confirms no Claude-Code-spawned
Codex is active — an independent session may still be
live and editing the workspace. When the user has
separately dispatched Codex, the codex-completion
verification protocol above does not apply mechanically;
ask the user to confirm completion before touching the
working tree.

## Executive summary of the rules you'll trip over most

Every item here is the short form of something documented in
full in `CONTRIBUTING.md`. Read the full section before acting
— this summary is a prompt, not a spec.

- **Ground yourself in JST time — regularly, not just once.**
  Run `./scripts/xtask.sh calendar-jp` — prints a 5-week grid
  centred on today (JST), marks weekends and Japanese public
  holidays, lists each 祝日 in the window with its Japanese
  name, and shows the current JST wall-clock timestamp. Run
  it at session start, **again after any significant unit of
  work completes** (commit landed, Codex dispatch finished,
  publish done) so your next decision uses fresh time, and
  any time you reason about a deadline, a release window, a
  "before Thursday" commitment, or anything else where "today"
  and "which days are non-working" matter. Long sessions drift
  across the 10:00 / 19:00 / 21:00 thresholds and sometimes
  midnight; a stale timestamp is the failure mode. The host's
  timezone and your training-data cutoff are both unreliable;
  this bin's output is authoritative for deadline reasoning
  on this project, and it's cheap to re-run. **Never pipe
  the output through `tail` or `head`** — the full output
  is short (~15 lines) and every line matters (the grid,
  the holiday list, the timestamp). Clipping it loses
  context that agents need for correct deadline reasoning.
- **Work rhythm: never refuse on time, but note out-of-hours
  sessions as commentary.** Regular hours are 10:00–19:00 JST
  Mon–Fri; extended is fine up to 21:00; nights, weekends
  (土/日), and 祝日 are **allowed** (Yuka compensates herself
  separately for off-hours work) but **availability is not
  assumed** — don't queue work that needs a human hand-off at
  23:00 on a Sunday. If the current JST time from
  `calendar-jp` is outside regular hours, add a one-sentence
  note in your reply so the session transcript carries the
  context (*"(JST now 21:47 木 — outside regular hours;
  proceeding.)"*). The note is a log artefact, not a
  permission request — **never** stall or wait-for-morning
  because of the clock.
  ([CONTRIBUTING.md §8 → "Work rhythm and out-of-hours
  commentary"](CONTRIBUTING.md#work-rhythm-and-out-of-hours-commentary))
- **All Git state changes go through `scripts/*.sh`** —
  `status.sh`, `pull-all.sh`, `commit-all.sh`, `push-all.sh`,
  `heads.sh`. Never raw `git commit` / `git push` / `git add`
  outside the scripts. Every commit is `-s` sign-off + `-S`
  signature + `Audit-Info:` trailer; hooks enforce this.
  ([§4](CONTRIBUTING.md#4-git-workflow))
- **Git history is append-only.** No amend, no rebase, no reset,
  no force-push, and no `git revert` either. Two narrow
  script-enforced exceptions — the `post-commit`
  unsigned-rollback and `pull-all.sh`'s `--rebase`. Mistakes
  ship as new fix-forward commits only.
  ([§4.4](CONTRIBUTING.md#44-no-history-modification))
- **Push early, push often — every sensible-sized step.**
  Mid-work pushes on `main` are not just allowed but expected:
  after each discrete unit of work (a doc reconciliation, a
  script fix, one cohesive refactor), `commit-all.sh` then
  `push-all.sh`, then start the next unit. Do not batch
  unrelated topics into one commit, do not let pushes queue up
  locally between steps, and do not save it all for end-of-
  session — a session that crashes mid-flight should leave a
  clean origin trail of completed steps, not a pile of
  unpushed work. Narrow exceptions: a sequence whose
  intermediate states wouldn't compile / pass `pre-landing.sh`
  (land the sequence as one commit), and edits the user is
  actively iterating on (wait for closure before committing).
  When in doubt, ask. Hooks + GitHub ruleset accept WIP as
  long as it's signed, signed-off, and not force-pushed.
  ([§4.4](CONTRIBUTING.md#44-no-history-modification))
- **Always use `scripts/*.sh` wrappers for cargo operations.**
  The wrappers set `CARGO_TARGET_DIR=target-main` (via
  `lib/cargo-target-dir.sh`) so CLI and Codex builds use
  `target-main/` instead of the default `target/`.
  `rust-analyzer` uses `target/`, so this avoids lock
  contention, build-cache corruption, and "Blocking waiting
  for file lock" stalls. `xtask.sh` uses `target-xtask/`;
  `publish-crate.sh` uses `target-publish/`. **Never run raw
  `cargo build/test/check/clippy` without setting
  `CARGO_TARGET_DIR` first** — if you must go raw, prefix
  with `CARGO_TARGET_DIR=target-main`. Read-only queries
  (`cargo tree`, `cargo metadata`) are fine raw. If no
  wrapper covers your case, extend one.
  ([§5](CONTRIBUTING.md#5-script-wrappers-over-raw-cargo))
- **Run `./scripts/pre-landing.sh` before every commit that
  touches Rust.** fmt + check + clippy (`-D warnings`) +
  rustdoc + test, auto-detecting modified crates. The default
  flow runs `--workspace --exclude xtask` throughout — xtask
  is gated behind `pre-landing.sh --xtask` (uses
  `target-xtask/` instead of `target-main/`, so xtask checks
  don't fight Codex or workspace builds for the same build
  cache). When you've touched both workspace crates and
  xtask, run pre-landing twice: once default, once `--xtask`.
  CI runs the same script. Slow-by-design (minutes per run
  on this workspace's ~25 crates with `aws-lc-rs` C builds
  and Boa) — **run it once before the commit, not repeatedly
  within a single turn**. Stage all the turn's edits, then
  run pre-landing once. For focused mid-iteration debugging
  use a narrow `cargo test <name>`; save the full pre-landing
  pass for the commit. A re-run after fixing a real failure
  is fine; a tight edit/re-run loop in one turn just burns
  time. ([§11](CONTRIBUTING.md#11-pre-landing-checks))
- **Run `./scripts/miri-test.sh` on the crypto crate set at
  every checkpoint** — before publishing crypto-touching
  crates, after completing a phase/sub-phase with crypto
  changes, weekly during active development, and before
  milestones. The mandatory five:
  `philharmonic-policy`, `philharmonic-connector-client`,
  `philharmonic-connector-service`,
  `philharmonic-connector-common`, `philharmonic-types`.
  Track when the last run happened; flag missed checkpoints.
  ([§10.11](CONTRIBUTING.md#1011-miri))
- **Track doc/code volume regularly.** Run
  `./scripts/check-md-bloat.sh` (reports Markdown file sizes
  and flags bloated docs) and `./scripts/tokei.sh` (lines of
  code by language/crate) after completing a sub-phase,
  landing a doc reconciliation, or any session where
  significant volume was added. The output helps Yuka gauge
  whether the workspace is growing proportionally or drifting
  toward doc-heavy / code-light (or vice versa). Not a gate
  — just a hygiene check. Run both and note any surprises.
- **Never recall a crate version from memory.** Always look up
  with `./scripts/xtask.sh crates-io-versions -- <crate>`. Local
  "what we're about to publish" comes from
  `./scripts/crate-version.sh`. ([§5.1](CONTRIBUTING.md#51-crate-version-lookup))
- **No panics in library `src/`.** No `.unwrap()` / `.expect()`
  on `Result`/`Option`, no `panic!` / `unreachable!` / `todo!`
  / `unimplemented!` on reachable paths, no unbounded indexing,
  no unchecked integer arithmetic, no lossy `as` casts on
  untrusted widths. Narrow exceptions need an inline
  justification. Tests / dev-deps / `xtask/` bins are exempt.
  ([§10.3](CONTRIBUTING.md#103-panics-and-undefined-behavior))
- **Library crates take bytes, not file paths.** File I/O,
  env-var lookup, config-file parsing belong in the bin.
  Crypto-adjacent APIs especially. ([§10.4](CONTRIBUTING.md#104-library-crate-boundaries))
- **HTTP client split is strict.** Runtime crates (connector
  impls, realm service binaries, `philharmonic-api`,
  anything that ships) use **`reqwest` + `rustls-tls` + tokio**,
  or `hyper` + rustls directly when reqwest's abstraction is
  too thick. Workspace tooling (`xtask/` bins) uses **`ureq` +
  rustls** via `xtask::http::fetch_text`. **Never `ureq` in a
  runtime crate**, never `reqwest` + tokio in an xtask bin.
  rustls for both; no native-tls, no OpenSSL. No third HTTP
  client without scoping first.
  ([§10.9](CONTRIBUTING.md#109-http-client-runtime-stack-vs-tooling-stack))
- **Shell scripts are POSIX sh** (`#!/bin/sh`), not bash.
  Invoke by path (`./scripts/foo.sh`), never `bash foo.sh`.
  Validate with `./scripts/test-scripts.sh` after any change.
  ([§6](CONTRIBUTING.md#6-shell-script-rules-posix-sh))
- **No `python` / `perl` / `ruby` / `node` / `jq` / `curl` /
  `wget` in workspace tooling.** Shell for orchestration; Rust
  bins under `xtask/` for anything non-baseline. Use the
  `./scripts/mktemp.sh` and `./scripts/web-fetch.sh` wrappers
  for temp files and HTTP. **One narrow exception:**
  `./scripts/webui-build.sh` invokes Node.js (via `npx
  webpack`) solely to generate committed WebUI artifacts;
  general Node.js remains forbidden.
  ([§7](CONTRIBUTING.md#7-external-tool-wrappers),
  [§8](CONTRIBUTING.md#8-in-tree-workspace-tooling-xtask))
- **Every stable UUID (`KIND` consts, algorithm IDs) via
  `./scripts/xtask.sh gen-uuid -- --v4`.** Not `uuidgen`, not
  online generators, not `python -c "import uuid"`.
  ([§9](CONTRIBUTING.md#9-kind-uuid-generation))
- **Notes to humans.** When you tell Yuka anything substantial,
  also write it to
  `docs/notes-to-humans/YYYY-MM-DD-NNNN-<slug>.md` and commit
  via `./scripts/commit-all.sh --parent-only`. Session-only
  output isn't enough. ([§15.1](CONTRIBUTING.md#151-notes-to-humans))
- **Project status reports at milestones.** At sensible
  inflection points — phase / sub-phase landed, major
  refactor or doc reconciliation finished, before a long
  break (Golden Week etc.), or on explicit user request —
  run `./scripts/project-status.sh` to generate a
  point-in-time LLM summary into
  `docs/project-status-reports/YYYY-MM-DD-hh-mm-ss.md`,
  read it (model output can hallucinate SHAs / invent
  roadmap items), add an entry to `docs/SUMMARY.md` under
  "Project status reports", then commit it parent-only with
  a one-sentence "what milestone" message. Don't run it
  after every commit or speculatively — the value is in the
  milestone shape, not in a dense archive.
  ([§15.4](CONTRIBUTING.md#154-project-status-reports))
- **Update the Japanese executive summary at milestones.**
  At the same inflection points as project status reports,
  invoke the `docs-jp` skill to generate/update
  `docs-jp/YYYY-MM-DD-開発サマリー.md`. This is Claude's task
  (not Codex). Read `docs-jp/README.md` every time — it is
  the authoritative spec for content, tone, and constraints.
  No project names, no technical jargon, no FLOSS framing.
- **Archive every Codex prompt** *before* spawning — write to
  `docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>.md` and commit.
  See the `codex-prompt-archive` skill. ([§15.2](CONTRIBUTING.md#152-codex-prompt-archive))
- **Terminology follows §14.** Inclusive / neutral / technically
  accurate, FSF-preferred for free-software framing. Literal
  external identifiers (HTTP `Authorization`, `Win32`,
  `x86_64-pc-windows-msvc`) stay as they ship.
- **Prose is in English by default** — commit messages, code
  comments, docs, notes-to-humans, PR/review text. Multilingual
  contributors' grammar/typo issues are **fixed best-effort in
  review**, never grounds to reject a contribution. Non-English
  text is explicitly allowed when it's the artefact (i18n
  strings, Unicode tests, literal external identifiers); add an
  English gloss alongside when the meaning isn't self-evident.
  ([§14.6](CONTRIBUTING.md#146-english-as-the-default))

## Memory / persistence policy

- **Workspace-scoped conventions belong in the repo, not in
  machine-local memory.** When a rule applies to *this project*
  ("prefer X over Y here"), its durable home is `CONTRIBUTING.md`
  (or one of the named living docs above) — not your
  per-agent-install memory store. Memory is per-machine and
  doesn't travel.
- **Machine-local memory is for genuinely machine-local facts.**
  "On this box, rustup/gh were installed on <date>." "This box
  is the Yuka-home WSL." That's it.

If you learn a repo-wide rule during a session, propose an edit
to `CONTRIBUTING.md` (or the relevant agent-facing doc) rather
than writing it to memory.

## Fresh clone

```sh
git clone --recurse-submodules https://github.com/metastable-void/philharmonic-workspace.git
cd philharmonic-workspace
./scripts/setup.sh
```

`setup.sh` is idempotent. It configures submodule init,
`push.recurseSubmodules=check`, `core.hooksPath=.githooks`,
`commit.gpgsign=true` / `tag.gpgsign=true` /
`rebase.gpgsign=true`, and installs nightly+miri via rustup.
See [`CONTRIBUTING.md §1`](CONTRIBUTING.md#1-quick-start).
