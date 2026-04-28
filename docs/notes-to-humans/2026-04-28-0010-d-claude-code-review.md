# Sub-phase D — Claude code review

**Author:** Claude Code · **Audience:** Yuka ·
**Date:** 2026-04-28 (Tue) JST afternoon

Non-crypto sub-phase. No Gate-2 crypto review needed.

## Verdict

**PASSES.** All 13 workflow endpoints implemented per doc 10
§228-268. Pagination, WorkflowEngine integration, stub
executor/lowerer, instance-scope enforcement via
`RequestInstanceScope` — all present and tested.

## What landed

- **`src/routes/workflows.rs`** (1180 lines) — all 13
  endpoint handlers: 5 template (create, list, read, update,
  retire) + 8 instance (create, list, read, history, steps,
  execute, complete, cancel). Each route attaches
  `RequiredPermission` per the atom spec. Instance routes
  attach `RequestInstanceScope` for ephemeral-scope enforcement.
- **`src/pagination.rs`** — cursor-based pagination: default
  50, max 200, opaque base64url-encoded cursor (created_at +
  entity_id), overfetch-by-one for next-page detection.
  Unit tests for round-trip and limit clamping.
- **`src/workflow.rs`** — `ApiWorkflowEngine` type alias,
  `SharedStepExecutor` / `SharedConfigLowerer` (trait-object
  wrappers for `Arc<dyn StepExecutor>` / `Arc<dyn
  ConfigLowerer>`), `StubExecutor` / `StubLowerer`
  (placeholder impls for testing).
- **`src/lib.rs`** — builder gains `step_executor` +
  `config_lowerer` deps, constructs `ApiWorkflowEngine`,
  wires `WorkflowState` into routes.
- **`src/error.rs`** — added `InvalidRequest` + `NotFound`
  (with message) + `Forbidden` + `Unauthenticated` variants.
- **`tests/workflow_endpoints.rs`** — 5 integration tests
  covering template CRUD lifecycle, instance lifecycle,
  permission enforcement, instance-scope enforcement, and
  execute-step with subject context flow.

## Security notes

- **Tenant isolation**: every endpoint checks
  `ensure_revision_tenant` to confirm the entity belongs to
  the request's tenant. Cross-tenant reads/writes return 404
  (not 403, to avoid leaking entity existence).
- **Permission enforcement**: `require_tenant_principal`
  rejects ephemeral callers on management endpoints;
  `execute_instance` accepts either auth type. All route
  groups attach `RequiredPermission` matching doc 10.
- **Instance-scope**: `attach_instance_scope` middleware
  extracts the instance ID from the URL path and attaches
  `RequestInstanceScope` before the authz middleware runs.
  The authz middleware's `instance_scope_allows` check
  (from sub-phase C) enforces it. Tested in
  `instance_scope_enforcement_blocks_wrong_instance`.
- **Abstract-config validation**: `validate_abstract_config`
  checks every referenced endpoint-config UUID exists within
  the tenant and is not retired before creating/updating a
  template.
- **No panics on library paths** — `unwrap_or_else` only in
  `StubExecutor` (test stub, not library code). ✅
- **No `unsafe`** ✅

## Test coverage

51 total tests across `philharmonic-api` (up from 42).
5 new workflow integration tests:
- Template CRUD lifecycle (create → read → list → update →
  retire)
- Instance lifecycle (create → execute → complete)
- Permission enforcement (create without permission → 403)
- Instance-scope enforcement (ephemeral scoped to instance A,
  execute on instance B → 403)
- Execute with subject context flows ephemeral claims
