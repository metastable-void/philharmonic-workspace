# D6 — Embedding-datasets WebUI (initial dispatch)

**Date:** 2026-05-10
**Slug:** `d6-embed-datasets-webui`
**Round:** 01 (initial dispatch — D6, ROADMAP §3.A, single
crate `philharmonic/webui`, closes the embedding-datasets
feature end-to-end)
**Subagent:** `codex:codex-rescue`

## Motivation

The embedding-datasets backend is operational end-to-end as
of today's commits:

- Round 01 (`bbc26f9`) — data layer (entity, atoms, codec,
  `data_config` slot).
- Round 02 (`b134d44`) — workflow-engine `data` assembly + 7
  embed-datasets API routes + template-route `data_config`
  extension.
- Round 03 (`e37f956`) — D5 dispatcher with D4's Gate-1
  approved synthesized inst, caps wire-up, ApiError::Conflict.

D6 is the WebUI surface so admins can use the feature without
hand-rolling JSON. Per design 16 §"WebUI" + the HUMANS.md
"friendly UI, not raw JSON" erratum: structured table editor
for source items, bulk-import modal accepting pasted CSV/JSON,
detail page with metadata + Source Items + Corpus tabs (corpus
vector view collapsed by default — rendering 1024-dim float
arrays per row crashes the page), polling refresh for status
transitions, full i18n in `en.ts` + `ja.ts`.

After D6, the embedding-datasets feature is shippable
end-to-end. Only D7-D9 (Tier 2/3 connectors) and D11 (doc
rewrite) remain in the post-v1 plan.

## References

- [`docs/ROADMAP.md` §3.A](../ROADMAP.md#a-embedding-datasets-6-dispatches--1-gate-1)
  — D6 spec.
- [`docs/design/16-embedding-datasets.md`](../design/16-embedding-datasets.md)
  §"WebUI" — authoritative spec for every page, tab, and
  modal. **If anything in this prompt contradicts the design,
  the design wins; flag in your structured output.**
- Round-02 prompt + outcome:
  [`docs/codex-prompts/2026-05-10-0003-d3-embed-datasets-integration-01.md`](2026-05-10-0003-d3-embed-datasets-integration-01.md)
  — context for the seven API routes the WebUI calls.
- Round-03 prompt + outcome:
  [`docs/codex-prompts/2026-05-10-0004-embed-backend-completion-01.md`](2026-05-10-0004-embed-backend-completion-01.md)
  — context for the embed-job dispatch + 409-on-Embedding
  semantics the UI must surface.
- `philharmonic-api 0.1.5` API surface:
  - `POST /v1/embed-datasets` (create)
  - `GET /v1/embed-datasets` (cursor-paginated list)
  - `GET /v1/embed-datasets/{id}` (metadata only)
  - `POST /v1/embed-datasets/{id}/update` (replace items, 409
    on `status=Embedding`)
  - `POST /v1/embed-datasets/{id}/retire`
  - `GET /v1/embed-datasets/{id}/source-items` (decoded JSON)
  - `GET /v1/embed-datasets/{id}/corpus` (decoded JSON, 404
    when no corpus available yet)
- `EmbeddingDatasetStatus` discriminants from
  `philharmonic-policy 0.2.2`: `Created=0`, `Embedding=1`,
  `Ready=2`, `Failed=3`.
- D10's `philharmonic/webui/src/components/CodeEditor.tsx`
  (CodeMirror 6 wrapper) for the nested-payload sub-editor.
- HUMANS.md "WebUI → Code editor" + "WebUI → friendly UI for
  embedding datasets" entries — the friendly-UI mandate is
  load-bearing.

## Context files pointed at

`philharmonic/webui/src/`:

- `App.tsx` — react-router routes; add `/embed-datasets` and
  `/embed-datasets/:id`. Existing Authorities/Audit routes
  show the pattern.
- `components/Layout.tsx` — sidebar navigation; insert the
  new "Embedding Datasets" entry between "Authorities" and
  "Audit" per design 16.
- `api/client.ts` — typed API client. Existing
  `TemplateSummary` / `TemplateDetail` /
  `CreateTemplateRequest` / `EndpointSummary` / etc. show
  the pattern; D6 adds analogous embed-dataset types and
  fetch functions.
- `i18n/en.ts` + `i18n/ja.ts` + `i18n/index.ts` —
  translation tables. `nav.embedDatasets` joins the
  existing nav cluster; `embedDatasets.*` becomes a new
  page-strings section.
- `pages/Endpoints.tsx` + `pages/EndpointDetail.tsx` —
  closest analogue (list + detail of a tenant resource with
  CRUD + retire). Use for the list-page and metadata-tab
  pattern.
- `pages/Templates.tsx` + `pages/TemplateDetail.tsx` —
  shows the create-form with multi-field-editor + tab
  pattern.
- `pages/Instances.tsx` + `pages/InstanceDetail.tsx` —
  shows polling-refresh patterns (instances have a status
  that changes; the embed-dataset status is similar).
- `components/CodeEditor.tsx` — CodeMirror 6 wrapper from
  D10. Use for the per-item nested-payload editor and (if
  appropriate) the bulk-import modal's JSON paste area.
- `components/Pagination.tsx` — for cursor pagination on
  the list page.
- `components/permissions.ts` — the `<prefix>:` group split
  picks up `embed_dataset:*` automatically since round 01.
  WebUI permission grouping needs an i18n label for the new
  group.
- `package.json` — dependencies. Note the existing
  CodeMirror 6 packages.

`scripts/webui-build.sh` — build script. Use
`./scripts/webui-build.sh --production` to produce the
committed artifacts.

## Outcome

Pending — will be updated after Codex run.

---

## STRUCTURED-OUTPUT-CONTRACT — READ THIS FIRST

Rounds 02 / 03 / D12 all honored the contract: `RUN STATUS:
COMPLETE` token + six-section report emitted before
`task_complete`. Round 01 missed it and Claude had to
reconstruct verification state manually. Maintain the bar.

The contract is repeated at the end of the prompt; it's on
you to actually emit it before `task_complete`.

---

## Pre-landing-sh hygiene equivalent for the WebUI

Run BEFORE invoking `webui-build.sh --production`:

```bash
cd philharmonic/webui
npx tsc --noEmit
```

Catches TypeScript errors fast (faster than letting webpack
report them). Then run the production build to regenerate the
committed artifacts.

The Rust-side `pre-landing.sh` is also relevant because the
API server bin embeds the WebUI bundle — a stale or broken
WebUI bundle breaks `cargo build` of `philharmonic-api-server`
which is part of the workspace test phase.

---

## Prompt (verbatim)

<task>
Build the embedding-datasets WebUI for `philharmonic/webui`.
Single submodule, single dispatch, six logical surfaces:

- **A** — Types & API client (`api/client.ts`)
- **B** — Routes + sidebar nav (`App.tsx` + `Layout.tsx` +
  `i18n/{en,ja}.ts` `nav` keys)
- **C** — List page (`pages/EmbedDatasets.tsx`)
- **D** — Detail page with tabs + polling
  (`pages/EmbedDatasetDetail.tsx`)
- **E** — Create form + bulk-import modal (lives inside
  `EmbedDatasets.tsx` as a modal/section, mirror how
  `Templates.tsx` and `Endpoints.tsx` do creates)
- **F** — Page-specific i18n (`embedDatasets.*` namespace
  in both `en.ts` and `ja.ts`, plus the new
  `embed_dataset:` permission-group label)

Suggested order: **A → F → B → C → D → E**. Build the
type-and-string foundation first; D and E are the largest
visual pieces and benefit from the type/i18n layer being
already-typechecked.

If anything below contradicts
[`docs/design/16-embedding-datasets.md`](docs/design/16-embedding-datasets.md)
§"WebUI", the design wins — flag the contradiction in your
structured output.

If you hit scope limits, finish whichever surface is closest
to done and report what's left in the structured output.
**Do not** silently abandon a half-done component, and do
**not** ship a UI that compiles but renders broken pages.

## Friendly-UI mandate (load-bearing per HUMANS.md)

Source items get a **structured table editor**, not a JSON
textarea. Per the HUMANS.md erratum:

> "No raw JSON editor for Embedding DB: please add a friendly
> UI."

This is the whole reason D6 exists as a dedicated UI rather
than reusing the generic JSON editor. The structured editor
shape is: one row per source item, columns `id` / `text` /
`payload`, with `payload` as a structured key/value table for
flat payloads + an "Edit as JSON" expand option for nested
payloads (using the existing `CodeEditor.tsx` component).
Add / remove / reorder rows. Per-row validation surfacing the
failing item index.

**Bulk import** is a separate affordance: an "Import" modal
that accepts pasted CSV or JSON (auto-detect format), runs
client-side validation, and populates the structured table.
After import, the user edits via the structured table; there
is no persistent raw-JSON view of the dataset itself.

## Deliverable A — Types & API client (`api/client.ts`)

Add typed entries mirroring the existing
`TemplateSummary`/`EndpointSummary` shape style:

```ts
export type EmbeddingDatasetStatus =
  | "created"
  | "embedding"
  | "ready"
  | "failed";

export interface EmbeddingDatasetSummary {
  dataset_id: string;
  display_name: string;
  status: EmbeddingDatasetStatus;
  item_count: number;
  embed_endpoint_id: string;
  created_at: UnixMillis;
  updated_at: UnixMillis;
  is_retired: boolean;
}

export type EmbeddingDatasetDetail = EmbeddingDatasetSummary;

export interface SourceItem {
  id: string;
  text: string;
  payload?: JsonValue;
}

export interface CorpusItem {
  id: string;
  vector: number[];
  payload?: JsonValue;
}

export interface CreateEmbeddingDatasetRequest {
  display_name: string;
  embed_endpoint_id: string;
  items: SourceItem[];
}

export interface CreateEmbeddingDatasetResponse {
  dataset_id: string;
}

export interface UpdateEmbeddingDatasetRequest {
  items: SourceItem[];
}

export interface RetireEmbeddingDatasetResponse {
  dataset_id: string;
  is_retired: boolean;
}
```

Status discriminant convention: the API returns
`status: i64` (0/1/2/3 per `EmbeddingDatasetStatus` in
`philharmonic-policy`); inspect the existing
`InstanceStatus` mapping in `client.ts` (or wherever it
lives) for the exact serde-rendered shape — the API
serialises the i64 either as a number or a snake_case
string. **Read the API code first** to confirm the wire
shape; if it's a number, the WebUI converts to the union
type at the client-layer boundary. If it's a snake_case
string, no conversion needed.

Fetch functions (mirroring the existing fetch-helper
pattern in `client.ts`):

- `listEmbeddingDatasets(cursor?: string)` → returns
  `PaginatedResponse<EmbeddingDatasetSummary>`.
- `readEmbeddingDataset(id: string)` →
  `EmbeddingDatasetDetail`.
- `createEmbeddingDataset(req: CreateEmbeddingDatasetRequest)`
  → `CreateEmbeddingDatasetResponse`.
- `updateEmbeddingDataset(id: string, req:
  UpdateEmbeddingDatasetRequest)` → updated detail (or
  `RetireEmbeddingDatasetResponse`-like envelope —
  inspect the route's actual return shape).
- `retireEmbeddingDataset(id: string)` →
  `RetireEmbeddingDatasetResponse`.
- `readEmbeddingDatasetSourceItems(id: string)` →
  `SourceItem[]`.
- `readEmbeddingDatasetCorpus(id: string)` →
  `CorpusItem[]`. **404 means no corpus yet** —
  surface as a typed `null` return or a typed error the
  detail page can branch on. (See how the existing
  client.ts handles 404s for `read*` functions; match the
  pattern.)

The `update*` and `create*` functions need to handle the
new **409 Conflict** response shape from
`update_dataset` (round 03 added `ApiError::Conflict` for
the `status=Embedding` rejection). Surface this as a
typed error the UI can show as "dataset is currently
embedding — try again after it finishes" rather than a
generic 4xx.

## Deliverable B — Routes + sidebar nav

1. **`App.tsx`**: add two routes:

   ```tsx
   <Route path="/embed-datasets" element={<EmbedDatasets />} />
   <Route path="/embed-datasets/:id" element={<EmbedDatasetDetail />} />
   ```

   Wrap in `<ProtectedRoute>` per the existing pattern. Place
   in route declaration order between `/authorities` and
   `/audit` (matches design 16's sidebar placement).

2. **`components/Layout.tsx`** sidebar: insert "Embedding
   Datasets" entry between "Authorities" and "Audit" with
   the i18n key `nav.embedDatasets`. Match the existing
   nav-item shape (icon, link, label).

3. **`i18n/en.ts` + `i18n/ja.ts`** — add `nav.embedDatasets`:
   - en: `"Embedding Datasets"`
   - ja: `"埋め込みデータセット"` (or whatever Yuka would prefer
     — the JP nav uses katakana for some technical terms;
     pick the form that matches the surrounding nav entries'
     style).

## Deliverable C — List page (`pages/EmbedDatasets.tsx`)

Per design 16 §"List page":

- **Table columns**: ID, Display Name, Status (badge), Item
  Count, Updated.
- **Status badges** (use existing badge styling from other
  pages):
  - `Created` → info badge
  - `Embedding` → warning badge with spinner / "in progress"
    text
  - `Ready` → good/success badge
  - `Failed` → bad/error badge
- **Cursor pagination** via the existing `Pagination.tsx`
  component.
- **Filters**: include retired by default off; add a toggle
  to show retired (mirror `Endpoints.tsx`'s pattern if it
  has one — otherwise add a simple checkbox).
- **"Create dataset" button** opens the create form
  (deliverable E lives here as a modal or inline section,
  matching whichever pattern `Templates.tsx` uses).

The retired-toggle question: design 16 §"Retired datasets are
excluded from all queries and are not served to workflows" —
admins still need a way to view retired datasets to confirm
retirement happened. The toggle is admin-side; the API call
is the same `GET /v1/embed-datasets`, the UI filters
`is_retired` client-side from the returned page (or, if the
list endpoint supports a query param, send that — inspect
the route).

## Deliverable D — Detail page (`pages/EmbedDatasetDetail.tsx`)

Per design 16 §"Detail page":

- **Metadata grid** at the top: ID, Display Name, Status
  (badge), Item Count, Embed Endpoint (display name +
  link to endpoint detail), Created, Updated, Retired flag.
- **Three tabs**:
  - **Source Items**: shows the structured table from
    deliverable E in **read-only mode** when status is
    `Created` or `Embedding`; **editable mode** when
    `Ready` or `Failed`. Editable mode triggers an
    `updateEmbeddingDataset` call on save (which will 409
    if the status flipped to `Embedding` between read and
    save — handle that case with a clear "dataset moved to
    embedding state — please refresh" toast).
    - Backed by `readEmbeddingDatasetSourceItems`.
  - **Corpus**: a separate table view, one row per item.
    Columns: `id`, `payload` (collapsed/expandable JSON via
    `JsonViewer.tsx` or similar), and a **"vector"
    expand-on-click** showing dimensionality + the first
    few components. The full f32 array stays
    **collapsed by default** because rendering a 1024-dim
    float array per row crashes the page. Backed by
    `readEmbeddingDatasetCorpus`. If the call returns 404
    (no corpus yet), show a clear empty state explaining
    "Corpus not available yet — embedding job in progress
    or first embed has not completed."
- **Polling refresh button**: per design 16 §"Detail page",
  available for observing `status` transitions during
  embedding. Inspect `InstanceDetail.tsx` for the existing
  polling pattern; mirror it. The detail page polls every
  ~5 seconds while status is `Embedding`; when status
  leaves `Embedding`, polling stops automatically and the
  user is shown a toast (`"Dataset is now Ready"` /
  `"Dataset failed"`).
- **Clear UI states** for the four design-16-listed
  scenarios:
  1. "First embed in progress" (status=Embedding, no
     fallback corpus visible — corpus tab shows the empty
     state).
  2. "Re-embed in progress with previous corpus served"
     (status=Embedding, corpus tab shows the previous
     corpus per the carry-forward rule).
  3. "Failed with previous corpus served" (status=Failed,
     corpus tab shows previous corpus, plus a clear
     "previous embed failed — items are still searchable
     against the prior corpus" notice).
  4. "Failed without fallback corpus" (status=Failed,
     corpus tab shows the empty state plus the failure
     notice).
- **"Retire" button** at the bottom of the metadata grid.
  Standard confirm-modal-then-call pattern (mirror
  `EndpointDetail.tsx`'s retire flow).

## Deliverable E — Create form + bulk-import modal

Per design 16 §"Create form" + the friendly-UI mandate:

**Create form** (lives in `EmbedDatasets.tsx` as a modal
or inline section, mirroring whichever pattern
`Endpoints.tsx` uses):

- **Display Name**: text input.
- **Embed Endpoint**: dropdown of active endpoints whose
  `implementation` is `embed`. The list comes from a
  `listEndpoints({ implementation: "embed",
  is_retired: false })` filtered call. If the endpoint
  list supports filtering by implementation server-side,
  use it; otherwise filter client-side.
- **Items**: structured table editor, one row per item.
  Columns:
  - `id` — text input. Unique within the dataset
    (client-side check: highlight collisions).
  - `text` — multiline textarea.
  - `payload` — structured key/value editor for simple
    flat object payloads, with an "Edit as JSON" toggle
    that swaps the row's payload cell to the existing
    `CodeEditor.tsx` (CodeMirror 6 with JSON syntax).
    Defaults to empty (omits the field entirely from the
    submitted JSON, matching the storage-layer omit rule).
  - **Add row** button at the bottom.
  - **Delete row** + **Move up/down** controls per row.
  - **Per-row validation**: failing rows highlighted with
    the index + which field failed.
- **Server-side limits surfaced inline**: per design 16
  §"Detail page" final paragraph, the editor shows the
  caps (10,000 items / 64 KiB text / 64 KiB payload / 256
  MiB total source-items blob) inline in the editor so the
  user sees the caps **before** submission, not after the
  API rejects. Hardcode them on the WebUI side for round
  D6 (round-03 hardcoded them as `pub const`s in the API;
  fetching them from a `/v1/_meta/limits` endpoint is
  out of D6 scope — flag in residual risks if you want to
  raise it).
- **Submit** calls `createEmbeddingDataset`. On success:
  navigate to the detail page for the new dataset. On 4xx:
  surface the API's error envelope message in a clear
  toast.

**Import modal** (separate trigger button, e.g. "Import
items from CSV/JSON" near the "Add row" button):

- **Pasted text area** that accepts:
  - CSV with header row `id,text,payload` (payload column
    optional; values are JSON-encoded if non-empty).
  - JSON: either a top-level array of `{id, text,
    payload?}` objects, or NDJSON (one object per line).
  - **Auto-detect**: try JSON parse first; if that fails,
    parse as CSV.
- **Validate** client-side before populating the table:
  every row needs an `id` and `text`; `payload` (if
  present) must be valid JSON.
- **Populate** the structured table with the parsed rows
  on success. The user then edits via the table; **no
  persistent raw-JSON view** of the dataset itself.
- **Error display**: invalid CSV → row+col indication;
  invalid JSON → parse error with line+col; cap violation
  (>10k rows) → clear "too many rows — embedding datasets
  are limited to 10,000 items" message.

## Deliverable F — Page-specific i18n

Add an `embedDatasets` section to both `i18n/en.ts` and
`i18n/ja.ts` covering every visible string from
deliverables C / D / E. Group by sub-feature:

```ts
embedDatasets: {
  title: "Embedding Datasets",
  list: {
    create: "Create dataset",
    columns: { ... },
    statusBadges: { ... },
    showRetired: "Show retired",
    emptyState: "No embedding datasets yet.",
  },
  detail: {
    tabs: {
      sourceItems: "Source Items",
      corpus: "Corpus",
    },
    polling: { ... },
    states: { ... },
    retire: "Retire dataset",
    retireConfirm: "Retire this dataset? Workflows will stop seeing it.",
  },
  create: { ... },
  import: { ... },
  errors: {
    embeddingInProgress: "Dataset is currently embedding — try again after it finishes.",
    capsExceeded: { ... },
  },
}
```

Use the existing en/ja translation file structure for the
exact namespacing and key style. JP translations should
match the surrounding nav/page style; if Yuka has a
preferred terminology pattern in adjacent sections, mirror
it. Don't invent new katakana spellings or kanji where
existing pages have established forms.

Also add a **permission-group label** for the new
`embed_dataset:` group. The WebUI's
`components/permissions.ts` groups by `<prefix>:` already,
so the group falls out automatically — but the group **name**
needs an i18n entry (e.g. en: "Embedding Datasets", ja:
"埋め込みデータセット"). Add it under whichever key the
existing groups use (probably
`permissions.groups.embedDataset` or similar — inspect
`PermissionChecklist.tsx` for the lookup pattern).

## Cross-deliverable: build, verification, artifacts

After all six deliverables are in place:

1. **TypeScript typecheck first** (catches errors fast):

   ```sh
   cd philharmonic/webui
   npx tsc --noEmit
   ```

   Fix any errors before invoking webpack.

2. **Production build**:

   ```sh
   ./scripts/webui-build.sh --production
   ```

   This regenerates the four committed artifacts in
   `philharmonic/webui/dist/`. The Rust API server bin
   embeds these at compile time.

3. **Workspace `pre-landing.sh`**: needed because the API
   server bin compiles the new bundle into the binary; a
   broken bundle breaks `cargo build`.

4. **Run the existing WebUI tests** if any (check
   `philharmonic/webui/package.json` for a `test` script;
   may not exist).

The committed artifacts (post-build) are part of the
diff — `dist/main.js`, `dist/main.css`, `dist/index.html`,
`dist/icon.svg` change with every functional UI change.
That's expected — webpack's deterministic-output config (per
`scripts/webui-build.sh`'s comments) makes the bundle
content-hash-stable.

## Cross-deliverable: no version bump on the WebUI

The WebUI lives in `philharmonic/webui` (a sub-tree of the
`philharmonic` meta-crate's submodule, NOT a published
crate). No `Cargo.toml` version bump on the WebUI itself.
The `philharmonic` meta-crate's CHANGELOG (if any) is
where the user-facing entry goes — inspect the existing
WebUI-only commit pattern to see what the convention is.

If the package.json has a meaningful version (currently
`0.0.0`), leave it alone — it's an unpublished package.

<structured_output_contract>
**Critical: emit this report before `task_complete`.**

Six sections, in this order:

1. **Summary** — one paragraph: what landed, which
   surfaces (A/B/C/D/E/F) are complete vs partial vs not
   started. Include the verbatim string "RUN STATUS:
   COMPLETE" or "RUN STATUS: PARTIAL — <reason>" for grep.

2. **Touched files** — exhaustive list with `(new|edited|deleted)
   <path> — <one-line note>`. Include the four
   regenerated artifacts in `philharmonic/webui/dist/` (they
   change with every UI edit; verify they are in the diff).

3. **Verification results** — exact commands + outcomes:
   - `npx tsc --noEmit` (in `philharmonic/webui/`) —
     pass/fail/output excerpt.
   - `./scripts/webui-build.sh --production` —
     pass/fail/exit code.
   - `./scripts/pre-landing.sh` — pass/fail/exit code.
   - `./scripts/test-scripts.sh` — pass/fail.

4. **Residual risks / known issues** — including:
   - Any `any` types you had to use because the API client
     types were unclear, and where.
   - Any UI states from design 16 §"Detail page" you
     couldn't fully implement (e.g. couldn't find the
     existing toast pattern, polling pattern was unclear,
     etc.).
   - Whether the JP translations are placeholders that
     Yuka should review, or you matched established style.
   - Bundle-size delta from the new pages (in KB, gzipped).
   - Any cap-display scenarios where the inline limit
     display doesn't match the API's actual rejection
     boundary.

5. **Git state** — current `HEAD` SHAs in
   `philharmonic/webui` (or wherever the WebUI submodule's
   git lives — `philharmonic` is the parent submodule).
   Confirm no commits made.

6. **Open questions** — questions for Yuka or Claude:
   - JP translation quality / terminology choices.
   - Whether to add a `/v1/_meta/limits` endpoint (round-04
     candidate) so the WebUI fetches caps server-side
     instead of hardcoding.
   - Whether the Source Items tab's read-only-when-
     Embedding rule should be relaxed (queue updates during
     embedding) — design-16 v1 limitation.
</structured_output_contract>

<default_follow_through_policy>
- Suggested order: A → F → B → C → D → E. Types + i18n
  first; D and E are the largest, save for last.
- Run `npx tsc --noEmit` after each surface, before moving
  on. WebUI debug loops are slow if you let webpack flush
  the type errors at build time.
- Add JSDoc on all new exported types and functions in
  `api/client.ts` for downstream consumers.
- If a deliverable's component references a type that's
  not yet defined in `client.ts`, define the type first
  (in deliverable A) — don't inline interface definitions
  in pages.
- If you discover that an API route's actual response shape
  differs from this prompt's spec (e.g. the status field is
  a number rather than a string), trust the API route —
  flag the discrepancy and adapt.
</default_follow_through_policy>

<completeness_contract>
"Done" means:

- All six surfaces (A/B/C/D/E/F) functionally complete.
- TypeScript typecheck clean.
- `./scripts/webui-build.sh --production` clean.
- `./scripts/pre-landing.sh` clean.
- The four artifacts in `philharmonic/webui/dist/`
  regenerated.
- Structured output report emitted before `task_complete`.

Partial completion is acceptable if you hit a token limit
or genuine blocker — but you must say so explicitly with
"RUN STATUS: PARTIAL — <reason>". Half-built UI surfaces
are worse than missing surfaces because they crash on user
interaction; if you can't finish a surface end-to-end,
either revert it or guard it behind a "not yet
implemented" placeholder so the rest of the UI still
works.

A run without the structured-output report is
**incomplete**, even if all six surfaces landed.
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
If you find yourself needing information not in this prompt
or the cited authoritative docs, **stop** and report what's
missing in the structured output's "Open questions" section.

Specifically: do **not**:

- Touch any Rust crate (this is WebUI-only scope).
- Add a new npm dependency without explicit reason — the
  existing CodeMirror 6 + react-router + redux-toolkit
  stack should cover everything D6 needs. If you genuinely
  need something new (e.g. a CSV parser), surface in
  residual risks before adding.
- Edit `webpack.config.js` or `tsconfig.json` —
  configuration is settled.
- Mint new permission atoms (round 01 settled the four).
- Edit `philharmonic-api`, `philharmonic-policy`,
  `philharmonic-workflow`, or any backend crate.
- Change the `embed_dataset:` permission-group prefix.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `philharmonic/webui/src/api/client.ts` (edited — types +
  fetch fns).
- `philharmonic/webui/src/App.tsx` (edited — routes).
- `philharmonic/webui/src/components/Layout.tsx` (edited —
  sidebar nav).
- `philharmonic/webui/src/i18n/en.ts` (edited).
- `philharmonic/webui/src/i18n/ja.ts` (edited).
- `philharmonic/webui/src/i18n/index.ts` (edited only if
  the existing pattern requires per-namespace registration).
- `philharmonic/webui/src/pages/EmbedDatasets.tsx` (new).
- `philharmonic/webui/src/pages/EmbedDatasetDetail.tsx`
  (new).
- `philharmonic/webui/src/components/<helpers>.tsx` (new
  if you need to extract a shared structured-table-editor
  component for reuse between Create and Detail-edit modes;
  prefer keeping it inline first, extract only if
  duplication crosses 100 LOC).
- `philharmonic/webui/src/app.css` (edited if new styles
  needed; reuse existing classes first).
- `philharmonic/webui/dist/index.html`,
  `dist/main.js`, `dist/main.css`, `dist/icon.svg`
  (regenerated by `webui-build.sh --production` — these
  WILL change with every UI edit; commit them alongside the
  source).

Files NOT to touch (flag if you find a reason to):

- Any file under `bins/`, `philharmonic-api/`,
  `philharmonic-policy/`, `philharmonic-workflow/`,
  `philharmonic-store*/`, `mechanics-*/`, or any
  connector crate.
- The workspace `Cargo.toml` `[patch.crates-io]` block.
- `philharmonic/webui/webpack.config.js`,
  `philharmonic/webui/tsconfig.json`,
  `philharmonic/webui/package.json` (unless an explicit
  new dependency is needed — surface first).
- `philharmonic/webui/src/components/CodeEditor.tsx`
  (D10's CodeMirror wrapper — use as-is, don't rewrite).
- `philharmonic/webui/src/components/permissions.ts`
  (the `<prefix>:` group split picks up `embed_dataset:*`
  automatically since round 01).
- Any `.claude/`, `docs/`, or `scripts/` content.

Do **not** run `git add`, `git commit`, `git push`,
`commit-all.sh`, `push-all.sh`, or `cargo publish`. Codex
does not commit on this workspace.
</action_safety>

## Git rules (workspace-specific, mandatory)

- **Never** run `git commit` / `git push` / `git add`.
- **Never** invoke `scripts/commit-all.sh` or
  `scripts/push-all.sh`.
- **Never** run `cargo publish` (the WebUI isn't published
  but the rule applies generally).
- All cargo commands must use `CARGO_TARGET_DIR=target-main`.
- Don't `--no-verify` around any hooks.

Read-only git is fine: `git status`, `git diff`, `git log`,
`git show`, `git branch`, `git submodule status`.

## Verification commands (mandatory before declaring done)

1. `cd philharmonic/webui && npx tsc --noEmit` — TypeScript
   typecheck.
2. `./scripts/webui-build.sh --production` — production
   build.
3. `./scripts/pre-landing.sh` — full workspace pass
   (catches Rust-side bin embedding regressions).
4. `./scripts/test-scripts.sh` — POSIX shell-script syntax
   check (no scripts touched here, no-op pass).

</task>
