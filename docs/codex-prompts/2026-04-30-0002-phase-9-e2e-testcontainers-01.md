# Phase 9 task 6 — End-to-end integration tests with testcontainers

**Date:** 2026-04-30
**Slug:** `phase-9-e2e-testcontainers`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

The full API stack is wired: three bin targets, WebUI,
schema migration at startup. But the `philharmonic-api` crate's
tests use an in-memory `MockStore`. We need real integration
tests that spin up MySQL via testcontainers, run schema
migration, wire the real `SqlStore` through
`PhilharmonicApiBuilder`, and exercise the API end-to-end
with HTTP requests.

## References

- `philharmonic-store-sqlx-mysql/tests/integration.rs` — the
  existing testcontainer pattern (MySQL setup, migrate, SqlStore).
- `philharmonic-api/tests/common/mod.rs` — test signing keys,
  `FixedResolver`, `MockStore` pattern.
- `philharmonic-api/src/lib.rs` — `PhilharmonicApiBuilder`.
- `philharmonic-store-sqlx-mysql/src/schema.rs` — `migrate()`.
- `ROADMAP.md` §Phase 9 — task 6.

## Context files pointed at

- `philharmonic-store-sqlx-mysql/tests/integration.rs`
- `philharmonic-api/tests/common/mod.rs`
- `philharmonic-api/src/lib.rs`
- `philharmonic-api/src/routes/mod.rs`
- `philharmonic-store-sqlx-mysql/src/schema.rs`
- `philharmonic-policy/src/api_token.rs`

## Scope

### In scope

Create an integration test file in the `philharmonic-api` crate
that uses testcontainers for MySQL and exercises real API
endpoints.

#### File: `philharmonic-api/tests/e2e_mysql.rs`

**Setup** (follow the pattern from `philharmonic-store-sqlx-mysql/tests/integration.rs`):

1. Start MySQL via `testcontainers_modules::mysql::Mysql`.
2. Connect with `MySqlPoolOptions`.
3. Run `philharmonic_store_sqlx_mysql::migrate(&pool)`.
4. Create `SqlStore::from_pool(pool.clone())`.
5. Set up test signing key + verifying key registry (reuse
   the test seed/public key constants from
   `philharmonic-api/tests/common/mod.rs`).
6. Build the API with `PhilharmonicApiBuilder`:
   - `.request_scope_resolver(Arc::new(FixedResolver::new(
       RequestScope::Tenant(tenant_id))))`
   - `.store(Arc::new(store))`
   - `.api_verifying_key_registry(registry)`
   - `.api_signing_key(signing_key)`
   - `.issuer("test-e2e")`
   - `.step_executor(Arc::new(StubExecutor))`
   - `.config_lowerer(Arc::new(StubLowerer))`
   - `.build()?`
7. Serve on `127.0.0.1:0` (random port) using
   `axum::serve(listener, api.into_router())`.
8. Use `reqwest::Client` to make HTTP requests against the
   running server.

**Test flows** (each as a separate `#[tokio::test]`,
all `#[ignore = "requires MySQL testcontainer"]`):

1. **Health check**: `GET /v1/_meta/health` returns 200.
   `GET /v1/_meta/version` returns the version string.

2. **Workflow template CRUD**: Create a template
   (`POST /v1/workflows/templates` with JSON body),
   list templates (`GET /v1/workflows/templates`), read
   by ID, update (`PATCH`), retire.

3. **Workflow instance lifecycle**: Create an instance
   from a template, read it, check its state.

4. **Principal CRUD**: Create a principal, list, rotate
   credentials, retire.

5. **Role + membership**: Create a role, create a membership
   linking a principal to the role.

6. **Tenant settings**: Read tenant settings
   (`GET /v1/tenant`), update (`PATCH /v1/tenant`).

7. **Audit log**: After performing operations, query
   `GET /v1/audit` and verify events appear.

**Auth**: Generate a `pht_` long-lived API token using
`philharmonic_policy::generate_api_token()`, store the hash
in the store (create a `Principal` entity with the token hash
as a content attribute), then use the raw token as
`Authorization: Bearer pht_...` in requests.

Read the API handler code to understand the exact request/
response shapes. The mock-based tests in
`philharmonic-api/tests/` are the best reference for what
the API expects.

**Dependencies to add** to `philharmonic-api/Cargo.toml`
`[dev-dependencies]`:

```toml
testcontainers = "0.27"
testcontainers-modules = { version = "0.15", features = ["mysql"] }
reqwest = { version = "0.13", features = ["json", "rustls"] }
philharmonic-store-sqlx-mysql = "0.1.0"
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "mysql"] }
```

(Check if any of these are already present.)

### Out of scope

- Ephemeral token flow (complex, involves minting authority
  setup — can be a follow-up test).
- Connector integration (would need a running connector
  service).
- WebUI testing (browser automation).

## Outcome

Pending — will be updated after Codex run.

---

## Prompt (verbatim)

<task>
You are working inside the `philharmonic-workspace` Cargo workspace.
Your target crate is `philharmonic-api` at `philharmonic-api/`
— it's a git submodule with its own repo.

## IMPORTANT: Do NOT commit or push

**Do NOT run `./scripts/commit-all.sh`.** Do NOT run any git
commit command. Do NOT run `./scripts/push-all.sh`. Leave the
working tree dirty.

## What to build

Create `philharmonic-api/tests/e2e_mysql.rs` — end-to-end
integration tests that use a real MySQL database via
testcontainers, a real `SqlStore`, and the full
`PhilharmonicApiBuilder` pipeline.

**Read these files first** (they are the authoritative
patterns and APIs):

- `philharmonic-store-sqlx-mysql/tests/integration.rs`
  — testcontainer + MySQL setup pattern. Follow this exactly
  for container creation, pool setup, and migration.
- `philharmonic-api/tests/common/mod.rs` — test signing key
  constants (`TEST_API_SEED`, `TEST_API_PUBLIC`, `TEST_API_KID`,
  `TEST_API_ISSUER`), `FixedResolver`, `MockStore`.
- `philharmonic-api/tests/*.rs` — existing mock-based tests
  show the exact request/response shapes for each endpoint.
  Read several to understand what the API expects.
- `philharmonic-api/src/lib.rs` — `PhilharmonicApiBuilder`
  API.
- `philharmonic-api/src/routes/*.rs` — handler implementations.
- `philharmonic-store-sqlx-mysql/src/schema.rs` — `migrate()`.
- `philharmonic-policy/src/api_token.rs` — `generate_api_token`.
- `CONTRIBUTING.md` — workspace conventions.

### Test structure

```rust
// philharmonic-api/tests/e2e_mysql.rs

// Standard testcontainer setup: MySQL, pool, migrate, SqlStore.
// Build PhilharmonicApiBuilder with real SqlStore.
// Serve on 127.0.0.1:0 (random port).
// Use reqwest::Client for HTTP calls.

// Each test: #[tokio::test(flavor = "multi_thread")]
//            #[ignore = "requires MySQL testcontainer"]
//            #[serial_test::file_serial(docker)]
```

**Test serialization is required.** Use `serial_test`'s
`#[file_serial(docker)]` on every test to prevent multiple
testcontainer instances from running concurrently (OOM risk
on CI and dev machines). This matches the pattern in the
SQL crate integration tests. Add `serial_test` to dev-deps.

### Test cases

1. **`health_and_version`**: GET /v1/_meta/health → 200.
   GET /v1/_meta/version → 200 + version string.

2. **`workflow_template_crud`**: POST create → GET list →
   GET by ID → PATCH update → POST retire. Verify response
   shapes and status codes.

3. **`workflow_instance_lifecycle`**: Create a template first,
   then POST create instance → GET read → check state.

4. **`principal_crud`**: POST create principal → list →
   rotate → retire.

5. **`role_and_membership`**: Create role → create membership
   → list memberships → delete membership.

6. **`tenant_settings`**: GET tenant → PATCH tenant →
   GET tenant (verify update persisted).

7. **`audit_log_records_operations`**: Perform several
   operations, then GET /v1/audit → verify events appear.

### Auth setup

The API requires a Bearer token for all non-meta endpoints.
To authenticate:

1. Call `philharmonic_policy::generate_api_token()` to get
   a `(Zeroizing<String>, TokenHash)` pair.
2. The raw token string is `pht_...` — use it as
   `Authorization: Bearer pht_...`.
3. The `TokenHash` needs to be stored as a content attribute
   on a `Principal` entity in the database so the auth
   middleware can look it up.

Read `philharmonic-api/src/middleware/auth.rs` to understand
the exact lookup flow for `pht_` tokens. The auth middleware
calls `find_by_content` on the store to find the principal
by token hash.

Read the existing mock-based tests (e.g.
`philharmonic-api/tests/principals.rs` or
`philharmonic-api/tests/workflows.rs`) to see how they set
up auth — they create a principal in the MockStore with the
token hash. Replicate that pattern with the real SqlStore.

### Dependencies

Add to `philharmonic-api/Cargo.toml` `[dev-dependencies]`:

```toml
testcontainers = "0.27"
testcontainers-modules = { version = "0.15", features = ["mysql"] }
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls"] }
philharmonic-store-sqlx-mysql = "0.1.0"
serial_test = "3"
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "mysql"] }
```

Check if any are already present before adding duplicates.

### Error handling in tests

Tests may use `.unwrap()` and `.expect()` freely — they're
test code, not library code. But prefer informative
`.expect("meaningful message")` over bare `.unwrap()`.

## Rules

- **Do NOT commit, push, or publish.**
- You MAY run `CARGO_TARGET_DIR=target-main cargo test -p
  philharmonic-api --test e2e_mysql -- --ignored` to verify
  the tests pass against a real Docker daemon. If Docker is
  not available, verify at least that the tests compile.
- Use `CARGO_TARGET_DIR=target-main` for raw cargo commands.
- You MAY modify `philharmonic-api/Cargo.toml` dev-deps.
- Do NOT modify files outside `philharmonic-api/` except
  `Cargo.lock`.

## Authoritative references

- `CONTRIBUTING.md` — if anything contradicts it, the doc wins.
- `ROADMAP.md` §Phase 9.
</task>

<structured_output_contract>
When you are done, produce a summary listing:
1. Files created or modified.
2. All verification commands run and their pass/fail status.
3. Any design decisions or assumptions about API shapes.
4. Confirmation that you did NOT commit or push.
</structured_output_contract>

<completeness_contract>
Every test case listed above must be implemented with real HTTP
requests and real assertions. Do not leave TODO stubs. If you
can't determine an API response shape, read the handler code
and the mock-based tests — they're the authoritative reference.
</completeness_contract>

<verification_loop>
Before finishing:
1. `CARGO_TARGET_DIR=target-main cargo check -p philharmonic-api
   --tests` — the test file must compile.
2. If Docker is available: `CARGO_TARGET_DIR=target-main cargo
   test -p philharmonic-api --test e2e_mysql -- --ignored
   --test-threads=1` — run the tests.
3. If Docker is not available: compilation is sufficient.
</verification_loop>

<missing_context_gating>
If the auth middleware's token-lookup flow doesn't match what
you expect, describe the gap rather than guessing. Read the
actual middleware code.
</missing_context_gating>

<action_safety>
- Do NOT commit. Do NOT push. Do NOT publish.
- Do NOT modify non-dev-dependency files in philharmonic-api.
- Do NOT modify files outside philharmonic-api/ except Cargo.lock.
</action_safety>
