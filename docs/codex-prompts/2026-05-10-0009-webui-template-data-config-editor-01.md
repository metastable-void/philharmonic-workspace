# Follow-up — WebUI template-form `data_config` editor (initial dispatch)

**Date:** 2026-05-10
**Slug:** `webui-template-data-config-editor`
**Round:** 01 (initial dispatch — D11 follow-up #3
flagged in `docs/codex-prompts/2026-05-10-0008-d11-...`,
ROADMAP §3.D `data_config` UX gap)
**Subagent:** `codex:codex-rescue`

## Motivation

D3 round 02 (`b134d44`) added `data_config` to the workflow
template wire shape. D5 (`e37f956`) wired
`data.embed_datasets.<name>` through the engine. D6
(`b581b50`) shipped the embedding-datasets WebUI surface
end-to-end. D11 (`10acd7f`) documented all of this in
the workflow-authoring guide.

But the **WebUI template form itself** (Create + Detail
edit) currently does not expose `data_config` at all.
The current TemplateDetail page exposes only:

- `display_name`
- `script_source`
- `abstract_config` (raw JSON via CodeMirror)

So today, the only way to bind an embedding dataset to a
workflow template is:

1. Create the template via WebUI (data_config omitted →
   empty `{}` server-side).
2. PATCH the template via direct API call (or, for fresh
   creation, POST through the API directly with curl /
   shell), supplying `data_config.embed_datasets`.

This is the friction the new D11 guide explicitly calls out:
"The current WebUI template form exposes display name,
script source, and `abstract_config`; it does not expose a
structured `data_config` editor."

This dispatch closes the gap with a structured editor
(no raw JSON for `data_config` per the HUMANS.md
"friendly UI, not raw JSON" mandate that justified D6).
Once landed, admins can bind embedding datasets to
templates entirely through the WebUI.

## References

- [`docs/ROADMAP.md` §3.D](../ROADMAP.md#d-webui-infrastructure-features-and-docs-3-dispatches)
  — D11 outcome notes `data_config` UI gap as a tracked
  follow-up.
- [`docs/codex-prompts/2026-05-10-0008-d11-...`](2026-05-10-0008-d11-workflow-authoring-guide-rewrite-01.md)
  — D11 outcome surfaced this gap; one of three
  Claude-side follow-ups (this one is delegated to Codex
  because it is non-trivial code).
- [`docs/guide/workflow-authoring.md`](../guide/workflow-authoring.md)
  §"Creating a template" — authoritative spec for the
  `data_config` shape on the wire and binding-name rules:
  "1 to 64 bytes, starting with an ASCII letter, `_`, or
  `$`, and continuing with ASCII letters, digits, `_`, or
  `$`."
- D6 prompt + outcome:
  [`docs/codex-prompts/2026-05-10-0006-d6-embed-datasets-webui-01.md`](2026-05-10-0006-d6-embed-datasets-webui-01.md)
  — closest WebUI dispatch precedent; mirror the
  structured-editor conventions established there
  (`SourceItemsEditor.tsx` is the structured editor
  Codex extracted in D6).
- D13 prompt + outcome:
  [`docs/codex-prompts/2026-05-10-0007-d13-chat-testing-ui-01.md`](2026-05-10-0007-d13-chat-testing-ui-01.md)
  — most recent WebUI dispatch precedent; mirror the
  i18n / api-client / ts-typecheck / production-build
  workflow.
- API surface (no version bump for this dispatch — the
  routes already accept `data_config`):
  - `POST /v1/workflows/templates` with `data_config:
    Option<JsonValue>` per `philharmonic-api/src/routes/
    workflows.rs:540-545`.
  - `PATCH /v1/workflows/templates/{id}` with
    `data_config: Option<JsonValue>` per
    `philharmonic-api/src/routes/workflows.rs:552-559`.
  - `GET /v1/workflows/templates/{id}` returns
    `data_config: Option<JsonValue>` per
    `philharmonic-api/src/routes/workflows.rs:577-588`.
  - `validate_data_config` server-side check at
    `philharmonic-api/src/routes/workflows.rs:1190` —
    rejects non-object `data_config`, rejects non-object
    `embed_datasets`, rejects non-UUID-string values,
    rejects bindings that reference unknown / retired
    datasets.
- `philharmonic/webui/src/api/client.ts` — current
  `TemplateDetail` interface lines 35-38 and
  `CreateTemplateRequest` lines 40-44 and
  `UpdateTemplateRequest` lines 50-54. None mention
  `data_config`. This is the gap.
- `philharmonic/webui/src/api/client.ts` — existing
  `listEmbeddingDatasets` helper line 392 (covers the
  dataset dropdown's data source).
- `philharmonic/webui/src/pages/Templates.tsx` (235
  lines) — Create form lives here.
- `philharmonic/webui/src/pages/TemplateDetail.tsx` (210
  lines) — edit form + display lives here.
- `philharmonic/webui/src/components/SourceItemsEditor.tsx`
  — D6's structured-table-editor extraction; the closest
  precedent for the `data_config` editor's table layout.

## Context files pointed at

`philharmonic/webui/src/`:

- `api/client.ts` — extend `TemplateDetail`,
  `CreateTemplateRequest`, `UpdateTemplateRequest` with
  `data_config?: TemplateDataConfig`. Add the
  `TemplateDataConfig` type:

  ```ts
  export interface TemplateDataConfig {
    embed_datasets?: Record<string, string>;
  }
  ```

  Forward-compatible: keep `embed_datasets` as the only
  documented key, but the type allows additional keys for
  future expansion. The wire shape is JsonValue so the API
  accepts arbitrary fields; the WebUI exposes only
  `embed_datasets`.

- `pages/Templates.tsx` — Create form: add a
  "Data bindings" section between `abstract_config` and
  the submit button. The section contains a structured
  `data_config.embed_datasets` editor (binding name +
  dataset dropdown rows). When a user adds at least one
  binding, the request sends `data_config: {
  embed_datasets: { <name>: <uuid>, ... } }`. When the
  user has no bindings, omit `data_config` from the
  request body (cleaner than sending `{embed_datasets:
  {}}`; matches the `Option<JsonValue>` server-side
  shape).

- `pages/TemplateDetail.tsx` — Edit form: same structured
  editor, populated from the template's existing
  `data_config.embed_datasets` on load. Submit sends
  `data_config: TemplateDataConfig` on PATCH (always
  include the field on update — leaving it out on PATCH
  preserves the prior value, but always sending the full
  current shape is clearer for "edit means replace this
  state"). If the user clears all bindings, send
  `data_config: { embed_datasets: {} }` rather than
  omitting (PATCH semantics: omitted = unchanged; sending
  empty = explicitly empty).

- `components/DataConfigEditor.tsx` (new, ~80-150 LOC) —
  extract the editor as a reusable component since
  Templates and TemplateDetail both use it. Mirror
  `SourceItemsEditor.tsx`'s extraction pattern.

  Component shape:

  ```tsx
  interface DataConfigEditorProps {
    value: TemplateDataConfig;
    onChange: (value: TemplateDataConfig) => void;
    availableDatasets: EmbeddingDatasetSummary[];
    disabled?: boolean;
  }
  ```

  Internally renders a table of (binding_name,
  dataset_id) rows with:
  - Binding name text input with client-side validation
    against the JS-property regex (1-64 bytes, starts
    with `[A-Za-z_$]`, continues with `[A-Za-z0-9_$]`).
    Per-row error highlighting on invalid name; disable
    the form's submit button while any row is invalid.
  - Dataset dropdown populated from
    `availableDatasets`. Show `display_name` (with
    `dataset_id` short-hash in parens for
    disambiguation if multiple datasets share a
    display_name). Filter retired datasets out of the
    dropdown options (but keep an existing binding's
    retired dataset visible with a warning badge —
    don't silently drop user data).
  - Add row / remove row controls.
  - Empty state: "No data bindings. Add one to expose an
    embedding dataset to this template's script."

- `i18n/en.ts` + `i18n/ja.ts` — add `templates.dataConfig.*`
  i18n entries covering every visible string from the
  editor (column headers, error messages, empty state,
  add/remove labels, retired-binding warning).

- `pages/EmbedDatasets.tsx` / `EmbedDatasetDetail.tsx` —
  **read-only references**. Do not modify these files;
  they are D6's surface and unrelated to this dispatch.

`scripts/webui-build.sh` — production build command. Use
`./scripts/webui-build.sh --production` to regenerate the
four committed `dist/` artifacts.

## Outcome

Pending — will be updated after Codex run.

---

## STRUCTURED-OUTPUT-CONTRACT — READ THIS FIRST

The contract has been honored for **seven** consecutive
rounds (since D3 round 02). `RUN STATUS: COMPLETE` token +
six-section report emitted before `task_complete`.
**Maintain the bar — do not break the streak.**

The contract is repeated at the end of the prompt; it's on
you to actually emit it before `task_complete`.

---

## Pre-landing-sh hygiene equivalent for the WebUI

Run BEFORE invoking `webui-build.sh --production`:

```sh
cd philharmonic/webui
npx tsc --noEmit
```

Catches TypeScript errors fast (faster than letting webpack
report them at build time). Then run the production build to
regenerate the four committed `dist/` artifacts. Then run
`./scripts/pre-landing.sh` once at the end (the API server
bin embeds the WebUI bundle, so a broken bundle breaks
`cargo build`).

---

## Prompt (verbatim)

<task>
Add a structured `data_config.embed_datasets` editor to the
WebUI template form (Create on `pages/Templates.tsx` + Edit
on `pages/TemplateDetail.tsx`). Extract the editor as
`components/DataConfigEditor.tsx` for reuse. Single
submodule `philharmonic/webui`. No backend changes.

Five logical surfaces:

- **A** — Types & API client extensions
  (`api/client.ts`)
- **B** — `components/DataConfigEditor.tsx` (new
  reusable structured editor)
- **C** — Create-form integration
  (`pages/Templates.tsx`)
- **D** — Edit-form integration
  (`pages/TemplateDetail.tsx`)
- **E** — Page-specific i18n
  (`templates.dataConfig.*` namespace in both `en.ts`
  and `ja.ts`)

Suggested order: **A → B → E → C → D**. Build types,
component, and i18n strings first; then wire C and D.

If anything below contradicts the API server's actual
behavior at `philharmonic-api/src/routes/workflows.rs`,
**the API server wins**. Flag the divergence in your
structured output.

If you hit scope limits, finish whichever surface is
closest to done and report what's left in the structured
output. **Do not** silently abandon a half-done component,
and do **not** ship a UI that compiles but renders broken
pages.

## Friendly-UI mandate (load-bearing per HUMANS.md)

Per HUMANS.md §"Embedding DB component" final erratum:
"No raw JSON editor for Embedding DB: please add a
friendly UI." This applies transitively to `data_config`
since `data_config.embed_datasets` is the workflow-side
binding for the same data. **No raw JSON editor for
`data_config`** — the structured editor is mandatory.

The workflow-authoring guide
[`docs/guide/workflow-authoring.md`](docs/guide/workflow-authoring.md)
documents the wire shape:

```json
{
  "data_config": {
    "embed_datasets": {
      "knowledge_base": "<embedding-dataset-uuid>"
    }
  }
}
```

The `<binding_name>` keys are JavaScript-property-like:

> "1 to 64 bytes, starting with an ASCII letter, `_`, or
> `$`, and continuing with ASCII letters, digits, `_`,
> or `$`."

Validate this client-side; surface invalid names per-row
with an inline error.

## Deliverable A — Types & API client (`api/client.ts`)

Add a typed `TemplateDataConfig` interface:

```ts
/** Data bindings declared on a workflow template.
 *  Currently only `embed_datasets` is exposed; the type
 *  permits future expansion. */
export interface TemplateDataConfig {
  embed_datasets?: Record<string, string>;
}
```

Extend the existing template types:

```ts
export interface TemplateDetail extends TemplateSummary {
  script_source: string;
  abstract_config: JsonValue;
  data_config: TemplateDataConfig | null;  // null when omitted server-side
}

export interface CreateTemplateRequest {
  display_name: string;
  script_source: string;
  abstract_config: JsonValue;
  data_config?: TemplateDataConfig;  // omit to leave empty
}

export interface UpdateTemplateRequest {
  display_name?: string;
  script_source?: string;
  abstract_config?: JsonValue;
  data_config?: TemplateDataConfig;  // omit to preserve, send to replace
}
```

JSDoc on every new field.

**Important wire-shape note**: the API serializes
`data_config: Option<JsonValue>` — the response includes
`"data_config": null` when the template has no
data_config, or `"data_config": { "embed_datasets":
{...} }` when it does. Treat both `null` and missing as
"no bindings"; the WebUI normalizes on read.

## Deliverable B — `components/DataConfigEditor.tsx` (new)

Reusable component for both Create and Edit forms.

```tsx
import { type JSX, useId } from "react";
import {
  type EmbeddingDatasetSummary,
  type TemplateDataConfig,
} from "../api/client";
import { useT } from "../hooks/useT";

interface DataConfigEditorProps {
  value: TemplateDataConfig;
  onChange: (next: TemplateDataConfig) => void;
  availableDatasets: EmbeddingDatasetSummary[];
  disabled?: boolean;
}

export default function DataConfigEditor(
  props: DataConfigEditorProps,
): JSX.Element {
  // ...
}
```

Internal state: render a list of (name, dataset_id) rows
derived from `value.embed_datasets`. Add a row when the
user clicks "Add binding". Remove a row when the user
clicks the row's "Remove" button. Update both name and
dataset via row-level controls.

**Binding-name validation** (client-side):

```ts
const BINDING_NAME_REGEX = /^[A-Za-z_$][A-Za-z0-9_$]{0,63}$/;
```

(1-64 chars total, but the regex captures it as
`first-char + 0-63 continuation chars`.)

Per-row inline error display: highlight the row when
invalid; show the error text under the name input
(`t.templates.dataConfig.invalidBindingName`). The parent
form should observe overall validity through a derived
prop or a callback (Codex's choice — keep the surface
small; the parent only needs to know "is the form
submittable").

**Dataset dropdown options**:

- Source: `props.availableDatasets` (filtered list of
  active embed datasets passed in by the parent).
- Filter retired datasets **out** of options for new
  rows.
- For an existing row whose `dataset_id` references a
  retired-or-missing dataset, **keep the row visible**
  with a warning badge ("retired" / "missing"). Don't
  silently drop user data — surfacing the warning lets
  the admin re-bind to a different dataset.

**Empty state**: when `value.embed_datasets` is empty or
absent, show:

```
No data bindings.
Add one to expose an embedding dataset to this template's script.
[+ Add binding]
```

(via `t.templates.dataConfig.emptyState`.)

**Disabled mode**: when `props.disabled` is true, render
the table read-only. Used while the parent form is
saving.

**Duplicate-name check**: client-side; if two rows have
the same name, highlight both. Server-side, duplicate
keys collapse on the JSON object — the second wins. The
WebUI should prevent this confusion before submit.

## Deliverable C — Create form integration (`pages/Templates.tsx`)

The Templates list page hosts the Create form (modal or
inline section per the existing pattern; inspect
`Templates.tsx` first to see whether Create is a modal
or inline).

In the Create form:

1. Fetch the active embed-datasets list via
   `listEmbeddingDatasets()` once on mount. Filter for
   `is_retired === false`. Pass to the
   `DataConfigEditor` as `availableDatasets`.
2. Add a `dataConfig: TemplateDataConfig` state slot
   alongside the existing displayName /
   scriptSource / abstractConfig state. Initial value:
   `{ embed_datasets: {} }`.
3. Render `<DataConfigEditor value={dataConfig}
   onChange={setDataConfig}
   availableDatasets={...} />` in the form, between
   the `abstract_config` editor and the submit button.
4. On submit:
   - If `dataConfig.embed_datasets` is empty (no rows),
     omit `data_config` from the request body.
   - Otherwise send `data_config: { embed_datasets:
     dataConfig.embed_datasets }`.
   - Surface server-side validation errors (e.g. "data
     config: dataset uuid <x> not found") as toasts.

## Deliverable D — Edit form integration (`pages/TemplateDetail.tsx`)

In the TemplateDetail edit form:

1. Fetch active embed-datasets list as in Deliverable C.
2. On template-load, normalize `template.data_config`:
   - If `null` or undefined: initial state `{
     embed_datasets: {} }`.
   - Otherwise: copy `data_config.embed_datasets` (or
     `{}` if missing) into the local state slot.
3. Render the editor between the `abstract_config`
   editor and the existing form's actions row.
4. On submit (PATCH):
   - **Always send `data_config`** — sending the full
     current shape on PATCH is clearer than "omit to
     preserve" semantics. If the user has cleared all
     bindings, send `data_config: { embed_datasets:
     {} }`.
   - Surface server-side validation errors as toasts.

5. After successful save, re-fetch the template (or
   apply the response if the API returns the updated
   detail — inspect the response shape) so the editor's
   state matches server state.

## Deliverable E — Page-specific i18n

Add to both `i18n/en.ts` and `i18n/ja.ts` under the
existing `templates` namespace:

```ts
templates: {
  // existing entries...
  dataConfig: {
    sectionTitle: "Data bindings",
    description:
      "Bind embedding datasets to this template; scripts read them via data.embed_datasets.<name>.",
    emptyState: "No data bindings.",
    emptyStateHint:
      "Add one to expose an embedding dataset to this template's script.",
    addBinding: "Add binding",
    removeBinding: "Remove",
    bindingName: "Binding name",
    bindingNamePlaceholder: "knowledge_base",
    dataset: "Embedding dataset",
    datasetPlaceholder: "Select a dataset...",
    invalidBindingName:
      "Binding names use ASCII letters, digits, _ or $; 1-64 bytes; cannot start with a digit.",
    duplicateBindingName: "Duplicate binding name.",
    retiredDatasetWarning: "Retired",
    missingDatasetWarning: "Missing",
    noActiveDatasets: "No active embedding datasets.",
    noActiveDatasetsHint:
      "Create one in the Embedding Datasets section first.",
  },
},
```

JP translations should match the surrounding i18n style
(katakana for technical terms like データバインディング,
kanji for verbs / states). If unsure on a term, leave a
`// TODO(jp):` comment and note in residual risks.

## Cross-deliverable: build, verification, artifacts

After all five deliverables are in place:

1. **TypeScript typecheck first**:

   ```sh
   cd philharmonic/webui
   npx tsc --noEmit
   ```

   Fix any errors before invoking webpack.

2. **Production build**:

   ```sh
   ./scripts/webui-build.sh --production
   ```

3. **Workspace `pre-landing.sh`**: needed because the API
   server bin compiles the new bundle into the binary.

4. **Run the existing WebUI tests** if any (check
   `philharmonic/webui/package.json` for a `test` script;
   may not exist).

The committed artifacts (post-build) are part of the
diff — `dist/main.js`, `dist/main.css`, `dist/index.html`,
`dist/icon.svg` change with every functional UI change.

## Cross-deliverable: no version bump on the WebUI

The WebUI lives in `philharmonic/webui` (a sub-tree of the
`philharmonic` meta-crate's submodule, NOT a published
crate). No `Cargo.toml` version bump.

<structured_output_contract>
**Critical: emit this report before `task_complete`.**

Six sections, in this order:

1. **Summary** — one paragraph: what landed, which
   surfaces (A/B/C/D/E) are complete vs partial vs not
   started. Include the verbatim string "RUN STATUS:
   COMPLETE" or "RUN STATUS: PARTIAL — <reason>" for
   grep.

2. **Touched files** — exhaustive list with `(new|edited|deleted)
   <path> — <one-line note>`. Include the four
   regenerated artifacts in `philharmonic/webui/dist/`.

3. **Verification results** — exact commands + outcomes:
   - `npx tsc --noEmit` (in `philharmonic/webui/`) —
     pass/fail.
   - `./scripts/webui-build.sh --production` —
     pass/fail.
   - `./scripts/pre-landing.sh` — pass/fail.
   - `./scripts/test-scripts.sh` — pass/fail (run only if
     you touched any `scripts/*.sh`; this dispatch
     should not).

4. **Residual risks / known issues** — including:
   - Any `any` types you had to use because the JsonValue
     widening hit limits, and where.
   - Whether you handled the retired-dataset-already-
     bound case (warning badge vs. silent drop) and how.
   - Whether the JP translations are placeholders Yuka
     should review, or you matched established style.
   - Bundle-size delta from the new editor (in KB,
     gzipped).
   - Whether the editor handles arbitrary other keys
     under `data_config` (the type permits them; the UI
     only renders `embed_datasets`).
   - Any case where the binding-name regex client-side
     rejects names the server would accept (or vice
     versa) — divergence is a server-side change request,
     not a client fix.
   - Whether you exposed the editor to read-only mode
     when the template is retired, and if so how.

5. **Git state** — current `HEAD` SHA in
   `philharmonic` submodule (the WebUI's git lives there).
   Confirm no commits made.

6. **Open questions** — questions for Yuka or Claude:
   - Whether to expose `data_config` keys other than
     `embed_datasets` (currently only `embed_datasets`
     is documented; the type allows others).
   - JP translation quality / terminology choices.
   - Whether the dataset dropdown should show short-hash
     UUIDs alongside display names for disambiguation
     when multiple datasets share a display_name.
   - Whether retired-dataset bindings should auto-prune
     on save or remain (current implementation: remain,
     with warning badge).
</structured_output_contract>

<default_follow_through_policy>
- Suggested order: A → B → E → C → D. Types, component,
  and i18n first; then wire C and D.
- Run `npx tsc --noEmit` after each surface, before
  moving on.
- Add JSDoc on all new exported types and functions in
  `api/client.ts`.
- If a deliverable's component references a type that's
  not yet defined in `client.ts`, define the type first
  (in deliverable A).
- If you discover that an API route's actual response shape
  differs from this prompt's spec, **trust the API route**
  and adapt — flag the discrepancy.
- The editor lives entirely client-side; no backend
  changes. If you find yourself wanting to add a field to
  any Rust crate, **stop** and surface in residual risks.
</default_follow_through_policy>

<completeness_contract>
"Done" means:

- All five surfaces (A/B/C/D/E) functionally complete.
- TypeScript typecheck clean.
- `./scripts/webui-build.sh --production` clean.
- `./scripts/pre-landing.sh` clean.
- The four artifacts in `philharmonic/webui/dist/`
  regenerated.
- Structured output report emitted before
  `task_complete`.

Partial completion is acceptable if you hit a token
limit or genuine blocker — but you must say so
explicitly with "RUN STATUS: PARTIAL — <reason>".
Half-built UI surfaces are worse than missing surfaces
because they crash on user interaction; if you can't
finish a surface end-to-end, either revert it or guard
it behind a "not yet implemented" placeholder.

A run without the structured-output report is
**incomplete**, even if all five surfaces landed.
</completeness_contract>

<verification_loop>
For every surface:
1. Implement the component / type / i18n.
2. `cd philharmonic/webui && npx tsc --noEmit` — green.
3. Move on. Don't run webpack between surfaces (slow).
4. Once all surfaces are typechecked clean, run
   `./scripts/webui-build.sh --production` once.
5. Run `./scripts/pre-landing.sh` once.
6. Emit the structured output report.
7. Then `task_complete`.
</verification_loop>

<missing_context_gating>
If you find yourself needing information not in this
prompt or the cited authoritative docs, **stop** and
report what's missing in the structured output's "Open
questions" section.

Specifically: do **not**:

- Touch any Rust crate (this is WebUI-only scope).
- Add a new npm dependency.
- Edit `webpack.config.js` or `tsconfig.json`.
- Mint new permission atoms or groups.
- Edit `philharmonic-api`, `philharmonic-policy`,
  `philharmonic-workflow`, or any backend crate.
- Add a new route to `App.tsx`.
- Edit `pages/EmbedDatasets.tsx` or
  `pages/EmbedDatasetDetail.tsx` (D6 surfaces;
  unrelated to this dispatch).
- Edit `HUMANS.md`.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `philharmonic/webui/src/api/client.ts` (edited —
  `TemplateDataConfig` type + extensions to
  TemplateDetail / CreateTemplateRequest /
  UpdateTemplateRequest).
- `philharmonic/webui/src/components/DataConfigEditor.tsx`
  (new — reusable structured editor).
- `philharmonic/webui/src/pages/Templates.tsx` (edited —
  Create form integration + datasets fetch).
- `philharmonic/webui/src/pages/TemplateDetail.tsx`
  (edited — Edit form integration + datasets fetch +
  initial-state normalization).
- `philharmonic/webui/src/i18n/en.ts` (edited —
  `templates.dataConfig.*` namespace).
- `philharmonic/webui/src/i18n/ja.ts` (edited —
  `templates.dataConfig.*` namespace).
- `philharmonic/webui/src/i18n/index.ts` (edited only if
  the existing pattern requires per-namespace
  registration).
- `philharmonic/webui/src/app.css` (edited if new
  table-row / warning-badge styles needed; reuse
  existing classes first).
- `philharmonic/webui/dist/index.html`,
  `dist/main.js`, `dist/main.css`, `dist/icon.svg`
  (regenerated by `webui-build.sh --production`).

Files NOT to touch (flag if you find a reason to):

- Any file under `bins/`, `philharmonic-api/`,
  `philharmonic-policy/`, `philharmonic-workflow/`,
  `philharmonic-store*/`, `mechanics-*/`, or any
  connector crate.
- The workspace `Cargo.toml` `[patch.crates-io]` block.
- `philharmonic/webui/webpack.config.js`,
  `philharmonic/webui/tsconfig.json`,
  `philharmonic/webui/package.json`.
- `philharmonic/webui/src/components/CodeEditor.tsx`
  (D10's CodeMirror wrapper — no need to touch).
- `philharmonic/webui/src/components/SourceItemsEditor.tsx`
  (D6's editor — extract patterns only, don't edit).
- `philharmonic/webui/src/components/permissions.ts` (no
  permission changes for this dispatch).
- `philharmonic/webui/src/App.tsx` (no new routes).
- `philharmonic/webui/src/components/Layout.tsx` (no new
  sidebar entries).
- `philharmonic/webui/src/pages/EmbedDataset*.tsx` (D6's
  pages — unrelated to this dispatch).
- `HUMANS.md`, `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md`,
  any `.claude/`, `docs/`, `docs-jp/`, or `scripts/`
  content.

Git rules: signed-off + signed commits via
`scripts/commit-all.sh` only. **Codex never runs
`commit-all.sh`** — Claude commits after reviewing your
work. No `cargo publish`. No raw `git commit` /
`git push` / `git add` / `git reset` / `git rebase` /
`git revert`. Read-only `git log` / `git diff` /
`git show` is fine.
</action_safety>
</task>
