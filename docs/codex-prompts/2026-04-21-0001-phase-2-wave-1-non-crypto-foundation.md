# Phase 2 — `philharmonic-policy` Wave 1: non-crypto foundation

**Date:** 2026-04-21
**Slug:** `phase-2-wave-1-non-crypto-foundation`
**Round:** 02 (rewritten 2026-04-21 — regenerated every UUID
via `./scripts/xtask.sh gen-uuid -- --v4`; reworked the test
plan to require **both** a mock-substrate tier and a real-MySQL
tier. The original round-01 text was never dispatched to
Codex.)
**Subagent:** `codex:codex-rescue`

## Motivation

Kicks off Phase 2 of the v1 roadmap: implement the six non-crypto
entity kinds of `philharmonic-policy` (`Tenant`, `Principal`,
`RoleDefinition`, `RoleMembership`, `MintingAuthority`,
`AuditEvent`), the permission-atom vocabulary, and the permission
evaluation algorithm. Wave 2 (`TenantEndpointConfig` + SCK
encryption + `pht_` API token generation) is blocked on Yuka's
Gate-1 crypto review and is **not** part of this dispatch —
those paths are explicitly out of scope for this prompt so that
Wave 1 can land without waiting on the review.

## References

- `ROADMAP.md` §Phase 2 — scope, acceptance criteria.
- `docs/design/09-policy-and-tenancy.md` — authoritative
  specification: entity shapes, permission atoms, evaluation
  semantics, subdomain naming rules.
- `docs/design/05-storage-substrate.md` — `ContentStore`,
  `IdentityStore`, `EntityStore` trait surfaces; typed extension
  traits; the append-only retry loop.
- `docs/design/11-security-and-cryptography.md` §"Long-lived API
  tokens" — contextual only; no code from this section lands in
  Wave 1.
- `docs/design/13-conventions.md` — shell-script, cargo-wrapper,
  and general Rust conventions; MSRV 1.88.
- `docs/notes-to-humans/2026-04-21-0007-what-next-after-phase-1.md`
  — wave ordering rationale.

## Scope

### In scope (Wave 1)

- Six entity kinds with their `Entity` impls, exactly matching
  `09-policy-and-tenancy.md`:
  - `Tenant`
  - `Principal` (reserves `epoch` scalar, unused in v1)
  - `RoleDefinition`
  - `RoleMembership`
  - `MintingAuthority` (with active `epoch`)
  - `AuditEvent`
- Stable `KIND: Uuid` constants — use the UUIDs listed below
  **verbatim**. Do not regenerate.
- `Tenant.status` discriminant enum (0=active, 1=suspended,
  2=retired).
- `Principal.kind` discriminant enum (0=user, 1=service_account).
- Permission atom string constants — all 26 atoms listed in
  `09-policy-and-tenancy.md` §"Permission atoms".
- `PermissionDocument` = serde-parseable `{ "permissions":
  [String; ...] }` parser (the content of a `RoleDefinition`'s
  `permissions` content slot — per the design doc, a JSON array
  is accepted; a future `{ permissions: [...], constraints:
  {...} }` object shape is a compatible superset, so implement
  the parser tolerantly: accept both the bare array and an object
  with `permissions` key).
- `evaluate_permission` — walks `RoleMembership` entities for the
  principal within the tenant, reads each `RoleDefinition`,
  checks whether any non-retired role grants the required atom.
  Async; takes `&impl EntityStoreExt` (or whatever the correct
  typed-ext trait is — match the published crate's actual API).
- **Both tiers of tests** (see "Required tests" below):
  - **Mock-substrate tests** — an in-crate `MockEntityStore` /
    `MockContentStore` under `#[cfg(test)]` or `tests/common/`
    that implements the same substrate traits with in-memory
    backing. Exercises every permission-evaluation branch
    deterministically, with no container startup, on every
    `cargo test` run.
  - **Real-MySQL integration tests** — `testcontainers` +
    `philharmonic-store-sqlx-mysql`, gated with
    `#[ignore = "requires MySQL testcontainer"]`, matching the
    pattern in `philharmonic-store-sqlx-mysql/tests/integration.rs`.
- Unit tests (colocated `#[cfg(test)] mod tests`) for in-process
  logic that doesn't touch the substrate: the `PermissionDocument`
  parser, the `Tenant.status` / `Principal.kind` enum discriminant
  round-trips, subdomain name validation.

### Out of scope (Wave 2 — do NOT implement here)

- `TenantEndpointConfig` entity kind. Its slot shape is in the
  design doc; do not land the `Entity` impl in this prompt.
- SCK encryption (`Sck`, `sck_encrypt`, `sck_decrypt`).
- `pht_` API token generation / parsing / hashing
  (`generate_api_token`, `parse_api_token`, `TokenHash`).
- Any dependency on `aes-gcm`, `sha2`, `base64`, `zeroize`,
  `rand_core`, `getrandom`, or any crypto crate. Wave 1's
  Cargo.toml **must not** introduce these.

If something you're writing starts wanting to reach into a
crypto concern — flag it and stop, don't improvise a crypto
implementation. This includes "just hashing something for storage"
or "just a quick compare" — all crypto touches route through
Wave 2's Gate-1-reviewed code.

### Definitely out of scope (any phase)

- Publishing to crates.io. Wave 1 lands with the crate still at
  `0.0.0`; Wave 2 lands the real `0.1.0`.
- Commits, pushes, tags, any `git` state-changing operation.
  Claude drives Git via `./scripts/*.sh` after reviewing Codex's
  diffs.

## Stable `KIND: Uuid` constants (use verbatim)

These are UUIDv4s generated on 2026-04-21 via
`./scripts/xtask.sh gen-uuid -- --v4` — the workspace-canonical
UUID source. They're part of the substrate wire format and
**must never change** after this commit. Embed them in the
source code literally, do not regenerate, do not "clean up."

```rust
Tenant::KIND           = uuid!("6a79e7a2-ea05-46d8-a578-b24c3b62c860")
Principal::KIND        = uuid!("3676b722-928b-4b3b-9417-659c5c1ea216")
RoleDefinition::KIND   = uuid!("da0d6fee-d989-44d1-b67e-f18b36a95043")
RoleMembership::KIND   = uuid!("cae4d1de-8f2f-4598-9ff0-2629819ca3ba")
MintingAuthority::KIND = uuid!("932c30fc-9b31-488d-badb-62b1c49b7d6d")
AuditEvent::KIND       = uuid!("92474986-4b6b-48c9-b902-8629061ef619")
```

A seventh UUID reserved for Wave 2's `TenantEndpointConfig` —
**do not use this in Wave 1**; it's listed here only so you know
not to collide:

```
TenantEndpointConfig::KIND = 19d1a8f5-6ef0-49b0-adf5-48e1cd3daea9
```

## Repository shape

The workspace is at `/home/mori/philharmonic`, 23 submodules.
`philharmonic-policy/` submodule is currently at `0.0.0` with a
placeholder `src/lib.rs`. Other crates in the family are already
published and usable: `philharmonic-types = "0.3.3"`,
`philharmonic-store = "0.1.0"`, `philharmonic-store-sqlx-mysql =
"0.1.0"`. The workspace's root `Cargo.toml` has a
`[patch.crates-io]` section redirecting these to local paths, so
you can assume the types are as they exist in the submodule trees
(check `philharmonic-types/src/entity.rs` for the `Entity` trait
shape — `KIND` is a `uuid::Uuid`, slot arrays are `&'static
[ContentSlot]` / `[EntitySlot]` / `[ScalarSlot]`).

Cargo dependencies to add to `philharmonic-policy/Cargo.toml`:

```toml
[dependencies]
philharmonic-types = "0.3"
philharmonic-store = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
thiserror = "2"

[dev-dependencies]
philharmonic-store-sqlx-mysql = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "sync"] }
testcontainers = "0.27"
testcontainers-modules = { version = "0.15", features = ["mysql"] }
async-trait = "0.1"  # only if needed to impl substrate traits on the mock
```

Exact minor versions: match whatever `philharmonic-store-sqlx-mysql/Cargo.toml`
uses for `testcontainers` / `testcontainers-modules` / `tokio`,
so dev-dep resolution stays coherent.

**Do not add crypto deps** — no `aes-gcm`, `sha2`, `base64`,
`zeroize`, `rand_core`, `getrandom`, `subtle`, `secrecy`,
`ed25519-dalek`, `x25519-dalek`, `ml-kem`, `hkdf`. Those land in
Wave 2.

## Conventions you must follow

- **Edition 2024, MSRV 1.88.** Match the existing workspace.
- **Licensing:** `license = "Apache-2.0 OR MPL-2.0"` in Cargo.toml
  (already there; don't change). `LICENSE-APACHE` and `LICENSE-MPL`
  files are already at the crate root.
- **Error type:** `PolicyError` with `thiserror`, partitioned by
  what the caller does (semantic violation vs. substrate-backed
  outcome). **Do not use `anyhow`** — this is a library crate.
  Propagate `StoreError` via a `#[from]` variant (check the
  `philharmonic-store` error module for the exact type name).
- **No `unsafe`.** Anywhere.
- **Async:** `async fn` where relevant (permission evaluation hits
  the substrate; must be async). No blocking in async fns.
- **No `println!` / `eprintln!` in library code.** Tests OK.
- **Comments:** default to none. Only add a one-line comment when
  the *why* is non-obvious (subtle invariant, workaround, hidden
  constraint). Don't narrate the *what* — names do that.
- **Terminology:** prose you author (doc comments, rustdoc, error
  messages, test names, your final summary) follows the
  workspace terminology conventions — see `README.md §Terminology
  and language`. Short form: no `master`/`slave` for technical
  relationships (use `primary`/`replica`, `leader`/`follower`,
  `parent`/`child`); no gendered defaults (prefer singular
  "they"); prefer `allowlist`/`denylist` over
  `whitelist`/`blacklist`; `stub`/`placeholder`/`fake` over
  "dummy"; GNU/Linux (OS) vs. Linux kernel; no `win*` shorthand
  for Microsoft Windows; prefer "free software" / "FLOSS" over
  standalone "open-source". Technical accuracy beats aesthetic
  neutrality — external identifiers (HTTP `Authorization`, DB
  `MASTER` commands) are used literally.
- **Clippy -D warnings:** the workspace CI runs
  `cargo clippy --all-targets -- -D warnings`. Fix the root cause;
  only use `#[allow(clippy::<lint>)]` at the narrowest scope with
  a one-line explanation, when a lint is genuinely wrong for that
  call site.
- **Tests:** see "Required tests" below.

## Entity kind definitions (from `09-policy-and-tenancy.md`, exact shapes)

Each of the six kinds needs an `Entity` impl matching the design
doc verbatim. Slot names, slot types, pinning, and indexing must
match exactly — they're wire-visible.

### `Tenant`

- `NAME = "tenant"`
- CONTENT_SLOTS: `display_name`, `settings`.
- ENTITY_SLOTS: (none).
- SCALAR_SLOTS: `status: I64 (indexed)`.

`Tenant.status` discriminant values (define as a public enum
with `#[repr(i64)]` or equivalent; the I64 scalar holds the
discriminant):
- 0 → `Active`
- 1 → `Suspended`
- 2 → `Retired`

Also provide a `validate_subdomain_name(name: &str) -> Result<(),
PolicyError>` helper per the design doc's subdomain naming rules:
- Must match `[a-z0-9][a-z0-9-]{1,62}`.
- No leading digit (already implied by the first-char class —
  the design doc's rule "[a-z0-9][...]" starts with a letter or
  digit, but the doc separately states "no leading digit." Honor
  the stricter reading: first character must be `[a-z]`.
- No consecutive hyphens.
- 2–63 chars.
- Not in the reserved set: `admin`, `api`, `www`, `app`,
  `connector`. (Realm names are added at deployment time and
  aren't hardcoded here.)

### `Principal`

- `NAME = "principal"`
- CONTENT_SLOTS: `credential_hash`, `display_name`.
- ENTITY_SLOTS: `tenant` → `Tenant`, pinning `Pinned`.
- SCALAR_SLOTS:
  - `kind: I64 (indexed)` — 0=user, 1=service_account.
  - `epoch: I64 (indexed)` — reserved; unused in v1, document
    as such in a module-level doc comment so consumers know.
  - `is_retired: Bool (indexed)`.

`Principal.kind` discriminant:
- 0 → `User`
- 1 → `ServiceAccount`

### `RoleDefinition`

- `NAME = "role_definition"`
- CONTENT_SLOTS: `permissions` (JSON array — see
  `PermissionDocument` below), `display_name`.
- ENTITY_SLOTS: `tenant` → `Tenant`, pinning `Pinned`.
- SCALAR_SLOTS: `is_retired: Bool (indexed)`.

### `RoleMembership`

- `NAME = "role_membership"`
- CONTENT_SLOTS: (none).
- ENTITY_SLOTS:
  - `principal` → `Principal`, pinning `Pinned`.
  - `role` → `RoleDefinition`, pinning `Pinned`.
  - `tenant` → `Tenant`, pinning `Pinned` (stored explicitly
    for tenant-filtered query performance, even though it's
    derivable).
- SCALAR_SLOTS: `is_retired: Bool (indexed)`.

### `MintingAuthority`

- `NAME = "minting_authority"`
- CONTENT_SLOTS: `credential_hash`, `display_name`,
  `permission_envelope`, `minting_constraints`.
- ENTITY_SLOTS: `tenant` → `Tenant`, pinning `Pinned`.
- SCALAR_SLOTS:
  - `epoch: I64 (indexed)`.
  - `is_retired: Bool (indexed)`.

### `AuditEvent`

- `NAME = "audit_event"`
- CONTENT_SLOTS: `event_data`.
- ENTITY_SLOTS: `tenant` → `Tenant`, pinning `Pinned`.
- SCALAR_SLOTS:
  - `event_type: I64 (indexed)`.
  - `timestamp: I64 (indexed)`.

**Do not enumerate `event_type` discriminants in Wave 1.** The
API layer (Phase 8) decides which events land; Wave 1 only
declares the entity kind so the storage shape is in place.

## Permission atoms (from `09-policy-and-tenancy.md`)

Exact strings; export each as a `pub const` in a
`permission::atom` module (or similar). The full list (26
atoms) — do not invent any:

```
// Workflow template
"workflow:template_create"
"workflow:template_read"
"workflow:template_retire"

// Workflow instance
"workflow:instance_create"
"workflow:instance_read"
"workflow:instance_execute"
"workflow:instance_cancel"

// Endpoint config
"endpoint:create"
"endpoint:rotate"
"endpoint:retire"
"endpoint:read_metadata"
"endpoint:read_decrypted"

// Principal & role management
"tenant:principal_manage"
"tenant:role_manage"

// Minting authority
"tenant:minting_manage"
"mint:ephemeral_token"

// Tenant settings
"tenant:settings_read"
"tenant:settings_manage"

// Audit
"audit:read"

// Deployment operator
"deployment:tenant_manage"
"deployment:realm_manage"
"deployment:audit_read"
```

## Permission evaluation algorithm

Design-doc text, translated into the algorithm you must
implement:

```
fn evaluate_permission(
    store: &impl EntityStoreExt,
    principal: EntityId<Principal>,
    tenant: EntityId<Tenant>,
    required_atom: &str,
) -> Result<bool, PolicyError>:

    // 1. Verify principal.tenant == tenant (defense in depth).
    let principal_row = store.get_latest_revision_typed::<Principal>(principal)
        .await?
        .ok_or(PolicyError::PrincipalNotFound)?;
    if principal_row.entity_slot::<Tenant>("tenant")? != tenant {
        return Ok(false);  // Cross-tenant request; deny, don't err.
    }
    if principal_row.scalar_bool("is_retired")? {
        return Ok(false);
    }

    // 2. Find RoleMembership entities where principal == principal
    //    AND tenant == tenant AND is_retired == false.
    //    (Exact query API depends on philharmonic-store's
    //    extension trait. Use find_by_scalar / list_revisions_referencing
    //    as appropriate.)
    let memberships: Vec<EntityRow> = ...;

    // 3. For each membership, fetch its RoleDefinition.
    for m in memberships {
        let role_id: EntityId<RoleDefinition> = m.entity_slot("role")?;
        let role = store.get_latest_revision_typed::<RoleDefinition>(role_id)
            .await?
            .ok_or(PolicyError::RoleNotFound)?;
        if role.scalar_bool("is_retired")? { continue; }

        // 4. Parse the permissions content blob.
        let perms_hash = role.content_slot("permissions")?;
        let perms_bytes = content_store.get(perms_hash).await?
            .ok_or(PolicyError::MissingPermissionsBlob)?;
        let doc: PermissionDocument = serde_json::from_slice(&perms_bytes.as_bytes())?;

        // 5. Check membership.
        if doc.contains(required_atom) {
            return Ok(true);
        }
    }

    Ok(false)
```

The API above is illustrative — match it to the actual
`philharmonic-store` trait surface. If a method signature doesn't
exist (e.g. `get_latest_revision_typed`), look for the closest
existing helper in `philharmonic-store::ext` and use that;
**don't invent new trait methods on `philharmonic-store`**.

`PermissionDocument` deserializer must accept both:
- Bare array: `["workflow:template_read", ...]`.
- Wrapped: `{ "permissions": [...] }` (forward-compatible shape
  for later constraint additions).

Make the parser tolerant — try array first, fall back to object
with `permissions` key.

## Required tests

### Design rationale — two tiers, both mandatory

`philharmonic-policy` has two distinct classes of correctness to
verify, and neither alone is sufficient:

1. **Algorithmic correctness of `evaluate_permission`.** Does
   the evaluation loop visit the right rows, skip retired roles,
   deny across tenants, handle multi-role membership? This is a
   pure data-transformation question — in-memory fakes can
   simulate the substrate cheaply and deterministically, and the
   tests run on every `cargo test` without containers.
2. **Substrate contract correctness.** Do `Entity` impls
   round-trip through real storage? Do the slot types + indices
   the design doc specifies actually serialize, persist, and
   query against `philharmonic-store-sqlx-mysql`? Mocks can't
   answer this — only real MySQL can, because the mock is
   written by the same hand writing the entity impls and any
   divergence between them silently matches.

Prior Philharmonic history: mocked-substrate-only tests once
passed while a prod migration broke because the mock elided a
real schema constraint. The policy since: substrate-touching
behavior is validated against real MySQL via `testcontainers`,
with `#[ignore = "requires MySQL testcontainer"]` so workspace-
level CI stays fast while per-modified-crate `--ignored` runs
exercise them. The *new* rule added in this dispatch: fast
algorithmic coverage via a mock tier is *also* required, so
permission-evaluation branches are reachable without container
overhead and so every `cargo test` in CI exercises the logic.

Both tiers are mandatory; neither substitutes for the other.

### Tier 1 — Mock-substrate tests (fast, default `cargo test`)

Implement a `MockEntityStore` + `MockContentStore` (or a single
unified mock if the substrate traits naturally compose that way)
under either:
- `src/testing/mock.rs` behind `#[cfg(any(test, feature =
  "testing"))]`, OR
- `tests/common/mock.rs` shared via a `mod common;` declaration
  in each integration-test file.

Pick whichever composes better with the substrate traits you end
up calling from `evaluate_permission`. Don't expose the mock in
the crate's public API unless a `testing` feature is added
explicitly — if you add one, document it in the crate README.

The mock must:
- Be fully in-memory (no fs, no network, no channels to other
  processes).
- Implement the same async traits `evaluate_permission` calls on
  the real substrate, so the function under test is unmodified
  between tiers.
- Be deterministic — no `rand`, no wall-clock time, no
  iteration-order reliance on `HashMap` for any assertion-
  relevant ordering (use `BTreeMap` where ordering matters).
- Not cheat on error paths — if the real substrate can return
  "entity not found" or "content hash missing", the mock must
  be able to return the same.

Tests to implement against the mock (fast, no `#[ignore]`
attribute, run every time):

1. Permission evaluation — happy path: principal has role that
   grants required atom → `Ok(true)`.
2. Permission evaluation — permission denied: role exists but
   doesn't grant the atom → `Ok(false)`.
3. Permission evaluation — retired role: role exists, grants
   atom, but `is_retired = true` → `Ok(false)`.
4. Permission evaluation — retired membership: membership is
   retired → `Ok(false)`.
5. Permission evaluation — retired principal → `Ok(false)`.
6. Permission evaluation — cross-tenant principal: principal
   belongs to tenant A, caller asks about tenant B → `Ok(false)`.
7. Permission evaluation — multi-role membership: principal has
   two memberships, neither alone grants the atom but one of
   them does → `Ok(true)`.
8. Permission evaluation — principal not found → propagates
   `PolicyError::PrincipalNotFound` (or whatever variant you
   defined).
9. Permission evaluation — role not found (membership points
   at a role that doesn't exist) → propagates
   `PolicyError::RoleNotFound`.
10. Permission evaluation — content blob missing for a role →
    propagates `PolicyError::MissingPermissionsBlob`.
11. `PermissionDocument` tolerant-parser — bare array vs wrapped
    object, against the mock pipeline (parsed via
    `content_store.get(...)` → `serde_json::from_slice`), end
    to end.

### Tier 2 — Real-MySQL integration tests (contract, `--ignored`)

Every test uses a fresh testcontainer MySQL 8 instance, matching
the pattern in
`philharmonic-store-sqlx-mysql/tests/integration.rs` (global
async mutex for container startup, per-test schema cleanup). All
annotated `#[tokio::test(flavor = "multi_thread")]` +
`#[ignore = "requires MySQL testcontainer"]`.

Tests to implement against real MySQL:

12. **Entity round-trip for each of the 6 kinds** — create, append
    a revision with every slot type populated (content, entity,
    scalar), read it back, validate every slot matches. This is
    the substrate-contract test — it's the reason Tier 2 exists
    and why Tier 1 can't replace it.
13. Permission evaluation — happy path end-to-end (mirror of
    Tier 1 test 1, but through `sqlx-mysql`).
14. Permission evaluation — retired role (mirror of Tier 1 test
    3, real MySQL).
15. Permission evaluation — cross-tenant denial (mirror of Tier
    1 test 6, real MySQL).
16. Permission evaluation — multi-role membership positive
    (mirror of Tier 1 test 7, real MySQL).

Tier 2 doesn't need to re-cover every branch Tier 1 covers —
the point is to validate the substrate *contract*, not to
re-run algorithmic tests. Four well-chosen end-to-end cases
(one positive, one retirement-path, one tenant-isolation, one
multi-role) plus the six round-trips are enough. If Tier 1 and
Tier 2 diverge on a shared case, that's a bug — fix the mock
or fix the impl, don't silence the test.

### Tier 3 — Unit tests (no substrate at all)

In-process logic that has no substrate dependency. Colocated
(`#[cfg(test)] mod tests` inside the module whose logic they
exercise):

17. `PermissionDocument` parses bare array correctly.
18. `PermissionDocument` parses `{permissions: [...]}` object.
19. `PermissionDocument::contains` returns true/false correctly.
20. `validate_subdomain_name` accepts valid (e.g. `acme-corp`,
    `a1`, 63-char max-length) and rejects invalid (too short,
    leading digit, leading/trailing hyphen, double-hyphen,
    reserved name).
21. `Tenant.status` discriminant round-trip (Rust enum → i64 →
    Rust enum).
22. `Principal.kind` discriminant round-trip.

## Acceptance criteria (before Claude commits your work)

- `cargo fmt --check` clean on `philharmonic-policy`.
- `cargo check --workspace` passes at the workspace root.
- `cargo clippy --all-targets -- -D warnings` on `philharmonic-policy`
  passes.
- `cargo test --workspace` passes. **Tier 1 mock tests and Tier
  3 unit tests must pass here** (no `#[ignore]` on them — they
  run as part of the default workspace-level test flow); Tier 2
  tests are `#[ignore]`d and skipped, as expected.
- `cargo test -p philharmonic-policy -- --ignored` passes
  against MySQL testcontainers. **Tier 2 integration tests hit
  real MySQL — do not mock the substrate in this tier.** (The
  mock lives in Tier 1 where it belongs; Tier 2 is specifically
  the "mocks can't catch this" tier.)
- `cargo tree -p philharmonic-policy | grep -iE 'aes|sha2|base64|zeroize|rand_core|getrandom|subtle|secrecy|ed25519|x25519|ml-kem|hkdf'`
  returns **nothing** (crypto deps are not introduced).
- All 6 `KIND: Uuid` constants match the verbatim values above
  (`6a79e7a2-…`, `3676b722-…`, `da0d6fee-…`, `cae4d1de-…`,
  `932c30fc-…`, `92474986-…`).
- No `unsafe`, no `anyhow`, no `println!`/`eprintln!` in library
  code.

If any of these fail, flag the gap in your final summary rather
than work around it. Claude verifies everything before committing.

## Git handling

**Do not run any Git command that changes state** — no `commit`,
`push`, `add`, `checkout`, `reset`, `stash`, `tag`, nothing. Leave
the working tree dirty with your changes; Claude inspects the
diff, invokes `./scripts/pre-landing.sh philharmonic-policy` and
the per-crate ignored tests, and drives
`./scripts/commit-all.sh "..."` + `./scripts/push-all.sh` after
review. If you need to inspect state, read-only `git status`,
`git diff`, `git log` are fine.

## Final summary format

When you finish (or hit a wall), write a short summary covering:
- What's implemented and where (file paths).
- Where the mock lives (file path + visibility) and which
  substrate traits it implements.
- Test results, separately for each tier — Tier 1 + Tier 3 from
  `cargo test --workspace`, Tier 2 from
  `cargo test -p philharmonic-policy -- --ignored`.
- Any deviation from this prompt, with reasoning.
- Any places where you flagged-rather-than-fixed (crypto creep,
  `unsafe` in neighboring code, `anyhow` in neighboring code,
  ambiguous design-doc interpretation).
- The trait-method names you actually used from
  `philharmonic-store` (so I can review the substrate-query shape
  without re-deriving it).

No commits, no pushes, no publish. Claude handles all of that
post-review.

---

## Prompt (verbatim text to send to Codex)

<task>
Implement Phase 2 Wave 1 of the Philharmonic workspace — the non-crypto foundation of `philharmonic-policy`. Detailed spec and all constraints are in this repo at:

- `docs/codex-prompts/2026-04-21-0001-phase-2-wave-1-non-crypto-foundation.md` — this file; read it verbatim.
- `docs/design/09-policy-and-tenancy.md` — authoritative entity-kind shapes, permission atoms, evaluation semantics, subdomain naming.
- `docs/design/05-storage-substrate.md` — substrate trait surfaces you'll call.
- `docs/design/13-conventions.md` — shell-script and Rust conventions (including §Naming and terminology).
- `README.md` §Terminology and language — prose conventions for code comments, rustdoc, error messages, and your final summary.
- `ROADMAP.md` §Phase 2 — acceptance criteria at the phase level.

Repository: `/home/mori/philharmonic` — a Rust cargo workspace of 23 submodules. `philharmonic-policy/` submodule is currently at `0.0.0` with only a placeholder `src/lib.rs`. `philharmonic-types = "0.3"`, `philharmonic-store = "0.1"`, and `philharmonic-store-sqlx-mysql = "0.1"` are already published and patched locally via the workspace root's `[patch.crates-io]`.

Scope: implement exactly the six entity kinds listed in `09-policy-and-tenancy.md` except `TenantEndpointConfig` (which ships in Wave 2 after a crypto review). Also implement the permission atoms, `PermissionDocument` parser, permission evaluation algorithm, and the test matrix (Tier 1 mock-substrate, Tier 2 real-MySQL integration, Tier 3 unit) listed in this prompt. Use the exact `KIND: Uuid` values listed in this prompt — `6a79e7a2-ea05-46d8-a578-b24c3b62c860` for `Tenant`, `3676b722-928b-4b3b-9417-659c5c1ea216` for `Principal`, `da0d6fee-d989-44d1-b67e-f18b36a95043` for `RoleDefinition`, `cae4d1de-8f2f-4598-9ff0-2629819ca3ba` for `RoleMembership`, `932c30fc-9b31-488d-badb-62b1c49b7d6d` for `MintingAuthority`, `92474986-4b6b-48c9-b902-8629061ef619` for `AuditEvent` (do NOT regenerate — they are wire-format-stable; `19d1a8f5-6ef0-49b0-adf5-48e1cd3daea9` is reserved for Wave 2's `TenantEndpointConfig` and must not be used in Wave 1).

Do not add any crypto crate (`aes-gcm`, `sha2`, `base64`, `zeroize`, `rand_core`, `getrandom`, `subtle`, `secrecy`, or any dalek/ML-KEM/HKDF). Do not implement `TenantEndpointConfig`, SCK encryption, or `pht_` token code. Those are Wave 2 and are blocked on a separate crypto review. If you find yourself needing a crypto primitive, stop and flag — don't improvise.

No `unsafe`. No `anyhow` (this is a library crate; use `thiserror`). No `println!` / `eprintln!` in library code. `cargo clippy --all-targets -- -D warnings` must pass. `cargo fmt --check` must pass.

Tests must be implemented in **two tiers, both mandatory, neither substitutes for the other**:
- **Tier 1 — Mock-substrate** (fast, no `#[ignore]`): write an in-memory `MockEntityStore` + `MockContentStore` under `#[cfg(any(test, feature = "testing"))]` or `tests/common/`, exercising every permission-evaluation branch deterministically. These run on every `cargo test --workspace` without any container.
- **Tier 2 — Real MySQL** (contract, `#[ignore = "requires MySQL testcontainer"]`): `testcontainers` + `philharmonic-store-sqlx-mysql` against a real MySQL 8 container, matching the pattern in `philharmonic-store-sqlx-mysql/tests/integration.rs`. Six entity round-trips + four end-to-end permission-evaluation cases (happy path, retired role, cross-tenant denial, multi-role positive). **Do not mock the substrate in this tier — its purpose is specifically to catch drift that a mock can't.**
- **Tier 3 — Unit**: colocated `#[cfg(test)] mod tests` for `PermissionDocument` parsing, `validate_subdomain_name`, and the discriminant round-trips.

Prose conventions apply (`README.md §Terminology and language`): no `master`/`slave` for technical relationships, no gendered defaults, prefer `allowlist`/`denylist`, GNU/Linux (OS) vs. Linux kernel, no `win*` shorthand, prefer "free software"/"FLOSS" over standalone "open-source". External identifiers (HTTP `Authorization`, DB `MASTER`) are used literally.

Do not run any git state-changing command. Leave the working tree dirty with your changes. Claude reviews and commits.

When done, write a short summary covering: what's implemented and where (file paths); where the mock lives (file path + visibility) and which substrate traits it implements; test results separately for each tier (Tier 1 + Tier 3 from `cargo test --workspace`, Tier 2 from `cargo test -p philharmonic-policy -- --ignored`); any deviation from the prompt with reasoning; flagged-rather-than-fixed issues (crypto creep, unsafe in neighbors, anyhow in neighbors, design-doc ambiguity); and the exact trait-method names you used from `philharmonic-store`.

Don't publish. Don't commit. Don't push.
</task>
