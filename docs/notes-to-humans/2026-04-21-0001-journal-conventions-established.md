# Journal conventions established

**Date:** 2026-04-21
**Context:** Conversation with Yuka adding two workspace conventions.

## What's new

1. **Journal-like filename format:** `YYYY-MM-DD-NNNN-<slug>[-NN].md`.
   The `NNNN` is a required four-digit daily sequence, counted
   per-directory. The trailing `[-NN]` is an optional two-digit
   round suffix for multi-part work on the same logical entry.
   Applies to every journal-like directory Claude generates —
   currently `docs/codex-prompts/` and `docs/notes-to-humans/`;
   future directories inherit the format. Four digits on the
   daily sequence (not two) because 99-per-day is realistically
   tight for notes-to-humans in a busy session.

2. **`docs/notes-to-humans/`:** a new journal directory. When
   Claude tells Yuka anything significant in a session, the note
   must also be written to a file under this directory and
   committed. Session-only output is no longer enough for
   substantial findings.

Full convention text lives in `docs/design/13-conventions.md`
§"Journal-like files" and §"Notes to humans". The authoritative
rules are there; this file is the first real entry, doubling as
a record of the change.

## Why this matters (for future-you)

- Chat scrollback is disposable; Git history isn't. The review
  artifacts, audits, and surprising findings that used to live
  only in chat now live as files you can `grep` and link to.
- Codex prompt filenames previously looked like
  `YYYY-MM-DD-<slug>-NN.md`. The inserted daily sequence (`-NNNN-`
  before the slug) makes same-day entries sortable by creation
  order regardless of topic, and keeps the trailing round suffix
  meaningful only for multi-part work on one entry.

## What got renamed

The two existing codex-prompt files (both rounds of the Phase 1
mechanics-config extraction on 2026-04-20, which are one logical
entry at daily-sequence `0001`):

- `docs/codex-prompts/2026-04-20-phase-1-mechanics-config-extraction-01.md`
  → `docs/codex-prompts/2026-04-20-0001-phase-1-mechanics-config-extraction-01.md`
- `docs/codex-prompts/2026-04-20-phase-1-mechanics-config-extraction-02.md`
  → `docs/codex-prompts/2026-04-20-0001-phase-1-mechanics-config-extraction-02.md`

## Related

- `docs/design/13-conventions.md` §Journal-like files.
- `docs/design/13-conventions.md` §Notes to humans.
- `.claude/skills/codex-prompt-archive/SKILL.md` (filename-format references updated).
- `CLAUDE.md` (new bullet pointing at the notes-to-humans rule).
