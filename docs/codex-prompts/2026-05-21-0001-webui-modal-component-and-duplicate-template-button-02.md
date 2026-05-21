# WebUI: reusable `Modal`/`ConfirmModal` components, eliminate `window.confirm()`, add duplicate-template button, unsaved-changes guard — round 02 (with data-router conversion)

**Date:** 2026-05-21 (JST)
**Slug:** `webui-modal-component-and-duplicate-template-button`
**Round:** 02 — resume after the round-01 blocker. Round 01
stopped before any edits because `philharmonic/webui/src/App.tsx`
wraps the app in `<BrowserRouter>`, and React Router v7's
`useBlocker` requires a data-router context
(`createBrowserRouter` + `<RouterProvider>`) — the hook would
compile but throw at runtime. Yuka authorised the data-router
conversion as part of the same dispatch (2026-05-21 10:42 JST).
This round adds an explicit **task 0** for the conversion and
otherwise re-imports the entire round-01 scope by reference.
**Subagent:** `codex:rescue` (fresh thread; do not `--resume`).

## Motivation

Identical to round 01 — see
[`2026-05-21-0001-webui-modal-component-and-duplicate-template-button-01.md`](2026-05-21-0001-webui-modal-component-and-duplicate-template-button-01.md)
§Motivation. The four coupled WebUI changes (reusable Modal
component, duplicate-template button, `window.confirm`
elimination, unsaved-changes guard) are unchanged. What's
new in round 02 is the data-router conversion of `App.tsx`
that makes `useBlocker` actually work, plus a small adjustment
to where `VersionRefresh` is mounted (it currently lives
inside `<BrowserRouter>` so it can call `useLocation()`; it
needs to move into a root-layout route element under the
data router, since `<RouterProvider>` doesn't accept
children).

## References (authoritative if anything in this prompt contradicts them)

1. **The round-01 prompt archive
   ([`2026-05-21-0001-webui-modal-component-and-duplicate-template-button-01.md`](2026-05-21-0001-webui-modal-component-and-duplicate-template-button-01.md))
   is the canonical specification for everything except task
   0 below.** Read it end-to-end before starting. Its
   `<task>` block, "Shape (locked)", "Hard requirements",
   "i18n keys to add", "Verification", "Hand-off shape",
   and `<structured_output_contract>` all apply to this
   round verbatim.
2. [`CONTRIBUTING.md`](../../CONTRIBUTING.md) §§4, 6, 7, 11,
   14.6 — same set referenced in round 01.
3. React Router v7 docs:
   - [`createBrowserRouter`](https://reactrouter.com/api/data-routers/createBrowserRouter)
   - [`RouterProvider`](https://reactrouter.com/api/data-routers/RouterProvider)
   - [`useBlocker`](https://reactrouter.com/api/hooks/useBlocker)
   - Migration note: hooks (`useNavigate`, `useParams`,
     `useLocation`, `useSearchParams`, `<Link>`,
     `<NavLink>`, `<Outlet>`, `<Navigate>`) all keep
     working unchanged under data router; only the
     router-setup form moves.
4. [`docs/codex-reports/README.md`](../codex-reports/README.md)
   — write a short report if anything non-obvious surfaces
   in the data-router conversion (e.g. an effect ordering
   subtlety, an unexpected route-config shape).

## Task 0 (new in round 02) — Convert `App.tsx` to the data router

**What changes:** `philharmonic/webui/src/App.tsx` only. No
other route consumer needs to change — `useNavigate` /
`useParams` / `useLocation` / `<Link>` / `<NavLink>` /
`<Outlet>` / `<Navigate>` all work identically under the
data router.

**Current shape (round-01 read):**

```tsx
export default function App(): JSX.Element {
  // ... auth/branding/whoami/tenant-display-name effects (unchanged) ...

  return (
    <BrowserRouter>
      <VersionRefresh />
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route element={<ProtectedRoute />}>
          <Route element={<Layout />}>
            <Route index element={<Dashboard />} />
            <Route path="/templates" element={<Templates />} />
            <Route path="/templates/:id" element={<TemplateDetail />} />
            <Route path="/instances" element={<Instances />} />
            <Route path="/instances/:id" element={<InstanceDetail />} />
            <Route path="/endpoints" element={<Endpoints />} />
            <Route path="/endpoints/:id" element={<EndpointDetail />} />
            <Route path="/principals" element={<Principals />} />
            <Route path="/roles" element={<Roles />} />
            <Route path="/roles/:id" element={<RoleDetail />} />
            <Route path="/memberships" element={<Memberships />} />
            <Route path="/authorities" element={<Authorities />} />
            <Route path="/authorities/:id" element={<AuthorityDetail />} />
            <Route path="/embed-datasets" element={<EmbedDatasets />} />
            <Route path="/embed-datasets/:id" element={<EmbedDatasetDetail />} />
            <Route path="/audit" element={<AuditLog />} />
            <Route path="/settings" element={<TenantSettings />} />
          </Route>
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}
```

`VersionRefresh` currently lives inside `<BrowserRouter>`
because it calls `useLocation()` to detect route changes
(see lines 119-130). It cannot stay as a direct child of
`<RouterProvider>` because data routers do not render
children — they render their own route tree only.

**Target shape:**

```tsx
import { type JSX, useEffect } from "react";
import {
  createBrowserRouter,
  Navigate,
  Outlet,
  RouterProvider,
  useLocation,
} from "react-router-dom";
// ... unchanged imports ...

function RootLayout(): JSX.Element {
  return (
    <>
      <VersionRefresh />
      <Outlet />
    </>
  );
}

const router = createBrowserRouter([
  {
    element: <RootLayout />,
    children: [
      { path: "/login", element: <Login /> },
      {
        element: <ProtectedRoute />,
        children: [
          {
            element: <Layout />,
            children: [
              { index: true, element: <Dashboard /> },
              { path: "templates", element: <Templates /> },
              { path: "templates/:id", element: <TemplateDetail /> },
              { path: "instances", element: <Instances /> },
              { path: "instances/:id", element: <InstanceDetail /> },
              { path: "endpoints", element: <Endpoints /> },
              { path: "endpoints/:id", element: <EndpointDetail /> },
              { path: "principals", element: <Principals /> },
              { path: "roles", element: <Roles /> },
              { path: "roles/:id", element: <RoleDetail /> },
              { path: "memberships", element: <Memberships /> },
              { path: "authorities", element: <Authorities /> },
              { path: "authorities/:id", element: <AuthorityDetail /> },
              { path: "embed-datasets", element: <EmbedDatasets /> },
              { path: "embed-datasets/:id", element: <EmbedDatasetDetail /> },
              { path: "audit", element: <AuditLog /> },
              { path: "settings", element: <TenantSettings /> },
            ],
          },
        ],
      },
      { path: "*", element: <Navigate to="/" replace /> },
    ],
  },
]);

export default function App(): JSX.Element {
  const dispatch = useAppDispatch();
  // ... all existing effects unchanged ...

  return <RouterProvider router={router} />;
}
```

The route tree above mirrors the current nested-route
structure 1:1 — `RootLayout` (new, renders
`<VersionRefresh /><Outlet />`) replaces the
`<BrowserRouter>` direct-child slot; `ProtectedRoute`
wraps everything except `/login` and the catch-all; `Layout`
wraps the authenticated routes. The `/login` and `*` routes
sit at the same level as the `ProtectedRoute` wrapper, as
in round-01's source.

The four effects in `App()` (branding, whoami, tenant
display name — currently at lines 54-85) stay in `App`
because they need `useAppDispatch` / `useAppSelector`
and don't depend on router state. They run on every render
of the `App` component (which is once, since the component
itself renders only the `RouterProvider`).

**Module-level `router` is intentional** — React Router's
recommended pattern. It's not re-created on every `App`
render and it carries no React state.

**Path strings drop the leading `/`** under data router's
nested-route conventions, except at the root level. This
is the standard idiom — `{ path: "templates" }` resolves
to `/templates` because its parent has no path. The
absolute-path form (`{ path: "/templates" }`) also works
but is less idiomatic; pick the no-leading-slash form
above for consistency with React Router docs.

**`<Navigate>` in the catch-all keeps the leading slash**
(its `to` prop is a URL, not a route-config path).

**Verification for task 0 alone (before moving on):**

1. `cd philharmonic/webui && npx tsc --noEmit` — clean.
2. `./scripts/webui-build.sh` — produces regenerated dist
   artefacts.
3. Quick manual smoke-read: confirm `<RouterProvider>` is
   the only routing element; no leftover
   `<BrowserRouter>` or top-level `<Routes>` import. Imports
   pruned (`BrowserRouter`, `Route`, `Routes` removed;
   `createBrowserRouter`, `RouterProvider`, `Outlet`
   added).

If the build passes task 0, proceed to the rest of the
round-01 scope (tasks 1-6 below).

## Tasks 1-6 (carried forward from round 01, unchanged)

Refer to round-01 §"Context files pointed at" for the full
file-by-file specification. Summary:

1. **Build the reusable components:**
   - `philharmonic/webui/src/components/Modal.tsx` (NEW) —
     wrapper with Esc-close, backdrop-click-close (opt-out
     via `closeOnBackdropClick={false}`), body-scroll lock,
     focus capture+restore.
   - `philharmonic/webui/src/components/ConfirmModal.tsx`
     (NEW) — `{ title, body, confirmLabel?, cancelLabel?,
     danger?, onConfirm, onCancel }`.
   - `philharmonic/webui/src/components/DuplicateTemplateModal.tsx`
     (NEW) — `{ templateId, sourceDisplayName, onClose,
     onDuplicated }`; permission-gated on
     `workflow:template_create`; retired sources duplicable.
   - `philharmonic/webui/src/hooks/useUnsavedChanges.ts`
     (NEW) — `useBlocker` + `beforeunload`; returns
     `{ blocker }`.

2. **Add the Duplicate button:**
   - `Templates.tsx` per-row (next to "Test in chat",
     `button secondary compact-button`, shown for active
     **and** retired).
   - `TemplateDetail.tsx` header (before Retire,
     `button secondary`, shown for active **and** retired).
   - List-view success → refresh; detail-view success →
     `navigate(\`/templates/${newId}\`)`.

3. **Replace all 8 `window.confirm()` call sites** with
   `ConfirmModal` (the round-01 prompt's "Per-file scope"
   block lists each file + line + title-string per call):
   - `RoleDetail.tsx:69`
   - `EmbedDatasetDetail.tsx:166`
   - `AuthorityDetail.tsx:91, 112` (two calls)
   - `Memberships.tsx:69`
   - `TemplateDetail.tsx:179`
   - `Principals.tsx:90`
   - `EndpointDetail.tsx:129`

4. **Migrate `ImportItemsModal`** in `EmbedDatasets.tsx:302-350`
   to use the new `<Modal>` component (backdrop/panel
   boilerplate replaced; form / actions structure unchanged).

5. **Apply `useUnsavedChanges`** to the four detail pages
   with edit forms — `TemplateDetail`, `RoleDetail`,
   `EndpointDetail`, `TenantSettings`. Per round-01's
   "Shape (locked)" — track `isDirty` via a snapshot ref
   of last-saved values; render the blocker `ConfirmModal`
   when `blocker.state === "blocked"`.

6. **Add the new i18n keys** to BOTH `i18n/en.ts` and
   `i18n/ja.ts` per round-01 §"i18n keys to add". The
   per-entity `confirmRetireTitle` / `confirmBumpEpochTitle`
   strings, the new common keys (`confirm`, `discard`,
   `discardChanges`, `unsavedChangesPrompt`, `duplicate`,
   `duplicating`), and the templates-specific keys
   (`duplicate`, `duplicateTitle`,
   `duplicateNamePlaceholder`, `duplicated`, `copySuffix`).

## Hard requirements (carried forward + delta)

All round-01 hard requirements apply verbatim. The only
delta for round 02:

- **`<missing_context_gating>` for the BrowserRouter shape is
  REMOVED.** The conversion to `createBrowserRouter` +
  `RouterProvider` is now part of the work, not a blocker.
- **`<App.tsx>` must end with `<RouterProvider />` as its
  only JSX child.** No leftover `<BrowserRouter>`, no
  top-level `<Routes>`, no other wrapper around the
  provider.
- **`VersionRefresh`'s contract is unchanged.** It still
  calls `useLocation()`, still owns the visibility-driven
  refresh interval. Only its mount point moves (from
  direct child of `<BrowserRouter>` to child of the new
  `RootLayout` route element).
- **Route paths in the data-router config drop the leading
  `/` except at the root level.** Idiomatic React Router
  v7 form; matches the docs.
- **The module-level `router` constant is intentional.** Do
  not stash it in component state, do not recreate it on
  re-render.

Everything else — i18n keys, component contracts, file
list, verification commands, hand-off rules, structured
output contract — comes straight from round 01.

## Verification (mandatory before declaring done)

Identical to round 01:

```sh
./scripts/webui-build.sh
./scripts/pre-landing.sh
grep -rn 'window\.confirm\|window\.prompt\|window\.alert' philharmonic/webui/src/
```

All three must pass / be empty. See round-01 §"Verification"
for the full rules.

Additionally, paste the final `App.tsx` diff in the session
summary so the reviewer can confirm the data-router shape
landed cleanly (no leftover `<BrowserRouter>` import, the
`RootLayout` is defined, the route tree is intact).

## Hand-off shape

Identical to round 01 — Codex does not commit, push,
publish, or edit `HUMANS.md`. Leave the tree dirty across
`philharmonic/` + parent for Claude to commit.

## Codex report (encouraged)

If the data-router conversion surfaces anything non-obvious
(an effect-order quirk, a route-mapping subtlety, a focus
behaviour that shifts), write a short report to
`docs/codex-reports/2026-05-21-0001-webui-modal-component-and-duplicate-template-button.md`
per [`docs/codex-reports/README.md`](../codex-reports/README.md).
Routine specified-and-shipped work doesn't need one; the
session summary covers it.

## Outcome

Pending — will be updated after the Codex run.

---

<task>
Resume the WebUI work blocked in round 01 (see
[`2026-05-21-0001-webui-modal-component-and-duplicate-template-button-01.md`](2026-05-21-0001-webui-modal-component-and-duplicate-template-button-01.md)
§Outcome). The round-01 prompt is the canonical specification
for everything except **task 0** below; read it end-to-end
before starting.

**Task 0 — convert `philharmonic/webui/src/App.tsx` to the
React Router v7 data router** (`createBrowserRouter` +
`<RouterProvider>` + route objects). This unblocks
`useBlocker` for the unsaved-changes guard. The shape is
fully spelled out in the §"Task 0 (new in round 02) —
Convert `App.tsx` to the data router" section of THIS file's
preamble — read it, then implement. `RootLayout` is a new
local component that renders `<VersionRefresh /><Outlet />`
and replaces the `<BrowserRouter>` direct-child slot.

**Tasks 1-6** are exactly the round-01 work — build `Modal`
/ `ConfirmModal` / `DuplicateTemplateModal` /
`useUnsavedChanges`; add the Duplicate button to
`Templates.tsx` and `TemplateDetail.tsx`; replace all 8
`window.confirm()` call sites with `ConfirmModal`; migrate
`ImportItemsModal` in `EmbedDatasets.tsx` to use the new
`<Modal>`; apply `useUnsavedChanges` to the four detail
pages with edit forms; add the new i18n keys. The round-01
prompt has every file path, line number, prop contract,
default-name rule, permission-gating rule, and i18n key
needed — follow it verbatim.

**Reference docs (authoritative if they contradict this
prompt):**

1. [`2026-05-21-0001-webui-modal-component-and-duplicate-template-button-01.md`](2026-05-21-0001-webui-modal-component-and-duplicate-template-button-01.md)
   — the entire file, especially "Context files pointed at",
   "Shape (locked)", "Hard requirements", "i18n keys to
   add", `<completeness_contract>`, `<structured_output_contract>`.
2. `CONTRIBUTING.md` §§4, 6, 7, 11, 14.6.
3. React Router v7 data-router docs.

**Hard constraints (locked):**

All round-01 hard constraints carry over. Delta for round 02:

- `App.tsx` must end with `<RouterProvider />` as its only
  JSX return; no leftover `<BrowserRouter>` / `<Routes>` /
  `Route` JSX or imports.
- Route paths in the route-object config drop the leading
  `/` except at the root level (idiomatic v7 form).
- `VersionRefresh`'s implementation stays unchanged; only
  its mount point moves into the new `RootLayout` route
  element.
- The module-level `router = createBrowserRouter([...])`
  constant is intentional — do not stash it in component
  state.
- Everything else — no new npm deps, strict TypeScript, no
  `window.confirm` / `window.prompt` / `window.alert` in
  the final tree, regenerate the four committed
  `webui/dist/` artefacts, permission-gate duplicate on
  `workflow:template_create`, retired sources duplicable,
  i18n keys in both `en.ts` and `ja.ts`, etc. — is exactly
  round 01.

**Per-file scope (the full set of edits):**

NEW files:

- `philharmonic/webui/src/components/Modal.tsx`
- `philharmonic/webui/src/components/ConfirmModal.tsx`
- `philharmonic/webui/src/components/DuplicateTemplateModal.tsx`
- `philharmonic/webui/src/hooks/useUnsavedChanges.ts`

EDITED files (TypeScript):

- `philharmonic/webui/src/App.tsx` — convert to data router
  (task 0).
- `philharmonic/webui/src/pages/Templates.tsx` — Duplicate
  button per row.
- `philharmonic/webui/src/pages/TemplateDetail.tsx` —
  Duplicate button, `window.confirm` → ConfirmModal,
  `useUnsavedChanges`.
- `philharmonic/webui/src/pages/RoleDetail.tsx` —
  `window.confirm` → ConfirmModal, `useUnsavedChanges`.
- `philharmonic/webui/src/pages/EmbedDatasetDetail.tsx` —
  `window.confirm` → ConfirmModal.
- `philharmonic/webui/src/pages/AuthorityDetail.tsx` —
  both `window.confirm` calls → ConfirmModal.
- `philharmonic/webui/src/pages/Memberships.tsx` —
  `window.confirm` → ConfirmModal.
- `philharmonic/webui/src/pages/Principals.tsx` —
  `window.confirm` → ConfirmModal.
- `philharmonic/webui/src/pages/EndpointDetail.tsx` —
  `window.confirm` → ConfirmModal, `useUnsavedChanges`.
- `philharmonic/webui/src/pages/TenantSettings.tsx` —
  `useUnsavedChanges`.
- `philharmonic/webui/src/pages/EmbedDatasets.tsx` —
  migrate `ImportItemsModal` to use the new `<Modal>`.
- `philharmonic/webui/src/i18n/en.ts` — new keys.
- `philharmonic/webui/src/i18n/ja.ts` — new keys.

REGENERATED (build artefacts):

- `philharmonic/webui/dist/index.html`
- `philharmonic/webui/dist/main.js`
- `philharmonic/webui/dist/main.js.LICENSE.txt`
- `philharmonic/webui/dist/main.js.map`
- `philharmonic/webui/dist/main.css`
- `philharmonic/webui/dist/main.css.map`
- `philharmonic/webui/dist/icon.svg` (likely byte-identical)

**Verification (must run + pass before declaring done):**

```sh
./scripts/webui-build.sh
./scripts/pre-landing.sh
grep -rn 'window\.confirm\|window\.prompt\|window\.alert' philharmonic/webui/src/
```

- `webui-build.sh` clean; regenerated dist files visible
  in `git status`.
- `pre-landing.sh` ends with
  `=== pre-landing: all checks passed ===`.
- Final grep prints zero hits.

<default_follow_through_policy>
Round 02 is expected to land the full scope — data-router
conversion, the four new components/hook, all eleven edited
TS files, both i18n files, and the regenerated dist
artefacts — in a single round. "Data router converted,
unsaved-changes guard pending" is **not** a complete result
— keep going.

If a new hard blocker surfaces (e.g. a route consumer
breaks under data router in a way the conversion guide
didn't anticipate, or an i18n keying conflict you can't
resolve), STOP and report. Otherwise fix forward.
</default_follow_through_policy>

<completeness_contract>
"Complete" inherits all 19 items from round 01's
completeness contract, with one prepended:

0. `App.tsx` converted to `createBrowserRouter` +
   `<RouterProvider>` per the §"Task 0" preamble.
   `<BrowserRouter>` / `<Routes>` / `Route` imports are
   removed; `createBrowserRouter`, `RouterProvider`,
   `Outlet` are added. `RootLayout` is defined as a local
   component rendering `<VersionRefresh /><Outlet />`. The
   module-level `router` is constructed at module scope.
   `App()` returns only `<RouterProvider router={router} />`.

Then items 1-19 from round 01 — see that file's
`<completeness_contract>` block.

If any of (0)-(19) is incomplete, the dispatch is
INCOMPLETE. Report INCOMPLETE clearly with what's done and
what's left, and STOP — don't synthesise a half-result.
</completeness_contract>

<verification_loop>
Identical to round 01:

  cd philharmonic/webui && npx tsc --noEmit       # fast type-check between edits

  ./scripts/webui-build.sh                         # final, regenerates dist
  ./scripts/pre-landing.sh                         # final, Rust phases short-circuit
  grep -rn 'window\.confirm\|window\.prompt\|window\.alert' philharmonic/webui/src/

Run from the workspace root; do not `cd` first for the
final three.

The webui-build is single-shot — re-running it is fine
but slow; do the type-check loop with `tsc --noEmit` while
iterating, then run the build script once at the end.
</verification_loop>

<missing_context_gating>
`./scripts/status.sh` should print `(clean)` for the
parent repo and all submodules (docs-jp/ may carry stale
summary edits — ignore those for this dispatch). If
anything else is dirty, STOP and report.

The round-01 gating about `BrowserRouter` is REMOVED —
the conversion is now part of the work. Other gating
applies as round 01.

If the route consumer files (page components) reach for
something `useBlocker` can't provide (e.g. a custom
history shim outside `App.tsx`), STOP and report —
propose a follow-up shape rather than forcing it through.
</missing_context_gating>

<action_safety>
- You do not commit. Leave the tree dirty across
  `philharmonic/` + parent.
  `./scripts/commit-all.sh` (any flags) and raw git state
  changes are forbidden. The codex-guard will abort.
- Never invoke `./scripts/push-all.sh` or
  `./scripts/publish-crate.sh`.
- Never edit `HUMANS.md`.
- POSIX-ish host; no `bash`-only constructs.
- JST is the workspace's authoritative timezone; today is
  2026-05-21 (Thu). The codex-report filename, if
  written, follows the round-01 archive's filename
  (`docs/codex-reports/2026-05-21-0001-webui-modal-component-and-duplicate-template-button.md`)
  — the report tracks the work, not the round, so the
  filename stays at `-0001-` regardless of round number.
- `philharmonic/webui/dist/` files are committed; do not
  add them to `.gitignore`.
</action_safety>

<structured_output_contract>
At the end of the dispatch, return per round-01's
`<structured_output_contract>` (the 11-item list), with
one addition before item 4:

3a. **`App.tsx` data-router diff**: paste the
    before/after of `App.tsx` so the reviewer can confirm
    the route tree mapped 1:1, `VersionRefresh` is in
    `RootLayout`, imports are pruned, and the module-level
    `router` constant is in place.

Then items 1-11 from round 01.

Item 8 (working-tree state) should include `App.tsx` in
the "edited" list explicitly.

Item 11 (Outcome paragraph) should cover both the
data-router conversion (one or two sentences on the
shape that landed) and the rest of the work — 5-7
sentences total, ready to drop into `## Outcome` of
this file.
</structured_output_contract>
</task>
