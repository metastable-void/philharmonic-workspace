# Audit-log producer gap — WebUI "Audit Logs" page is empty by construction

**Date**: 2026-05-11 (Mon)
**Author**: Claude Code
**Trigger**: Yuka asked why the WebUI "Audit Logs" page is empty in her
deployment.

## TL;DR

The WebUI page isn't broken and the deployment isn't misconfigured.
**No production code path writes `AuditEvent` entities.** The read
endpoint, the `AuditEvent` entity kind, the `write_audit_event`
helper, the permission atom (`audit:read`), and the WebUI page all
exist and are wired end-to-end — but the only call sites of the
write helper are tests. With zero producers, the store contains
zero rows, and the page faithfully reports that.

The design doc ([`docs/design/09-policy-and-tenancy.md` §Status
line 816](../design/09-policy-and-tenancy.md)) currently claims
"audit events are shipped". That claim is **wrong**: only the
substrate and the read side are shipped. The producer side is
missing.

## Diagnosis

### What the WebUI is supposed to show

[`philharmonic/webui/src/pages/AuditLog.tsx`](../../philharmonic/webui/src/pages/AuditLog.tsx)
calls `GET /v1/audit` and renders one row per `AuditEvent`
scoped to the caller's tenant. Columns: timestamp,
`event_type` (deployment-defined `i64` discriminant),
`principal_id` extracted from `event_data`, and the full
`event_data` JSON via `JsonViewer`. Filters: date range,
limit, cursor-based pagination.

### What the design says should be recorded

From [`docs/design/09-policy-and-tenancy.md:737-751`](../design/09-policy-and-tenancy.md#audit-trail):

- Endpoint config created / rotated / retired
- Role definition created / modified / retired
- Role membership created / removed
- Minting authority created / modified / retired / epoch bumped
- Ephemeral token minted (subject + minting-authority only — no
  injected claims, by design)
- Tenant status changes
- Principal created / credential rotated / retired

The substrate already gives entity-level history for free; the
design's stated reason for `AuditEvent` on top of that is "who
initiated the change, via which API call, correlation ID" — i.e.
**contextual metadata that the per-entity revision log doesn't
capture**.

### What's wired today

- Entity kind: [`philharmonic-policy/src/entity.rs:231-244`](../../philharmonic-policy/src/entity.rs#L231-L244).
  KIND `92474986-4b6b-48c9-b902-8629061ef619`, one content slot
  (`event_data`), one pinned entity slot (`tenant`), two scalar
  slots (`event_type: i64`, `timestamp: i64`).
- Permission atom: [`philharmonic-policy/src/permission.rs:60`](../../philharmonic-policy/src/permission.rs#L60)
  `audit:read` (tenant-scoped) and `:67` `deployment:audit_read`
  (deployment-scoped, currently unused by any route).
- Read endpoint: [`philharmonic-api/src/routes/audit.rs:55-103`](../../philharmonic-api/src/routes/audit.rs#L55-L103)
  `GET /v1/audit` with `cursor`, `limit`, `event_type`, `since`,
  `until`, `principal_id` query params.
- Write helper: [`philharmonic-api/src/routes/audit.rs:118-150`](../../philharmonic-api/src/routes/audit.rs#L118-L150)
  `write_audit_event`, exported from
  [`philharmonic-api/src/lib.rs:97`](../../philharmonic-api/src/lib.rs#L97).
- WebUI page: [`philharmonic/webui/src/pages/AuditLog.tsx`](../../philharmonic/webui/src/pages/AuditLog.tsx).

### What's missing — production call sites

Workspace-wide grep for `write_audit_event` / `AuditEventInput`
outside `tests/` / `examples/` / `benches/` / `target*/`:

- **Production**: none.
- **Tests only**: `philharmonic-api/tests/e2e_mysql.rs:585,598`
  and `seed_audit_event` helper in
  `philharmonic-api/tests/audit_rate_admin.rs`.

The mutation routes that the design lists as audit-emitters all
return success without writing an event: principals, roles, role
memberships, endpoint configs, minting authorities, tenant
status, token mint. (Confirmed via Explore-agent sweep of
`philharmonic-api/src/routes/{principals,roles,endpoints,authorities,mint,tenant}.rs`.)

There is also no canonical `event_type` enumeration anywhere in
the workspace — the i64 discriminant is described as
"deployment-defined" in the entity doc-comment and in
[`AuditEventInput`'s doc-comment at audit.rs:108](../../philharmonic-api/src/routes/audit.rs#L108).
That means even if a producer existed, two crates emitting "role
membership created" might pick different integer codes; nothing
stops it.

## Why this slipped

Most likely a Phase-8 sequencing artefact: the read side and
entity machinery landed as part of `philharmonic-policy 0.2.0`
and the `philharmonic-api` initial cut, and the producer side
was implicitly punted to "wire as each mutation route lands".
Each mutation route then landed without it. The design's Status
line was written from the entity-and-read perspective and not
revisited.

The `audit_rate_admin.rs` test exercises the *read* side
against a seeded fixture, so test coverage doesn't catch the
missing producers — the e2e test asserts pagination and
filtering work, not that a real `POST /v1/principals` produces a
row.

## Proposed scope of fix

Three independent pieces; each is useful on its own.

### 1. Canonicalise `event_type` discriminants

Add a `pub mod audit_event_type` (or `pub enum AuditEventType:
i64`) to `philharmonic-policy` with named constants for every
event in the design list. Numbering should be stable
(append-only, never renumber) so historical rows stay
interpretable. Suggested first pass:

```
PRINCIPAL_CREATED            = 1
PRINCIPAL_CREDENTIAL_ROTATED = 2
PRINCIPAL_RETIRED            = 3
ROLE_CREATED                 = 10
ROLE_MODIFIED                = 11
ROLE_RETIRED                 = 12
ROLE_MEMBERSHIP_CREATED      = 13
ROLE_MEMBERSHIP_REMOVED      = 14
ENDPOINT_CREATED             = 20
ENDPOINT_ROTATED             = 21
ENDPOINT_RETIRED             = 22
AUTHORITY_CREATED            = 30
AUTHORITY_MODIFIED           = 31
AUTHORITY_RETIRED            = 32
AUTHORITY_EPOCH_BUMPED       = 33
TOKEN_MINTED                 = 40
TENANT_STATUS_CHANGED        = 50
```

Gaps in the numbering leave headroom per-category for additions
without forcing renumbering. This is a non-breaking add to
`philharmonic-policy` — bump 0.2.0 → 0.3.0 (new public API).
**Claude-sized** task; no Codex needed.

### 2. `event_data` schema convention

Establish a small, written convention for the JSON payload so
producers across routes agree on field names. The read endpoint
already extracts `principal_id` if present
([`audit.rs:234-237`](../../philharmonic-api/src/routes/audit.rs#L234-L237)),
so that field name is locked in. Beyond that, the design
mentions three contextual elements:

- **Who initiated**: `principal_id` (Uuid string) — locked.
- **Via which API call**: `route` (e.g. `"POST /v1/principals"`)
  and optionally `request_id` / `correlation_id`.
- **What changed**: a per-event-type subset, e.g.
  `target_entity_id`, before/after summaries for modifications.

Document in `docs/design/09-policy-and-tenancy.md` next to the
entity-kind definition. **Claude-sized** task.

### 3. Wire producers into mutation routes

For each route in
`philharmonic-api/src/routes/{principals,roles,endpoints,authorities,mint,tenant}.rs`,
after the successful mutation and before returning the response,
call `write_audit_event` with the appropriate `event_type`,
`principal_id` from `RequestContext`, and event-type-specific
`event_data`. Per the existing pattern: tenant scope comes from
`RequestContext`, timestamp from `UnixMillis::now()`.

Two subtleties:

- **Failure semantics.** If the audit write fails after the
  mutation succeeded, what should the request return? Logging
  a warning and returning 200 leaves a hole in the audit trail
  (worst case); returning 500 makes audit a hard dependency of
  every mutation (latency + availability coupling). My
  recommendation: in-transaction write where the store supports
  it, otherwise log+warn and return success — and add a
  separate background reconciliation if this matters in
  practice. Worth a design-doc decision before coding.
- **Token-mint payload restriction.** Per
  [`design 09 §line 746-748`](../design/09-policy-and-tenancy.md#L746-L748),
  token-mint events must record only subject identifier +
  minting-authority — **not** the injected claims. The
  producer for this event has to be deliberately careful;
  it's the easiest one to get wrong.

This piece is **Codex-sized** (>~6 mutation routes × ~15-30
LOC each + tests), and should be dispatched only **after**
pieces 1 and 2 are in.

## Recommendation

Land in three commits:

1. **`philharmonic-policy` 0.3.0** — canonical event-type
   constants + `event_data` schema doc (Claude, in-tree).
2. **`docs/design/09-policy-and-tenancy.md`** — correct the
   Status line (audit events are *not* shipped — producers
   pending), add the `event_data` convention, settle the
   failure-semantics question. Same commit as (1) or its
   own (Claude).
3. **`philharmonic-api`** — wire producers across mutation
   routes, with one new e2e test per route family that creates
   a fixture, performs the mutation, and asserts the matching
   audit row appears via `GET /v1/audit`. Dispatch to Codex
   with an archived prompt; this is the substantial coding
   piece.

This sequence keeps each commit reviewable, fixes the false
"shipped" claim immediately, and defers the larger Codex round
until the contract is locked.

## Status

Reporting only. No code changes proposed in this note — the
plan above is for Yuka to approve, modify, or reject before any
work starts.

### Resolution (2026-05-11 evening)

Yuka approved the plan; all three pieces landed the same day.

1. **Piece 1** — `philharmonic-policy` 0.2.2 → 0.2.3
   (`b37f894` philharmonic-policy submodule + `1ce191a`
   parent). New `audit_event_type` module with 17 canonical
   `i64` discriminants (per-category gaps for headroom),
   append-only numbering, `name(i64) -> Option<&'static str>`
   helper. Bumped patch rather than minor (per workspace D12 /
   D3 r01 precedent for additive changes) to avoid cascading
   the caret pin on philharmonic-api / philharmonic-workflow /
   philharmonic meta-crate.
2. **Piece 2** — design/09 contract lock-in (same parent
   commit `1ce191a`). §"`event_type` canonical discriminants",
   §"`event_data` JSON schema convention" (principal_id +
   route + correlation_id required; target_entity_id +
   subject per-event optional; token-mint payload restriction
   spelled out as load-bearing privacy decision), and
   §"Audit-write failure semantics" (log warn + return
   success on the underlying mutation; transactional
   multi-entity writes deferred until substrate support).
   §Status block corrected from the false "audit events are
   shipped" claim.
3. **Piece 3** — `philharmonic-api` audit producer wiring
   (`881c48a` philharmonic-api submodule + `8d20d1d` parent;
   Codex r01 under prompt
   `docs/codex-prompts/2026-05-11-0004-audit-event-producers-01.md`).
   19 producer call sites across 7 mutation route files
   (principals, roles, memberships, endpoints, authorities,
   mint, operator) using a shared `pub(crate)
   emit_audit_event` helper extracted in `audit.rs`. 7 e2e
   tests in new `tests/audit_producers.rs`, all green;
   mint.rs's test enforces the privacy restriction by
   absence-assertion on the keys `claims`, `permissions`,
   `token`, `token_hex`, `lifetime`, `expiry`. Routes use
   their actual paths (`/v1/role-memberships`,
   `/v1/minting-authorities`, `/v1/tenant`, operator
   `/unsuspend`) rather than the prompt's table's guesses
   — Codex correctly used the real paths and surfaced the
   divergence in residuals.

The producer wiring is **PARTIAL** in only one respect:
`tenant.rs`'s self-service `PATCH /v1/tenant` doesn't
currently expose a status-change branch, so
`TENANT_STATUS_CHANGED` fires only via the operator-scoped
`/v1/operator/tenants/{id}/{suspend,unsuspend}` routes.
Codex correctly chose not to invent a new API surface;
self-service tenant status changes can be added in a
future patch if/when the PATCH handler gains that branch.

Three open follow-up design questions queued (Yuka's call,
not blocking):

- Should authority key rotation get a distinct
  `AUTHORITY_ROTATED = 34` discriminant rather than
  sharing `AUTHORITY_MODIFIED`?
- Should tenant non-status updates (display_name etc.)
  produce a future `TENANT_MODIFIED` event in a follow-up
  `philharmonic-policy` 0.2.4 patch?
- Should `GET /v1/audit` surface canonical event-type
  names via `audit_event_type::name` in the response,
  rather than only opaque `i64` values?

This note's TL;DR ("empty by construction") is now stale.
The audit log read-side will populate as mutation routes
get exercised in the deployment. Note kept as historical
record of the gap and the three-piece fix sequence.
