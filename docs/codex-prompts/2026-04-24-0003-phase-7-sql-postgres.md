# Phase 7 Tier 1 — `sql_postgres` implementation (initial dispatch)

**Date:** 2026-04-24
**Slug:** `phase-7-sql-postgres`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Phase 7 Tier 1 connector implementation — the Postgres half of
the two SQL driver impls. Data-layer connectors (SQL + vector +
embed) unblock the data-access shape of real workflows; SQL
ships first. This prompt is dispatched in parallel with its
sibling `2026-04-24-0004-phase-7-sql-mysql.md`; the two crates
are independent and will land in separate submodule commits.

Non-crypto task: no Gate 1/2, no key material. Doc 08 §"SQL"
owns the wire protocol in full.

## References (read before coding)

- **Authoritative wire-protocol spec**:
  [`docs/design/08-connector-architecture.md`](../design/08-connector-architecture.md)
  §"SQL" (config / request / response / SQL-to-JSON type
  mapping / error cases). This is complete enough that no
  focused-impl spec is being written separately; the prompt
  + doc 08 together define what 0.1.0 must do. If anything
  below contradicts doc 08, doc 08 wins.
- [`ROADMAP.md`](../../ROADMAP.md) §"Phase 7 — Additional
  implementations" — priority tiers + per-implementation
  pattern + acceptance criteria.
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md):
  - §10.3 no panics in library `src/`.
  - §10.4 libraries take bytes, not file paths.
  - §10.9 HTTP-client rule (not directly relevant; `sqlx`
    speaks the DB wire protocol directly, not HTTP).
  - §4 git workflow, §5 script wrappers, §11 pre-landing.
- `philharmonic-connector-impl-api` 0.1.0 — source the trait
  and re-exports from there (`Implementation`, `async_trait`,
  `ConnectorCallContext`, `ImplementationError`, `JsonValue`).
- `philharmonic-connector-impl-http-forward` 0.1.0 and
  `philharmonic-connector-impl-llm-openai-compat` 0.1.0 —
  reference impls at one layer of remove (HTTP, not SQL).
  Read their `src/lib.rs` for the trait-impl shape + error
  pattern. Do not copy HTTP-specific mechanics (retry
  policies, reqwest client) — `sql_postgres` does not retry
  at the connector layer (see Decisions below).

If anything in this prompt contradicts the docs above, the
docs win. Flag any contradiction and stop rather than
guessing.

## Crate state (starting point)

- `philharmonic-connector-impl-sql-postgres` — currently a
  0.0.0 placeholder submodule at
  `philharmonic-connector-impl-sql-postgres/`. Has
  `Cargo.toml` (placeholder `[dependencies]` empty),
  `src/lib.rs` (empty or placeholder), `README.md`,
  `CHANGELOG.md`, `LICENSE-*`.
- Never published substantively. Drop any aspirational
  `[0.0.0] Name reservation` CHANGELOG entry — same precedent
  as impl-api, http_forward, llm_openai_compat.
- Workspace-internal `[patch.crates-io]` entry already in
  root `Cargo.toml`; no workspace-root edits needed.

Target: `0.1.0` implementing the `sql_query` capability
against Postgres via `sqlx`, deterministic tests against a
`testcontainers`-backed Postgres (or `sqlx-test` if preferred;
see Testing below), pre-landing green, working tree dirty.

## Scope

### In scope

1. `Cargo.toml`:
   - Version `0.0.0` → `0.1.0`.
   - Deps: `philharmonic-connector-impl-api = "0.1"`,
     `philharmonic-connector-common = "0.2"`, `async-trait =
     "0.1"`, `serde = { version = "1", features = ["derive"]
     }`, `serde_json = "1"`, `thiserror = "2"`, `tokio =
     { version = "1", features = ["rt", "macros", "time"] }`,
     `sqlx = { version = "0.8", default-features = false,
     features = ["postgres", "runtime-tokio-rustls", "json",
     "chrono", "uuid"] }`. Verify 0.8.6 is still latest via
     `./scripts/xtask.sh crates-io-versions -- sqlx` before
     committing.
   - Dev-deps: `tokio = { version = "1", features = ["rt",
     "macros", "time", "test-util"] }`, `testcontainers =
     "0.27"` with the `blocking` feature if needed (confirm
     API for 0.27.3; see Testing below).
   - `default-features = false` on sqlx drops native-tls in
     favor of rustls, matching the workspace HTTP-client
     discipline (rustls everywhere).
2. Module layout — pattern after http_forward's clean
   separation. Suggested:
   - `src/lib.rs` — crate-root rustdoc + module plumbing +
     public `SqlPostgres` type + `impl Implementation for
     SqlPostgres` + re-exports from impl-api.
   - `src/config.rs` — `SqlPostgresConfig` with
     `connection_url`, `max_connections`,
     `default_timeout_ms`, `default_max_rows`;
     `deny_unknown_fields`; `prepare()` that builds a
     `PgPool` with the configured pool size.
   - `src/request.rs` — `SqlQueryRequest` with `sql`,
     `params`, optional `max_rows`, optional `timeout_ms`;
     `deny_unknown_fields`.
   - `src/response.rs` — `SqlQueryResponse` with `rows`,
     `row_count`, `columns`, `truncated`; `Column` with
     `name`, `sql_type`.
   - `src/execute.rs` — the core `execute` logic: parameter
     binding, query execution with per-request timeout,
     row collection with `max_rows` truncation, column
     metadata extraction.
   - `src/types.rs` — SQL-to-JSON value conversion (see
     doc 08 §"SQL-to-JSON type mapping"). One function per
     SQL type family.
   - `src/error.rs` — internal `Error` enum + `From<Error>
     for ImplementationError` (mapping below).
3. Unit tests colocated with each module (pure-logic tests
   that don't need a live DB):
   - `config::tests` — deny-unknown-fields, default values,
     connection-url schema validation (accepts `postgres://`,
     rejects others).
   - `request::tests` — deserialization, clamping behavior
     for `max_rows` / `timeout_ms`.
   - `response::tests` — column-metadata ordering matches
     `rows[]` keys.
   - `types::tests` — each SQL-to-JSON mapping with a fixed
     input/output (integers at i64 boundaries, decimal
     numeric, bytea, timestamptz, JSON/JSONB, null, arrays).
   - `error::tests` — every internal variant maps to the
     right `ImplementationError`.
4. Integration tests under `tests/`:
   - `happy_path.rs` — SELECT, INSERT/UPDATE/DELETE,
     parameterized queries with various types, empty result
     set with `columns` still populated, `truncated: true`
     when `max_rows` clips.
   - `error_cases.rs` — SQL syntax error → `InvalidRequest`
     (`invalid_sql`), parameter count mismatch →
     `InvalidRequest` (`parameter_mismatch`), constraint
     violation → `UpstreamError`, connection-refused →
     `UpstreamUnreachable`, per-request timeout →
     `UpstreamTimeout`, integer overflow (i64 range) →
     `UpstreamError`.
   - `types.rs` — every SQL-to-JSON mapping verified
     end-to-end by writing to a DB table, reading back, and
     checking the JSON value matches.
   - Tests use `testcontainers` to spin up a Postgres 16
     container per test module (see Testing below).
5. `CHANGELOG.md` — `[0.1.0] - 2026-04-24` entry; drop any
   `[0.0.0]` line.
6. Crate-root rustdoc on `src/lib.rs` matching the density of
   `philharmonic-connector-impl-http-forward/src/lib.rs`:
   what the crate does, Postgres-specific notes (placeholder
   syntax, type mapping quirks), usage snippet.
7. `README.md` — one-paragraph expansion above Contributing.

### Out of scope (flag; do NOT implement)

- Any change to `philharmonic-connector-impl-api`,
  `philharmonic-connector-common`, `philharmonic-connector-service`,
  or the wire protocol in doc 08.
- Connector-level retries. SQL queries are not retried at
  this layer (idempotency concerns; scripts retry if needed).
- Schema migrations, DDL management, or connection-string
  parsing beyond what `sqlx` provides.
- Query plan inspection, `EXPLAIN`, prepared-statement
  caching control (sqlx handles these internally).
- Streaming result sets (buffered; `max_rows` enforces the
  cap).
- A shared `philharmonic-connector-impl-sql-common` crate.
  v1 has two SQL impls; the ~80% duplication between them
  is acceptable. If a 3rd SQL driver joins in v2, extract
  then.
- `cargo publish`, `git tag`, any commit or push — Claude
  handles post-review.
- Workspace-root `Cargo.toml` edits — already in place.

### Decisions fixed upstream (do NOT deviate)

1. **`sqlx = "0.8"`** with `default-features = false` +
   features `["postgres", "runtime-tokio-rustls", "json",
   "chrono", "uuid"]`. `rustls`, not `native-tls`, per the
   workspace TLS discipline.
2. **No connector-level retry.** Database errors surface to
   the caller verbatim; the script decides whether to retry.
3. **`UpstreamError.status` for DB errors**: use **`500`**
   as a sentinel (database-side runtime failure, no HTTP
   equivalent). `body` carries the formatted sqlx error
   message. Scripts parse `body` for database-specific
   detail.
4. **`max_rows` truncation via streaming**: use
   `sqlx::query(...).fetch(&pool)` + a `StreamExt::take(max_rows
   + 1)` pattern. If the accumulator reaches `max_rows + 1`,
   drop the extra row and return `truncated: true`. Do NOT
   inject `LIMIT` into the script's SQL — the script owns
   the SQL, and modifying it would break prepared-statement
   caching and be surprising.
5. **`timeout_ms` enforcement**: wrap the query execution in
   `tokio::time::timeout(...)` at the request's effective
   timeout (request.timeout_ms if set, else config.default_timeout_ms).
   On timeout → `UpstreamTimeout`.
6. **Clamping semantics** (per doc 08): request values for
   `max_rows` and `timeout_ms` may override the config
   defaults *downward* but not upward. If the request value
   exceeds the config default, clamp to the config default
   silently (not an error — doc 08 treats the config as a
   cap, not a violation-triggering bound).
7. **Implementation name**: `Implementation::name()` returns
   **`"sql_postgres"`** (snake_case, matches the `impl`
   field in the decrypted connector payload).
8. **Integer range checking**: i64 is the signed SQL integer
   carrier; values outside i64 range (e.g. Postgres `numeric`
   with huge values promoted through an integer path) →
   `UpstreamError` per doc 08's type-mapping rule. `numeric`
   / `decimal` SQL types serialize as JSON *string* to
   preserve precision.

## Testing

`testcontainers` for Postgres: spin up a container per
integration-test file, run the tests against it, tear down.
Recommended image: `postgres:16-alpine`.

Constraint: Codex may not have Docker available in its
sandbox. If `testcontainers` can't start Docker, the
integration tests should gracefully `#[ignore]` with a clear
message; pre-landing will skip them in that environment but
they'll run on a developer with Docker. Similar pattern to
the `#[ignore]`-gated smokes in `llm_openai_compat`. Unit
tests (pure logic, no DB) must always run and must pass clean.

If `testcontainers` isn't viable in any reasonable form, fall
back to unit-test-only coverage + document the gap explicitly
in `src/lib.rs` crate rustdoc + `tests/README.md`. Flag under
Residual risks.

## Error mapping (doc 08 § "Error cases" → `ImplementationError`)

| Doc 08 case              | Internal `Error` variant      | Wire `ImplementationError`       |
| ---                      | ---                           | ---                              |
| `invalid_sql`            | `InvalidSql(detail)`          | `InvalidRequest { detail }`      |
| `parameter_mismatch`     | `ParameterMismatch { expected, actual }` | `InvalidRequest { detail: "parameter count mismatch: expected N, got M" }` |
| `upstream_error`         | `UpstreamDbError(sqlx_error_string)` | `UpstreamError { status: 500, body: <formatted> }` |
| `upstream_timeout`       | `UpstreamTimeout`             | `UpstreamTimeout`                |
| `upstream_unreachable`   | `UpstreamUnreachable(detail)` | `UpstreamUnreachable { detail }` |
| Integer range overflow   | `IntegerOverflow { column, value }` | `UpstreamError { status: 500, body: <formatted> }` (per doc 08's "overflow returns upstream_error") |
| Config deser fails       | `InvalidConfig(detail)`       | `InvalidConfig { detail }`       |
| Request deser fails      | `InvalidRequest(detail)`      | `InvalidRequest { detail }`      |
| Internal bug             | `Internal(detail)`            | `Internal { detail }`            |

`sqlx::Error` has a rich variant set; classifying it correctly
into `InvalidSql` vs `UpstreamDbError` vs `UpstreamUnreachable`
is a key correctness check. Rough guide:
- `sqlx::Error::Database(db_err)` with a syntax-error
  SQLSTATE class (42xxx) → `InvalidSql`.
- `sqlx::Error::Database(db_err)` with any other SQLSTATE →
  `UpstreamDbError`.
- `sqlx::Error::Io(_)` / `sqlx::Error::PoolTimedOut` /
  `sqlx::Error::Protocol(_)` → `UpstreamUnreachable`.
- `sqlx::Error::RowNotFound` / `sqlx::Error::TypeNotFound` /
  decoding errors → typically `Internal` (our impl bug).

Adapt as needed; flag any ambiguous case under Residual risks.

## Workspace conventions

- Edition 2024, MSRV 1.88.
- License `Apache-2.0 OR MPL-2.0`.
- `thiserror` for library errors; no `anyhow`.
- **No panics in library `src/`.** No `.unwrap()` /
  `.expect()` / `panic!` / `unreachable!` / `todo!` /
  `unimplemented!` on reachable paths, no unbounded indexing,
  no unchecked integer arithmetic, no lossy `as` casts.
- **Library crates take bytes, not file paths.** The
  `connection_url` is a string from the decrypted config;
  never read from disk or env.
- **No `unsafe`.**
- **Rustdoc on every `pub` item.**
- **Re-export discipline**: re-export `Implementation`,
  `ImplementationError`, `ConnectorCallContext`, `JsonValue`,
  `async_trait` from `connector-impl-api` so consumers depend
  on just this crate for the common case.
- **Use `./scripts/*.sh` wrappers** for git / test / lint.

## Pre-landing

```sh
./scripts/pre-landing.sh philharmonic-connector-impl-sql-postgres
```

Runs fmt + check + clippy (`-D warnings`) + test. Must pass
clean. Do NOT run raw `cargo fmt` / `cargo check` / `cargo
clippy` / `cargo test` — the script normalizes flag choices.

If integration tests need Docker and Docker isn't present,
pre-landing should still pass via `#[ignore]` on those tests.
Unit tests must be exercised and must pass.

## Git

You do NOT commit, push, branch, tag, or publish. Leave the
working tree dirty in the
`philharmonic-connector-impl-sql-postgres` submodule. Claude
runs `scripts/commit-all.sh` after review and
`scripts/publish-crate.sh` once ready.

Read-only git is fine (`git log`, `git diff`, `git show`,
`git blame`, `git status`, `git rev-parse`).

## Deliverables

1. `philharmonic-connector-impl-sql-postgres/Cargo.toml`
   populated; version `0.1.0`.
2. `philharmonic-connector-impl-sql-postgres/src/` — modules
   per the layout above with full implementation and
   colocated unit tests.
3. `philharmonic-connector-impl-sql-postgres/tests/` —
   integration tests (`happy_path.rs`, `error_cases.rs`,
   `types.rs`); `#[ignore]`-gated if Docker absent.
4. `philharmonic-connector-impl-sql-postgres/CHANGELOG.md` —
   `[0.1.0] - 2026-04-24` entry.
5. `philharmonic-connector-impl-sql-postgres/README.md` —
   one-paragraph expansion.

Working tree: dirty. Do not commit.

## Structured output contract

Return in your final message:

1. **Summary** (3–6 sentences): what landed, what tests
   pass, any deviations from the spec and why.
2. **Files touched**: bulleted list.
3. **Verification results**:
   - Output of `./scripts/pre-landing.sh
     philharmonic-connector-impl-sql-postgres`.
   - Test counts (unit / integration, passed / failed /
     ignored). Note whether Docker was available.
4. **Residual risks / TODOs**.
5. **Git state**: `git -C philharmonic-connector-impl-sql-postgres
   status --short`. Confirm no commit, no push.
6. **Dep versions used**: exact resolved `sqlx`,
   `testcontainers`, `tokio` versions.

## Follow-through, completeness, verification, missing-context, action safety

Same rules as round 01 of `phase-6-llm-openai-compat` and
round 01 of `phase-6-http-forward`:

- Pre-landing failures: fix, re-run; don't return red.
- Spec ambiguity: pick the minimal public-surface
  interpretation that matches doc 08 most closely; flag.
- Yanked dep: try next-older compatible minor; record.
- Docker unavailable: `#[ignore]` the integration tests that
  need it; document in tests/README.md and under Residual
  risks.
- Stop and flag if: required doc missing, impl-api surface
  differs from expectation, sqlx 0.8 has a breaking API
  change that makes the approach above materially wrong.
- No `cargo publish`, no `git push`, no branch creation, no
  tags.
- No edits outside `philharmonic-connector-impl-sql-postgres/`.
- No `rm -rf` or destructive ops.

Run the verification loop before returning:

```sh
./scripts/pre-landing.sh philharmonic-connector-impl-sql-postgres
cargo test -p philharmonic-connector-impl-sql-postgres --all-targets
git -C philharmonic-connector-impl-sql-postgres status --short
git -C . status --short
```

---

## Outcome

Pending — will be updated after Codex run.
