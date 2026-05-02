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

## Embedding DB component

Not in the original v1 scope; MVP done, so we want the
following:

- Extend what JS codes take from:
  `{context, args, input, subject}` to
  `{context, args, input, subject, data}`.
- `data` is conceptually `Record<string, any_data_defined>`.
- We define a new data field named `embed_datasets`:
  `data.embed_datasets: Record<string, CorpusItem[]>`
  where CorpusItem is defined by connector-impl-vector-search.
- Any data field can be absent to run a workflow successfully
  (unless JS requires it). `data` is `{}` when no data fields
  exist.
- A new WebUI management target named `Embedding Datasets`
  and respective API endpoints.
- A user can create/update a dataset with items each containing
  an `id` (string identifier) plus an optional JSON payload,
  and a text to embed.
- On create/update, an ephemeral JS task with long timeouts
  (not saved to DB) is created, and it calls embed connector
  for each item, and returns `CorpusItem[]`. This JS code
  is authored by Codex, and embedded into the API bin so the
  users never touches/sees it. Informative UI about the embed
  status of a dataset. The embed connector is the one of the
  tenant that creates the dataset.
- A user can assign an Embedding Dataset to a workflow template.
- An assigned Embedding Dataset appears to JS as
  `data.embed_datasets.<assigned_name>`. Not-yet-embedding-done
  dataset appears with previous contents, or is missing from
  the Record.

This is Codex's work.

**Eratta**:

- Embedding DB content slots should use deterministic CBOR for more dense storage (wording might differ per latest RFC; please check).
- Migrate content slots to `LONGBLOB`s; add a migration to do that, run automatically on startup.
- No raw JSON editor for Embedding DB: please add a friendly UI.

## WebUI

Note: Keep WebUI up-to-date with any API features added
in the future.

- **Code editor**. Please add a sensible and well-maintained code editor (syntax highlighting, auto indents) dependencies to WebUI, and use that in JSON/JS editors.

## Update the workflow authoring guide

Re-read the docs/codex of everything related, and re-write
workflow authoring guides in en/jp to reflect the facts.
This is to be dispatched to Codex.
