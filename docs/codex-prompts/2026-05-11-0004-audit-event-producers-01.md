# Audit-event producer wiring across mutation routes (initial dispatch)

**Date:** 2026-05-11
**Slug:** `audit-event-producers`
**Round:** 01 (initial dispatch — single-crate
`philharmonic-api`; closes the audit-producer gap surfaced
in `docs/notes-to-humans/2026-05-11-0001-audit-event-producer-gap.md`)
**Subagent:** `codex:codex-rescue`

## Motivation

Audit infrastructure is fully wired except for the
producers. `AuditEvent` entity kind, `write_audit_event`
helper, `GET /v1/audit` read endpoint, `audit:read`
permission atom, and the WebUI AuditLog page all exist
end-to-end. But **no production code path writes
`AuditEvent` entities** — only tests call the writer.
With zero producers the audit store is empty by
construction, and the WebUI page (correctly) renders
blank.

The contract for the producer wiring landed in
`b37f894` + `1ce191a` (2026-05-11):

- `philharmonic-policy` 0.2.3 ships
  `audit_event_type::{PRINCIPAL_CREATED, ROLE_CREATED, ...}`
  — canonical `i64` discriminants for every audit category.
- `docs/design/09-policy-and-tenancy.md` §"Audit trail"
  documents the `event_data` JSON schema convention
  (`principal_id`, `route`, `correlation_id` required;
  `target_entity_id`, `subject` per-event optional) and
  the failure-semantics rule (log warn + return success
  on audit-write failure).
- `docs/design/09-policy-and-tenancy.md` §Status flagged
  as "Implemented except audit producers" pending this
  dispatch.

This dispatch wires producers into every mutation route
the design lists, plus one e2e test per route family.

## References

- [`docs/notes-to-humans/2026-05-11-0001-audit-event-producer-gap.md`](../notes-to-humans/2026-05-11-0001-audit-event-producer-gap.md)
  — full investigative note, including the per-route grep
  showing zero production call sites of `write_audit_event`.
- [`docs/design/09-policy-and-tenancy.md` §Audit trail](../design/09-policy-and-tenancy.md#audit-trail)
  — locked contract: `event_type` discriminants, `event_data`
  schema, failure semantics.
- [`philharmonic-policy/src/audit_event_type.rs`](../../philharmonic-policy/src/audit_event_type.rs)
  — the canonical constants Codex will reference. Re-exported
  via `philharmonic_policy::audit_event_type::*`.
- [`philharmonic-api/src/routes/audit.rs:104-150`](../../philharmonic-api/src/routes/audit.rs#L104-L150)
  — `AuditEventInput` + `write_audit_event` signatures, the
  exact API to call. `AuditEventInput { tenant, event_type:
  i64, timestamp: UnixMillis, event_data: JsonValue }`. Helper
  returns `Result<EntityId<AuditEvent>, ApiError>`.
- [`philharmonic-api/src/routes/audit.rs:200-260`](../../philharmonic-api/src/routes/audit.rs#L200-L260)
  — read endpoint's `principal_id` extraction at
  `event_data.principal_id`. Confirms the locked field name
  the producers must use.
- `RequestContext` exposes `correlation_id: Uuid` and
  `auth: AuthContext` (with `auth.principal_id() ->
  Option<EntityId<Principal>>` for principal auth and
  ephemeral subject for ephemeral auth) per
  `philharmonic-api/src/context.rs` and `auth.rs`. Codex:
  inspect both for the exact accessor names.
- Existing pattern for `tracing::warn` in the codebase at
  `philharmonic-api/src/routes/mint.rs:108-112` — mirror
  this style for the audit-write failure path.

## Per-route → event_type mapping (authoritative)

Codex: this table is the lock; cross-check each entry
against the actual route handler before wiring.

| Route family | Route | event_type constant |
|---|---|---|
| `principals.rs` | `POST /v1/principals` (create) | `PRINCIPAL_CREATED` |
| `principals.rs` | `POST /v1/principals/{id}/rotate` | `PRINCIPAL_CREDENTIAL_ROTATED` |
| `principals.rs` | `POST /v1/principals/{id}/retire` | `PRINCIPAL_RETIRED` |
| `roles.rs` | `POST /v1/roles` (create) | `ROLE_CREATED` |
| `roles.rs` | `PATCH /v1/roles/{id}` | `ROLE_MODIFIED` |
| `roles.rs` | `POST /v1/roles/{id}/retire` | `ROLE_RETIRED` |
| `memberships.rs` | `POST /v1/memberships` (create) | `ROLE_MEMBERSHIP_CREATED` |
| `memberships.rs` | `DELETE /v1/memberships/{id}` | `ROLE_MEMBERSHIP_REMOVED` |
| `endpoints.rs` | `POST /v1/endpoints` (create) | `ENDPOINT_CREATED` |
| `endpoints.rs` | `POST /v1/endpoints/{id}/rotate` | `ENDPOINT_ROTATED` |
| `endpoints.rs` | `POST /v1/endpoints/{id}/retire` | `ENDPOINT_RETIRED` |
| `authorities.rs` | `POST /v1/authorities` (create) | `AUTHORITY_CREATED` |
| `authorities.rs` | `PATCH /v1/authorities/{id}` (update) | `AUTHORITY_MODIFIED` |
| `authorities.rs` | `POST /v1/authorities/{id}/rotate` (key rotate) | `AUTHORITY_MODIFIED` (signing-key rotation is a modification) |
| `authorities.rs` | `POST /v1/authorities/{id}/bump-epoch` | `AUTHORITY_EPOCH_BUMPED` |
| `authorities.rs` | `POST /v1/authorities/{id}/retire` | `AUTHORITY_RETIRED` |
| `mint.rs` | `POST /v1/tokens/mint` | `TOKEN_MINTED` |
| `tenant.rs` | `PATCH /v1/tenants/me` (status change branch only) | `TENANT_STATUS_CHANGED` |
| `operator.rs` | `POST /v1/operator/tenants/{id}/suspend` | `TENANT_STATUS_CHANGED` |
| `operator.rs` | `POST /v1/operator/tenants/{id}/activate` | `TENANT_STATUS_CHANGED` |

**Notes**:

- Authority `rotate` (key rotation) maps to
  `AUTHORITY_MODIFIED` rather than introducing a new
  discriminant — rotation is functionally a modification.
  If you find this maps poorly to operator log-reading
  conventions, surface in residuals; introducing a
  separate `AUTHORITY_ROTATED = 34` would be a
  philharmonic-policy 0.2.4 patch bump (out of scope
  here).
- `tenant.rs::update_tenant` may handle non-status fields
  (display_name etc.); the audit event fires **only when
  the status field actually changes**, not on every PATCH.
  Producers gate on the diff.
- `operator.rs` routes are operator-scoped (cross-tenant);
  the `tenant` field on `AuditEventInput` should be the
  **target tenant's** ID (the tenant being suspended /
  activated), not the operator's home tenant.

## `event_data` JSON shape per event

All events include the three required fields per the
locked schema convention:

```json
{
  "principal_id": "<uuid>",
  "route": "<METHOD /v1/...>",
  "correlation_id": "<uuid>",
  "target_entity_id": "<uuid>"    // optional, see per-event below
}
```

Per-event additions:

- **PRINCIPAL_CREATED / _RETIRED / _CREDENTIAL_ROTATED**:
  `target_entity_id` = the affected principal's public
  UUID. (For credential rotation: the principal whose
  credential rotated; not the rotator's principal_id —
  that's already in `principal_id`.)
- **ROLE_CREATED / _MODIFIED / _RETIRED**: `target_entity_id`
  = the affected role's public UUID.
- **ROLE_MEMBERSHIP_CREATED / _REMOVED**: `target_entity_id`
  = the membership's public UUID. Optionally also include
  `subject_principal_id` and `role_id` (UUIDs) for direct
  read-side filtering. Codex's call on whether to include
  these subfields; surface in residuals.
- **ENDPOINT_CREATED / _ROTATED / _RETIRED**:
  `target_entity_id` = the affected endpoint's public UUID.
- **AUTHORITY_CREATED / _MODIFIED / _RETIRED / _EPOCH_BUMPED**:
  `target_entity_id` = the affected authority's public UUID.
  For epoch bumps, include `new_epoch` (i64) so the audit
  log reads cleanly without joining against the entity.
- **TOKEN_MINTED**: **special — read carefully.**
  - `target_entity_id` = the minting-authority's public UUID.
  - `subject` = `{ "subject_id": "<opaque-string>",
    "authority_id": "<uuid>" }` per the design's privacy
    rule. **Do NOT include** the requested permissions, the
    injected claims object, the resulting token string, or
    any tenant-private end-user data. The audit row records
    "principal X minted a token via authority Y with subject
    Z" — operator-visible by design — and nothing more.
- **TENANT_STATUS_CHANGED**:
  - `target_entity_id` = the affected tenant's public UUID.
  - `from_status` (string) and `to_status` (string), e.g.
    `{"from_status": "active", "to_status": "suspended"}`.
    Required so the audit log is readable without joining
    against the tenant entity's history.

## Failure semantics — locked

Per the design doc convention: on audit-write failure
after a successful mutation, **log a warning at error
level and return success** on the underlying mutation.
Returning 500 would couple every mutation's availability
to the audit substrate's; small risk of audit gaps
(recoverable via structured logs) beats hard coupling.

Concrete pattern, mirror at every producer:

```rust
// AFTER the mutation has succeeded; BEFORE returning the
// success response to the caller:
let audit_input = AuditEventInput {
    tenant,
    event_type: audit_event_type::PRINCIPAL_CREATED,
    timestamp: UnixMillis::now(),
    event_data: json!({
        "principal_id": principal_id_for_audit(&context),
        "route": "POST /v1/principals",
        "correlation_id": context.correlation_id.to_string(),
        "target_entity_id": created_principal_id.to_string(),
    }),
};
if let Err(error) = write_audit_event(&state.store, audit_input).await {
    tracing::warn!(
        ?error,
        event_type = audit_event_type::PRINCIPAL_CREATED,
        correlation_id = %context.correlation_id,
        "audit-event write failed; mutation succeeded but audit-trail row is missing",
    );
}
```

The `tracing::warn!` form mirrors
`philharmonic-api/src/routes/mint.rs:108-112`. Make sure
the warn message includes enough fields for a deployment
operator reading the log to reconstruct the missing row
(at minimum: `event_type`, `correlation_id`, the
relevant target entity IDs).

A small helper in `audit.rs` is acceptable if it
de-duplicates the boilerplate cleanly — something like:

```rust
pub(crate) async fn emit_audit_event(
    store: &dyn ApiStore,
    input: AuditEventInput,
) {
    if let Err(error) = write_audit_event(store, input).await {
        tracing::warn!(
            ?error,
            event_type = input.event_type,
            "audit-event write failed; mutation succeeded but audit-trail row is missing",
        );
    }
}
```

(Tricky: the helper needs to either consume `input` and
log the relevant fields before calling `write_audit_event`,
or accept the relevant fields separately. Codex's call —
the constraint is just that the warn line carries
recoverable context.)

## Tests

For each route family (`principals`, `roles`, `memberships`,
`endpoints`, `authorities`, `mint`, `tenant`, `operator`),
**one** new e2e test (the file may already have a tests/
e2e_mysql.rs setup — extend that, don't create new test
files unless the existing file is structurally hostile):

1. Seed a fixture: tenant, a principal with the required
   atom for the route under test (and `audit:read` for the
   read-back check), an API token for the principal.
2. Perform the mutation via the API (e.g. POST a new
   principal).
3. Call `GET /v1/audit` and assert a row exists with:
   - matching `event_type`,
   - matching `principal_id` in `event_data`,
   - matching `target_entity_id` in `event_data` (where
     applicable),
   - non-empty `correlation_id`,
   - `timestamp` within the last few seconds of "now".

`mint.rs`'s test gets extra coverage:

- Assert that the audit row's `event_data` does **NOT**
  contain the requested permissions, the injected claims,
  or the token string. (Use `serde_json::Value` field-
  presence checks; assert the keys are absent.)

The existing `philharmonic-api/tests/e2e_mysql.rs` and
`audit_rate_admin.rs` files already exercise the read
side against a seeded fixture; extend those, or add a new
`tests/audit_producers.rs` if the existing structure is
hostile to the additions.

## Outcome

Pending — will be updated after Codex run.

---

## STRUCTURED-OUTPUT-CONTRACT — READ THIS FIRST

The contract has been honored for **10/10 consecutive
rounds** since it was added. The recent dispatch
(2026-05-11 webui-permission-aware-ui r01) stalled
mid-verification due to storage pressure — Codex was
working cleanly but the host disk filled and the process
died before emitting the six-section report. The
working tree was self-consistent and Claude verified
directly, but the report itself wasn't emitted.

**For this dispatch**: the workspace's root partition was
freed and ZFS compression enabled (Yuka), so storage
shouldn't be a constraint. Same contract applies: emit
the six-section report (Summary / Touched files /
Verification results / Residual risks / Git state / Open
questions) with the verbatim `RUN STATUS: COMPLETE`
token before `task_complete`. **If you hit the
context-window edge or storage edge after verification
passes, prioritise emitting the report.**

---

## Pre-landing-sh hygiene

This dispatch is Rust-only (no WebUI bundle change), so
the WebUI typecheck step doesn't apply. Run:

```sh
./scripts/pre-landing.sh
./scripts/check-api-breakage.sh philharmonic-api
```

`pre-landing.sh` covers fmt + check + clippy
(`-D warnings`) + rustdoc + workspace tests + the
`--ignored` phase for touched crates. `check-api-breakage.sh`
runs `cargo-semver-checks`; expect it to fail for the
unrelated path-pinned-vs-crates.io cascade (philharmonic-
policy 0.2.3 has the new audit_event_type module not on
crates.io yet); pre-landing's clean compile is
authoritative. **Surface the semver-checks output in
residuals.**

---

## Prompt (verbatim)

<task>
Wire audit-event producers into every mutation route in
`philharmonic-api`. Single crate. No public-trait change.
No crypto path touched. No version bump on
`philharmonic-api` (still 0.1.8 locally — the audit-write
adds are internal to handler bodies, no API surface
change).

Three logical deliverables, in this order:

- **A** — `emit_audit_event` helper (or equivalent) in
  `philharmonic-api/src/routes/audit.rs` that wraps
  `write_audit_event` with the log-on-failure pattern.
  Single helper across all producers to keep the audit-
  write boilerplate from spreading.
- **B** — Per-route producer wiring (~20 mutation routes
  across ~7 route files). After each successful mutation
  and before returning the success response, build the
  appropriate `AuditEventInput` per the per-route table
  + per-event-type `event_data` shape above; call the
  helper.
- **C** — One e2e test per route family (~7 tests) that
  seeds a fixture, performs the mutation via the API,
  and reads `GET /v1/audit` back to assert the producer
  fired. `mint.rs`'s test has the extra
  injected-claims-absence assertion.

## Hard constraints

- **No public-trait change** to `philharmonic-api`. The
  audit writes are inside handler bodies; the helper
  is `pub(crate)` or module-local.
- **No crypto path touched.** Tokens, COSE, SCK,
  payload hashes are out of scope. `mint.rs`'s changes
  are limited to: after the mint succeeds and before
  returning the response, write an audit event with the
  privacy-restricted payload (subject + authority only,
  NO injected claims).
- **`mint.rs` audit-event injected-claims restriction
  is load-bearing.** The audit row must record
  `{"subject_id": ..., "authority_id": ...}` only. Do
  NOT serialise the requested permissions, the injected
  claims object, the resulting token bytes/hex/string,
  or the lifetime. The test's absence-assertions are
  the test contract.
- **Failure semantics are non-negotiable**: on audit-
  write failure after successful mutation, log warn at
  error level + return success on the mutation. Do NOT
  return 500. Do NOT return the original mutation's
  error envelope.
- **`tenant.rs` PATCH gating**: only emit
  `TENANT_STATUS_CHANGED` when the status field
  actually changes. Other field updates (display_name
  etc.) do not produce audit events for now — surface
  in residuals if the design implies they should.
- **`operator.rs`'s tenant field**: the audit row's
  `tenant` is the **target** tenant being suspended/
  activated, NOT the operator's home tenant. The
  operator's `principal_id` goes in `event_data` as
  usual.
- **Existing tests must remain green byte-for-byte**:
  the `audit_rate_admin.rs` and `e2e_mysql.rs` tests
  currently exercise the read side; the new producer
  wiring shouldn't break them. If their fixtures end
  up writing audit rows incidentally (because the
  fixture now performs auditable mutations), update
  the read-side assertions to tolerate or filter the
  new rows — surface in residuals.
- **No publish**. Claude reviews and decides
  post-Codex.

<structured_output_contract>
Six sections, in this order:

1. **Summary** — what landed across A/B/C. Include
   `RUN STATUS: COMPLETE` or `RUN STATUS: PARTIAL —
   <reason>`.

2. **Touched files** — exhaustive list with
   `(new|edited|deleted) <path> — <one-line note>`.

3. **Verification results** — exact commands + outcomes:
   - `./scripts/pre-landing.sh` (pass/fail/exit).
   - `./scripts/check-api-breakage.sh philharmonic-api`
     (expected: fails for the
     path-pinned-vs-crates.io cascade on the new
     audit_event_type module; pre-landing's clean
     compile is authoritative — surface either way).

4. **Residual risks / known issues** — including:
   - For each event family with subtle decisions:
     - `ROLE_MEMBERSHIP_*`: did you include the
       `subject_principal_id` / `role_id` subfields
       or stick to `target_entity_id` only?
     - Authority `rotate` mapped to `AUTHORITY_MODIFIED`
       per the prompt — should this become a separate
       `AUTHORITY_ROTATED = 34` discriminant in a
       follow-up philharmonic-policy 0.2.4 patch?
     - `tenant.rs` PATCH: were any other fields you
       found that should also produce audit events?
   - Any divergence from the per-route table (e.g. a
     route you couldn't find, or a route whose
     semantics differ from the table's assumption).
   - Whether you added a `pub(crate) fn
     emit_audit_event` helper or a different shape;
     why.
   - Whether the warn-log message format you settled on
     carries enough fields for a deployment operator to
     reconstruct the missing audit row from the log
     line.
   - Whether the test coverage gap on `e2e_mysql.rs` /
     `audit_rate_admin.rs` (where existing fixtures
     might now incidentally write audit rows) required
     test updates, and what you did.

5. **Git state** — current `HEAD` SHA in
   `philharmonic-api` submodule and parent workspace.
   Confirm no commits made.

6. **Open questions** — questions for Yuka or Claude:
   - Whether `AUTHORITY_ROTATED` deserves a separate
     discriminant.
   - Whether `tenant.rs` non-status field changes
     (display_name etc.) should produce a separate
     `TENANT_MODIFIED` event in a follow-up
     philharmonic-policy 0.2.4 patch.
   - Whether the read endpoint at
     `philharmonic-api/src/routes/audit.rs` needs a
     follow-up to surface the canonical `event_type`
     names (via `audit_event_type::name`) in the
     response — current spec is opaque-i64.
</structured_output_contract>

<default_follow_through_policy>
- Implement deliverables A → B → C in order. A is the
  shared helper used by every B-instance.
- Use the existing `tracing::warn!` style at
  `philharmonic-api/src/routes/mint.rs:108-112` as the
  log-line template — same fields, same level.
- Cross-check each route's permission-protector with the
  per-route event_type mapping table BEFORE wiring. If
  the per-route table assumes a route exists or has a
  certain shape that the actual handler doesn't have,
  flag in residuals and skip rather than guess.
- For tests, prefer extending the existing
  `tests/e2e_mysql.rs` / `tests/audit_rate_admin.rs`
  files over creating new files. Existing fixtures
  already seed the tenant + principal + role +
  permissions infrastructure; reuse them.
- For the `mint.rs` test specifically: add absence-
  assertions that the audit row's `event_data` does NOT
  contain `claims`, `permissions`, `token`, `token_hex`,
  `lifetime_ms`, or any other key that would leak
  tenant-private data. These keys should literally not
  be present in the JSON.
- The audit-write happens AFTER the mutation has fully
  committed to the substrate. Do NOT interleave the
  audit write with the mutation write — that's a
  pre-transactional-substrate footgun (the audit row
  would persist even if the mutation rolled back, in
  the rare case the substrate ever supports rollback).
</default_follow_through_policy>

<completeness_contract>
"Done" means:

- All ~20 producer call sites listed in the per-route
  table are wired with the correct `event_type` constant
  and a complete `event_data` shape per the schema
  convention.
- The helper in `audit.rs` is in place (or, if you
  inlined the boilerplate, the inlined form is
  consistent across producers).
- Existing `tracing::warn` style mirrored at every
  failure-path log line.
- One new e2e test per route family (~7 tests) covers
  the producer firing and the read-back via
  `GET /v1/audit`. `mint.rs`'s test includes the
  injected-claims absence assertions.
- `./scripts/pre-landing.sh` green.
- `./scripts/check-api-breakage.sh philharmonic-api`
  run; output surfaced in residuals.
- Structured output report emitted before
  `task_complete`.

Partial completion is acceptable if you hit a genuine
blocker (e.g. one route family's handler structure
makes the audit-write awkward in a way the prompt
didn't anticipate) — but explicit `RUN STATUS:
PARTIAL — <reason>` is required, and the completed
producers must all be functional. A half-wired route
that emits audit events for create but not for retire
is worse than no producer on that route at all
(asymmetric coverage misleads the read side).
</completeness_contract>

<verification_loop>
For each route family:
1. Read the route's handler.
2. Add the audit-event write after the mutation
   commits.
3. Add the e2e test.
4. `CARGO_TARGET_DIR=target-main cargo test -p
   philharmonic-api -- <new-test-name>` — green.
5. Move on.

Once all families are wired:
6. `./scripts/pre-landing.sh` once.
7. `./scripts/check-api-breakage.sh philharmonic-api`.
8. Emit the structured-output report.
9. `task_complete`.

If you hit the context-window or storage edge after step
6 passes: **prioritise emitting the report**.
</verification_loop>

<missing_context_gating>
If you find yourself needing information not in this
prompt or the cited sources, **stop** and report in the
structured output's "Open questions" section.

Specifically: do **not**:

- Touch the route-protector enforcement, the auth
  middleware, or the request-context construction.
- Add a new permission atom, change `PermissionDocument`
  semantics, or modify role-membership evaluation.
- Modify the `AuditEvent` entity kind in
  `philharmonic-policy/src/entity.rs`. The shape is
  settled.
- Add new event-type discriminants to
  `philharmonic-policy/src/audit_event_type.rs`. The
  17-value set is settled; gaps in numbering are for
  *future* additions, not for this dispatch.
- Edit the `write_audit_event` helper at
  `philharmonic-api/src/routes/audit.rs:118-150`. Use
  it as-is.
- Touch crypto / SCK / COSE / token / payload-hash
  paths. Tokens, signing keys, and encrypted payloads
  are out of scope.
- Edit `mechanics-*` or any connector crate.
- Edit `HUMANS.md`, `CLAUDE.md`, `AGENTS.md`,
  `CONTRIBUTING.md`, `docs/`, `docs-jp/`,
  `scripts/`, or `philharmonic/webui/`.
- Publish to crates.io. No `cargo publish`. Claude
  reviews and decides post-Codex.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `philharmonic-api/src/routes/audit.rs` (edited —
  optional helper).
- `philharmonic-api/src/routes/principals.rs` (edited).
- `philharmonic-api/src/routes/roles.rs` (edited).
- `philharmonic-api/src/routes/memberships.rs` (edited).
- `philharmonic-api/src/routes/endpoints.rs` (edited).
- `philharmonic-api/src/routes/authorities.rs` (edited).
- `philharmonic-api/src/routes/mint.rs` (edited).
- `philharmonic-api/src/routes/tenant.rs` (edited).
- `philharmonic-api/src/routes/operator.rs` (edited).
- `philharmonic-api/src/routes/mod.rs` (edited only if
  the helper requires a new pub(crate) re-export).
- `philharmonic-api/tests/e2e_mysql.rs` (edited or
  preferred — extend existing fixtures).
- `philharmonic-api/tests/audit_rate_admin.rs` (edited
  to handle incidental audit rows from fixture
  mutations, if needed).
- `philharmonic-api/tests/audit_producers.rs` (new, only
  if the existing test files are structurally hostile to
  the additions; surface in residuals if you create this).

Files NOT to touch (flag if you find a reason to):

- Any file under `philharmonic-policy/`,
  `philharmonic-store*/`, `philharmonic-types/`,
  `philharmonic-workflow/`, `mechanics-*/`, any connector
  crate, or any WebUI / docs file.
- `philharmonic-api/Cargo.toml` (no version bump).
- `philharmonic-api/CHANGELOG.md` (no entry — this is
  internal handler-body wiring, not a public-API change).
- `philharmonic-api/src/auth.rs`,
  `philharmonic-api/src/context.rs`,
  `philharmonic-api/src/middleware/`,
  `philharmonic-api/src/lib.rs` (unless adding a
  re-export the helper requires).
- The workspace `Cargo.toml` `[patch.crates-io]` block.

Git rules: signed-off + signed commits via
`scripts/commit-all.sh` only. **Codex never runs
`commit-all.sh`** — Claude commits after reviewing your
work. No `cargo publish`. No raw `git commit` /
`git push` / `git add` / `git reset` / `git rebase` /
`git revert`. Read-only `git log` / `git diff` /
`git show` is fine.
</action_safety>
</task>
