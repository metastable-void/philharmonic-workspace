# WebUI permission-aware nav + disabled non-actionable buttons + sticky sidebar footer (initial dispatch)

**Date:** 2026-05-11
**Slug:** `webui-permission-aware-ui-and-sidebar-sticky`
**Round:** 01 (initial dispatch — single bundled dispatch
touching `philharmonic-api` 0.1.7 → 0.1.8 + `philharmonic/webui`,
per Yuka's 2026-05-11 directive)
**Subagent:** `codex:codex-rescue`

## Motivation

During the 2026-05-11 deployment-time testing session, Yuka
surfaced three related WebUI UX issues:

1. **Pages the caller can't read are still visible in the
   sidebar nav.** A principal whose role doesn't include
   `workflow:template_read` still sees "Templates" in the
   sidebar; clicking it produces a 403 response that the page
   doesn't gracefully recover from. The sidebar should hide
   pages the caller has no read permission for.
2. **Action buttons that the caller can't actually use produce
   runtime 403 errors when clicked**, rather than being
   visibly disabled up front. Examples: the "Create dataset"
   button is visible-and-clickable to a viewer lacking
   `embed_dataset:create`; clicking surfaces the API's
   `403 forbidden: missing permission ...` toast. The expected
   behavior is: the button renders disabled with clear styling
   (and ideally a `title` tooltip naming the missing
   permission), so the user immediately sees the action is
   unavailable.
3. **The sidebar footer (language switcher + session token +
   logout) currently requires scrolling to reach** when the
   nav list grows past the viewport height. The footer should
   stay docked at the bottom of the sidebar — it's already
   declared `position: sticky; bottom: 0;` in
   `app.css:103-111`, but the sidebar's flex layout +
   `overflow-y: auto` combination prevents the sticky from
   anchoring correctly. Fix the CSS so the footer is reachable
   without scrolling regardless of nav-list length.

Bundled into one dispatch because all three are WebUI surface
work in `philharmonic/webui`, (1) and (2) need the same
backend extension (the WebUI doesn't currently know the
caller's permission set — see §"Backend extension" below),
(1) and (2) share the same `usePermissions` hook + helper,
and (3) is a small CSS-only fix that doesn't justify its own
dispatch.

## References

- [`docs/ROADMAP.md` §3.D](../ROADMAP.md#d-webui-infrastructure-features-and-docs-5-dispatches)
  — D14/D15 just landed; this dispatch is post-D15 polish, not
  a numbered roadmap item.
- [`philharmonic-api/src/routes/whoami.rs`](../../philharmonic-api/src/routes/whoami.rs)
  — current `WhoamiResponse` shape (`{tenant_id, auth_type}`)
  needs extension with `permissions: Vec<String>`.
- [`philharmonic-api/src/routes/identity.rs:218`](../../philharmonic-api/src/routes/identity.rs#L218)
  — `permissions_from_revision` helper Codex can compose for
  the principal-auth case.
- Per-route → permission atom mapping (Codex: re-read each
  routes file in `philharmonic-api/src/routes/` to extract
  the authoritative map). Examples found via grep:
  - `audit.rs`: `GET /v1/audit` → `atom::AUDIT_READ`
  - `endpoints.rs`: create/rotate/retire/read_metadata/read_decrypted → respective atoms
  - `workflows.rs`: template create/read/retire, instance create/read/execute/cancel
  - `embed_datasets.rs`: create/read/update/retire
  - `roles.rs`: all routes → `atom::TENANT_ROLE_MANAGE`
  - `memberships.rs`: all routes → `atom::TENANT_ROLE_MANAGE`
  - `principals.rs`: all routes → `atom::TENANT_PRINCIPAL_MANAGE`
  - `authorities.rs`: all routes → `atom::TENANT_MINTING_MANAGE`
  - `mint.rs`: `atom::MINT_EPHEMERAL_TOKEN`
  - `tenants.rs`: settings read/manage → respective atoms
- D11 follow-up #3 prompt
  [`docs/codex-prompts/2026-05-10-0009-webui-template-data-config-editor-01.md`](2026-05-10-0009-webui-template-data-config-editor-01.md)
  — `DataConfigEditor`'s retired-warning-badge pattern;
  reuse the same "warning" CSS class approach for the
  disabled-button visual state if needed.
- D6 prompt
  [`docs/codex-prompts/2026-05-10-0006-d6-embed-datasets-webui-01.md`](2026-05-10-0006-d6-embed-datasets-webui-01.md)
  — `SourceItemsEditor`'s disabled mode pattern (the `disabled`
  prop convention).
- [`philharmonic/webui/src/components/permissions.ts`](../../philharmonic/webui/src/components/permissions.ts)
  — existing `permissionAtoms` list. Extend with a
  per-page-required permission map (see deliverable B below)
  rather than spreading the mapping across each page file.
- [`philharmonic/webui/src/store/authSlice.ts`](../../philharmonic/webui/src/store/authSlice.ts)
  — auth state (currently holds the token); add the
  permission set here.

## Context files pointed at

**Backend** (philharmonic-api):

- `philharmonic-api/src/routes/whoami.rs` — extend
  `WhoamiResponse` with `permissions: Vec<String>`. Populate
  in `whoami()` handler.
- `philharmonic-api/src/routes/identity.rs` — has the
  permission-loading helpers (`permissions_from_revision`)
  that the principal-auth path can compose. For
  ephemeral-token auth, the permissions are already in the
  token's clipped claims — accessible via the
  `RequestContext.auth` (inspect the existing `Auth` /
  `EphemeralAuth` / `PrincipalAuth` types and pick the
  right access path; the route-protector already enforces
  per-route permission so the access path exists).
- `philharmonic-api/Cargo.toml` — version bump 0.1.7 → 0.1.8
  (the new field on `WhoamiResponse` is additive +
  back-compat — older WebUI bundles ignore the new field;
  patch bump matches D12/D16 convention).
- `philharmonic-api/CHANGELOG.md` — entry under
  `[0.1.8] - 2026-05-11`.

**WebUI** (philharmonic/webui):

- `src/api/client.ts` — extend `WhoamiResponse` with
  `permissions: string[]`. Add a `whoami()` helper if not
  already present.
- `src/store/authSlice.ts` — store the permission set
  alongside the token. Reducer to set/clear permissions on
  login/logout. Selector `selectPermissions(state)`.
- `src/components/permissions.ts` — keep the existing
  `permissionAtoms` constant + `permissionGroups` /
  `permissionsForGroup`. **Add** a `navPermissions` map:
  one required atom per route in the sidebar (Dashboard ≡
  no permission required; Templates ≡ `workflow:template_read`;
  Instances ≡ `workflow:instance_read`; etc., per the
  per-route → atom map below). Also add a generic
  `hasPermission(envelope, atom)` pure helper.
- `src/hooks/usePermissions.ts` (new) — small custom hook
  that returns `{ has(atom): boolean, hasAny(atoms[]):
  boolean, all: string[] }` reading from the auth slice via
  `useAppSelector`. Lives alongside the existing
  `src/hooks/useT.ts`.
- `src/components/Layout.tsx` — sidebar nav filter:
  iterate `navItems`, drop entries where the corresponding
  `navPermissions[<to>]` is set and `!has(atom)`. Dashboard
  is always shown (it's the landing page and doesn't gate
  on a specific atom). The footer's three controls
  (language switcher, token display, logout) are always
  shown — they're session-scoped, not permission-gated.
- `src/components/Layout.tsx` (sticky footer fix) — adjust
  the sidebar CSS so the `sidebar-footer` is reliably
  reachable without scrolling. See deliverable D for the
  recommended approach.
- `src/app.css` — sidebar layout adjustments + disabled-
  button visual styling. The existing `:disabled` selectors
  exist; ensure they're applied consistently and that
  disabled buttons get a clear visual treatment (lower
  contrast, `cursor: not-allowed`, no hover effect). Don't
  add new design tokens.
- `src/pages/*.tsx` (every page with action buttons) —
  swap `onClick` handlers into `disabled={!has("...")}` +
  `title={!has("...") ? t.permissions.missingAtom("...") :
  undefined}`. Buttons to convert (Codex: enumerate by
  grepping `<button` across `src/pages/*.tsx`):
  - `Templates.tsx`: "Create" → `workflow:template_create`
  - `TemplateDetail.tsx`: "Update" / "Retire" / "Test in
    chat" → `workflow:template_create` (update via PATCH),
    `workflow:template_retire`, `workflow:instance_create`
    (Test in chat creates an instance)
  - `Endpoints.tsx`: "Create" →
    `endpoint:create`
  - `EndpointDetail.tsx`: "Rotate" / "Retire" / "Show
    decrypted" → `endpoint:rotate`, `endpoint:retire`,
    `endpoint:read_decrypted`
  - `EmbedDatasets.tsx`: "Create dataset" / bulk import →
    `embed_dataset:create`
  - `EmbedDatasetDetail.tsx`: "Save source items" /
    "Retire" → `embed_dataset:update`, `embed_dataset:retire`
  - `Instances.tsx`: "Create instance" →
    `workflow:instance_create`
  - `InstanceDetail.tsx`: "Execute" / "Complete" /
    "Cancel" / "Test in chat" send-button →
    `workflow:instance_execute`, `workflow:instance_execute`
    (complete), `workflow:instance_cancel`,
    `workflow:instance_execute` (chat send)
  - `Principals.tsx`: "Create" / "Rotate" / "Retire" →
    `tenant:principal_manage`
  - `Roles.tsx`: "Create" →
    `tenant:role_manage`
  - `RoleDetail.tsx`: "Update" / "Retire" →
    `tenant:role_manage`
  - `Memberships.tsx`: "Create" / "Retire" →
    `tenant:role_manage`
  - `Authorities.tsx`: "Create" →
    `tenant:minting_manage`
  - `AuthorityDetail.tsx`: "Update" / "Rotate" / "Bump
    epoch" / "Retire" →
    `tenant:minting_manage`
  - `TenantSettings.tsx`: "Save" →
    `tenant:settings_manage`
  - `AuditLog.tsx`: read-only page; no action buttons to
    disable, but should be hidden from sidebar if the
    caller lacks `audit:read`.
- `src/i18n/en.ts` + `src/i18n/ja.ts` — add a
  `permissions.missingAtom(atom: string) => string`
  function for the tooltip text (e.g.
  `"Missing permission: ${atom}"` in en; appropriate JP
  phrasing).

## Outcome

Pending — will be updated after Codex run.

---

## STRUCTURED-OUTPUT-CONTRACT — READ THIS FIRST

Streak is **10/10** since the contract was added. **Maintain
it** — six-section report (Summary / Touched files /
Verification results / Residual risks / Git state / Open
questions) with the verbatim `RUN STATUS: COMPLETE` token,
emitted before `task_complete`. D16 r01 died post-verification
before emitting the report; if you hit the context-window edge,
**prioritise emitting the report** over any final polish.

---

## Pre-landing-sh hygiene equivalent

Backend touched, so run **both** pipelines in this order:

```sh
# WebUI side first (catches TS errors fast)
cd philharmonic/webui
npx tsc --noEmit

# Production WebUI build (Rust pre-landing depends on this)
cd /home/ubuntu/philharmonic-workspace
./scripts/webui-build.sh --production

# Full Rust pre-landing (covers philharmonic-api + the
# API server bin that embeds the WebUI bundle)
./scripts/pre-landing.sh
```

Also for the back-compat enum-style additive change:

```sh
./scripts/check-api-breakage.sh philharmonic-api
```

The new field on `WhoamiResponse` is additive — semver-checks
should report no breaking changes. Patch bump 0.1.7 → 0.1.8
matches D12/D16 convention. **Surface the semver-checks output
in residuals.**

---

## Prompt (verbatim)

<task>
Four logical deliverables, single dispatch:

- **A** — Backend whoami extension (philharmonic-api).
- **B** — WebUI permission state + hook (api/client.ts +
  authSlice + permissions.ts + new usePermissions hook).
- **C** — Sidebar filtering by read-permission (Layout.tsx).
- **D** — Per-button disabled state across every page
  (pages/*.tsx + i18n).
- **E** — Sidebar-footer sticky CSS fix (app.css).

Suggested order: **A → B → C → D → E**. A unblocks B; B
unblocks C and D; E is independent CSS work and can land
last.

**Out of scope** (do not touch):

- Any change to the route-protector enforcement
  (`philharmonic-api/src/routes/<module>::protected`).
  The server-side check is the security boundary; the
  WebUI changes here are pure UX — they don't replace or
  weaken the server check, they just stop the user from
  hitting it accidentally.
- Any change to `permissionAtoms` itself (already settled).
- Markdown / chat changes (D14 just landed; unrelated).
- `abstract_config` editor (D15 just landed; unrelated).

If any deliverable hits a blocker (the WhoamiResponse
extension fails semver-checks unexpectedly, a page file
turns out to have a structure the prompt didn't anticipate,
etc.), say so explicitly with `RUN STATUS: PARTIAL —
<reason>` and finish the others.

## Deliverable A — Backend whoami extension

`philharmonic-api/src/routes/whoami.rs`:

```rust
#[derive(Debug, Serialize)]
pub struct WhoamiResponse {
    pub tenant_id: uuid::Uuid,
    pub auth_type: &'static str,
    /// Effective permission atoms after envelope/membership
    /// clipping. The set the route-protector enforces against.
    pub permissions: Vec<String>,
}
```

In the handler, compute `permissions` from the
`RequestContext.auth`:

- **Principal auth** (`auth.is_principal() == true`):
  effective permissions = union over the principal's
  current role memberships' permission documents. Use
  `permissions_from_revision` from
  `philharmonic-api/src/routes/identity.rs:218` per role.
  If the workspace already exposes a helper for "effective
  permissions for a principal", use that instead.
- **Ephemeral auth** (`auth.is_principal() == false`):
  the token's clipped permission claims. These live on the
  `EphemeralAuth` (or whichever variant your code uses);
  inspect the existing route-protector to see how it
  extracts them.

Sort the permissions list before serialising for stable
output (deterministic test fixtures + cleaner diffs in
audit logs).

Bump `philharmonic-api` 0.1.7 → 0.1.8 in `Cargo.toml`.
CHANGELOG entry under `[0.1.8] - 2026-05-11`:

> Extended `WhoamiResponse` with `permissions: Vec<String>`
> — the effective permission-atom set the route-protector
> enforces against for the authenticated caller. WebUI uses
> this to hide unreadable pages from the sidebar and to
> disable non-actionable buttons. Additive field; older
> clients ignore it.

## Deliverable B — WebUI permission state + hook

1. **`philharmonic/webui/src/api/client.ts`**: extend
   `WhoamiResponse`:

   ```ts
   export interface WhoamiResponse {
     tenant_id: string;
     auth_type: string;
     permissions: string[];
   }
   ```

2. **`philharmonic/webui/src/store/authSlice.ts`**: add
   `permissions: string[]` to the auth state. Update reducers
   so the login flow sets it from the whoami response;
   `clearToken` (or whatever the logout reducer is called)
   resets to `[]`. Add a `selectPermissions` selector.

3. **`philharmonic/webui/src/components/permissions.ts`**:

   ```ts
   // Add this map alongside the existing permissionAtoms /
   // permissionGroups / permissionsForGroup.
   export const navPermissions: Record<string, string | null> = {
     "/": null,                                 // Dashboard — always shown
     "/templates": "workflow:template_read",
     "/instances": "workflow:instance_read",
     "/endpoints": "endpoint:read_metadata",
     "/embed-datasets": "embed_dataset:read",
     "/principals": "tenant:principal_manage",
     "/roles": "tenant:role_manage",
     "/memberships": "tenant:role_manage",
     "/authorities": "tenant:minting_manage",
     "/audit": "audit:read",
     "/settings": "tenant:settings_read",
   };

   /** Pure helper: does `envelope` grant `atom`? */
   export function hasPermission(
     envelope: readonly string[],
     atom: string,
   ): boolean {
     return envelope.includes(atom);
   }
   ```

4. **`philharmonic/webui/src/hooks/usePermissions.ts`**
   (new — alongside `useT.ts`):

   ```ts
   import { useAppSelector } from "../store";
   import { selectPermissions } from "../store/authSlice";
   import { hasPermission } from "../components/permissions";

   export function usePermissions(): {
     all: readonly string[];
     has: (atom: string) => boolean;
     hasAny: (atoms: readonly string[]) => boolean;
   } {
     const all = useAppSelector(selectPermissions);
     return {
       all,
       has: (atom) => hasPermission(all, atom),
       hasAny: (atoms) => atoms.some((atom) => hasPermission(all, atom)),
     };
   }
   ```

   (Adjust imports / store-hook names to match the existing
   pattern — inspect `useT.ts` first.)

5. **Wire whoami's permissions into the login flow**: find
   the existing whoami call site (probably in `Login.tsx`
   after a successful token POST, or in an app-startup
   effect) and dispatch the new permissions into the auth
   slice. Inspect the existing flow — the token gets stored
   and `WhoamiResponse` is consumed somewhere; just extend
   that path.

## Deliverable C — Sidebar filtering by read-permission

`philharmonic/webui/src/components/Layout.tsx`:

The existing pattern is:

```tsx
const navItems: { to: string; labelKey: NavLabelKey; end?: boolean }[] = [
  // ...
];

// inside render:
{navItems.map((item) => (
  <NavLink key={item.to} to={item.to} ...>
    {t.nav[item.labelKey]}
  </NavLink>
))}
```

Change to:

```tsx
const { has } = usePermissions();

const visibleNavItems = navItems.filter((item) => {
  const required = navPermissions[item.to];
  if (required === null || required === undefined) {
    return true;            // no permission required (Dashboard)
  }
  return has(required);
});

// inside render:
{visibleNavItems.map((item) => ( ... ))}
```

The sidebar footer (language switcher + session token +
logout) is **not** filtered — those are session-scoped, not
permission-gated.

### Edge case: direct URL navigation to a hidden page

A caller who lacks `workflow:template_read` but who types
`/templates` directly into the URL bar reaches
`Templates.tsx`. The page's API calls will already 403 from
the server. Don't add a separate page-level permission
check — the API's 403 plus the empty-state-on-error pattern
that the pages already have is sufficient. Don't redirect
elsewhere either; the user clicked the URL intentionally.

## Deliverable D — Per-button disabled state across pages

For each `<button>` on each page whose `onClick` invokes an
action requiring a permission, change:

```tsx
<button className="button primary" type="button" onClick={create}>
  {t.templates.create}
</button>
```

to:

```tsx
const missing = !has("workflow:template_create");
// ...
<button
  className="button primary"
  type="button"
  onClick={create}
  disabled={missing /* || existing disable condition */}
  title={missing ? t.permissions.missingAtom("workflow:template_create") : undefined}
>
  {t.templates.create}
</button>
```

The mapping of button → required atom (Codex: cross-check
each against the route the button hits in `client.ts`):

| Page | Button | Required atom |
|---|---|---|
| `Templates.tsx` | Create template | `workflow:template_create` |
| `Templates.tsx` | Test in chat (per-row action from D13) | `workflow:instance_create` |
| `TemplateDetail.tsx` | Update | `workflow:template_create` (PATCH route uses the same atom) |
| `TemplateDetail.tsx` | Retire | `workflow:template_retire` |
| `TemplateDetail.tsx` | Test in chat (page-header button from D13) | `workflow:instance_create` |
| `Endpoints.tsx` | Create endpoint | `endpoint:create` |
| `EndpointDetail.tsx` | Rotate | `endpoint:rotate` |
| `EndpointDetail.tsx` | Retire | `endpoint:retire` |
| `EndpointDetail.tsx` | Show decrypted (if present) | `endpoint:read_decrypted` |
| `EmbedDatasets.tsx` | Create dataset | `embed_dataset:create` |
| `EmbedDatasets.tsx` | Import items (modal trigger) | `embed_dataset:create` |
| `EmbedDatasetDetail.tsx` | Save source items | `embed_dataset:update` |
| `EmbedDatasetDetail.tsx` | Retire | `embed_dataset:retire` |
| `Instances.tsx` | Create instance | `workflow:instance_create` |
| `InstanceDetail.tsx` | Execute step | `workflow:instance_execute` |
| `InstanceDetail.tsx` | Complete | `workflow:instance_execute` |
| `InstanceDetail.tsx` | Cancel | `workflow:instance_cancel` |
| `InstanceDetail.tsx` | Chat tab Send button | `workflow:instance_execute` |
| `Principals.tsx` | Create | `tenant:principal_manage` |
| `Principals.tsx` | Rotate / Retire (per-row) | `tenant:principal_manage` |
| `Roles.tsx` | Create | `tenant:role_manage` |
| `RoleDetail.tsx` | Update / Retire | `tenant:role_manage` |
| `Memberships.tsx` | Create / Retire | `tenant:role_manage` |
| `Authorities.tsx` | Create | `tenant:minting_manage` |
| `AuthorityDetail.tsx` | Update / Rotate / Bump epoch / Retire | `tenant:minting_manage` |
| `TenantSettings.tsx` | Save | `tenant:settings_manage` |

`AuditLog.tsx` is read-only; no buttons need this
treatment, but the sidebar entry is hidden via
deliverable C when `audit:read` is absent.

**i18n** (Deliverable B's tail): add to
`philharmonic/webui/src/i18n/en.ts` and
`philharmonic/webui/src/i18n/ja.ts`:

```ts
// en.ts
permissions: {
  // existing entries (groups, etc.)
  missingAtom: (atom: string) => `Missing permission: ${atom}`,
},

// ja.ts
permissions: {
  // existing entries
  missingAtom: (atom: string) => `権限が不足しています: ${atom}`,
},
```

If the existing `permissions` namespace already has a
function for this, just slot in. Don't duplicate.

**CSS** (`app.css`): the disabled styling for buttons
should give a clear visual signal. Add or extend:

```css
.button:disabled,
.button:disabled:hover {
  opacity: 0.55;
  cursor: not-allowed;
  background: var(--surface-2, var(--surface));
  color: var(--text-muted);
  /* no hover state */
}
```

(Adjust variable names to match the existing palette.
Inspect existing `.button` styles in `app.css` first.)

## Deliverable E — Sidebar-footer sticky CSS fix

Current layout in `app.css`:

- `.app-shell { display: grid; grid-template-columns: 252px
  1fr; min-height: 100vh; }`
- `.sidebar { display: flex; flex-direction: column;
  overflow-y: auto; }`
- `.sidebar-footer { position: sticky; bottom: 0;
  margin-top: auto; ... }`

The sticky fails when the **page** scrolls (window
scrollbar) rather than the sidebar's own
`overflow-y: auto`. Recommended fix: make the sidebar
itself pin to the viewport top, then the footer's existing
`position: sticky; bottom: 0` anchors correctly to the
sidebar's own scrolling area:

```css
.sidebar {
  position: sticky;
  top: 0;
  align-self: start;     /* required so the grid item doesn't
                            stretch to row height */
  max-height: 100vh;     /* keep sidebar within the viewport */
  /* existing flex / overflow-y: auto / padding etc. */
}
```

With those three additions, the sidebar is viewport-anchored
and the footer inside it stays at the bottom of the visible
sidebar regardless of nav-list length.

The mobile breakpoint at `app.css:957` (`.sidebar-footer`)
may need a parallel adjustment — inspect the
`@media (max-width: ...)` block and ensure the mobile
layout still works after the changes.

## Cross-deliverable: build, verification, artifacts

After all five deliverables are in place:

1. **TypeScript typecheck**:

   ```sh
   cd philharmonic/webui
   npx tsc --noEmit
   ```

2. **Production WebUI build**:

   ```sh
   ./scripts/webui-build.sh --production
   ```

3. **Workspace pre-landing.sh** (Rust + WebUI bundle):

   ```sh
   ./scripts/pre-landing.sh
   ```

4. **API breakage check** (the WhoamiResponse field
   addition is additive):

   ```sh
   ./scripts/check-api-breakage.sh philharmonic-api
   ```

Surface all outcomes in the structured-output report.

<structured_output_contract>
**Critical: emit before `task_complete`.**

Six sections in this order:

1. **Summary** — what landed across A/B/C/D/E. Include
   the verbatim string `RUN STATUS: COMPLETE` or
   `RUN STATUS: PARTIAL — <reason>`.

2. **Touched files** — exhaustive list with
   `(new|edited|deleted) <path> — <one-line note>`.
   Include the four regenerated artifacts in
   `philharmonic/webui/dist/`.

3. **Verification results** — exact commands + outcomes:
   - `npx tsc --noEmit` (in `philharmonic/webui/`).
   - `./scripts/webui-build.sh --production`.
   - `./scripts/pre-landing.sh`.
   - `./scripts/check-api-breakage.sh philharmonic-api`.

4. **Residual risks / known issues** — including:
   - Bundle-size delta from the new hook + per-page logic
     + i18n.
   - The semver-checks output for the WhoamiResponse field
     addition (expected: clean / additive, not breaking).
   - Whether you found pages whose buttons don't map cleanly
     to a single atom (e.g. a button that triggers multiple
     API calls with different atoms) and how you handled
     them.
   - Whether the sticky-footer fix had unexpected mobile
     breakpoint side effects.
   - Whether the existing `permissions` i18n namespace
     already had `missingAtom` or a similar function (if so,
     reuse rather than duplicate).
   - Any divergence from the per-button table above
     (e.g. a page button you couldn't find, or a button
     whose semantics differ from what the table assumed).
   - JP translation of `missingAtom` — whether you matched
     established style.
   - Whether you preserved Codex's permission `[Unreleased]`
     CHANGELOG anchor or added a new dated section.

5. **Git state** — current `HEAD` SHA in `philharmonic-api`
   submodule, `philharmonic` submodule, and parent
   workspace. Confirm no commits made.

6. **Open questions** — questions for Yuka or Claude:
   - Whether the "direct URL navigation to a hidden page"
     edge case should get a friendlier error page than the
     current 403 toast (out of scope here, but worth
     tracking).
   - Whether `endpoint:read_decrypted`'s gating on the
     "Show decrypted" button should also hide the button
     entirely (more sensitive operation; current spec
     disables but shows it).
   - JP terminology for "Missing permission" — does Yuka
     prefer 「権限が不足しています」 or another phrasing?
   - Whether the `navPermissions` map should also gate
     entire route entries in `App.tsx` (e.g. redirect to
     Dashboard if accessed without permission) — current
     spec lets pages handle 403s themselves.
</structured_output_contract>

<default_follow_through_policy>
- Suggested order: A → B → C → D → E. A is required for B
  to have a permission source; B is required for C/D to
  have a hook; E is CSS-only and independent.
- Run `npx tsc --noEmit` after each major surface
  (whoami extension → store wired → hook ready → first
  page converted) so type errors surface fast.
- For deliverable D, sweep pages in alphabetical order; do
  not skip any page in the table.
- If a page button's atom doesn't match the table — e.g.
  you find a button that calls two endpoints with different
  atoms — use `hasAny([...])` and surface in residuals.
- The disabled button MUST keep its existing role-styling
  classes (`button primary`, `button secondary`, `button
  danger`, etc.) — just add the `disabled` attribute. CSS
  `:disabled` selector handles the visual treatment.
- The `title` attribute is the simplest accessible tooltip
  surface; no need to introduce a tooltip component.
- The whoami extension is additive on the wire — older
  WebUI bundles ignore the new field. Don't introduce a
  WebUI fallback for "permissions field missing" beyond
  treating it as an empty array (= user has no
  permissions = everything hidden, which gracefully
  degrades).
</default_follow_through_policy>

<completeness_contract>
"Done" means:

- `WhoamiResponse` extended on the backend; whoami handler
  populates it; CHANGELOG + version bumped.
- WebUI store carries permissions; `usePermissions` hook
  ships; nav filtered; every page button in the table
  gates correctly.
- Sticky sidebar footer works (reachable without scrolling
  on the desktop layout regardless of nav-list length).
- All four verification commands green.
- The four artifacts in `philharmonic/webui/dist/`
  regenerated.
- Structured output report emitted before
  `task_complete`.

Partial completion is acceptable if a specific deliverable
hits a genuine blocker — but explicit `RUN STATUS:
PARTIAL — <reason>` is required, and the completed
deliverables must all be functional (no half-converted
pages with a mix of new-style and old-style buttons).
</completeness_contract>

<verification_loop>
For every surface:
1. Implement.
2. Type-check (TS) or `cargo check -p philharmonic-api`
   (Rust) — green.
3. Move on. Don't run webpack between TS surfaces.
4. Once all surfaces are clean, run the four verification
   commands in order.
5. Emit structured output report.
6. Then `task_complete`.

If you hit the context-window edge after step 4 passes:
**prioritise emitting the report**.
</verification_loop>

<missing_context_gating>
If you find yourself needing information not in this prompt
or the cited sources, **stop** and report in the structured
output's "Open questions" section.

Specifically: do **not**:

- Touch the route-protector enforcement in
  `philharmonic-api/src/routes/<module>::protected`. The
  server-side check stays.
- Add a new permission atom — the set is settled.
- Mint new permission groups or change
  `permissionsForGroup` semantics.
- Add a per-page Permission check that 403s in the page
  body — the API's 403 is sufficient.
- Add a new npm dependency.
- Edit `webpack.config.js` or `tsconfig.json`.
- Edit `mechanics-core`, `mechanics-config`, `mechanics`,
  `mechanics-worker`, or any connector crate. These are
  unrelated.
- Edit `HUMANS.md`, `CLAUDE.md`, `AGENTS.md`,
  `CONTRIBUTING.md`, `docs/`, `docs-jp/`, or `scripts/`
  content.
- Publish to crates.io. No `cargo publish`. Claude
  reviews and decides post-Codex.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `philharmonic-api/src/routes/whoami.rs` (edited).
- `philharmonic-api/Cargo.toml` (version bump).
- `philharmonic-api/CHANGELOG.md` (new entry).
- `philharmonic/webui/src/api/client.ts` (extend
  WhoamiResponse).
- `philharmonic/webui/src/store/authSlice.ts` (permissions
  field + reducers + selector).
- `philharmonic/webui/src/components/permissions.ts`
  (navPermissions map + hasPermission helper).
- `philharmonic/webui/src/hooks/usePermissions.ts` (new).
- `philharmonic/webui/src/components/Layout.tsx` (sidebar
  filter).
- `philharmonic/webui/src/pages/*.tsx` (per-button
  disabled state — Templates / TemplateDetail / Endpoints /
  EndpointDetail / EmbedDatasets / EmbedDatasetDetail /
  Instances / InstanceDetail / Principals / Roles /
  RoleDetail / Memberships / Authorities / AuthorityDetail /
  TenantSettings).
- `philharmonic/webui/src/app.css` (sidebar layout + sticky
  footer fix + disabled-button styling).
- `philharmonic/webui/src/i18n/en.ts` and
  `philharmonic/webui/src/i18n/ja.ts` (missingAtom helper).
- `philharmonic/webui/dist/{index.html, main.js, main.css,
  icon.svg}` (regenerated by `webui-build.sh --production`).
- `Cargo.lock` (regenerates).

Files NOT to touch (flag if you find a reason to):

- Any other Rust crate.
- The workspace `Cargo.toml` `[patch.crates-io]` block.
- Any connector implementation crate.
- `philharmonic/webui/src/pages/Login.tsx` only as needed
  to wire the whoami permissions into the auth slice on
  login — if the existing flow doesn't dispatch the
  permissions, add it there minimally.
- `philharmonic/webui/src/pages/Dashboard.tsx` (no action
  buttons; always shown).
- `philharmonic/webui/src/pages/AuditLog.tsx` (no action
  buttons; sidebar visibility handled by deliverable C).
- `philharmonic/webui/src/components/CodeEditor.tsx`,
  `permissions.ts`'s `permissionAtoms` constant,
  `SourceItemsEditor.tsx`, `DataConfigEditor.tsx`,
  `AbstractConfigEditor.tsx`, `MarkdownView.tsx`,
  `chatStorage.ts`.
- `HUMANS.md`, `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md`,
  `docs/`, `docs-jp/`, `scripts/`.

Git rules: signed-off + signed commits via
`scripts/commit-all.sh` only. **Codex never runs
`commit-all.sh`** — Claude commits after reviewing your
work. No `cargo publish`. No raw `git commit` /
`git push` / `git add` / `git reset` / `git rebase` /
`git revert`. Read-only `git log` / `git diff` /
`git show` is fine.
</action_safety>
</task>
