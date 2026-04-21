# Instructions for coding agents

This directory holds human-authored rules for coding agents
(Claude Code, Codex, and any future agent) working in this
workspace. Unlike `docs/design/13-conventions.md` (workspace-
level design invariants) or the repo `README.md` (public-facing
docs), files here are the human developer's direct instructions
*to* the agents about how to behave around specific artifacts
or situations.

Agents: treat these as binding. If a rule here conflicts with
something elsewhere in the repo, surface the conflict rather
than silently picking one.

---

## HUMANS.md access

`HUMANS.md` at the repo root is a living note-to-self written
by the human developer (Yuka). It holds context, preferences,
current focus, and whatever other musings she finds useful to
record. Its audience is primarily herself; secondarily, the
coding agents that need to understand her thinking.

### Agents MAY read HUMANS.md freely

Reading `HUMANS.md` is encouraged whenever context on the
human's thinking would inform your work — deciding between
framings, honoring a preference that isn't written elsewhere,
or understanding what she's currently worrying about. Both
Claude Code and Codex (and any future agent) are welcome to
read it at any time.

### Agents MUST NOT modify HUMANS.md

No edits. No additions. No "helpful" reformatting. No
auto-generated sections. The file is authored by the human, for
the human. This rule has no exceptions and applies to every
coding agent operating in this workspace.

If something in `HUMANS.md` looks wrong, outdated, or
contradicts the code, surface the observation in your response
or write a `docs/notes-to-humans/` entry — don't touch the
file itself.

### Claude Code commits the human's pending edits

When `./scripts/commit-all.sh` runs, `git add -A` picks up any
pending human edits to `HUMANS.md` and includes them in the
commit Claude Code is making. This is the intended flow:

- Claude Code did **not** edit `HUMANS.md` (forbidden).
- The human edited it between sessions.
- Claude Code, while committing its own unrelated work,
  sweeps the pending `HUMANS.md` edits into the same commit.

Don't try to split `HUMANS.md` out into its own commit or stage
around it; let the normal commit flow carry it alongside
whatever Claude is landing.

Codex does not commit in this workspace (see `AGENTS.md` §Git),
so the commit-sweeping behavior applies to Claude Code only.
Codex's obligation is simply: read freely, never modify.
