# Codex reports

Codex-authored journal entries: findings, design rationale, and
implementation details that outlive the session-summary Codex
returns to Claude.

Parallel to:
- `docs/codex-prompts/` — Claude → Codex (prompts).
- `docs/notes-to-humans/` — Claude → Yuka (significant findings).
- `docs/codex-reports/` — **Codex → the repo** (this directory).

## Filename format

```
YYYY-MM-DD-NNNN-<slug>[-NN].md
```

Same format as the other two journal directories. `NNNN` is a
per-directory daily counter (this directory's sequence is
independent of the others). See
[`../design/13-conventions.md` §Journal-like files](../design/13-conventions.md).

## When Codex writes here

- The prompt asks for a report.
- Non-obvious design call made during implementation that the
  prompt didn't spell out — rationale worth preserving.
- Substantial findings beyond the session-summary scope:
  test-matrix results beyond acceptance criteria,
  blocker-then-resolution sequences, cross-dependency version
  notes.
- Flag-vs-fix policy items (crypto review, zeroization gaps,
  `unsafe` in neighboring code) — documented in enough detail
  that Yuka can act later without re-running the investigation.

Routine, well-specified, no-surprises work doesn't need a
report — the session-summary covers it.

## Commit handling

Codex leaves the report dirty in the working tree. Claude
reviews and commits via `scripts/*.sh`, usually alongside the
implementation diff.

## Authoritative rule

- Full convention: [`../design/13-conventions.md` §Codex reports](../design/13-conventions.md).
- Codex instructions: [`../../AGENTS.md` §Reports](../../AGENTS.md).
