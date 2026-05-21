# WebUI: reusable `Modal`/`ConfirmModal` components, eliminate `window.confirm()`, add duplicate-template button, unsaved-changes guard

**Date:** 2026-05-21 (JST)
**Slug:** `webui-modal-component-and-duplicate-template-button`
**Round:** 01 — initial dispatch. Four coupled WebUI changes
bundled per Yuka's "also" directives this morning:
(1) build a small reusable `Modal` component (no current
reusable infrastructure — only `EmbedDatasets`' inline
`ImportItemsModal` uses the existing CSS); (2) build a
`ConfirmModal` on top of it and eliminate all 8 in-tree
`window.confirm()` calls; (3) add a "Duplicate" button to
the workflow templates **list view** and **detail view**
that asks for a new name and POSTs a clone via the existing
`POST /workflows/templates` endpoint; (4) guard
detail-page edit forms with an "unsaved changes?" prompt on
in-app navigation and browser close/refresh. Single
submodule (`philharmonic/webui/`) plus the regenerated
committed `webui/dist/` artifacts.
**Subagent:** `codex:rescue`

## Motivation

Yuka 2026-05-21 morning, in order:

1. **Duplicate button.** Workflow template authoring is
   iterative — operators currently can only retire and
   recreate from scratch, or copy the script source into a
   new Create form by hand. "Add a duplicate button … that
   asks for a new name and duplicates the template." The
   API surface already supports the clone-via-create flow:
   `GET /workflows/templates/{id}` returns full
   `script_source` + `abstract_config` + `data_config`, and
   `POST /workflows/templates` accepts the same shape. No
   server change.

2. **Reusable Modal component.** Yuka follow-up: "Please
   also migrate existing modals to the new convention." The
   audit found exactly one inline modal in the WebUI today,
   `ImportItemsModal` at
   [`philharmonic/webui/src/pages/EmbedDatasets.tsx:302-350`](../../philharmonic/webui/src/pages/EmbedDatasets.tsx#L302-L350),
   which uses the existing `.modal-backdrop` / `.modal-panel`
   CSS classes at
   [`philharmonic/webui/src/app.css:1041-1066`](../../philharmonic/webui/src/app.css#L1041-L1066)
   but is implemented ad-hoc. Extract a shared `Modal`
   component, build the new duplicate dialog on top of it,
   and migrate `ImportItemsModal` to use it. The shared
   component is required for (3) and (4) below to land
   without two new ad-hoc copies of the same JSX.

3. **Eliminate `window.confirm()`.** Yuka follow-up:
   "`window.confirm()` should also be eliminated." The
   browser dialog is platform-styled, unstyleable, and
   blocks the JS event loop. The audit found 8 call sites
   across 8 files; all are simple retire / bump-epoch
   confirmations. Replace each with a `ConfirmModal`
   (built on `Modal`) so the wording is consistent with
   the rest of the app and the styling is ours.

4. **Unsaved-changes guard.** Yuka follow-up: "Also, ask
   the user when closing/leaving a page with unsaved
   changes." Scope (per the in-session question Yuka
   answered): the four detail pages with edit forms —
   `TemplateDetail` (PATCH), `RoleDetail` (PATCH),
   `EndpointDetail` (rotate POST), `TenantSettings`
   (PATCH). Triggers: in-app navigation (a custom
   `ConfirmModal` via React Router's `useBlocker`) and
   browser close/refresh/back (the native `beforeunload`
   prompt — browser-controlled wording is the only
   cross-browser path).

## References (authoritative if anything in this prompt contradicts them)

1. [`CONTRIBUTING.md`](../../CONTRIBUTING.md):
   - §4 Git workflow — `./scripts/commit-all.sh` only;
     **Codex does not commit** (see Hand-off shape below).
   - §6 Shell scripts are POSIX `#!/bin/sh`. Not directly
     relevant here (no shell edits) but `webui-build.sh`
     stays as-is.
   - §7 External tool ban (no `python` / `perl` / `ruby` /
     `node` / `jq` / `curl` / `wget` in workspace tooling).
     `webui-build.sh` is the **explicit exception** that
     invokes Node.js via `npx webpack`. Do not introduce
     any other Node-using script.
   - §11 Pre-landing checks — `./scripts/pre-landing.sh`
     is mandatory before declaring done. This dispatch
     touches only TypeScript + the regenerated
     `webui/dist/` artifacts. Pre-landing's Rust phases
     still run; they must pass (no Rust changes expected
     so they should be a no-op except cargo's dep graph
     timestamp).
   - §14.6 English as the default for prose (i18n is the
     artefact exception — Japanese strings live in
     `i18n/ja.ts`).
2. [`docs/design/`](../design/) — no design-doc rewrite
   needed; this is a UI-layer change with no architectural
   surface.
3. [`docs/codex-reports/README.md`](../codex-reports/README.md)
   — write a short report if anything non-obvious surfaces.
4. React Router v7 docs for
   [`useBlocker`](https://reactrouter.com/en/main/hooks/use-blocker)
   — the package is already at `^7.14.2`
   ([`philharmonic/webui/package.json`](../../philharmonic/webui/package.json)),
   so `useBlocker` is available without bumping.

## Context files pointed at

**The new components (NEW files):**

- `philharmonic/webui/src/components/Modal.tsx` — the shared
  modal wrapper. Props:
  ```ts
  interface ModalProps {
    ariaLabel: string;
    onClose: () => void;
    children: ReactNode;
    // Optional: prevent backdrop click from closing.
    // Default true (clicking backdrop closes).
    closeOnBackdropClick?: boolean;
  }
  ```
  Renders the existing `.modal-backdrop` / `.modal-panel`
  structure. Adds:
  - **Esc-to-close**: `keydown` listener on `document` while
    mounted; calls `onClose` on `Escape`.
  - **Body scroll lock** while open: set
    `document.body.style.overflow = "hidden"` on mount,
    restore on unmount. Concurrent modals are not expected
    (the app opens one at a time), but use a small
    ref-counter so nested modals don't unlock prematurely
    — store the count on `document.body.dataset.modalCount`
    or a module-level counter.
  - **Focus management**: on mount, focus the panel
    (`tabIndex={-1}` on the `<section>` + `ref.current.focus()`
    in an effect). On unmount, restore focus to the
    previously-focused element (capture
    `document.activeElement` in the same effect).
  - **Backdrop click**: if `closeOnBackdropClick !== false`,
    a click on the backdrop (not on the panel) calls
    `onClose`. Use the standard
    `event.target === event.currentTarget` check on the
    backdrop's `onClick`.
  - **No portal**: render inline. The existing
    `ImportItemsModal` renders inline and it works — the CSS
    uses `position: fixed; z-index: 20`.

  Do NOT add: animations, header content (caller composes
  via `children`), close button (caller composes), or any
  Tailwind / styled-components dep. Pure CSS + className.

- `philharmonic/webui/src/components/ConfirmModal.tsx` — a
  binary yes/no confirmation built on `Modal`. Props:
  ```ts
  interface ConfirmModalProps {
    title: string;
    body: string;                // single-paragraph body
    confirmLabel?: string;       // default: t.common.confirm
    cancelLabel?: string;        // default: t.common.cancel
    danger?: boolean;            // default false; true
                                 //   styles confirm as "danger"
                                 //   (matches retire flow)
    onConfirm: () => void;
    onCancel: () => void;
  }
  ```
  Renders `<Modal>` with a `<header>` (h2 = title), a `<p>`
  with body text, and an `.actions` div with Cancel +
  Confirm buttons. Confirm button uses `button.primary`
  (or `button.danger` when `danger === true`); Cancel uses
  `button.secondary`. Enter on the Confirm button submits;
  Esc / Cancel / backdrop calls `onCancel`. Read the labels
  from props with `useT()` fallbacks.

- `philharmonic/webui/src/components/DuplicateTemplateModal.tsx`
  — the name-input dialog. Props:
  ```ts
  interface DuplicateTemplateModalProps {
    templateId: string;          // source template UUID
    sourceDisplayName: string | null;
    onClose: () => void;
    onDuplicated: (newTemplateId: string) => void;
  }
  ```
  Renders `<Modal>` with a `<header>` (h2 =
  `t.templates.duplicateTitle`), a `<form>` with a single
  text input pre-filled with `<sourceDisplayName ?? "">`
  + `t.templates.copySuffix` (e.g. `" (copy)"`); a
  Cancel + Duplicate button. On submit:
  1. `setIsSaving(true)` + clear error.
  2. `GET /workflows/templates/${templateId}` →
     `TemplateDetail` (use `apiCall` from
     `../api/client`).
  3. POST a new template via `apiCall("workflows/templates",
     { method: "POST", body: JSON.stringify(request) })`
     with `request = { display_name: trimmed,
     script_source: source.script_source, abstract_config:
     source.abstract_config, data_config:
     hasDataBindings(source.data_config) ?
       { embed_datasets: source.data_config.embed_datasets }
       : undefined }`. (`data_config` is `null` when the
     server omitted it; copy it across only when there are
     bindings, mirroring the existing `CreateTemplateForm`
     pattern at
     [`Templates.tsx:288-290`](../../philharmonic/webui/src/pages/Templates.tsx#L288-L290).)
  4. On 200: `onDuplicated(response.template_id)`. The
     parent decides what to do (list refreshes; detail
     view navigates).
  5. On error: set local error state, keep modal open,
     show the error inside the modal.
  - Permission gating: the action requires
    `workflow:template_create`. Disable the Duplicate
    button + add a `title=` tip when missing (mirror the
    existing pattern at
    [`Templates.tsx:31-35`](../../philharmonic/webui/src/pages/Templates.tsx#L31-L35)).
  - Retired source templates ARE duplicable (Yuka's call).
    Don't gate by `is_retired`.

**The new hook (NEW file):**

- `philharmonic/webui/src/hooks/useUnsavedChanges.ts` —
  ```ts
  export function useUnsavedChanges(isDirty: boolean, message: string): {
    // Blocker state from React Router; the caller uses
    // it to render a ConfirmModal when in-app nav is
    // intercepted.
    blocker: Blocker;
  }
  ```
  Implementation:
  1. `useBlocker((tx) => isDirty)` from `react-router-dom`
     — intercepts in-app nav when dirty.
  2. `useEffect` registers a `beforeunload` listener that
     sets `event.preventDefault()` and `event.returnValue
     = message` (legacy) when `isDirty`. Cleanup on
     unmount and on `isDirty` flipping false.
  3. Return `{ blocker }` so the caller can render
     ```tsx
     <ConfirmModal
       title={t.common.discardChanges}
       body={t.common.unsavedChangesPrompt}
       confirmLabel={t.common.discardButton}
       cancelLabel={t.common.cancel}
       danger
       onConfirm={() => blocker.proceed?.()}
       onCancel={() => blocker.reset?.()}
     />
     ```
     when `blocker.state === "blocked"`.
  - Browser dialog wording is browser-controlled — the
    `message` parameter is best-effort (modern browsers
    ignore it and show their own copy), but pass it
    anyway for legacy support.

**Existing files to edit:**

- `philharmonic/webui/src/pages/Templates.tsx` — add per-row
  "Duplicate" button:
  - Header actions stay as-is. Per-row, add a second
    button next to the existing `t.chat.testInChat`
    button at
    [`Templates.tsx:175-184`](../../philharmonic/webui/src/pages/Templates.tsx#L175-L184).
    Label: `t.templates.duplicate`. Class:
    `button secondary compact-button`. Disable +
    `title=` when `workflow:template_create` is missing.
    Show for both active and retired templates.
  - Track `duplicateSourceTemplate: { id: string,
    displayName: string | null } | null`. When the button
    is clicked, set it. When `null`, the modal is not
    rendered.
  - On `onDuplicated`: clear the source, reset cursor /
    history, bump `refreshKey` so the list reloads
    (mirror the existing `onCreated` pattern at
    [`Templates.tsx:133-138`](../../philharmonic/webui/src/pages/Templates.tsx#L133-L138)).
  - On `onClose`: just clear the source state.

- `philharmonic/webui/src/pages/TemplateDetail.tsx`:
  - Add a "Duplicate" button to the header actions row at
    [`TemplateDetail.tsx:227-268`](../../philharmonic/webui/src/pages/TemplateDetail.tsx#L227-L268).
    Place it **before** the danger "Retire" button.
    Label: `t.templates.duplicate`. Class:
    `button secondary`. Disable + `title=` when
    `workflow:template_create` is missing. Show for both
    active and retired.
  - Track `showDuplicate: boolean`. On click: set true.
  - On `onDuplicated(newTemplateId)`: `navigate(\`/templates/${newTemplateId}\`)`.
  - On `onClose`: set false.
  - **Replace `window.confirm()` at line 179** with
    `ConfirmModal`. Track `showRetireConfirm` state;
    `retire()` becomes the on-confirm callback (the
    existing async fn body, without the prompt check).
    Title: `t.templates.confirmRetireTitle`. Body:
    `t.templates.confirmRetire` (the existing string).
    `danger: true`. confirmLabel `t.common.retire`.
  - **Apply `useUnsavedChanges`**. Track `isDirty` by
    comparing the four edit-form values
    (`displayName`, `scriptSource`,
    JSON-stringified `abstractConfig`, JSON-stringified
    `dataConfig`) against the values last hydrated from
    the server (the values set in the `load()` effect at
    [`TemplateDetail.tsx:122-127`](../../philharmonic/webui/src/pages/TemplateDetail.tsx#L122-L127)).
    Stash a `lastSavedSnapshot` ref (or state) right
    after the load and after a successful `update()`.
    `isDirty` is `true` when any of the four differs.
    Render the blocker `ConfirmModal` when
    `blocker.state === "blocked"`.

- `philharmonic/webui/src/pages/RoleDetail.tsx`:
  - Replace `window.confirm()` at line 69 with a
    `ConfirmModal` (same shape as TemplateDetail). Title:
    `t.roles.confirmRetireTitle`. Body:
    `t.roles.confirmRetire`. `danger: true`.
  - Apply `useUnsavedChanges`. RoleDetail's edit form
    landed yesterday
    (`2026-05-20-0002-entity-list-revision-seq-display-01.md`).
    Track `displayName` + permissions selection dirty.

- `philharmonic/webui/src/pages/EmbedDatasetDetail.tsx`:
  - Replace `window.confirm()` at line 166 with
    `ConfirmModal`. Title:
    `t.embedDatasets.detail.retireConfirmTitle`. Body:
    `t.embedDatasets.detail.retireConfirm` (the existing
    string). `danger: true`.

- `philharmonic/webui/src/pages/AuthorityDetail.tsx`:
  - Replace **both** `window.confirm()` calls (lines 91
    and 112). Each gets its own `ConfirmModal`. For
    bumpEpoch: `t.authorities.confirmBumpEpochTitle` /
    `t.authorities.confirmBumpEpoch` (existing); `danger:
    false` (not destructive — it's a rotation). For
    retire: `t.authorities.confirmRetireTitle` /
    `t.authorities.confirmRetire`; `danger: true`.

- `philharmonic/webui/src/pages/Memberships.tsx`:
  - Replace `window.confirm()` at line 69. Title:
    `t.memberships.confirmRetireTitle`. Body:
    `t.memberships.confirmRetire` (existing). `danger: true`.

- `philharmonic/webui/src/pages/Principals.tsx`:
  - Replace `window.confirm()` at line 90. Title:
    `t.principals.confirmRetireTitle`. Body:
    `t.principals.confirmRetire` (existing). `danger: true`.

- `philharmonic/webui/src/pages/EndpointDetail.tsx`:
  - Replace `window.confirm()` at line 129. Title:
    `t.endpoints.confirmRetireTitle`. Body:
    `t.endpoints.confirmRetire` (existing). `danger: true`.
  - Apply `useUnsavedChanges` to the rotate form. The
    rotate form has `displayName` + a config JSON — track
    those.

- `philharmonic/webui/src/pages/TenantSettings.tsx`:
  - Apply `useUnsavedChanges` to whatever editable
    fields exist. (Codex: open the file and identify the
    form state. Default-name field, custom-domain field,
    whatever lives there — the snapshot/dirty comparison
    is the same pattern.)

- `philharmonic/webui/src/pages/EmbedDatasets.tsx`:
  - Migrate `ImportItemsModal` at lines 302-350 to use the
    new `<Modal>` component. Replace the `<div
    className="modal-backdrop">` + `<section
    className="modal-panel">` boilerplate with `<Modal
    ariaLabel={t.embedDatasets.import.title}
    onClose={onClose}>`. The header / form / actions
    structure inside stays the same. Keep the explicit
    Close button in the header (callers expect it).

**i18n keys to add** (both `i18n/en.ts` and `i18n/ja.ts`):

- `common.confirm` — "Confirm" / "確認"
- `common.discard` — "Discard" / "破棄"
- `common.discardChanges` — "Discard unsaved changes?" /
  "未保存の変更を破棄しますか？"
- `common.unsavedChangesPrompt` — "You have unsaved
  changes. Leave this page and discard them?" /
  "未保存の変更があります。このページを離れて破棄しますか？"
- `common.duplicate` — "Duplicate" / "複製"
- `common.duplicating` — "Duplicating" / "複製中"
- `templates.duplicate` — "Duplicate" / "複製"
- `templates.duplicateTitle` — "Duplicate template" /
  "テンプレートを複製"
- `templates.duplicateNamePlaceholder` — "Display name for
  the new template" / "新しいテンプレートの表示名"
- `templates.duplicated` — "Template duplicated." /
  "テンプレートを複製しました。"
- `templates.copySuffix` — `" (copy)"` / `"（コピー）"`
  — appended to the source display name to form the
  default value in the rename input.
- Per-entity `confirmRetireTitle` / `confirmBumpEpochTitle`
  strings (existing keys are the body text; add the title
  variants):
  - `templates.confirmRetireTitle` — "Retire template?" /
    "テンプレートを廃止しますか？"
  - `roles.confirmRetireTitle` — "Retire role?" /
    "ロールを廃止しますか？"
  - `embedDatasets.detail.retireConfirmTitle` — "Retire
    dataset?" / "データセットを廃止しますか？"
  - `authorities.confirmBumpEpochTitle` — "Bump authority
    epoch?" / "発行権限のエポックを更新しますか？"
  - `authorities.confirmRetireTitle` — "Retire authority?"
    / "発行権限を廃止しますか？"
  - `memberships.confirmRetireTitle` — "Retire role
    membership?" / "ロール所属を廃止しますか？"
  - `principals.confirmRetireTitle` — "Retire principal?"
    / "プリンシパルを廃止しますか？"
  - `endpoints.confirmRetireTitle` — "Retire endpoint?" /
    "エンドポイントを廃止しますか？"

If a title-vs-body distinction reads weird for one of the
existing strings (e.g. the existing body is already a
question that fits as a title), it's fine to reuse the
same key for both — but **default to adding the title
key** so future tweaks don't have to thread two consumers.

The Japanese translations above are starting suggestions
— Codex may adjust for fluency / consistency with the
rest of `i18n/ja.ts`. The English strings are locked.

## CSS

The existing `.modal-backdrop` / `.modal-panel` rules at
[`app.css:1041-1066`](../../philharmonic/webui/src/app.css#L1041-L1066)
are sufficient. **Do not add new modal-related CSS** unless
the focus outline on the panel needs visibility — in that
case add a single `.modal-panel:focus-visible { outline:
... }` rule. No animations, no Tailwind, no styled JSX.

## Shape (locked)

**Component hierarchy:**

```
<Modal>                       // generic wrapper + esc-close
  └─ <ConfirmModal>           // yes/no confirmation
  └─ <DuplicateTemplateModal> // template clone dialog
  └─ <ImportItemsModal>       // EmbedDatasets, migrated
```

**Duplicate flow:**

```
Templates.tsx (list view):
  row click "Duplicate" → setDuplicateSource(template)
  → renders <DuplicateTemplateModal templateId=... sourceDisplayName=... />
  → on success: clear source + setRefreshKey(+1)

TemplateDetail.tsx (detail view):
  header click "Duplicate" → setShowDuplicate(true)
  → renders <DuplicateTemplateModal templateId=id sourceDisplayName=template.display_name />
  → on success: navigate(`/templates/${newId}`)
```

**Default name pre-fill:**

```
`${sourceDisplayName ?? ""}${t.templates.copySuffix}`
```

For English, that's `"My template (copy)"` or `" (copy)"`
when the source has no display name. The user can edit or
clear the field before submitting. The submit button is
disabled while the trimmed name is empty.

**ConfirmModal wiring (replaces `window.confirm`):**

```
// Before:
async function retire() {
  if (!window.confirm(t.templates.confirmRetire)) return;
  // ... do the retire
}

// After:
const [showRetireConfirm, setShowRetireConfirm] = useState(false);
async function doRetire() {
  setShowRetireConfirm(false);
  // ... do the retire (no prompt check)
}
// ... button onClick={() => setShowRetireConfirm(true)}
// ... {showRetireConfirm && <ConfirmModal title=... body=... danger onConfirm={doRetire} onCancel={() => setShowRetireConfirm(false)} />}
```

**`useUnsavedChanges` wiring:**

```
const snapshot = useRef<Snapshot>(initialSnapshot);
const isDirty = computeDirty(currentValues, snapshot.current);
const { blocker } = useUnsavedChanges(isDirty, t.common.unsavedChangesPrompt);
// after successful save: snapshot.current = newSnapshot;
// JSX:
{blocker.state === "blocked" && (
  <ConfirmModal
    title={t.common.discardChanges}
    body={t.common.unsavedChangesPrompt}
    confirmLabel={t.common.discard}
    cancelLabel={t.common.cancel}
    danger
    onConfirm={() => blocker.proceed?.()}
    onCancel={() => blocker.reset?.()}
  />
)}
```

## Hard requirements

- **No new npm dependencies.** Everything is built on
  `react`, `react-router-dom` (already at v7), and the
  existing CSS classes. No `react-modal`, no `headlessui`,
  no `react-aria`. If you find yourself reaching for a
  dep, **STOP and report** — the WebUI keeps dep count
  low by policy.
- **Pre-existing TypeScript strictness must hold.** No
  `any`, no `as unknown as T` outside the
  established-pattern sites. Match the existing strict
  posture (look at how
  [`AbstractConfigEditor.tsx`](../../philharmonic/webui/src/components/AbstractConfigEditor.tsx)
  or
  [`DataConfigEditor.tsx`](../../philharmonic/webui/src/components/DataConfigEditor.tsx)
  type their props).
- **The committed `philharmonic/webui/dist/` artifacts must
  be regenerated.** They're consumed at compile time by
  the Rust binary that serves the WebUI. Run
  `./scripts/webui-build.sh` after the TypeScript edits;
  the script regenerates the four files and the source
  maps. Both the source and the dist artifacts are
  committed as part of this dispatch.
- **No raw `cargo` calls** — `./scripts/pre-landing.sh` is
  the only verification entry point. (No Rust change is
  expected, but pre-landing still runs the Rust phases as
  a safety net.)
- **POSIX-ish host.** No `bash`-only constructs. (Not
  expected to come up — no shell edits.)
- **Esc closes every modal.** Backdrop click closes by
  default. Both behaviours are part of the `Modal`
  contract.
- **Focus the modal on open; restore focus on close.**
  Capture `document.activeElement` at mount, restore on
  unmount.
- **No `window.confirm`, `window.prompt`, or `window.alert`
  in the final tree.** Grep at the end to confirm; the
  only acceptable remaining hit is in `Modal.tsx` if you
  reference the names in comments (don't).
- **No `dangerouslySetInnerHTML`.**
- **No business logic in `Modal` / `ConfirmModal`.** They
  are presentation only. The duplicate-template logic
  lives in `DuplicateTemplateModal`.
- **Permission gating** for the Duplicate button uses
  `workflow:template_create` (same atom that gates Create).
  Disable + `title=` when missing — mirror the existing
  pattern.
- **Retired source templates ARE duplicable.** Do not gate
  the Duplicate button on `is_retired`.
- **Unsaved-changes guard scope is exactly the four
  pages** Yuka approved: `TemplateDetail`, `RoleDetail`,
  `EndpointDetail`, `TenantSettings`. Do NOT add the
  guard to inline Create forms (`CreateTemplateForm`
  etc.) in this round.
- **`beforeunload` listeners must clean up on unmount and
  when `isDirty` flips false.** Leaked listeners block
  navigation across the app.
- **No `useEffect` that re-fires on every render** —
  every effect needs a stable dep array.

## i18n consistency

Add the new keys to BOTH `en.ts` and `ja.ts`. Both files
have a flat-then-nested structure (see existing
`common` / `templates` / `endpoints` blocks). Maintain
alphabetical / logical order within each block — match
neighbouring keys' position. Where this prompt's
suggested Japanese translation reads awkward, adjust for
naturalness (Codex has done several rounds of Japanese
WebUI strings — the voice should match the rest of
`ja.ts`).

The `ConfirmModal`'s default `confirmLabel` /
`cancelLabel` fall back to `t.common.confirm` /
`t.common.cancel` if the caller omits them. Both keys
must exist.

## Tests

The WebUI has no test suite today (verified by absence of
a `test/` directory or `vitest` / `jest` in
`package.json`). Do NOT add a test harness in this round
— that's out of scope. Verification is by:

1. `./scripts/webui-build.sh` succeeding (the build itself
   type-checks via `tsc` indirectly through Webpack's
   `ts-loader` config — verify by inspecting
   `webpack.config.js` / `tsconfig.json` if unsure).
2. `./scripts/pre-landing.sh` passing the Rust phases.
3. A manual smoke read of the source — grep for residual
   `window.confirm` / `window.prompt` / `window.alert`.

## Verification (mandatory before declaring done)

Run, in order:

```sh
./scripts/webui-build.sh
./scripts/pre-landing.sh
```

`webui-build.sh` must complete and produce regenerated
`philharmonic/webui/dist/{index.html,main.js,main.js.map,main.js.LICENSE.txt,main.css,main.css.map,icon.svg}`.
A diff in `philharmonic/webui/dist/main.js` and
`main.js.map` is **expected** — they're the artefacts of
the source edits.

`pre-landing.sh` must print
`=== pre-landing: all checks passed ===`. If the script
detects no Rust changes (likely), it short-circuits
the Rust phases; if it does run them, they should pass
unchanged.

Final grep, paste the output in the session summary:

```sh
grep -rn 'window\.confirm\|window\.prompt\|window\.alert' philharmonic/webui/src/
```

Must print nothing (zero hits).

If `webui-build.sh` fails with a TypeScript error, **fix
forward** — the type system is the contract. If
`pre-landing.sh` fails on something orthogonal, **STOP
and report**.

## Hand-off shape: Codex does not commit

**Leave the working tree dirty.** Claude commits via
`./scripts/commit-all.sh` after reviewing the diff. The
script has a `codex-guard` that aborts if any ancestor
process is named `*codex*`; calling `commit-all.sh` from
inside a Codex run will hard-fail.

Specifically:

- Do **not** run `./scripts/commit-all.sh` (any flags,
  including `--dry-run`, `--parent-only`, `--exclude`).
- Do **not** run raw `git commit` / `git push` / `git add`.
  The pre-commit hooks enforce signoff + signature +
  `Audit-Info:` trailer.
- Do **not** run `git commit --no-verify` / `--no-gpg-sign`.
- Do **not** run `git reset` / `git rebase` / `git amend`.
  History is append-only.
- Do **not** run `./scripts/push-all.sh`. Claude pushes
  after reviewing.
- Do **not** run `./scripts/publish-crate.sh`. Yuka
  publishes — not expected to come up here.
- Do **not** edit `HUMANS.md`. Agent-readable,
  agent-writable forbidden.

Edits land in the working tree across:

- `philharmonic/webui/src/` — new files + edited pages +
  edited i18n.
- `philharmonic/webui/dist/` — regenerated artefacts.
- Parent repo — `docs/codex-prompts/...` (this file's
  `## Outcome` update, plus optionally a
  `docs/codex-reports/...` entry).

`philharmonic/webui/` is a sub-path of the
`philharmonic` submodule — both the `webui/src/` source
and the `webui/dist/` artefacts are inside the same
submodule. The parent repo gets the prompt-archive
update only.

Codex's session summary should mention which submodule +
the parent have dirty trees.

## Codex report (encouraged)

If anything non-obvious surfaced during this round —
React Router `useBlocker` gotchas, a `useEffect` lifecycle
edge case with the body-scroll lock, a focus-restore
quirk, an i18n nesting decision — write a short report
to
`docs/codex-reports/2026-05-21-0001-webui-modal-component-and-duplicate-template-button.md`
per [`docs/codex-reports/README.md`](../codex-reports/README.md).
Routine specified-and-shipped work doesn't need one. Leave
the report **dirty** in the working tree; Claude commits
it alongside the implementation diff.

If you skip the report, say so in the session summary.

## Outcome

Pending — will be updated after the Codex run.

---

<task>
Build a small reusable `Modal` component for the WebUI,
build `ConfirmModal` and `DuplicateTemplateModal` on top
of it, add a "Duplicate" button to the workflow templates
list view and detail view that asks for a new name and
clones via the existing `POST /workflows/templates`
endpoint, replace all 8 in-tree `window.confirm()` calls
with `ConfirmModal`, migrate the existing inline
`ImportItemsModal` in `EmbedDatasets.tsx` to use the new
`Modal` component, and add a "discard unsaved changes?"
guard (custom `ConfirmModal` for in-app navigation via
React Router's `useBlocker`, plus the native
`beforeunload` prompt for browser close/refresh) to the
four detail pages with edit forms (`TemplateDetail`,
`RoleDetail`, `EndpointDetail`, `TenantSettings`).
Regenerate the committed `philharmonic/webui/dist/`
artefacts via `./scripts/webui-build.sh`. Single
submodule (`philharmonic/`) plus the parent for the
prompt-archive update.

**Reference docs (authoritative if they contradict this
prompt):**

1. `CONTRIBUTING.md` §§4, 6, 7, 11, 14.6.
2. The full preamble above — especially "Context files
   pointed at", "Shape (locked)", "Hard requirements",
   "i18n consistency", "Verification".
3. React Router v7 `useBlocker` semantics.

**Hard constraints (locked):**

- No new npm dependencies. Build on `react`,
  `react-router-dom`, and the existing CSS classes only.
- Strict TypeScript — no `any`, no `as unknown as T`
  outside established patterns. Match the prevailing
  posture.
- The four committed `webui/dist/` artefacts must be
  regenerated via `./scripts/webui-build.sh`. Build the
  source first, then run the script; commit both source
  and dist as part of the same dispatch (Claude commits
  — you do not).
- `Modal` provides Esc-to-close, backdrop-click-close
  (default; opt-out via `closeOnBackdropClick={false}`),
  body-scroll lock while open, focus capture on open +
  restore on close. No portal — render inline.
- `ConfirmModal` props: `title`, `body`, `confirmLabel?`
  (default `t.common.confirm`), `cancelLabel?` (default
  `t.common.cancel`), `danger?` (default false),
  `onConfirm`, `onCancel`.
- `DuplicateTemplateModal` props: `templateId`,
  `sourceDisplayName`, `onClose`, `onDuplicated`. Behavior
  per the preamble: fetch the source template detail,
  POST a new template with the new name + copied
  `script_source` + `abstract_config` +
  `data_config` (only when bindings present).
  Permission-gated on `workflow:template_create`. Retired
  sources ARE duplicable.
- `useUnsavedChanges(isDirty: boolean, message: string)`:
  React Router `useBlocker` for in-app nav + native
  `beforeunload` for browser unload. Returns
  `{ blocker }` so the caller renders a `ConfirmModal`
  when `blocker.state === "blocked"`. Cleans up the
  `beforeunload` listener on unmount and on `isDirty`
  flipping false.
- Unsaved-changes guard scope is exactly: `TemplateDetail`,
  `RoleDetail`, `EndpointDetail`, `TenantSettings`. Do
  NOT add it to inline Create forms or list pages.
- All 8 `window.confirm()` call sites replaced with
  `ConfirmModal`. Final grep
  `grep -rn 'window\.confirm\|window\.prompt\|window\.alert' philharmonic/webui/src/`
  must print nothing.
- `ImportItemsModal` in `EmbedDatasets.tsx` rewritten to
  use the new `Modal` component (the form / actions stay
  the same; only the backdrop/panel boilerplate moves
  to `Modal`).
- Add the new i18n keys per the preamble to BOTH
  `i18n/en.ts` and `i18n/ja.ts`. Maintain the existing
  ordering / nesting style.
- Duplicate button placement:
  - Templates.tsx (list view): per-row, next to the
    existing "Test in chat" button. `button secondary
    compact-button`. Shown for both active and retired.
  - TemplateDetail.tsx (detail view): header actions
    row, **before** the danger "Retire" button.
    `button secondary`. Shown for both active and
    retired.
- Behaviour on duplicate success:
  - From list view: refresh the list (clear cursor +
    bump refreshKey). Stay on the list page.
  - From detail view: `navigate(\`/templates/${newId}\`)`.
- Duplicate default name: `${sourceDisplayName ?? ""}${t.templates.copySuffix}`.
  Submit disabled while trimmed name is empty.

**Per-file scope (the full set of edits):**

NEW files:

- `philharmonic/webui/src/components/Modal.tsx`
- `philharmonic/webui/src/components/ConfirmModal.tsx`
- `philharmonic/webui/src/components/DuplicateTemplateModal.tsx`
- `philharmonic/webui/src/hooks/useUnsavedChanges.ts`

EDITED files (TypeScript):

- `philharmonic/webui/src/pages/Templates.tsx` — add
  Duplicate button per row + modal wiring.
- `philharmonic/webui/src/pages/TemplateDetail.tsx` —
  add Duplicate button to header + modal wiring; replace
  `window.confirm` at line 179 with `ConfirmModal`;
  apply `useUnsavedChanges`.
- `philharmonic/webui/src/pages/RoleDetail.tsx` —
  replace `window.confirm` at line 69; apply
  `useUnsavedChanges`.
- `philharmonic/webui/src/pages/EmbedDatasetDetail.tsx` —
  replace `window.confirm` at line 166.
- `philharmonic/webui/src/pages/AuthorityDetail.tsx` —
  replace both `window.confirm` calls (lines 91, 112).
- `philharmonic/webui/src/pages/Memberships.tsx` —
  replace `window.confirm` at line 69.
- `philharmonic/webui/src/pages/Principals.tsx` —
  replace `window.confirm` at line 90.
- `philharmonic/webui/src/pages/EndpointDetail.tsx` —
  replace `window.confirm` at line 129; apply
  `useUnsavedChanges`.
- `philharmonic/webui/src/pages/TenantSettings.tsx` —
  apply `useUnsavedChanges`.
- `philharmonic/webui/src/pages/EmbedDatasets.tsx` —
  migrate `ImportItemsModal` to use the new `<Modal>`
  component.
- `philharmonic/webui/src/i18n/en.ts` — add new keys.
- `philharmonic/webui/src/i18n/ja.ts` — add new keys.

REGENERATED (build artefacts):

- `philharmonic/webui/dist/index.html`
- `philharmonic/webui/dist/main.js`
- `philharmonic/webui/dist/main.js.LICENSE.txt`
- `philharmonic/webui/dist/main.js.map`
- `philharmonic/webui/dist/main.css`
- `philharmonic/webui/dist/main.css.map`
- `philharmonic/webui/dist/icon.svg` (likely unchanged
  byte-for-byte; if it shows up dirty, that's OK — the
  build copies the source SVG verbatim).

**Verification (must run + pass before declaring done):**

```sh
./scripts/webui-build.sh
./scripts/pre-landing.sh
grep -rn 'window\.confirm\|window\.prompt\|window\.alert' philharmonic/webui/src/
```

- `webui-build.sh` clean (regenerates dist files);
- `pre-landing.sh` clean (Rust phases short-circuit or
  pass);
- final grep prints zero hits.

<default_follow_through_policy>
Codex is expected to land all eleven edited TS files, four
new TS files, both i18n files, and the regenerated dist
artefacts in this single round. "Components built, page
migrations pending" is **not** a complete result — keep
going.

If a hard blocker surfaces (e.g. React Router v7's
`useBlocker` doesn't compose with the `BrowserRouter`
setup currently used in `App.tsx`, requiring a router
swap), **STOP and report the blocker before partial
landing**. A partial result that has half the
`window.confirm` sites migrated and the others still on
`window.confirm` is worse than a clean "blocker found,
here's what I'd recommend" report.

If `webui-build.sh` fails on a TypeScript error, fix
forward — the type system is the contract.

If `pre-landing.sh` fails on something orthogonal (a
pre-existing flake unrelated to this change), STOP and
report.
</default_follow_through_policy>

<completeness_contract>
"Complete" means:

1. New file `Modal.tsx` exists with the props contract
   above; renders the existing `.modal-backdrop` /
   `.modal-panel` structure; provides Esc-close,
   backdrop-click-close (with opt-out), body-scroll
   lock, focus-on-open + focus-restore-on-close.
2. New file `ConfirmModal.tsx` exists; built on
   `<Modal>`; the props contract is as above; default
   labels resolve via `useT()` to
   `t.common.confirm` / `t.common.cancel`; `danger` flag
   switches the confirm button class.
3. New file `DuplicateTemplateModal.tsx` exists; props
   contract as above; performs `GET` then `POST` per
   the Shape (locked) section; permission-gated on
   `workflow:template_create`; default name is
   `${sourceDisplayName ?? ""}${t.templates.copySuffix}`;
   trimmed-empty name disables submit; error stays
   inside the modal.
4. New file `useUnsavedChanges.ts` exists; returns
   `{ blocker }`; registers + cleans up the
   `beforeunload` listener correctly; uses
   `useBlocker` from `react-router-dom` for in-app
   nav.
5. `Templates.tsx`: per-row Duplicate button next to
   "Test in chat"; clicking it opens the modal; on
   success the list refreshes (cursor cleared,
   refreshKey bumped).
6. `TemplateDetail.tsx`: header Duplicate button before
   Retire; on success navigates to the new template's
   detail page; `window.confirm` at line 179 replaced
   with `ConfirmModal`; `useUnsavedChanges` applied.
7. `RoleDetail.tsx`: `window.confirm` at line 69
   replaced; `useUnsavedChanges` applied.
8. `EmbedDatasetDetail.tsx`: `window.confirm` at line
   166 replaced.
9. `AuthorityDetail.tsx`: both `window.confirm` calls
   (lines 91 and 112) replaced.
10. `Memberships.tsx`: `window.confirm` at line 69
    replaced.
11. `Principals.tsx`: `window.confirm` at line 90
    replaced.
12. `EndpointDetail.tsx`: `window.confirm` at line 129
    replaced; `useUnsavedChanges` applied to the rotate
    form.
13. `TenantSettings.tsx`: `useUnsavedChanges` applied to
    its PATCH form.
14. `EmbedDatasets.tsx`: `ImportItemsModal` migrated to
    use `<Modal>`.
15. `i18n/en.ts` and `i18n/ja.ts` both have the new
    keys per the preamble.
16. `./scripts/webui-build.sh` ran and the
    `webui/dist/` artefacts are regenerated. They
    appear in `git status` (dirty).
17. `./scripts/pre-landing.sh` passes.
18. `grep -rn 'window\.confirm\|window\.prompt\|window\.alert' philharmonic/webui/src/`
    prints nothing.
19. Working tree left dirty across the `philharmonic/`
    submodule + parent (prompt-archive Outcome
    update). **No commits, no pushes** — Claude
    commits.
20. Session summary lists which submodule + the parent
    have dirty trees so Claude can scope the
    `commit-all.sh` run. The summary also pastes the
    final grep output and the diff stats per the
    structured-output contract below.

If any of (1)–(19) is incomplete, the dispatch is
INCOMPLETE. Report INCOMPLETE clearly with what's done
and what's left, and STOP — don't synthesise a
half-result.
</completeness_contract>

<verification_loop>
Between rounds of edits, you may run a quick incremental
build:

  cd philharmonic/webui && npx tsc --noEmit

(That's tolerable here as a fast type-check during the
edit loop — but it does NOT regenerate the `dist/`
artefacts. The final mandatory step is the full script
run below.)

Final, in order:

  ./scripts/webui-build.sh
  ./scripts/pre-landing.sh
  grep -rn 'window\.confirm\|window\.prompt\|window\.alert' philharmonic/webui/src/

`webui-build.sh` runs from the workspace root; do not
`cd` first.

`pre-landing.sh` is gated by the codex-guard around
commits but NOT around running it for verification —
running it is allowed (it doesn't try to commit). On a
contended box, check headroom first:

  ./scripts/xtask.sh resource-pressure

Back off if `load1/cpus` climbs well above 1.0 — Yuka's
dev box is shared with rust-analyzer.

Do not run raw `cargo fmt` / `cargo clippy` / `cargo
test` — `pre-landing.sh` covers them with the right
`CARGO_TARGET_DIR`.
</verification_loop>

<missing_context_gating>
Before you start editing, the workspace state must match
the prompt's claims:

  ./scripts/status.sh

Should print `(clean)` for the parent repo and all
submodules except possibly `docs-jp/` (sometimes carries
uncommitted summary edits). If anything else is dirty,
**STOP and report**.

If `package.json` or `package-lock.json` has unrelated
pending edits, STOP — don't conflict with someone
else's npm work.

If `App.tsx` uses something other than React Router's
`BrowserRouter` / `RouterProvider` (custom history,
hash router, etc.) such that `useBlocker` won't
function, STOP and report — propose an alternative
shape (e.g. a `<Link>` interceptor + `beforeunload`-only
approach) rather than forcing the swap silently.
</missing_context_gating>

<action_safety>
- **You do not commit.** Leave the working tree dirty
  across `philharmonic/` + parent. `./scripts/commit-all.sh`
  (any flags) and raw `git commit` / `git push` /
  `git add` / `git reset` / `git rebase` / `git amend`
  are forbidden. The script's `codex-guard` will
  hard-abort; the same guard fires from the pre-commit
  hooks. Claude commits + pushes after reviewing the
  diff.
- **Never** invoke `./scripts/push-all.sh`. Claude
  pushes.
- **Never** invoke `./scripts/publish-crate.sh`. Not
  expected to come up here (no Rust crate changes).
- **Never** edit `HUMANS.md`. Agent-readable,
  agent-writable forbidden.
- POSIX-ish host: no `bash`-only constructs in any
  shell you invoke.
- The workspace's authoritative timezone is JST
  (Asia/Tokyo). Today is 2026-05-21 (Thu). Any
  timestamp you write (in a CHANGELOG bullet or the
  codex-report) belongs in JST. The codex-report
  filename follows the journal-file rule —
  `docs/codex-reports/2026-05-21-0001-webui-modal-component-and-duplicate-template-button.md`
  with the same daily counter (`0001`) used for this
  prompt.
- `philharmonic/webui/dist/` files are committed — do
  NOT add them to `.gitignore` or otherwise exclude
  them. They are consumed by the Rust binary at
  compile time.
</action_safety>

<structured_output_contract>
At the end of the dispatch, return:

1. **Summary** (2-3 sentences): which sites were
   migrated, total new/changed lines split by category
   (new components / page edits / i18n / dist
   regeneration).
2. **Touched files**: full list, grouped by
   submodule + parent.
3. **Final grep output** for
   `grep -rn 'window\.confirm\|window\.prompt\|window\.alert' philharmonic/webui/src/`
   — must be empty; paste the empty result inline.
4. **Component contracts**: paste the final exported
   props interfaces for `Modal`, `ConfirmModal`,
   `DuplicateTemplateModal`, and the
   `useUnsavedChanges` hook so the reviewer can
   eyeball them.
5. **`useBlocker` integration notes**: confirm the
   blocker is set up with a callback (not the
   deprecated boolean form), confirm `App.tsx` uses a
   compatible router shape, and note any
   `useBlocker` lifecycle quirks you tripped over.
6. **i18n additions**: paste the diff for both
   `en.ts` and `ja.ts` so the reviewer can confirm
   placement + translations.
7. **Verification results**:
   - `webui-build.sh`: PASS / FAIL (one-line summary
     if FAIL).
   - `pre-landing.sh`: PASS / FAIL (one-line summary
     if FAIL).
8. **Working-tree state at hand-off**:
   - List which submodule + parent have dirty trees.
   - Paste `git status --short` from the
     `philharmonic/` submodule and from the parent
     so the reviewer sees the diff scope.
   - No commits expected. Claude will commit + push
     after reviewing.
9. **Codex report**: if you wrote
   `docs/codex-reports/2026-05-21-0001-webui-modal-component-and-duplicate-template-button.md`,
   note its presence (dirty in working tree; Claude
   commits it). If you skipped, say so.
10. **Residual risks**: anything you'd flag for
    Claude or Yuka before merge — e.g. a focus-restore
    edge case in a specific browser, a `beforeunload`
    behaviour difference between Chrome and Safari,
    an i18n string you weren't sure about.
11. **Outcome paragraph** for the prompt-archive
    file: 4-6 sentences summarising the round for
    posterity, ready to drop into `## Outcome` of
    this file.
</structured_output_contract>
</task>
