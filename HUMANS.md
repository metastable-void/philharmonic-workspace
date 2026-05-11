# Instructions/Notes for human developers

**For coding agents**: You can freely read the contents of
this file, but never edit this file.

**The following is notes for humans and not for coding agents**

---

## Reminders

- make sure we always make docs/roadmaps up-to-date.
- not blocking for this project: we should refactor all
  the best practices for AI coding brewed inside this
  workspace repo into a reusable template/Rust crate/etc.
  this can happen after the MVP work is done.

## Follow-up tasks from 2026-05-10 work

- Parse/render markdown in chat UI, with DOMPurify hardening.
- Workflow template edit UI should expose a pull-down-menu-based
  UI (with the endpoint name field) for configuring endpoints
  for the template.
- Some OpenAI-compatible inference providers (some local LLM
  server implementations and some Inference Providers on
  Hugging Face) don't support a forced `tool_choice` for
  faked tool structured outputs: add an option to set
  `tool_choice: "auto"` (or `tool_call_fallback_auto` flavor).

## WebUI

Note: Keep WebUI up-to-date with any API features added
in the future.

- **Code editor**. Please add a sensible and well-maintained code editor (syntax highlighting, auto indents) dependencies to WebUI, and use that in JSON/JS editors.

## Keep the workflow authoring guide up-to-date

Re-read the docs/codex of everything related, and re-write
workflow authoring guides in en/jp to reflect the facts
on any surface changes.
