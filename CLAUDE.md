# Philharmonic Workspace — Claude Code briefing

Generic workflow orchestration infrastructure — a company
project with Yuka MORI as the sole developer. Rust crate
family; most member crates are git submodules, with an
in-tree `xtask/` crate for workspace dev tooling (never
published).

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

- **[Hard] POSIX-ish host required.** Check env block's `Platform:`
  field. `linux` / `darwin` / `freebsd` / `openbsd` / `netbsd`:
  proceed. `win32` (raw Windows): STOP, surface the mismatch,
  instruct the user to switch to WSL2. Git Bash / MSYS / Cygwin:
  proceed with caution. ([§2](CONTRIBUTING.md#2-development-environment))
- **[Hard] Crypto-sensitive paths are gated.** SCK, COSE_Sign1,
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

## Command execution via `rexec`

**[Hard] Every command invocation goes through `rexec`. No
exceptions.** `rexec` is the command-execution aggregator for
AI agents; running raw commands bypasses its transcript and
host infrastructure. If the binary is missing, install with
`cargo install rexec` first, then proceed. This wraps the
other execution rules (`scripts/*.sh` Git / cargo wrappers,
`xtask.sh`, raw read-only tools) rather than replacing them:
`rexec` is the outer envelope; the existing wrapper is the
inner command.

**[Hard] When the rexec MCP server is loaded, do NOT use the
`Bash` tool.** All command execution — including read-only
checks, transcript inspection, host status, anything — goes
through `mcp__rexec__exec` (or `mcp__rexec__check_host` for
the dedicated host-status call). The `Bash` tool is the
fallback surface only, reserved for sessions where the MCP
rexec server isn't loaded (a fresh session that predates the
workspace config, a host where MCP isn't available, etc.).
Verify presence at session start by looking for
`mcp__rexec__*` in your tool list; if they're there, use
them exclusively and treat the `Bash` tool as unavailable
for this session.

Rationale: the Bash tool adds a redundant layer (Bash shell
→ `rexec` binary → rexec host) where MCP goes directly
(MCP client → rexec MCP server → rexec host). The shell
layer is where the workspace's quoting / heredoc / pager
traps live; eliminating the shell altogether for the
common path eliminates those traps entirely. The MCP
server also integrates with the agent's permission model
natively (see the `mcp__rexec__*` allow entries in
[`.claude/settings.json`](.claude/settings.json)) instead
of routing through `Bash` permissions. The Bash form
remains a documented fallback so the rule degrades
gracefully when MCP isn't available — it doesn't go
away, it just isn't the primary surface.

**Workspace config:** the rexec MCP server is declared in
[`.mcp.json`](.mcp.json) (Anthropic's standard MCP config
location; loaded by Claude Code workspace-wide) and
[`.codex/config.toml`](.codex/config.toml)
(`[mcp_servers.rexec]` for Codex). Both bake the agent's
identity into startup args (`-m --whoami Claude` /
`-m --whoami Codex`), so per-call MCP tool invocations
only need to supply the working directory and the inner
command.

**Bash form (`rexec --whoami ... --dir ... -- <cmd>`) is
the documented fallback** when the MCP server isn't
loaded. It does not coexist with MCP in normal use — pick
one surface per session based on what's available, and
under the rule above MCP wins when both are present. Host
status check via Bash is `rexec -c`; transcript inspection
is `rexec --list N` / `rexec --print <name>`. The
equivalent MCP forms are `mcp__rexec__check_host` (no
args, dedicated tool) and `mcp__rexec__exec` with
`argv = ["rexec", "--list", "N"]` /
`argv = ["rexec", "--print", "<name>"]` — the MCP `exec`
runs the rexec binary inside the host's environment, so
the read-only transcript commands hit the same surface as
run-mode invocations and produce identical output.

Usage shapes (from `rexec --help`, v0.2.0):

- **Run a command via MCP (preferred).** When the rexec MCP
  server is loaded for the session, run-mode invocations go
  through MCP tools rather than the Bash form below.
  Tool-call shapes are discoverable via the agent's MCP tool
  list at session start (Anthropic's standard `mcp__<server>__<tool>`
  naming). The `exec` tool takes:
  - `dir` — working directory the host should `chdir` into
    before exec.
  - `argv` — non-empty array; `argv[0]` is the program
    (resolved via PATH), rest are arguments.
  - `envs` — optional, each entry `"VAR=VAL"`; adds to (not
    replaces) the host's environment.
  - `stdin` — optional UTF-8 string fed to the child's fd 0
    via a pipe that closes after writing (so the child sees
    real EOF instead of blocking on the PTY slave). This is
    the preferred surface for piping commit messages and
    other stdin payloads — bytes land verbatim, with no
    shell-expansion boundary between the tool call and the
    child. The server already knows the agent's `--whoami`
    (baked into its startup args), so the tool call doesn't
    need to re-specify it. If the MCP rexec tools aren't
    visible in your tool list, fall back to the Bash form
    and surface the MCP-not-loaded mismatch (the workspace
    config in [`.mcp.json`](.mcp.json) /
    [`.codex/config.toml`](.codex/config.toml) should make
    them available; if they aren't, restart the session or
    ask Yuka).
- **Run a command via the Bash form (fallback).** `--whoami`
  and `--dir` are mandatory, repeat `--env` per override,
  `--` separates the inner command from `rexec`'s flags:
  ```sh
  rexec --whoami <agent-id> --dir <workdir> \
      [--env VAR=VAL]... [--read-stdin] -- <command> [args...]
  ```
  This form remains valid; use it when MCP isn't loaded or
  when you have a specific reason (e.g. piping a tempfile
  path that the MCP shape would obscure). Both forms hit the
  same rexec host and produce the same transcripts.
- **MCP-stdio server (`-m` / `--mcp-stdio`, new in v0.2.0):**
  starts a stdio MCP server that forwards tool calls to the
  rexec host. Configured workspace-wide via
  `mcpServers.rexec` in [`.mcp.json`](.mcp.json) (Claude)
  and `[mcp_servers.rexec]` in
  [`.codex/config.toml`](.codex/config.toml) (Codex).
  Agents don't invoke `rexec -m` directly — the harness
  spawns it at session start with `--whoami` baked in. The
  flag is mentioned here only so the configuration is
  legible; touching the server config is a settings task,
  not an agent task.
- **Forwarding stdin (`--read-stdin`, v0.1.1+):** without
  this flag, the inner child's stdin is the PTY slave and any
  read blocks because nothing is written to it. Pass
  `--read-stdin` to read the client's stdin to EOF and forward
  it to the inner child. With it, heredocs and pipes through
  `rexec` now work end-to-end — e.g.:
  ```sh
  rexec --whoami claude --dir <workdir> --read-stdin -- \
      ./scripts/commit-all.sh --message-file - <<'EOF'
  subject line ≤ 72

  body wrapped at ≈ 72 cols. `backticked` / `$VAR` /
  `$(cmd)` references all land verbatim under the
  single-quoted `<<'EOF'` delimiter.
  EOF
  ```
  This is now a viable second form for the commit-message
  recipe (see [§4.10](CONTRIBUTING.md#410-commit-message-format)).
  **The tempfile form (Write to `/tmp` → `--message-file <path>`)
  remains the canonical recipe** for commits — it's auditable
  (the body lives on disk, can be re-read with `Read`) and
  immune to outer-quote slip-ups that the heredoc form is still
  vulnerable to. Use `--read-stdin` when piping small data
  inline (test fixtures, tarball pipes, ad-hoc `cat | tool`
  flows) where a tempfile would just be ceremony.
- **Host status:** check via the dedicated MCP tool
  (`check_host`, no arguments — returns `HOST RUNNING` or
  `HOST NOT FOUND`) or via the Bash form (`rexec -c` /
  `--check-host`). A host must be up before run-mode
  invocations. **Starting the host is a human's job, not an
  agent's** — `rexec -s` / `--start-host` runs a foreground
  process the operator owns (^C to stop). If `check_host`
  reports `HOST NOT FOUND`, stop and ask Yuka to start one;
  never run `--start-host` yourself.
- **Transcripts:** `rexec --list <N>` lists the N most recent
  transcripts in `YYYY-MM-DD-hh:mm:ss commands=K` form (newest
  first). `rexec -p <name>` / `--print <name>` shows one by
  its name; add `-f` / `--follow` to stream new entries as
  they arrive. **Use them to verify executed commands** —
  especially after a multi-step run, when output capture got
  truncated, or when something looks off. Both commands work
  via either surface:
  - **MCP (preferred):** call `exec` with the rexec binary
    inside, e.g. `argv = ["rexec", "--list", "1"]` or
    `argv = ["rexec", "--print", "<name>"]`. `dir` can be
    any valid path (the read-only commands ignore it; pass
    the workspace root for consistency).
  - **Bash form:**
    ```sh
    rexec --list 1                # → 2026-05-21-11:23:09 commands=4
    rexec --print 2026-05-21-11:23:09
    ```
  Both produce the same output. The transcript carries
  timestamps + the `<whoami>:<dir> $` prompt line + the
  verbatim command + its full stdout/stderr — what actually
  ran, not what the agent intended. This is the post-hoc
  audit trail; lean on it when in doubt.

**Caveat — beware pagers (stdin is no longer a caveat in v0.1.1).**
With `--read-stdin`, the previous "stdin doesn't pass through"
trap is gone. The remaining gotcha: commands that auto-page
when stdout is a TTY (`git diff`, `git log`, `git show`,
`less`-using tools) hang waiting for keystrokes — pass
`--no-pager` (e.g. `git --no-pager diff`) or pipe through
`cat`. Prefer the workspace `scripts/*.sh` wrappers; they
already handle this.

**ANSI colour is stripped automatically.** `rexec` removes
ANSI escape sequences from the inner command's stdout / stderr
before handing the text back to the agent — so `heads.sh`,
`status.sh`, `log.sh`, and any other workspace script that
colourises output renders as plain text in the agent
transcript. No need to pass `--no-color` (though it remains
safe to do so), and no need to post-process; what you see is
the cleaned stream. Colour escapes survive only when the inner
command writes to a file the agent then `Read`s, since that
bypasses rexec's filter.

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

- **How to dispatch Codex from a Claude session.** The
  `codex:codex-rescue` subagent is **not** available via the
  `Agent` tool here — that path errors with "agent type not
  found". The `/codex:rescue` slash command is the
  user-facing entry point and Claude can't fire slash
  commands itself. The canonical Claude-driven dispatch
  uses the plugin's companion script directly:

  1. Write the dispatch prompt body to a tempfile via the
     `Write` tool (long prompts hit shell arg-length limits
     otherwise):
     ```
     Write file_path=/tmp/<slug>-codex-prompt.txt content=<dispatch text>
     ```
  2. Invoke `codex-companion.mjs task` through the rexec
     MCP `exec` tool. The script lives under the resolved
     plugin cache root — find it with
     `ls ~/.claude/plugins/cache/openai-codex/codex/*/scripts/codex-companion.mjs`
     (the version subdirectory changes over time). Pass
     `--cwd` for the workspace root, `--write` to enable
     workspace-write sandbox (without it the run is
     read-only), `--effort high` for substantive work, and
     `--background` so the run doesn't block this
     conversation:
     ```
     mcp__rexec__exec(
       dir  = <workspace-root>,
       argv = ["node",
               "~/.claude/plugins/cache/openai-codex/codex/<ver>/scripts/codex-companion.mjs",
               "task", "--background", "--write",
               "--effort", "high",
               "--cwd", <workspace-root>,
               "--prompt-file", "/tmp/<slug>-codex-prompt.txt"]
     )
     ```
  3. The script prints a `jobId` immediately. Monitor with
     `./scripts/codex-status.sh` and `./scripts/codex-logs.sh`
     (both filter on `originator: Claude Code` — they see
     this dispatch). Invoke them via MCP `exec` too —
     `argv = ["./scripts/codex-status.sh"]` or
     `argv = ["./scripts/codex-logs.sh", "--no-tool-output"]`.
  4. Clean up the tempfile via MCP `exec` after
     `task_complete`: `argv = ["rm", "-f",
     "/tmp/<slug>-codex-prompt.txt"]`.

  The companion script also accepts `--prompt-file -` to
  read the prompt from stdin — under MCP, pass the prompt
  body in the `exec` call's `stdin` parameter instead of
  using a tempfile (same shape as the commit-message
  recipe). The tempfile-path form remains preferred for
  prompts because the body lives on disk and can be
  re-read by Claude or by a reviewer (long-prompt
  auditability), but stdin is the right surface for
  shorter prompts where the tempfile is genuine ceremony.
  The script supports `--resume` / `--resume-last` /
  `--fresh` for continuing or starting fresh threads;
  default-fresh is right for a new prompt-archive round.

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

## Hard rules vs. soft rules

Source: [`HUMANS.md`](HUMANS.md) §"Hard rules and soft rules"
plus per-rule classifications recorded with Yuka on 2026-05-20.

Each operational rule in this file carries a tag:

- **[Hard]** — cannot be overridden by any prompt. Refuse and
  surface the mismatch.
- **[Soft]** — can be overridden by an explicit prompt, but
  the override must be **surfaced** in the same turn (via
  `AskUserQuestion`, a notes-to-humans entry, or an inline
  call-out) so it's auditable.

Special caveat: pre-landing is **[Hard]** but the rule is
satisfied by *any* green pre-landing run — Codex's, Claude's,
or Yuka's — so the gate is the green pass, not which agent
ran it.

Rules without a tag are descriptive guidance (e.g., who does
what, monitoring tips) rather than rules per se. Codex's
counterpart classification lives in [`AGENTS.md`](AGENTS.md);
some Claude-only rules (commit / push / publish / Claude-side
authorship norms) do not appear there.

## Executive summary of rules you'll trip over most

Every item below is the short form of something in
`CONTRIBUTING.md`. Read the referenced section before acting on
anything non-trivial — this summary is a prompt, not a spec.

- **[Soft] JST is authoritative.** Every human-facing wall-clock
  reading defaults to JST (Asia/Tokyo, UTC+09:00). Wire-format
  fields stay in spec-mandated zones, formatted to JST for
  display. `chrono_tz::Asia::Tokyo` in Rust; `TZ=Asia/Tokyo`
  or `calendar-jp` in shell.
  ([§JST](CONTRIBUTING.md#jst-is-this-workspaces-authoritative-timezone))
- **[Soft] Ground yourself in JST time — mechanically, not by judgment.**
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
- **[Hard] All Git state changes via `scripts/*.sh`.** Never raw `git
  commit` / `git push` / `git add`. Every commit is `-s` +
  `-S` + `Audit-Info:` trailer (hooks enforce).
  ([§4](CONTRIBUTING.md#4-git-workflow))
- **[Hard] Commit messages: subject ≤ 72, blank line, body wrapped
  at ≈ 72 cols.** Imperative subject; body covers per-file
  scope / rationale / residual risks. Hard-wrap the body by
  hand. ([§4.10](CONTRIBUTING.md#410-commit-message-format))
- **[Soft] ALWAYS pass commit messages via the MCP `exec`
  tool's `stdin` field.** Never as a positional argument.
  Canonical recipe (single tool call):
  ```
  mcp__rexec__exec(
    dir   = <workspace-root>,
    argv  = ["./scripts/commit-all.sh", "--message-file", "-"],
    stdin = "<message body>"
  )
  ```
  The stdin bytes reach `commit-all.sh` verbatim via a pipe
  the MCP server attaches to fd 0 — no shell-expansion
  boundary, so backticked `tokens`, `$VAR` references, and
  `$(cmd)` substitutions all survive as literal text. The
  stdin parameter is part of the tool-call JSON so the body
  is auditable in the agent transcript without needing a
  tempfile sitting around.

  **Tempfile alternative** (Write to
  `/tmp/<slug>-commit-msg.txt` → MCP `exec` with
  `argv = [..., "--message-file", "/tmp/<slug>-commit-msg.txt"]`
  → `rm`) remains valid; reach for it when the body is large
  enough that having it on disk for a second-look pass is
  worth the extra steps. The Bash form via `rexec
  --read-stdin -- ... --message-file - <<'EOF'` is fragile
  (bash heredoc parsing — easy to slip into the broken
  legacy `"$(cat <<'EOF' ... EOF)"` shape, which lost ≈ 8
  backticked tokens at incident `a5833d5`); prefer the MCP
  stdin form. History is append-only, so a mangled message
  is unfixable except via a fix-forward errata note. Codex
  never commits — only Claude does.
  ([§4.10](CONTRIBUTING.md#410-commit-message-format))
- **[Hard] Git history is append-only.** No amend, no rebase, no
  reset, no force-push, no `git revert`. Two narrow
  script-enforced exceptions: `post-commit` unsigned-rollback
  and `pull-all.sh --rebase`. Mistakes ship as fix-forward
  commits. ([§4.4](CONTRIBUTING.md#44-no-history-modification))
- **[Hard] Read `HUMANS.md` before every commit.** Yuka's
  note-to-self may have grown pending guidance — a new
  workflow rule, a pause on something you were about to do,
  a question for you to surface. Re-read the file before
  invoking `commit-all.sh`; new content there can change
  whether the commit should land at all, what its message
  should say, or whether you need to revert work-in-progress
  first. Codex never commits, so this rule is Claude-only.
- **[Soft] Doc-first: don't paper over code/doc contradictions.**
  When code contradicts the docs, consider correcting the
  *code* before rushing to "fix" the *docs*. Surface the
  question to Yuka (e.g., via `AskUserQuestion` or a
  notes-to-humans entry) before either side is silently
  changed. Adding *new* features to the docs is fine — this
  rule is specifically about handling pre-existing
  contradictions. Pairs with the posture's *structural
  correctness over surface fixes* umbrella.
- **[Soft] Push early, push often.** After each discrete unit of work:
  `commit-all.sh`, then `push-all.sh`, then next unit. Don't
  batch unrelated topics; don't queue local pushes; don't save
  for end-of-session. Narrow exceptions: sequences whose
  intermediate states wouldn't pass pre-landing (land as one
  commit); edits the user is actively iterating on (wait for
  closure). ([§4.4](CONTRIBUTING.md#44-no-history-modification))
- **[Soft] Always use `scripts/*.sh` wrappers for cargo.** The wrappers
  set `CARGO_TARGET_DIR=target-main` so CLI/Codex builds don't
  fight `rust-analyzer`'s `target/` for the lock. `xtask.sh`
  uses `target-xtask/`; `publish-crate.sh` uses
  `target-publish/`. Raw `cargo check` / `cargo test` /
  `cargo fmt` / `cargo clippy` / `cargo doc` are
  soft-banned — use `./scripts/rust-lint.sh` (with
  `--phase check`/`clippy`/`fmt`/`doc` to scope to one
  phase) and `./scripts/rust-test.sh` (with `--filter`,
  `--features`, `--release` for the common bespoke cases).
  Read-only queries (`cargo tree`, `cargo metadata`) remain
  fine raw. Anything else not covered by an existing wrapper:
  extend the wrapper rather than bypassing.
  ([§5](CONTRIBUTING.md#5-script-wrappers-over-raw-cargo))
- **[Soft] Raw `git status` / `git log` / `git diff` are
  banned.** Use `./scripts/status.sh` (workspace-wide working
  tree, ahead/behind, detached HEAD, unpushed tags;
  `--diff` for the per-repo diff view), `./scripts/heads.sh`
  (HEAD-per-repo with signature char), or `./scripts/log.sh`
  (history / audit / stats modes; replaces raw `git log`).
  Wrappers know about submodules; raw forms only see the
  current repo and miss the workspace picture.
- **[Hard, any green run satisfies] Run `./scripts/pre-landing.sh` before every Rust-touching
  commit.** cargo-deny bans + fmt + check + clippy
  (`-D warnings`) + rustdoc + test, auto-detects modified
  crates. **Lint phase auto-applies fmt + clippy autofixes
  on a dirty tree** (`--allow-dirty --allow-staged`), so don't
  burn a round trip running `cargo fmt` manually — re-running
  pre-landing after a fmt failure used to be the right move;
  now the script does it for you. Non-fixable warnings still
  fail via `-D warnings`. Pass `--dry-run` for legacy check-
  only behaviour (no source rewrites; useful before publish
  or when verifying a clean tree). xtask is gated behind
  `pre-landing.sh --xtask` (uses `target-xtask/`). When you've
  touched both, run twice. Slow-by-design — run once before
  the commit, not in a tight edit/re-run loop within a turn.
  Pre-landing green is the banned-dep guarantee — no need for
  separate `cargo tree --invert` sweeps after.
  ([§11](CONTRIBUTING.md#11-pre-landing-checks) /
  [§11.0.0](CONTRIBUTING.md#1100-pre-landing-green-is-the-banned-dep-guarantee) /
  [§11.0.2](CONTRIBUTING.md#1102-autofix-on-default---dry-run-opts-out))
- **[Soft] Claude runs `publish-crate.sh` on Yuka's signal.**
  Publishing is Claude's job — the publish-and-owner-read
  token is on this machine for that reason. Flow: Yuka reviews
  the commit (version bump + CHANGELOG + cascade), signals
  "ready", Claude runs `./scripts/publish-crate.sh <crate>` in
  dep-order. A release-ready commit on `main` is not a signal
  by itself. ([§12.5](CONTRIBUTING.md#125-publish-checklist))
- **[Hard] Pre-landing.sh before every publish is non-negotiable.**
  `cargo publish --dry-run` only verifies the tarball against
  *currently-published* deps; it misses workspace-internal dep
  mismatches being staged in the same cascade. Skipping has
  forced a yank (mechanics 0.5.2 → 0.5.3, 2026-05-14).
  ([§12.5](CONTRIBUTING.md#125-publish-checklist))
- **[Hard] Yanks aren't Claude's job.** The token here is
  publish-and-owner-read scoped; `cargo yank` returns 403.
  Fix forward with a new patch + dep-floor bump on consumers,
  ask Yuka to yank from her separate token / web UI. Don't
  work around the 403.
  ([§12.5](CONTRIBUTING.md#125-publish-checklist))
- **[Soft] Don't re-run a Rust-build-heavy script after losing
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
- **[Soft] Never pipe `scripts/*.sh` output through `head` /
  `tail`** — applies to *every* workspace script, not just
  the Rust-build-heavy ones. Truncation happens before the
  Bash capture file is written, so the trimmed lines are gone
  and the next question forces a re-run. Redirect to a file
  or let Bash capture everything, then `grep` / `Read` with
  offsets. The previous carve-out for "cheap" scripts
  (`status.sh`, `heads.sh`, etc.) is removed — cheap scripts
  must produce output concise enough to read whole, and the
  Rust-build-heavy ones (`pre-landing.sh`, `miri-test.sh`,
  `release-build.sh`, `check-api-breakage.sh`) make the head/
  tail trap worst. Raw Unix tool output (`grep`, `find`,
  `git` direct from a script consuming it) is still fine
  through head/tail; the ban targets workspace scripts.
- **[Soft] Run `./scripts/miri-test.sh` on the crypto crate set at
  every checkpoint** — before publishing crypto-touching
  crates, after a phase / sub-phase with crypto changes,
  weekly during active development, before milestones.
  Mandatory five: `philharmonic-policy`,
  `philharmonic-connector-client`,
  `philharmonic-connector-service`,
  `philharmonic-connector-common`, `philharmonic-types`. Track
  the last run; flag missed checkpoints.
  ([§10.11](CONTRIBUTING.md#1011-miri))
- **[Soft] Track doc/code volume.** Run `./scripts/check-md-bloat.sh`
  and `./scripts/tokei.sh` after sub-phases, doc
  reconciliations, or volume-heavy sessions. Hygiene check, not
  a gate.
- **[Hard] Never recall a crate version from memory.** Use
  `./scripts/xtask.sh crates-io-versions -- <crate>` for
  published versions, `./scripts/crate-version.sh` for local.
  ([§5.1](CONTRIBUTING.md#51-crate-version-lookup))
- **[Hard] No panics in library `src/`.** No `.unwrap()` / `.expect()`
  on `Result`/`Option`, no `panic!` / `unreachable!` / `todo!`
  / `unimplemented!` on reachable paths, no unbounded indexing,
  no unchecked arithmetic, no lossy `as` on untrusted widths.
  Narrow exceptions need an inline justification. Tests /
  dev-deps / `xtask/` bins exempt.
  ([§10.3](CONTRIBUTING.md#103-panics-and-undefined-behavior))
- **[Hard] Library crates take bytes, not file paths.** File I/O,
  env-var lookup, config-file parsing belong in the bin.
  Crypto-adjacent especially.
  ([§10.4](CONTRIBUTING.md#104-library-crate-boundaries))
- **[Hard] HTTP client split.** Runtime crates use
  **`mechanics-http-client`** (hyper-rustls + webpki-roots +
  aws-lc-rs; opt-in HTTP/3 via `http3`). **`reqwest` is
  banned** via `deny.toml`; extend mhc rather than reaching
  back for reqwest. xtask tooling uses **`ureq` + rustls** via
  `xtask::http::fetch_text`. `hyper` itself is **not** banned
  (mhc + server crates consume it); the ban scopes the
  outbound-client abstraction layer only. rustls everywhere;
  no native-tls, no OpenSSL.
  ([§10.9](CONTRIBUTING.md#109-http-client-runtime-stack-vs-tooling-stack))
- **[Hard] Shell scripts are POSIX sh** (`#!/bin/sh`), not bash.
  Invoke by path (`./scripts/foo.sh`). Validate with
  `./scripts/test-scripts.sh` after any change.
  ([§6](CONTRIBUTING.md#6-shell-script-rules-posix-sh))
- **[Soft] No `python` / `perl` / `ruby` / `node` / `jq` / `curl` /
  `wget` in workspace tooling.** Shell for orchestration; Rust
  bins under `xtask/` otherwise. Use `./scripts/mktemp.sh` and
  `./scripts/web-fetch.sh`. One narrow exception:
  `webui-build.sh` invokes Node.js (via `npx webpack`) to
  generate committed WebUI artefacts.
  ([§7](CONTRIBUTING.md#7-external-tool-wrappers) /
  [§8](CONTRIBUTING.md#8-in-tree-workspace-tooling-xtask))
- **[Hard] Every stable UUID via `./scripts/xtask.sh gen-uuid --
  --v4`.** Not `uuidgen`, not online, not Python.
  ([§9](CONTRIBUTING.md#9-kind-uuid-generation))
- **[Hard] Notes to humans.** Substantial things you tell Yuka also
  go in `docs/notes-to-humans/YYYY-MM-DD-NNNN-<slug>.md`,
  committed via `./scripts/commit-all.sh --parent-only`.
  ([§15.1](CONTRIBUTING.md#151-notes-to-humans))
- **[Soft] Project status reports at milestones.** At inflection
  points (phase landed, refactor done, before a long break,
  user request): `./scripts/project-status.sh` → writes to
  `docs/project-status-reports/`; read it (model can
  hallucinate), add a `docs/SUMMARY.md` entry, commit
  parent-only. Not after every commit.
  ([§15.4](CONTRIBUTING.md#154-project-status-reports))
- **[Soft] Japanese executive summary at milestones.** Same triggers
  as above — invoke the `docs-jp` skill to update
  `docs-jp/YYYY-MM-DD-開発サマリー.md`. Claude's task, not
  Codex's. Read `docs-jp/README.md` every time (authoritative
  spec).
- **[Soft] Archive every Codex prompt** *before* spawning — write to
  `docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>.md` and commit.
  See the [`codex-prompt-archive`](.claude/skills/codex-prompt-archive/SKILL.md)
  skill. ([§15.2](CONTRIBUTING.md#152-codex-prompt-archive))
- **Terminology follows §14.** Inclusive / neutral /
  technically accurate, FSF-preferred for free-software
  framing. Literal external identifiers (HTTP `Authorization`,
  `Win32`, `x86_64-pc-windows-msvc`) stay as they ship.
- **[Soft] Prose is English by default.** Commit messages, code
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
