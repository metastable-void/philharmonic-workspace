---
name: codex-prompt-archive
description: Use BEFORE spawning Codex via the codex:* plugin (codex-rescue, codex-run, etc.) for any substantive coding task in this workspace. Every Codex prompt must be archived to docs/codex-prompts/YYYY-MM-DD-<slug>[-NN].md and committed via scripts/commit-all.sh *before* Codex is spawned — there are no ephemeral Codex invocations. Also consult to decide whether a task should even go to Codex vs. stay with Claude.
---

# Codex prompt archive

In this workspace, Claude Code is the designer/reviewer/caretaker
and Codex is the implementation partner for substantive Rust work.
The division of labor matters (keeps Claude out of long coding
sessions it's not suited for) and the archive matters even more
(the prompts are the most useful artifact for understanding why
code landed the way it did).

Authoritative sources:
- `CLAUDE.md` → "Claude vs. Codex division of labor" and the
  "Archive every Codex prompt" paragraph.
- `docs/design/13-conventions.md` §"Codex prompt archive" —
  location, contents, commit cadence, rationale.
- Existing prompts under `docs/codex-prompts/` (e.g.
  `2026-04-20-phase-1-mechanics-config-extraction-{01,02}.md`) as
  working templates.

## When to hand off to Codex

**Hand off to Codex** (write a prompt, archive it, spawn):
- Implementing a crate's actual functionality — the real Rust.
- Writing a non-trivial algorithm, connector impl, storage
  backend adapter, protocol implementation.
- A test suite of real size.
- Anything where the work is "sit down and write a lot of code."

**Keep with Claude** (no Codex round-trip needed):
- Architecture, API shape, design docs, ROADMAP updates.
- Code review of what Codex produced.
- Workspace/repo management: scripts, `Cargo.toml` plumbing,
  submodule wrangling, doc reconciliation.
- Small fixes that are really housekeeping (typo, re-export, bump
  a version).

Rule of thumb: *"what should this look like?"* → Claude.
*"now write the thing"* → Codex, unless it's plumbing.

## The archive discipline

Every Codex prompt Claude writes is committed to the repo before
Codex is spawned. No exceptions. If the Codex run gets abandoned
partway through, the archive is still complete.

### Location

`docs/codex-prompts/YYYY-MM-DD-<slug>.md`

- `YYYY-MM-DD` is today's date.
- `<slug>` names the task concisely: `auth-middleware-rewrite`,
  `sqlx-mysql-store-skeleton`, `phase-1-mechanics-config-extraction`.
- One file per prompt.
- Multi-round tasks (Codex hit a limit and you're resuming, or a
  task naturally splits) get a numeric suffix: `-01`, `-02`, ...
  Never overwrite a prior round's file.

### Required contents

Each file has a short preamble, then the verbatim prompt. Use this
structure (copy it as a template):

```markdown
# <Task title> (<round description>)

**Date:** YYYY-MM-DD
**Slug:** `<slug>`
**Round:** NN (<what this round does — "initial dispatch",
  "resume after truncation", "fix clippy from previous round">)
**Subagent:** `codex:<plugin-agent-name>` (e.g. `codex:codex-rescue`)

## Motivation

One or two sentences: what this task exists to accomplish, why now.
Link to the ROADMAP phase or design-doc section that drives it.

## References

- `ROADMAP.md` §<phase>
- `docs/design/<doc>.md` §<section>
- Prior-round prompt if applicable: `docs/codex-prompts/…-0<N-1>.md`

## Context files pointed at

- `<crate>/src/…` (what Codex is working on)
- `<crate>/tests/…`
- Any other file paths included in the prompt.

## Outcome

(Filled in *after* the Codex run completes, before the next
commit. Summarize what happened: completed cleanly, truncated at N
tokens, produced partial result in file X, etc. This is where the
archive earns its keep.)

---

## Prompt (verbatim)

<the full prompt text sent to Codex, unedited>
```

### Commit cadence

1. Write the prompt file. Leave the `## Outcome` section
   placeholder explicit ("Pending — will be updated after Codex
   run.").
2. Commit it via `scripts/commit-all.sh --parent-only
   "archive codex prompt: <slug> round NN"`. Use `--parent-only`
   because submodules shouldn't pick up this commit.
3. Spawn Codex via the `codex:` plugin with the prompt.
4. When Codex finishes (or is abandoned), update the `## Outcome`
   section of the same file with what actually happened. Commit
   that edit — again via `scripts/commit-all.sh --parent-only`.

Ordering step 2 before step 3 is the whole point: even if Codex
never produces any code, the archive still records the intent.

## Writing the prompt

The existing archived prompts are strong templates. Reuse their
shape:

- Plain `<task>…</task>` XML-style wrapper at the top containing
  the full task description.
- Point Codex at authoritative docs rather than repeating their
  contents. "If anything below contradicts these docs, the docs
  win" is a good line to include.
- Name the current state of each crate involved (version,
  published-or-not, where the code lives).
- Name target versions explicitly if a version bump is expected.
- Spell out what moves, what stays, and what's out of scope.
- Include a `<structured_output_contract>` listing what Codex
  should return (summary, touched files, verification results,
  residual risks, git state with SHAs, pushed-or-not).
- Include `<default_follow_through_policy>`,
  `<completeness_contract>`, `<verification_loop>`,
  `<missing_context_gating>`, and `<action_safety>` blocks —
  see the existing prompts for the wording that works.
- **Always** reiterate the Git rules: signed-off commits,
  `scripts/commit-all.sh` only, no `push-all.sh`, no
  `cargo publish`. Codex will run raw git otherwise.
- **Always** list the verification commands Codex must run before
  finalizing (`cargo check --workspace`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`, plus
  any task-specific `cargo tree` greps).

## Do not

- Do not spawn Codex without first committing the prompt file.
- Do not edit a prior round's prompt to "patch" it for a new run —
  create a new `-0N` file.
- Do not let the `## Outcome` section go un-updated after a run
  completes. Losing the outcome defeats the archive.
- Do not hand Codex work that's really housekeeping (re-exports,
  version bumps, `Cargo.toml` edits). Do it yourself.
