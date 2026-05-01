# Complete WebUI: all missing CRUD pages (initial dispatch)

**Date:** 2026-05-01
**Slug:** `webui-complete-crud`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

The WebUI currently covers workflows (templates + instances),
audit log, and tenant settings. Five major API resource groups
have no WebUI support: Endpoints, Principals, Roles, Role
Memberships, and Minting Authorities. Without these, operators
cannot manage endpoint configs (required for workflow execution),
user access, or token minting from the browser — making the
product unusable for production without raw API calls.

## References

- Existing WebUI pages: `philharmonic/webui/src/pages/*.tsx`
- API client: `philharmonic/webui/src/api/client.ts`
- API routes (authoritative for request/response shapes):
  - `philharmonic-api/src/routes/endpoints.rs`
  - `philharmonic-api/src/routes/principals.rs`
  - `philharmonic-api/src/routes/roles.rs`
  - `philharmonic-api/src/routes/memberships.rs`
  - `philharmonic-api/src/routes/authorities.rs`

## Context files pointed at

- `philharmonic/webui/src/api/client.ts`
- `philharmonic/webui/src/pages/*.tsx`
- `philharmonic/webui/src/components/Layout.tsx`
- `philharmonic/webui/src/App.tsx`
- `philharmonic/webui/src/app.css`

## Outcome

Pending — will be updated after Codex run.

---

## Prompt (verbatim)

<task>
Add complete CRUD pages for five API resource groups to the
WebUI. The WebUI lives at `philharmonic/webui/src/`. It uses
React 19, Redux Toolkit, TypeScript, and the existing `apiCall`
helper in `src/api/client.ts`.

Follow the patterns established by the existing pages
(Templates.tsx, TemplateDetail.tsx, Instances.tsx, etc.). Match
the CSS class conventions in `src/app.css`.

## Files to modify

1. `src/api/client.ts` — add TypeScript interfaces + API call
   functions for all five resource groups.
2. `src/components/Layout.tsx` — add nav items for the new pages.
3. `src/App.tsx` — add routes for the new pages.
4. New page files in `src/pages/`.

## 1. Endpoints (`/v1/endpoints`)

### API surface

```
POST   /v1/endpoints                → { endpoint_id }
GET    /v1/endpoints?limit=N&cursor= → { items: [...], next_cursor }
GET    /v1/endpoints/{id}            → EndpointMetadataResponse
GET    /v1/endpoints/{id}/decrypted  → { config: {...} }
POST   /v1/endpoints/{id}/rotate    → EndpointMetadataResponse
POST   /v1/endpoints/{id}/retire    → { endpoint_id, is_retired }
```

### Types (add to client.ts)

```typescript
interface EndpointSummary {
  endpoint_id: string;
  display_name: string;
  latest_revision: number;
  created_at: UnixMillis;
  updated_at: UnixMillis;
  is_retired: boolean;
  key_version: number;
}

interface CreateEndpointRequest {
  display_name: string;
  config: JsonValue;
}

interface RotateEndpointRequest {
  display_name?: string;
  config: JsonValue;
}
```

### Pages

- `Endpoints.tsx` — list page with create form (display_name +
  JSON config textarea). Show table of endpoints with ID, name,
  status, key_version. Each row links to detail page. Add a
  "Create Endpoint" button that shows an inline form.

- `EndpointDetail.tsx` — detail page showing metadata. Buttons:
  "View Decrypted Config" (GET /decrypted, show in JsonViewer),
  "Rotate" (form with optional new display_name + new config JSON),
  "Retire" (confirm dialog, POST /retire).

### Nav

Add `{ to: "/endpoints", label: "Endpoints" }` to the nav items
in Layout.tsx, between "Instances" and "Audit".

### Routes (App.tsx)

```
/endpoints        → <Endpoints />
/endpoints/:id    → <EndpointDetail />
```

## 2. Principals (`/v1/principals`)

### API surface

```
POST   /v1/principals               → { principal_id, token }
GET    /v1/principals?limit=N&cursor= → { items: [...], next_cursor }
POST   /v1/principals/{id}/rotate   → { principal_id, token }
POST   /v1/principals/{id}/retire   → { principal_id, is_retired }
```

### Types

```typescript
interface PrincipalSummary {
  principal_id: string;
  display_name: string;
  kind: string;
  latest_revision: number;
  created_at: UnixMillis;
  updated_at: UnixMillis;
  is_retired: boolean;
}

interface CreatePrincipalRequest {
  display_name: string;
  kind: string; // "user" or "service"
}

interface TokenResponse {
  principal_id: string;
  token: string;
}
```

### Pages

- `Principals.tsx` — list page with create form (display_name +
  kind select: "user"/"service"). After creation, show the
  `pht_` token in a prominent alert box with "Copy" button and
  warning that it won't be shown again.

- No detail page needed — rotate and retire are actions on the
  list page (action buttons per row).

### Nav

Add `{ to: "/principals", label: "Principals" }` after
"Endpoints".

### Routes

```
/principals → <Principals />
```

## 3. Roles (`/v1/roles`)

### API surface

```
POST   /v1/roles                → { role_id }
GET    /v1/roles?limit=N&cursor= → { items: [...], next_cursor }
GET    /v1/roles/{id}            → RoleResponse
POST   /v1/roles/{id}/retire    → { role_id, is_retired }
```

### Types

```typescript
interface RoleSummary {
  role_id: string;
  display_name: string;
  permissions: string[];
  latest_revision: number;
  created_at: UnixMillis;
  updated_at: UnixMillis;
  is_retired: boolean;
}

interface CreateRoleRequest {
  display_name: string;
  permissions: string[];
}
```

### Pages

- `Roles.tsx` — list page with create form (display_name +
  permissions multi-select/checkbox list). Show the available
  permission atoms as checkboxes. The full list of atoms is:

  ```
  workflow:template_create, workflow:template_read,
  workflow:template_retire, workflow:instance_create,
  workflow:instance_read, workflow:instance_execute,
  workflow:instance_cancel, endpoint:create, endpoint:rotate,
  endpoint:retire, endpoint:read_metadata,
  endpoint:read_decrypted, tenant:principal_manage,
  tenant:role_manage, tenant:minting_manage,
  mint:ephemeral_token, tenant:settings_read,
  tenant:settings_manage, audit:read,
  deployment:tenant_manage, deployment:realm_manage,
  deployment:audit_read
  ```

  Hardcode this list in the page. Group them by prefix
  (workflow, endpoint, tenant, mint, audit, deployment).

- `RoleDetail.tsx` — detail page showing permissions as a
  checklist. "Retire" button.

### Nav

Add `{ to: "/roles", label: "Roles" }` after "Principals".

### Routes

```
/roles      → <Roles />
/roles/:id  → <RoleDetail />
```

## 4. Role Memberships (`/v1/role-memberships`)

### API surface

```
POST   /v1/role-memberships          → { membership_id }
GET    /v1/role-memberships?limit=N   → { items: [...], next_cursor }
POST   /v1/role-memberships/{id}      → { membership_id, is_retired }
```

Note: the retire endpoint is `POST /v1/role-memberships/{id}`
(not /retire). Read the routes file to confirm the exact path.

### Types

```typescript
interface MembershipSummary {
  membership_id: string;
  principal_id: string;
  role_id: string;
  latest_revision: number;
  created_at: UnixMillis;
  updated_at: UnixMillis;
  is_retired: boolean;
}

interface CreateMembershipRequest {
  principal_id: string;
  role_id: string;
}
```

### Pages

- `Memberships.tsx` — list page with create form (principal_id
  input + role_id input — both UUID text fields). Show table
  with membership_id, principal_id, role_id, is_retired.
  "Retire" button per row.

### Nav

Add `{ to: "/memberships", label: "Memberships" }` after "Roles".

### Routes

```
/memberships → <Memberships />
```

## 5. Minting Authorities (`/v1/minting-authorities`)

### API surface

```
POST   /v1/minting-authorities              → { authority_id, token }
GET    /v1/minting-authorities?limit=N       → { items: [...], next_cursor }
GET    /v1/minting-authorities/{id}          → AuthorityResponse
POST   /v1/minting-authorities/{id}/rotate   → { authority_id, token }
POST   /v1/minting-authorities/{id}/bump-epoch → { authority_id, epoch }
POST   /v1/minting-authorities/{id}/retire   → { authority_id, is_retired }
```

### Types

```typescript
interface AuthoritySummary {
  authority_id: string;
  display_name: string;
  permission_envelope: string[];
  max_lifetime_seconds: number;
  epoch: number;
  latest_revision: number;
  created_at: UnixMillis;
  updated_at: UnixMillis;
  is_retired: boolean;
}

interface CreateAuthorityRequest {
  display_name: string;
  permission_envelope: string[];
  max_lifetime_seconds: number;
}
```

### Pages

- `Authorities.tsx` — list page with create form. After creation,
  show the `pht_` token like Principals. Table shows authorities
  with name, epoch, permissions count, status.

- `AuthorityDetail.tsx` — detail page with permission envelope
  display, epoch, lifetime. Actions: "Rotate" (returns new
  token — show it), "Bump Epoch" (confirm dialog), "Retire"
  (confirm dialog).

### Nav

Add `{ to: "/authorities", label: "Authorities" }` after
"Memberships".

### Routes

```
/authorities      → <Authorities />
/authorities/:id  → <AuthorityDetail />
```

## General patterns to follow

Read the existing pages for patterns:

- **List pages**: use `useState` for items + loading + error.
  Fetch on mount via `useEffect`. Use the existing `Pagination`
  component. Match the table class from existing pages.
- **Detail pages**: use `useParams` for the ID, fetch on mount.
- **Forms**: inline forms toggled by a button, not modals.
  Use `<form className="stack">` with `<label className="field">`.
- **Actions**: POST actions use `apiCall` with `method: "POST"`.
  Show success/error feedback inline.
- **Token display**: When an API returns a `pht_` token (create
  principal, create authority, rotate), display it in a prominent
  box with a copy-to-clipboard button and a warning message.
  Use a `<pre>` or `<code>` element for the token value.
- **JSON input**: For config fields (endpoint config, abstract
  config), use a `<textarea>` with placeholder `{}`. Parse with
  `JSON.parse` before sending; show parse errors inline.
- **Retire confirm**: Before retiring, show a confirmation
  prompt (`window.confirm` is fine).
- **Date formatting**: Use `new Date(timestamp).toLocaleString()`
  same as existing pages.
- **Error handling**: Catch `ApiRequestError` and display
  `error.message` in a `<div className="alert error">`.

## CSS

No new CSS should be needed — reuse existing classes:
- `.section-title`, `.table`, `.button`, `.field`, `.alert`,
  `.stack`, `.primary`, `.secondary`, `.danger`

If you need a new class for the token display box, add it to
`src/app.css` with a descriptive name like `.token-display`.

## WebUI build

After all code changes, run:
```sh
./scripts/webui-build.sh --production
```

This builds the production bundle. The build must succeed with
no errors. Warnings about bundle size are acceptable.

## Verification

1. Run `./scripts/webui-build.sh --production` — must succeed.
2. Run `./scripts/pre-landing.sh --no-ignored` — must pass
   (the Rust crates are unchanged, but the script checks
   everything).
3. Commit via `./scripts/commit-all.sh "<message>"` ONLY.

## Git rules

- Commit via `./scripts/commit-all.sh "<message>"` ONLY.
- Do NOT run `./scripts/push-all.sh` or `cargo publish`.
- Do NOT run raw `git commit` / `git add` / `git push`.
- Do NOT modify any Rust source files.
</task>

<default_follow_through_policy>
If the WebUI build fails, fix the TypeScript errors immediately.
Do not proceed to verification with a broken build.
</default_follow_through_policy>

<completeness_contract>
The task is complete when:
1. All 5 resource groups have WebUI pages
2. All pages are wired in App.tsx routes
3. All pages are linked in Layout.tsx nav
4. `./scripts/webui-build.sh --production` succeeds
5. Changes are committed via `./scripts/commit-all.sh`
</completeness_contract>

<verification_loop>
1. Run `./scripts/webui-build.sh --production`
2. If it fails, fix TypeScript/webpack errors and retry
3. Run `./scripts/pre-landing.sh --no-ignored`
4. Only commit after both pass
</verification_loop>

<missing_context_gating>
Read the existing page files (Templates.tsx, Instances.tsx) before
writing new ones. Match their patterns exactly.
</missing_context_gating>

<action_safety>
- Never run `./scripts/push-all.sh`
- Never run `cargo publish`
- Never run raw git commands
- Never modify Rust source files
</action_safety>

<structured_output_contract>
When done, report:
- Summary: what pages were added
- Files created/modified: list
- WebUI build: pass/fail
- Pre-landing: pass/fail
- Git state: commit SHA, branch, pushed=no
</structured_output_contract>
