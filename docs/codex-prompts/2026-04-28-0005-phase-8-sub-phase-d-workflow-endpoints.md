# Phase 8 sub-phase D — workflow management endpoints

**Date:** 2026-04-28
**Slug:** `phase-8-sub-phase-d-workflow-endpoints`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Sub-phases A–C landed the skeleton, auth, and authz. The API
crate has a working middleware chain: scope resolution →
authentication → authorization → handler. The authorization
middleware checks `RequiredPermission` extensions per route
and enforces tenant-scope + instance-scope.

**This dispatch adds the first real endpoint handlers:**
workflow template and instance management per doc 10
§"Workflow management". These endpoints are the core
tenant-facing surface — creating templates, instantiating
workflows, executing steps, reading state.

Non-crypto sub-phase. No crypto-review gate.

## References

- [`docs/design/10-api-layer.md`](../design/10-api-layer.md)
  §"Workflow management" (lines 228-268) — full endpoint
  surface.
- [`docs/design/09-policy-and-tenancy.md`](../design/09-policy-and-tenancy.md)
  — entity model for templates, instances, steps.
- `philharmonic-workflow` crate — `WorkflowEngine`,
  `WorkflowTemplate`, `WorkflowInstance`, `StepRecord`,
  `InstanceStatus`, `SubjectContext`, `StepResult`.
- `philharmonic-api/src/middleware/authz.rs` —
  `RequiredPermission`, `RequestInstanceScope`.
- `philharmonic-policy::atom::*` — permission atom constants.
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md) — §4, §10.3, §11.

## Scope

### In scope

#### 1. Route module (`src/routes/workflows.rs`)

Add all workflow endpoints from doc 10 §228-268:

**Templates:**
- `POST /v1/workflows/templates` — create. Requires
  `workflow:template_create`. Body: `{display_name,
  script_source, abstract_config}`. Returns 201 + template
  ID.
- `GET /v1/workflows/templates` — list. Requires
  `workflow:template_read`. Paginated (cursor-based, default
  50, max 200, opaque cursor string). Returns array of
  template summaries.
- `GET /v1/workflows/templates/{id}` — read. Requires
  `workflow:template_read`. Returns template + latest
  revision.
- `PATCH /v1/workflows/templates/{id}` — update (new
  revision). Requires `workflow:template_create`. Body:
  partial update of `{script_source?, abstract_config?,
  display_name?}`. Returns updated template.
- `POST /v1/workflows/templates/{id}/retire` — retire.
  Requires `workflow:template_retire`. Returns 200 + status.

**Instances:**
- `POST /v1/workflows/instances` — create. Requires
  `workflow:instance_create`. Body: `{template_id, args}`.
  Returns 201 + instance ID.
- `GET /v1/workflows/instances` — list. Requires
  `workflow:instance_read`. Paginated.
- `GET /v1/workflows/instances/{id}` — read. Requires
  `workflow:instance_read`. Returns instance state.
- `GET /v1/workflows/instances/{id}/history` — revision
  history. Requires `workflow:instance_read`. Paginated.
- `GET /v1/workflows/instances/{id}/steps` — step records.
  Requires `workflow:instance_read`. Paginated.
- `POST /v1/workflows/instances/{id}/execute` — execute
  step. **Accepts Principal OR Ephemeral.** Requires
  `workflow:instance_execute`. Body: `{input}`. Attaches
  `RequestInstanceScope` for instance-scope enforcement.
  Passes `SubjectContext` from `AuthContext` to the engine.
- `POST /v1/workflows/instances/{id}/complete` — mark
  complete. Requires `workflow:instance_execute`.
- `POST /v1/workflows/instances/{id}/cancel` — cancel.
  Requires `workflow:instance_cancel`.

Each route attaches `RequiredPermission(atom::*)` as a
layer so the authz middleware enforces it.

Instance-bearing routes (`/instances/{id}/*`) attach
`RequestInstanceScope(id)` so the authz middleware can
enforce ephemeral instance-scope tokens.

#### 2. WorkflowEngine integration

The builder gains a workflow-engine dependency. The engine
is generic over `S, E, L` (store, executor, lowerer). For
sub-phase D:

- The **store** is already available (`ApiStore`).
- The **executor** and **lowerer** are new builder
  dependencies. Sub-phase D introduces placeholder/stub
  implementations:
  - `StubExecutor` — implements `StepExecutor` with a
    no-op that returns a fixed `StepResult`. Good enough
    for endpoint-layer testing. The real executor lands
    in Phase 9 wiring.
  - `StubLowerer` — implements `ConfigLowerer` with a
    no-op. Same reasoning.

The builder stores the engine as a type-erased `Arc`
accessible by handlers via axum's `State` or `Extension`.

**Important:** `WorkflowEngine` is generic
(`WorkflowEngine<S, E, L>`). To store it in the router
state without leaking the generics into the API's public
surface, either:
- Box the engine behind a trait object, or
- Use a concrete type alias inside the crate (e.g.
  `type ApiWorkflowEngine = WorkflowEngine<ApiStoreHandle,
  Box<dyn StepExecutor>, Box<dyn StepLowerer>>`), or
- Accept the engine as `Arc<dyn WorkflowEngineApi>` where
  `WorkflowEngineApi` is an object-safe trait that wraps
  the engine's methods.

Pick whichever is cleanest. The public builder surface
should accept separate store/executor/lowerer dependencies
and construct the engine internally.

#### 3. Pagination (`src/pagination.rs`)

Doc 10 §"Pagination" (line 473): cursor-based, default 50,
max 200, opaque cursor string. Implement a shared
pagination utility:

```rust
pub struct PaginationParams {
    pub cursor: Option<String>,
    pub limit: u32,  // default 50, max 200
}

pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}
```

The cursor encodes the last item's sort key (e.g.
`created_at` + entity_id for time-ordered lists) as a
base64url-encoded opaque string. The store query uses
`WHERE (created_at, id) > (cursor_created_at, cursor_id)
ORDER BY created_at, id LIMIT limit + 1` — fetch one extra
to determine if there's a next page.

For sub-phase D, implement cursor encoding/decoding and
the query pattern. Later sub-phases reuse the same utility.

#### 4. JSON request/response types (`src/routes/workflows.rs`)

Serde structs for request bodies and response payloads.
Keep them in the route module (not public API surface —
they're wire shapes, not library types).

#### 5. Tests

Integration tests in `tests/workflow_endpoints.rs` using
mock stores:

- Template CRUD: create → read → list (verify in list) →
  update (new revision) → read (verify updated) → retire.
- Instance lifecycle: create → read → execute step → read
  (verify status change) → list steps → complete.
- Permission enforcement: create template without permission
  → 403.
- Instance-scope enforcement: ephemeral token scoped to
  instance A, try to execute on instance B → 403.
- Pagination: create N templates, list with limit, verify
  cursor + next page.

### Out of scope

- **Real executor/lowerer** — Phase 9 wiring.
- **Endpoint config CRUD** — sub-phase E.
- **Principal/role/authority CRUD** — sub-phase F.
- **Token minting** — sub-phase G.
- **Audit + rate limit** — sub-phase H.
- **`cargo publish`** — sub-phase I.
- **Testcontainers integration** — later.

## Workspace conventions

- Edition 2024, MSRV ≥ 1.88.
- **No panics in library `src/`** (§10.3).
- **No `unsafe`**.
- **Rustdoc on every `pub` item.**

## Pre-landing

```sh
./scripts/pre-landing.sh philharmonic-api
```

## Git

Do NOT commit, push, branch, tag, or publish. Leave dirty.

## Verification loop

```sh
./scripts/pre-landing.sh philharmonic-api
cargo test -p philharmonic-api --all-targets
cargo doc -p philharmonic-api --no-deps
git -C philharmonic-api status --short
git -C . status --short
```

## Action safety

- Edits only in `philharmonic-api/` + `Cargo.lock`.
- No new crypto.
- No edits to `philharmonic-workflow/` — consume its existing
  public surface only.

## Deliverables

1. `src/routes/workflows.rs` — all 13 endpoint handlers.
2. `src/routes/mod.rs` — wire workflow routes into router.
3. `src/pagination.rs` — shared cursor pagination utility.
4. `src/lib.rs` — builder gains executor + lowerer deps;
   engine construction.
5. `Cargo.toml` — add `philharmonic-workflow` dep.
6. `tests/workflow_endpoints.rs` — integration tests.

Working tree: dirty. Do not commit.

## Structured output contract

1. **Summary**.
2. **Files touched**.
3. **Verification results**.
4. **Residual risks / TODOs**.
5. **Git state**.

---

## Outcome

Pending — will be updated after Codex run.
