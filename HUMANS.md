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

## Day-to-day housekeeping: Audit & refactor

### Maintainability notes

Always watch the whole workspace (spawning subagents is
preferred) for maintainability issues, dirty/spaghetti codes,
and quality issues (e.g. memory leaks, deadlocks, races, etc.).

Refactor codes to make the code structured, small, de-duplicated.

### Clean separation of concerns

- Unpublished bin crates should be minimal;
  **they own Clap CLI** (that should not be upstreamed),
  but any real codes should be upstreamed, creating
  crates if really necessary.
- Chats are a workflow knowledge; the framework in principle
  should not know anything about the workflows, but it's really
  useful for testing, so Chat UI will live elsewhere (in-tree
  `philharmonic-chat-app` bin for frontend/backend unified, or
  in another project) in the future, although we don't remove
  the old Chat UI immediately right now. See below.

## Chat UI separation

TBD.

## WebUI

Note: Keep WebUI up-to-date with any API features added
in the future.

## Keep the workflow authoring guide up-to-date

Re-read the docs/codex of everything related, and re-write
workflow authoring guides in en/jp to reflect the facts
on any surface changes.
