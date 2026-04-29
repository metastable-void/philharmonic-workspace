---
name: docs-jp
description: Generate or update the Japanese executive summary document in the docs-jp/ submodule. Trigger at substantial milestones — phase completion, sub-phase completion with visible product progress, pre-milestone (Golden Week, reference deployment), or on explicit user request. This is Claude Code's task (not Codex). The document is for non-technical corporate decision makers; no project names, no technical jargon, no FLOSS framing.
---

# docs-jp — Japanese executive summary for corporate briefing

This skill generates the `YYYY-MM-DD-開発サマリー.md` file inside
the `docs-jp/` submodule. The document is aimed at non-technical
decision makers for in-corporate marketing and briefing.

## When to trigger

- **At substantial milestones**: phase completion, meaningful
  sub-phase completion (e.g. "all three bin targets running"),
  pre-holiday breaks, reference deployment readiness.
- **On explicit user request**: "update docs-jp", "write the
  Japanese summary", etc.
- **NOT on every commit.** Only when there's something
  structurally new to report to a non-technical audience.

## Procedure

1. **Read `docs-jp/README.md`** — it is the authoritative
   specification for content, tone, and constraints. Re-read it
   every time; the spec may have been updated.

2. **Read the current state of the project** — `README.md`
   (status section), `ROADMAP.md` (current phase), and any
   recent `docs/notes-to-humans/` entries that describe what
   just landed.

3. **Write the document** in Japanese to
   `docs-jp/YYYY-MM-DD-開発サマリー.md` (replacing `YYYY-MM-DD`
   with today's date from `./scripts/xtask.sh calendar-jp`).
   Overwrite the existing file if one exists for today's date.

4. **Content rules** (from `docs-jp/README.md` — these are
   non-negotiable):
   - **No project names**: no "Philharmonic", no "Mechanics",
     no internal crate names.
   - **No organization names** except common external vendors
     (Anthropic, Microsoft, Google, etc.).
   - **No FLOSS/open-source framing** — confusing for decision
     makers.
   - **No "published at GitHub/crates.io"** wording.
   - **No overly technical language** — no crate names, no
     Cargo, no COSE, no TOML, no axum, no rustls. Translate
     technical concepts into business value.
   - **Focus on differentiation**: why this product stands
     out, what makes it different.
   - **Careful "not LLM-centric" framing**: AI is a buzzword;
     position as "general-purpose workflow automation that
     also supports AI/LLM use cases" rather than "an AI
     product."
   - **Primary use case**: local-LLM-powered AI support chat
     app.
   - **Secondary focus**: usage at a Japanese tech media outlet
     they operate, and automating day-to-day company work as
     future possibilities.

5. **Language**: Japanese (日本語). This is the one place in the
   workspace where non-English content is the primary artifact.
   This submodule is exempt from the English-only rule in
   `CONTRIBUTING.md §14.6`.

6. **Commit**: via `./scripts/commit-all.sh "docs-jp: update
   Japanese executive summary (YYYY-MM-DD)"`. The submodule
   commit lands inside `docs-jp/`, and the parent commit bumps
   the submodule pointer.

## Do not

- Do not delegate this to Codex. Claude writes this directly.
- Do not include any English-language content in the document
  (aside from universally understood loanwords like "AI").
- Do not mention internal infrastructure (Git submodules,
  workspace structure, CI, scripts).
- Do not use marketing superlatives without substance — the
  audience is experienced decision makers, not consumers.
