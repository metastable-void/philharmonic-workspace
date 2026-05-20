# Entity list `rev N` + RoleDetail edit + Memberships combobox + virt on `/_meta/version`

**Date:** 2026-05-20 (JST)
**Slug:** `entity-list-revision-seq-display`
**Round:** 01 — initial dispatch. Four related WebUI / API
topics bundled per Yuka's "also" directives this session:
(1) display the latest `revision_seq` as `rev N` per row
across entity list views, (2) add an edit form to the WebUI's
role detail page (display name + permissions), (3) replace
the raw UUID text inputs in the Memberships add form with a
searchable combobox that filters by UUID or by display name,
and (4) extend `/v1/_meta/version` with a `virtualization`
field cached at startup (using the same probe the `detect-virt`
xtask uses) and display it on the Dashboard's API status panel.
Builds on the modified-sort feature that landed earlier today
([`2026-05-20-0001-entity-list-sort-modified-opt-in-02.md`](2026-05-20-0001-entity-list-sort-modified-opt-in-02.md)).
**Subagent:** `codex:rescue`

## Motivation

### Topic 1 — `rev N` display per list row

WebUI list pages currently show only the entity's display name
and a few summary attributes. With the modified-sort feature in
place, the underlying `entity_revision` table is already the
source of truth for "how many times has this entity been
updated" — but nothing surfaces that to the operator browsing
the list. Yuka's call (2026-05-20): show the latest
`revision_seq` as a small inline `rev N` indicator per row, on
every list view that already supports cursor pagination, except
the audit log (events are immutable).

The modified-sort feature added `EntityStore::latest_revision_timestamps`
which already does the `INNER JOIN (SELECT MAX(revision_seq))`
batched lookup. This dispatch extends that helper to also
return the `revision_seq` so the same one-round-trip serves
both the sort key (when active) and the per-row display.

### Topic 2 — RoleDetail edit form

The WebUI's `pages/RoleDetail.tsx` currently renders a role's
display name, permissions, and metadata in read-only mode. The
API has supported `PATCH /v1/roles/{id}` with the request shape
`UpdateRoleRequest { display_name: Option<String>, permissions:
Option<Vec<String>> }` since
[`philharmonic-api/src/routes/roles.rs:148-205`](../../philharmonic-api/src/routes/roles.rs#L148-L205);
the gap is purely UI-side. Add an edit form to the page so an
operator with `tenant:role_manage` can update the role's
display name and permissions in-place.

Mirror the pattern from
[`pages/TenantSettings.tsx`](../../philharmonic/webui/src/pages/TenantSettings.tsx)
— same controlled-input + PATCH-on-submit shape, scoped to the
two editable fields. The existing
[`components/PermissionChecklist.tsx`](../../philharmonic/webui/src/components/PermissionChecklist.tsx)
component handles the permissions multi-select.

### Topic 3 — Memberships add: searchable principal / role combobox

The WebUI's `pages/Memberships.tsx::CreateMembershipForm`
currently has two raw `<input>` fields where the operator must
paste / type a `principal_id` and `role_id` as UUIDs (see
[`Memberships.tsx:208-247`](../../philharmonic/webui/src/pages/Memberships.tsx#L208-L247)).
That's hostile to humans — UUIDs aren't memorable. Yuka's call
(2026-05-20): replace each text input with a searchable
combobox that fetches the principal / role list once and lets
the operator filter the dropdown by **either** the UUID prefix
**or** the display name (case-insensitive substring match).

On select, the form stores the chosen UUID; the wire request
shape (`CreateMembershipRequest { principal_id, role_id }`) is
unchanged. Allow free-form fallback (e.g., the user pastes a
UUID that isn't in the loaded list) so the form keeps working
even when the dropdown's list is stale or filtered out the
intended target — accept the raw UUID and submit it as-is.

### Topic 4 — `virtualization` on `/v1/_meta/version` + Dashboard display

The `/v1/_meta/version` endpoint at
[`philharmonic-api/src/routes/meta.rs:48-53`](../../philharmonic-api/src/routes/meta.rs#L48-L53)
currently returns `{version, git_commit_sha}`. Extend it with
a third field `virtualization: String` that exposes the same
`systemd-detect-virt(1)`-style identifier the
[`detect-virt` xtask](../../xtask/src/bin/detect-virt.rs)
produces (`kvm`, `docker`, `wsl`, `none`, etc.). The probe
runs once at API-server startup and the result is cached in
`VersionState`; per-request cost is zero.

Display the cached value on the Dashboard's API status panel
([`pages/Dashboard.tsx:59-70`](../../philharmonic/webui/src/pages/Dashboard.tsx#L59-L70))
as a third metric card alongside `Version` and `Health`.

**Hard constraint (from Yuka, locked):** the detection
**MUST NOT fail, ever.** Any error / panic / I/O failure in
the probe path is caught and converted to the string `"none"`.
The probe runs at startup; if it panics or times out, the
server still starts with `virtualization = "none"`. The API
endpoint always returns a non-null string. Tests must cover
the failure-fallback path.

**Extraction shape is locked.** The new crate
`philharmonic-virt-detect/` is already scaffolded in-tree as
an in-workspace published crate (workspace `Cargo.toml`
already lists it; `[patch.crates-io]` table already points at
the local path). The scaffold ships a stub
`pub fn detect_virtualization() -> &'static str` that returns
`"none"` and satisfies the never-fail contract by
construction. Your job is to move the substantive probe logic
from `xtask/src/bin/detect-virt.rs` into this crate's
`src/lib.rs` and turn the xtask binary into a thin CLI
wrapper over the library's public API. Do **not** add a
`philharmonic-types::virt` module; the types crate is for
shared wire-format types, not host-environment probing.

## Hard constraints

- **No schema change.** The `(entity_id, created_at)` index
  added by the modified-sort feature already covers the
  GROUP-BY path; this dispatch only changes Rust + TS code.
- **Helper rename + return-type extension is allowed.**
  `latest_revision_timestamps` was added in the same session
  and has exactly one caller (`pagination::sort_key_values_for_rows`)
  — renaming it to `latest_revisions` and changing the return
  to carry both `revision_seq` and `created_at` is a clean
  edit, not a backward-compat concern. Pick whichever shape
  reads cleanest:
  - **Preferred:** define a small `pub struct LatestRevision
    { pub revision_seq: u64, pub created_at: UnixMillis }` in
    `philharmonic-store/src/entity.rs` and rename the method
    to `latest_revisions(&[Uuid]) -> HashMap<Uuid, LatestRevision>`.
  - Alternative: keep the method name, change the return to
    `HashMap<Uuid, (u64, UnixMillis)>`. Less explicit but
    fewer file touches.
- **Fetch happens unconditionally** now (previously gated to
  `SortMode::ModifiedDesc`). Every list response carries the
  `revision_seq` regardless of sort mode. The JOIN is cheap
  (covered by the existing PK + the modified-sort index);
  the marginal cost on `created_desc` lists is one batched
  query per page.
- **`revision_seq = 0` is rendered as `rev 0`** (entities
  with no `append_revision` call yet are conceptually
  pre-revision; their `revision_seq` is 0 by the
  `next_revision_seq` contract). Don't elide the field.
  Entities not in the returned map (defensive fallback,
  shouldn't normally happen) render `rev —` or omit the
  badge — Codex chooses; document the call.
- **Audit log exempt** — `routes/audit.rs::list_audit`
  doesn't get a revision_seq field. Events are not revisioned.
- **No raw `cargo`, no raw `git`.** Use the wrappers
  (`rust-lint.sh --phase check -p <crate> --quiet` for
  iteration, `pre-landing.sh` for final).
- **No `head` / `tail` on `scripts/*.sh` output** — soft-banned.
  Redirect to a file and `grep`/`Read` to slice.
- **You do not commit or push.** Leave the working tree dirty.
  Claude commits + pushes after reviewing.

## Per-file scope — Topic 1 (`rev N` display)

### `philharmonic-store/src/entity.rs`

- Define `pub struct LatestRevision { pub revision_seq: u64,
  pub created_at: UnixMillis }` (Clone + Copy + Debug + PartialEq + Eq).
- Rename trait method `latest_revision_timestamps` →
  `latest_revisions`; change return to
  `HashMap<Uuid, LatestRevision>`.
- Update default trait impl error message string.
- Update the in-tree `MockEntityStore` to populate both fields
  from its in-memory revision log.
- Update the existing test
  `latest_revision_timestamps_returns_highest_revision_timestamp`
  → rename to `latest_revisions_returns_highest_revision_data`
  and add assertions that `revision_seq` matches the expected
  max sequence number per entity.

### `philharmonic-store-sqlx-mysql/src/entity.rs`

- Implement the renamed method. SQL gains a column:
  `SELECT er.entity_id, er.revision_seq, er.created_at FROM ...`
  (everything else unchanged — the JOIN already selects the
  row with the max revision_seq).
- `sqlx::query_as` tuple becomes `(Vec<u8>, u64, i64)` — note
  the `BIGINT UNSIGNED` -> `u64` mapping; if sqlx's MySQL
  driver doesn't decode `u64` directly, fall back to `i64`
  and convert with `u64::try_from(...)` rejecting negatives.
- Update the existing integration test if it asserts on
  the previous return type.

### `philharmonic-api/src/store.rs`

- Trait-wrapper method renamed alongside the underlying
  trait; signature mirrors `philharmonic-store`'s.

### `philharmonic-api/src/pagination.rs`

- Rename `sort_key_values_for_rows` → `latest_revisions_for_rows`
  (or similar — `Codex` picks the name; the helper now
  fetches unconditionally, not just on `ModifiedDesc`).
- Always perform the batched lookup; return
  `HashMap<Uuid, LatestRevision>`.
- Update `sort_key_value_for_row` to read the
  `created_at` field of the `LatestRevision` map entry
  (still fall back to `row.created_at` when missing).

### Route layer (`philharmonic-api/src/routes/{authorities,embed_datasets,endpoints,memberships,principals,roles,workflows}.rs`)

Each route's `*_items` helper (e.g., `principal_items`,
`endpoint_items`, etc.) gains the latest-revisions map as a
parameter and stamps each summary struct's new
`revision_seq: u64` field. The list endpoint handlers fetch
the map once via the new helper (now unconditional) and
thread it into the items builder.

For each summary response struct (the names vary by
route — `PrincipalSummaryResponse`,
`EndpointSummaryResponse`-equivalent, `DatasetSummaryResponse`,
`AuthoritySummaryResponse`, `RoleSummaryResponse`,
`MembershipSummaryResponse`, `TemplateSummaryResponse`,
`InstanceSummaryResponse`):

- Add `pub revision_seq: u64` field (or whatever the
  crate's idiom for nullable revision_seq is — fall back to
  `0` per the "no revisions yet" contract).
- Populate from the map at struct-construction time.

### `routes/audit.rs::list_audit`

Unchanged. Events don't have revisions; the audit summary
struct keeps its current shape.

### `routes/workflows.rs::instance_history` and any other
revision-listing endpoint

These endpoints already enumerate revisions; `revision_seq`
is part of the revision itself, not a derived field. Keep
their shape as-is.

### WebUI (`philharmonic/webui/src/`)

- `api/client.ts` — add `revision_seq: number` to every
  summary interface that the API now returns it on. Match
  the API struct names.
- `pages/{Authorities,AuthorityDetail,EmbedDatasets,Endpoints,
  Instances,Memberships,Principals,RoleDetail,Roles,
  Templates}.tsx` — render a small `rev N` indicator next to
  each row's display name. The exact placement is your
  call; aim for compact and unobtrusive (e.g., a small muted
  badge or trailing parenthetical). Use a new CSS class
  (`.revision-badge` or similar) so the styling is
  consistent across pages.
- `app.css` — `.revision-badge` style: muted color, smaller
  font (e.g., 0.78rem), maybe a subtle border or pill shape;
  picks up from the existing CSS variable palette (`--muted`).
- Localize the `rev` prefix via `i18n/en.ts` + `i18n/ja.ts`
  if the prefix needs translation. English `rev N` /
  Japanese `第N版` are reasonable. Your judgment; document
  the choice.
- `webui/dist/{main.css,main.css.map,main.js,main.js.map}` —
  regenerate via `./scripts/webui-build.sh --production`.

## Per-file scope — Topic 2 (RoleDetail edit form)

The API side is already in place — `PATCH /v1/roles/{id}` with
`UpdateRoleRequest { display_name?, permissions? }` exists at
[`routes/roles.rs:148-205, 274-278`](../../philharmonic-api/src/routes/roles.rs#L148-L205).
No Rust changes needed for this topic. Pure WebUI work.

### `philharmonic/webui/src/pages/RoleDetail.tsx`

- Mirror the edit-form pattern from `pages/TenantSettings.tsx`:
  - Controlled `displayName` state initialised from the loaded
    role.
  - Controlled `permissions` state initialised from the loaded
    role's permissions array.
  - Submit handler PATCHes `/v1/roles/{id}` with the
    `UpdateRoleRequest` shape (both fields optional; send only
    what changed, or always send both — your call, document the
    choice). On success, refresh the displayed role and clear
    any dirty / notice flags.
- Gate the form behind `tenant:role_manage` permission via the
  existing `usePermissions().has(...)` pattern already used at
  the top of the file (the `missingManage` flag).
- Use the existing `PermissionChecklist` component for the
  permissions multi-select. Same component the role-CREATE
  flow uses (`pages/Roles.tsx`).
- The `updated_at` display (rendered via `formatTimestamp` at
  line 128) keeps working unchanged after a successful save —
  the helper itself was switched to ISO-local-TZ format in
  commit `727edd0` before this dispatch.
- Surface API errors via the existing `setError` / `setNotice`
  state hooks already on the page.
- The retire / unretire action stays exactly as-is; it's a
  separate flow.

### `philharmonic/webui/src/api/client.ts`

- Add an `updateRole(id, request)` typed helper if it
  meaningfully reduces the call-site boilerplate. If the
  per-page pattern of inline
  `apiCall<RoleDetailResponse>(\`roles/${id}\`, { method:
  "PATCH", body: JSON.stringify(req) })` reads cleaner,
  skip the helper. Document the choice.
- Export `UpdateRoleRequest` interface mirroring the API
  struct: `{ display_name?: string; permissions?: string[] }`.

### `philharmonic/webui/src/i18n/{en,ja}.ts`

- Add edit-form labels under the existing role section. Reuse
  existing shared keys (`t.common.save`, `t.common.saving`,
  etc.) where possible to avoid duplication.

### `philharmonic/webui/dist/*`

- Regenerated via `./scripts/webui-build.sh --production` in
  the same final step Topic 1 already does.

## Per-file scope — Topic 3 (Memberships searchable combobox)

API side is already in place — the existing
`/v1/principals` and `/v1/roles` list endpoints provide the
data; the `CreateMembershipRequest` wire shape is unchanged.
Pure WebUI work.

### `philharmonic/webui/src/components/EntityCombobox.tsx` (NEW)

Create a reusable searchable-combobox component. Shape:

```ts
interface EntityComboboxProps {
  kind: "principal" | "role";
  value: string;            // currently-selected UUID (or empty / raw paste)
  onChange: (value: string) => void;
  required?: boolean;
  disabled?: boolean;
  // Optional: a `title` for the accessibility tooltip when disabled.
}
```

Behaviour:

- On mount, fetch the kind's full list via
  `apiCall<PaginatedResponse<...>>(\`principals\`)` or
  `\`roles\`` (use a generous `limit=200` — the
  workspace-side cap is `MAX_LIMIT = 200`). For tenants with
  more than 200 of the kind, the dropdown shows the first
  page; the free-form fallback (see below) covers the rest.
  No need to walk pagination — keep it simple.
- Render a `<input>` whose value is what the user has typed
  (controlled by component-local state). Below the input,
  render the list of matching entries from the fetched list.
  Match an entry if:
  - the entry's UUID **starts with** the input value, OR
  - the entry's display name (case-insensitive) **contains**
    the input value as a substring.
- On click of a list entry, set the form-bound `value` to the
  entry's UUID and clear the local typed-text (or set it to
  the display name — your choice; document it).
- If the user types a full UUID directly (and it doesn't match
  any loaded entry), accept it as-is on form-submit — pass
  through whatever's in the input field. This is the
  free-form fallback for stale dropdown / large-tenant cases.
- Empty entry list (still loading, or fetch failed) renders a
  plain `<input>` so the form is still usable.
- Keyboard: Up / Down to navigate the dropdown, Enter to
  select, Escape to dismiss. Nice-to-have, not blocking;
  skip if it's awkward without a UI library.
- A11y: associate the input with the dropdown via
  `aria-controls` / `aria-expanded` / `role="combobox"`
  attributes. Keep it minimal but correct.

### `philharmonic/webui/src/pages/Memberships.tsx`

- Replace the two raw `<input>` fields in
  `CreateMembershipForm` (lines 239-246) with two
  `<EntityCombobox kind="principal" value=principalId
  onChange=setPrincipalId/>` and `<EntityCombobox
  kind="role" .../>`. Submit path unchanged — the
  `CreateMembershipRequest` shape stays as
  `{principal_id, role_id}`.

### `philharmonic/webui/src/api/client.ts`

- No new interface needed — the existing
  `PrincipalSummary` / `RoleSummary` shapes (with the
  `revision_seq` field added by Topic 1) carry both UUID
  and display name. The combobox reads those.

### `philharmonic/webui/src/app.css`

- Add `.entity-combobox` + child class styles:
  - `.entity-combobox` is the container (relative
    positioning for the dropdown).
  - `.entity-combobox-list` is the dropdown panel
    (absolute, scrollable, max-height ~300px, border /
    shadow / background matching the workspace palette).
  - `.entity-combobox-option` is one row (clickable,
    hover highlight).
  - `.entity-combobox-empty` shows when no entries match
    (a faint "no matches; paste a UUID directly" hint).
- Match the existing form-field aesthetic — same
  border-radius / colors as `.field input` etc.

### `philharmonic/webui/src/i18n/{en,ja}.ts`

- Add combobox-related labels (placeholder text, the
  "no matches" hint, etc.). Reuse `t.common.*` where
  possible.

## Per-file scope — Topic 4 (`virtualization` field)

### Detection logic — extract into a shareable home

The probe currently lives in
[`xtask/src/bin/detect-virt.rs`](../../xtask/src/bin/detect-virt.rs)
as part of the xtask binary. Yuka's constraint: do NOT put
platform-detection code in `philharmonic-types`. Two
remaining options:

1. **Preferred: new tiny crate**
   (`philharmonic-virt-detect/` — pick a name that reads
   cleanly). Both `xtask/` and the API deployment binary
   depend on it. `xtask/src/bin/detect-virt.rs` becomes a
   thin CLI wrapper that calls the library's `pub fn
   detect() -> VmId`.
2. **Last resort: duplicate copy** in the API deployment
   binary with a `// keep in sync with
   xtask/src/bin/detect-virt.rs` comment. Only acceptable
   if option 1 turns out to have a non-obvious blocker.
   Document the blocker if you take this path.

The extracted API needs at least:

```rust
/// Probe for the current virtualization / container
/// environment. Returns the systemd-detect-virt-style
/// identifier (e.g., "kvm", "docker", "wsl") or "none" if
/// no virtualization is detected.
///
/// **Never fails.** Any internal error (I/O, unexpected
/// fixture contents, CPUID failure, etc.) is caught and
/// converted to "none". Safe to call at startup before any
/// logging is wired up.
pub fn detect_virtualization() -> &'static str;
```

The xtask CLI keeps its existing surface (the test fixtures
under `xtask/tests/fixtures/detect-virt/` still pass).

### `philharmonic-api/src/routes/meta.rs`

- Add `virtualization: &'static str` (or owned `String` —
  your call) to `VersionResponse` and `VersionState`.
- The route handler returns `state.virtualization` verbatim.

### `philharmonic-api/src/lib.rs` (or wherever the
deployment binary builds the axum router)

- At startup, call `detect_virtualization()` once and stash
  the result in `VersionState`. The call must complete
  before the router is built; if it somehow takes longer
  than a few hundred milliseconds, that's a bug in the
  extracted module, not a reason to delay startup further.
- Detection wrapped in
  `std::panic::catch_unwind(|| detect_virtualization())` —
  belt-and-suspenders on top of the library's own
  never-fail contract. If `catch_unwind` returns `Err`,
  cache `"none"`. This makes the never-fail contract
  bulletproof at the boundary.

### Tests

- **Detection library**: an `#[ignore]`-gated integration
  test that runs the probe and asserts the result is one of
  the documented IDs (or "none"). Plus a unit test that
  forces a failure path (e.g., a fixture filesystem that
  panics on read) and asserts the result is "none".
- **API endpoint**: assert that `VersionResponse.virtualization`
  is always a non-null string in the test harness's
  baseline `_meta/version` test.

### `philharmonic/webui/src/api/client.ts`

- Add `virtualization: string` to the `VersionResponse`
  interface.

### `philharmonic/webui/src/pages/Dashboard.tsx`

- Add a third `<article className="metric">` card after
  Version and Health, with `metric-label` "Virtualization"
  and the cached value as the `<strong>` content. Use the
  same `loading / unavailable` fallback shape the other
  cards use.

### `philharmonic/webui/src/i18n/{en,ja}.ts`

- Add `t.dashboard.virtualization` label string (English:
  "Virtualization"; Japanese: "仮想化" or "仮想化環境"
  — Codex picks).

### `philharmonic/webui/dist/*`

- Regenerated in the same final step as the other topics.

## Tests

- `philharmonic-store/src/entity.rs` — extend the existing
  `latest_revision_timestamps_returns_highest_revision_timestamp`
  test (renamed) to assert `revision_seq` matches expected
  max per entity.
- `philharmonic-store-sqlx-mysql/tests/integration.rs` — if
  the existing test asserts on the return type, update it.
- `philharmonic-api/tests/` — pick two representative
  endpoint integration tests (the modified-sort feature used
  `list_endpoints` and `list_templates`) and add assertions
  that the response carries `revision_seq` matching the
  underlying revisions. A row whose latest is revision 3
  should serialise as `"revision_seq": 3`.
- No new pagination unit tests needed; the existing
  cursor/sort tests still cover the cursor surface unchanged.

### Topic 2 tests

- No first-party JS / TS test framework exists in
  `philharmonic/webui/` (confirmed by file scan). Verification
  is "TS compiles + manual smoke against the dev server"
  — `webui-build.sh --production` exiting 0 is the gate.
- No API-side test additions needed for Topic 2; the
  `PATCH /v1/roles/{id}` handler and its tests are pre-
  existing and unchanged by this dispatch.

### Topic 3 tests

- Same "TS compiles + manual smoke" gate as Topic 2 — no
  JS / TS test framework available.
- Pre-flight check: confirm `apiCall<PaginatedResponse<
  PrincipalSummary>>(\`principals?limit=200\`)` returns the
  expected shape (the typed helper already exists; just a
  cross-check that the existing API endpoints accept the
  query string).

## Verification

```sh
./scripts/pre-landing.sh
./scripts/webui-build.sh --production
```

Both must exit clean.

## Hand-off shape — Codex does not commit

Leave the working tree dirty across `philharmonic/`,
`philharmonic-api/`, `philharmonic-store/`,
`philharmonic-store-sqlx-mysql/`, and parent (`Cargo.lock`).
Claude commits + pushes after reviewing.

- No `./scripts/commit-all.sh` (any flags).
- No raw `git commit` / `push` / `add` / `reset` /
  `rebase` / `amend`.
- No raw read-only `git status` / `log` / `diff` — use
  the wrappers (`status.sh`, `log.sh`, `heads.sh`).
- No `./scripts/push-all.sh`, no `./scripts/publish-crate.sh`.
- No `HUMANS.md` edits.

## Codex report (encouraged)

If anything non-obvious surfaces — sqlx's `u64` decoding
quirks for `BIGINT UNSIGNED`, an awkward call site after the
helper unconditionalises, a WebUI layout question — write a
short report to
`docs/codex-reports/2026-05-20-0002-entity-list-revision-seq-display.md`.
Routine specified-and-shipped work doesn't need one.

## Outcome

Implemented all four topics. Entity list summaries now carry `revision_seq`
from the renamed `latest_revisions` store API, pagination fetches latest
revision metadata unconditionally, and the WebUI renders localized `rev N`
badges with missing revision metadata falling back to `rev 0`. Role detail now
has a `tenant:role_manage`-gated controlled edit form that patches display name
and permissions using the existing role update API, while membership creation
uses a reusable `EntityCombobox` that filters loaded principals/roles and still
passes through pasted UUIDs outside the first 200 results. Virtualization
detection moved into `philharmonic-virt-detect`, `xtask detect-virt` became a
thin printer, the API caches the startup probe result in `VersionState`, and
the Dashboard displays it as a third metric card. Verification passed with
`./scripts/pre-landing.sh` ending in `=== pre-landing: all checks passed ===`
and `./scripts/webui-build.sh --production` exiting 0 with only pre-existing
webpack size warnings.

---

<task>
Four related WebUI / API topics in one dispatch:

**Topic 1 — `rev N` per row in entity list views.**
Display the entity's latest `revision_seq` as a small `rev N`
indicator per row in the WebUI list views (all kinds except
audit log). The data plumbing is straightforward: the
modified-sort feature already added a batched
`EntityStore::latest_revision_timestamps` helper that does
the `INNER JOIN (SELECT MAX(revision_seq))` lookup — extend
that helper to also return `revision_seq` and fetch it
unconditionally (not just on `?sort=modified_desc`).

**Topic 2 — RoleDetail edit form.**
Add an edit form to `philharmonic/webui/src/pages/RoleDetail.tsx`
so an operator with `tenant:role_manage` can edit a role's
display name and permissions in-place. API support is already
in place at `PATCH /v1/roles/{id}` with `UpdateRoleRequest
{ display_name?, permissions? }`; gap is purely WebUI.
Mirror the controlled-input + PATCH-on-submit pattern from
`pages/TenantSettings.tsx`; use the existing
`components/PermissionChecklist.tsx` for the permissions
multi-select.

**Topic 3 — Memberships searchable principal / role combobox.**
Replace the two raw text inputs in
`pages/Memberships.tsx::CreateMembershipForm` (lines 239-246)
with a new reusable `EntityCombobox` component that fetches
the kind's full list and filters by UUID prefix OR by display
name substring (case-insensitive). On select, store the
chosen UUID. Free-form fallback: if the user pastes a raw
UUID that doesn't match any loaded entry, accept it as-is
on submit. The wire shape (`CreateMembershipRequest`) is
unchanged.

**Topic 4 — `virtualization` on `/v1/_meta/version` + Dashboard.**
Extend `VersionResponse` (returned from
`philharmonic-api/src/routes/meta.rs::version`) with a
`virtualization` field exposing the same systemd-detect-virt-
style identifier the `detect-virt` xtask produces. Probe runs
once at API-server startup; result cached in `VersionState`;
per-request cost is zero. **The probe MUST NOT fail, ever** —
any error / panic / I/O failure converts to the string
`"none"` (Yuka's locked constraint). The deployment-binary
startup path wraps the call in `panic::catch_unwind` as
belt-and-suspenders on top of the library's own never-fail
contract. WebUI renders the cached value as a third metric
card on the Dashboard's API status panel, alongside Version
and Health.

**Hard constraints:**

- **No schema change.** The `entity_revision (entity_id,
  created_at)` index added by the modified-sort feature
  already covers the GROUP-BY path.
- **Rename + return-type-extend the helper.** Preferred shape:
  `pub struct LatestRevision { pub revision_seq: u64, pub
  created_at: UnixMillis }` in
  `philharmonic-store/src/entity.rs`; rename the trait method
  to `latest_revisions(&[Uuid]) -> HashMap<Uuid, LatestRevision>`.
  Acceptable alternative: keep the name, change return to
  `HashMap<Uuid, (u64, UnixMillis)>` (less explicit, less
  ceremony). Document your choice.
- **Fetch unconditionally** in `pagination.rs` (previously
  gated to `SortMode::ModifiedDesc`). Every list page now
  carries `revision_seq` regardless of sort mode.
- **`revision_seq = 0`** (entities with no revisions yet)
  renders as `rev 0` on the WebUI. Missing-from-map (defensive
  fallback) renders as `rev —` or hides the badge — your
  call, document it.
- **Audit log exempt** (`routes/audit.rs::list_audit`).
- **No raw cargo, no raw git.** Use the wrappers
  (`rust-lint.sh --phase check -p <crate> --quiet` for
  iteration; `pre-landing.sh` for final).
- **No `head`/`tail` on `scripts/*.sh` output** — soft-banned.
- **You do not commit or push.** Leave dirty across
  `philharmonic/`, `philharmonic-api/`, `philharmonic-store/`,
  `philharmonic-store-sqlx-mysql/`, parent (`Cargo.lock`).

**Reference docs (authoritative if they contradict this prompt):**

- The full preamble above (Per-file scope, Tests, Verification,
  Hand-off shape).
- The modified-sort archive at
  `docs/codex-prompts/2026-05-20-0001-entity-list-sort-modified-opt-in-{01,02}.md`
  — same architecture, mostly the same surfaces touched.
- `CONTRIBUTING.md` §§4, 5, 10.3, 11.
- `CLAUDE.md` §"Hard rules vs. soft rules" + the exec-summary
  bullets for raw cargo / raw git / head-tail.

**Per-file scope summary (Topic 1 — `rev N` display):**

- `philharmonic-store/src/entity.rs` — define `LatestRevision`
  struct, rename trait method, update default impl error,
  update `MockEntityStore`, extend the existing tests.
- `philharmonic-store-sqlx-mysql/src/entity.rs` — rename
  impl, extend SQL `SELECT` to include `revision_seq`,
  update query_as tuple shape.
- `philharmonic-api/src/store.rs` — trait wrapper rename.
- `philharmonic-api/src/pagination.rs` — rename helper, make
  unconditional, update `sort_key_value_for_row` to read
  `created_at` field from `LatestRevision`.
- `philharmonic-api/src/routes/{authorities,embed_datasets,
  endpoints,memberships,principals,roles,workflows}.rs` —
  each summary struct gains `revision_seq: u64`; each
  list-endpoint handler fetches the map once and threads it
  into the items builder. Audit and revision-listing
  endpoints unchanged.
- `philharmonic/webui/src/api/client.ts` — add `revision_seq:
  number` to summary interfaces.
- `philharmonic/webui/src/pages/{Authorities,AuthorityDetail,
  EmbedDatasets,Endpoints,Instances,Memberships,Principals,
  RoleDetail,Roles,Templates}.tsx` — render `rev N` indicator.
- `philharmonic/webui/src/app.css` — add `.revision-badge`
  styling (muted, smaller, picks up the existing CSS palette).
- `philharmonic/webui/src/i18n/{en,ja}.ts` — localize the
  `rev` prefix if needed.

**Per-file scope summary (Topic 2 — RoleDetail edit form):**

- `philharmonic/webui/src/pages/RoleDetail.tsx` — add an
  edit form mirroring `pages/TenantSettings.tsx`'s
  controlled-input + PATCH-on-submit pattern. Editable
  fields: `display_name` (text input) and `permissions`
  (via `components/PermissionChecklist.tsx`). Gate behind
  `tenant:role_manage` via the existing `usePermissions`
  hook. Retire action unchanged.
- `philharmonic/webui/src/api/client.ts` — export
  `UpdateRoleRequest` interface; optionally add an
  `updateRole(id, request)` typed helper.
- `philharmonic/webui/src/i18n/{en,ja}.ts` — edit-form
  labels; reuse `t.common.save` etc. where possible.

**Per-file scope summary (Topic 3 — Memberships combobox):**

- `philharmonic/webui/src/components/EntityCombobox.tsx`
  (NEW) — reusable searchable combobox. Props:
  `{kind: "principal"|"role", value: string,
  onChange: (string)=>void, required?, disabled?}`. Fetches
  the list on mount (`limit=200`), filters by UUID prefix
  OR display-name substring, free-form fallback for raw
  UUID paste.
- `philharmonic/webui/src/pages/Memberships.tsx` — replace
  the two raw `<input>` fields in `CreateMembershipForm`
  (lines 239-246) with two `EntityCombobox` instances.
  Submit path unchanged.
- `philharmonic/webui/src/app.css` — add
  `.entity-combobox` + child class styles matching the
  workspace form-field aesthetic.
- `philharmonic/webui/src/i18n/{en,ja}.ts` — combobox
  placeholder + "no matches" hint.

**Per-file scope summary (Topic 4 — virtualization field):**

- Extract `xtask/src/bin/detect-virt.rs` core probe logic
  into a shareable home (new crate, module on existing
  crate, or — last resort — duplicated copy). Public API
  must include a never-fail `detect_virtualization() ->
  &'static str` that returns one of the documented IDs or
  `"none"` on any error.
- `xtask/src/bin/detect-virt.rs` — becomes a thin CLI
  shim over the extracted module; existing fixtures under
  `xtask/tests/fixtures/detect-virt/` keep passing.
- `philharmonic-api/src/routes/meta.rs` — add
  `virtualization` field to `VersionResponse` +
  `VersionState`; route handler returns the cached value.
- `philharmonic-api/src/lib.rs` (or the API-server
  deployment binary's startup path) — probe once at
  startup wrapped in `panic::catch_unwind`; cache in
  `VersionState`. Belt-and-suspenders "never fail" boundary.
- `philharmonic-api/tests/` — assert `VersionResponse`
  always carries a non-null `virtualization` string.
- `philharmonic/webui/src/api/client.ts` — add
  `virtualization: string` to `VersionResponse` interface.
- `philharmonic/webui/src/pages/Dashboard.tsx` — third
  metric card after Version and Health.
- `philharmonic/webui/src/i18n/{en,ja}.ts` — add
  `t.dashboard.virtualization` label string.

**Shared per-file scope:**

- `philharmonic/webui/dist/{main.css,main.css.map,main.js,
  main.js.map}` — regenerated via
  `./scripts/webui-build.sh --production`. Same final
  regeneration step covers both topics.

**Verification (must run + pass before declaring done):**

- `./scripts/pre-landing.sh` clean.
- `./scripts/webui-build.sh --production` exit 0.

<default_follow_through_policy>
Land the store rename + return-type extension, the unconditional
fetch in pagination, all summary struct extensions across the
~8 route files, the WebUI client.ts + list-page renders + CSS,
and the regenerated dist artefacts in this single round.
Partial results (e.g., "store + routes done, WebUI pending")
are NOT complete.

If a hard blocker surfaces (e.g., sqlx's MySQL driver doesn't
decode `u64` cleanly for `BIGINT UNSIGNED` and the workaround
requires invasive refactoring), STOP and report.
</default_follow_through_policy>

<completeness_contract>
"Complete" means:

1. `EntityStore` trait method renamed (or kept with new return
   type — your choice, documented); default-impl error
   message updated; in-tree `MockEntityStore` implements the
   new shape.
2. `philharmonic-store-sqlx-mysql` impl renamed/updated; SQL
   selects `revision_seq` in addition to `created_at`; tuple
   decoding handles `BIGINT UNSIGNED` cleanly.
3. `philharmonic-api/src/store.rs` trait wrapper renamed.
4. `philharmonic-api/src/pagination.rs` helper renamed and
   made unconditional; `sort_key_value_for_row` reads the
   `created_at` portion.
5. Every entity list route's summary struct gains
   `revision_seq: u64`, populated from the batched lookup.
6. `routes/audit.rs` unchanged.
7. WebUI client.ts summary interfaces gain `revision_seq:
   number`.
8. Each WebUI list page renders a `rev N` indicator
   (CSS class `.revision-badge` or similar).
9. **Topic 2:** `pages/RoleDetail.tsx` carries an edit form
   for display name + permissions, gated by
   `tenant:role_manage`, mirroring the
   `pages/TenantSettings.tsx` pattern. PATCH on submit;
   reload on success.
10. **Topic 2:** `api/client.ts` exports
    `UpdateRoleRequest`; optionally adds an `updateRole`
    helper.
11. **Topic 3:** `components/EntityCombobox.tsx` is the
    new reusable searchable combobox; both inputs in
    `pages/Memberships.tsx::CreateMembershipForm` use it;
    `app.css` carries `.entity-combobox` styling matching
    the form-field aesthetic; free-form raw-UUID paste
    still works.
12. **Topic 4:** Virtualization probe extracted out of the
    `detect-virt` xtask into a shareable home (new crate
    preferred; duplicate-in-binary last resort; **NOT**
    `philharmonic-types`). Public API: never-fail
    `detect_virtualization() -> &'static str` returning a
    documented ID or `"none"` on any error.
13. **Topic 4:** API-server startup probes once, wraps in
    `panic::catch_unwind`, caches result in `VersionState`.
    `/v1/_meta/version` returns `virtualization` field
    (always a non-null string).
14. **Topic 4:** Dashboard renders the cached value as a
    third metric card alongside Version + Health.
15. `webui/dist/*` regenerated via `webui-build.sh --production`
    (one regeneration covers all four topics).
16. `./scripts/pre-landing.sh` passes.
17. `./scripts/webui-build.sh --production` exits 0.
18. Working tree dirty across `philharmonic/`,
    `philharmonic-api/`, `philharmonic-store/`,
    `philharmonic-store-sqlx-mysql/`, parent (`Cargo.lock`,
    plus new crate if you create one). No commits.
19. Session summary lists which submodules + parent are
    dirty + which naming choice you made (Topic 1: rename
    vs keep) + how you handled `revision_seq = 0` /
    missing-from-map + whether you added the `updateRole`
    helper (Topic 2) or inlined the PATCH + how the
    EntityCombobox keyboard / a11y story shipped (Topic 3,
    full keyboard nav or basic click-only) + which
    extraction shape you chose for Topic 4 (new crate vs
    duplicate copy).
20. `## Outcome` section of this prompt file updated with
    files touched, choices made, residual risks, hand-off
    SHAs — covering ALL FOUR topics.
</completeness_contract>

<verification_loop>
Mid-iteration:

  ./scripts/rust-lint.sh --phase check -p philharmonic-store --quiet
  ./scripts/rust-lint.sh --phase check -p philharmonic-store-sqlx-mysql --quiet
  ./scripts/rust-lint.sh --phase check -p philharmonic-api --quiet
  ./scripts/rust-test.sh philharmonic-store
  ./scripts/rust-test.sh philharmonic-store-sqlx-mysql
  ./scripts/rust-test.sh philharmonic-api

Final:

  ./scripts/pre-landing.sh
  ./scripts/webui-build.sh --production

No raw `cargo`. The wrappers cover fmt + check + clippy + doc
+ test with the right `CARGO_TARGET_DIR`.
</verification_loop>

<missing_context_gating>
Before editing, run:

  ./scripts/status.sh

Parent + every submodule should print `(clean)`. If anything
else is dirty, STOP and report.

If the `latest_revision_timestamps` method no longer exists
(name changed since this prompt was written), STOP and report
— don't second-guess; the prompt is current as of 2026-05-20
JST.
</missing_context_gating>

<action_safety>
- You do not commit, push, or publish.
- Use the script wrappers; no raw cargo, no raw git.
- POSIX-ish host: no bash-only constructs in any shell.
- JST is the workspace timezone; today is 2026-05-20 (Wed).
- No `head` / `tail` on `scripts/*.sh` output. Redirect to a
  file and `grep` / `Read` if you need to slice.
- No edits to `HUMANS.md`.
- `CARGO_TARGET_DIR=target-main` is set by the wrappers; if
  you call cargo directly, set it yourself.
</action_safety>

<structured_output_contract>
Return:

1. **Summary** (2-3 sentences): naming choice (rename vs keep);
   the unconditionalisation of the helper; the summary structs
   extended; the WebUI render style chosen.
2. **Touched files**: grouped by submodule + parent.
3. **`LatestRevision` definition** (or the tuple shape if you
   went with the alternative): paste the type definition.
4. **`latest_revisions` SQL**: paste the updated `SELECT`
   clause + tuple decoding.
5. **`pagination::sort_key_value_for_row` diff**: paste before
   / after.
6. **Summary-struct field**: paste an example summary struct
   (one of the ~8) before / after.
7. **WebUI render**: paste one list-page render block
   showing the `rev N` indicator.
8. **`.revision-badge` CSS**: paste the new rule.
9. **i18n decision**: localized the `rev` prefix or kept
   English-only; if localized, paste both en + ja entries.
10. **Topic 2 (RoleDetail edit form):** paste the new
    edit-form JSX block (just the form region of
    `RoleDetail.tsx`, not the whole file); note whether
    you added an `updateRole` helper or inlined the PATCH;
    list the new i18n keys.
11. **Topic 3 (Memberships combobox):** paste the
    `EntityCombobox` component's prop interface + a brief
    description of the filter logic (UUID prefix vs
    display-name substring) + how free-form fallback
    works; paste the `.entity-combobox` CSS rules; note
    the keyboard / a11y story shipped (full nav or
    click-only).
12. **Topic 4 (virtualization field):** name + location of
    the extracted detection crate / module; paste the
    `detect_virtualization` public signature + a one-line
    description of how the never-fail contract is enforced;
    paste the new `VersionResponse` struct (Rust side) and
    the new Dashboard metric-card JSX; paste the
    `Cargo.toml` dep additions if you created a new crate.
13. **Test additions**: list new / modified test names and
    what they assert.
13. **Verification results**:
    - `pre-landing.sh` PASS / FAIL.
    - `webui-build.sh --production` exit 0 / non-zero.
14. **Working-tree state at hand-off**: list dirty
    submodules + parent. No commits.
15. **Codex report**: presence (dirty in tree; Claude
    commits) or skipped.
16. **Residual risks**: anything Yuka should know before
    ship — e.g., a list page where the badge crowded the
    layout, a sqlx decoding edge case, a kind whose
    revision_seq is always 0 by construction, a combobox
    layout regression on narrow viewports.
17. **Outcome paragraph** for the prompt-archive file: 4-6
    sentences ready to paste into `## Outcome` covering
    all three topics.
</structured_output_contract>
</task>
