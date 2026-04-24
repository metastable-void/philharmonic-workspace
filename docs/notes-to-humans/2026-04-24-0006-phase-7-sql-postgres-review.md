# Phase 7 Tier 1 — `sql_postgres` Codex output review (round 01)

**Date:** 2026-04-24 (金), JST ~20:55
**Reviewer:** Claude Code
**Subject:** submodule `philharmonic-connector-impl-sql-postgres`,
  working tree dirty (not yet committed)
**Prompt archive:**
  [`docs/codex-prompts/2026-04-24-0003-phase-7-sql-postgres.md`](../codex-prompts/2026-04-24-0003-phase-7-sql-postgres.md)
**Codex session:**
  `019dbf1f-2702-70d0-84f1-042e09620c31` (10 min, 10:53→11:03 UTC),
  **terminated mid-run — no `task_complete` event, no structured
  summary, no pre-landing step**. The last action recorded in the
  rollout is a `cat > README.md` at `11:03:05.971 UTC`. No
  compile / lint / test was ever exercised inside the sandbox.
**Verdict:** **NOT ready to commit. Needs a narrow Codex round 02**
  (or Claude housekeeping fix) — the crate does not compile. Three
  real errors in [`src/types.rs`](../../philharmonic-connector-impl-sql-postgres/src/types.rs);
  one unused-import warning that would become a clippy error under
  `-D warnings`. One spec-drift flag on test layout (non-blocking).

## TL;DR

The Codex process ended before it ran the `./scripts/pre-landing.sh`
loop the prompt mandates, so **the errors below survived to the
working tree**. The crate structure and error mapping look right;
the bugs are narrow and localized in the SQL-to-JSON type module.
Easiest path: dispatch a round-02 that scopes to *exactly* the three
compile errors + the unused-import warning + the `tests/helpers.rs`
→ `tests/common/mod.rs` cargo-layout fix. No other changes to the
tree.

## Verification this session (what actually ran)

```
$ cargo fmt -p philharmonic-connector-impl-sql-postgres
  ↳ applied formatting; the bulk of it to src/types.rs (Codex
    wrote that file but never got to `cargo fmt --check`).
  ↳ this is the only edit Claude made to the working tree during
    review.

$ ./scripts/rust-lint.sh philharmonic-connector-impl-sql-postgres
  ↳ EXIT=1
  ↳ cargo fmt -p <crate> --check             : clean (after my fmt)
  ↳ cargo check -p <crate>                    : FAILED (3 errors)
  ↳ cargo clippy ... -- -D warnings           : not reached

$ ./scripts/rust-test.sh philharmonic-connector-impl-sql-postgres
  ↳ EXIT=101
  ↳ same 3 errors prevent compilation; no tests ran.
```

## Blocking issues (crate does not compile)

All three live in
[`src/types.rs`](../../philharmonic-connector-impl-sql-postgres/src/types.rs).

### B1. `u32` is not `sqlx::Decode<Postgres>` / `sqlx::Type<Postgres>`

[`src/types.rs:42`](../../philharmonic-connector-impl-sql-postgres/src/types.rs#L42):

```rust
"oid" => integer_to_json(
    column_name,
    i128::from(try_get::<u32>(row, index, column_name, sql_type)?),
),
```

`try_get::<u32>` requires both `Decode` and `Type` impls for the
target type — sqlx 0.8 implements neither for `u32`. The correct
carrier for the Postgres `OID` SQL type is
`sqlx::postgres::types::Oid` (a newtype wrapper around `u32`). Fix
is one line: swap the inner type to `sqlx::postgres::types::Oid`
and unwrap via `.0` before widening to `i128`.

### B2. `PgTimeTz` does not implement `Display`

[`src/types.rs:88–93`](../../philharmonic-connector-impl-sql-postgres/src/types.rs#L88-L93):

```rust
"timetz" => {
    let value = try_get::<
        sqlx::postgres::types::PgTimeTz<chrono::NaiveTime, chrono::FixedOffset>,
    >(row, index, column_name, sql_type)?;
    Ok(JsonValue::String(value.to_string()))
}
```

`PgTimeTz` does not implement `Display` in sqlx-postgres 0.8.6. The
`.to_string()` call fails E0599. Fix: format the two fields manually
— e.g.
`format!("{}{}", value.time.format("%H:%M:%S%.f"), format_offset(value.offset))`
— and document the chosen wire format in the rustdoc. This is a
spec-shaped call, so picking the exact format string may want to
pick up the same RFC 3339 shape used elsewhere in the module. (Doc
08 does not mention `timetz` specifically; Codex added it
defensively. Dropping it entirely is also defensible for 0.1.0.)

### B3. Unused import `sqlx::Column`

[`src/types.rs:6`](../../philharmonic-connector-impl-sql-postgres/src/types.rs#L6):

```rust
use sqlx::{Column, Row, ValueRef};
```

`Column` is unused in this module — the `SqlxColumn` trait is
imported in [`src/execute.rs`](../../philharmonic-connector-impl-sql-postgres/src/execute.rs)
instead. Currently surfaces only as a warning, but the workspace's
pre-landing rule runs
`cargo clippy --all-targets -- -D warnings`, which would elevate it
to a fatal error once B1/B2 are fixed. Fix: drop `Column` from the
use list.

## Non-blocking flags

### N1. `tests/helpers.rs` gets treated as its own test binary

Postgres used the same `tests/helpers.rs` layout as the spec's
suggested file-tree block. Cargo's integration-test harness picks up
every top-level `.rs` under `tests/` as a standalone test crate,
which means `helpers.rs` appears in the test output as a 0-test
binary. This was also flagged in the `llm_openai_compat` review
(flag #6 there). MySQL side avoided the issue by using
`tests/common/mod.rs` instead. Fix: rename to `tests/common/mod.rs`
and update the `mod helpers;` declarations in the three integration
files to `mod common;` (and their `use helpers::...` lines). Not a
correctness issue, just noise in test output.

### N2. Connection URL scheme check accepts only `postgres://`

[`src/config.rs:41`](../../philharmonic-connector-impl-sql-postgres/src/config.rs#L41):

```rust
if !self.connection_url.starts_with("postgres://") { ... }
```

sqlx also accepts `postgresql://` as a synonym (per the Postgres
libpq convention). The `llm_openai_compat` precedent argued for
being permissive on scheme aliases where the underlying driver
handles both; MySQL side accepted both `mysql://` and `mariadb://`
for the same reason. Low severity — scripts can normalize — but
worth harmonizing with the MySQL approach.

### N3. `PgTimeTz` scope decision

If B2 is fixed by synthesising a format string, the resulting
`timetz` wire value is something we invented. Doc 08 §"SQL-to-JSON
type mapping" does not list `timetz`. Cleanest v1 move is probably
to **drop the `timetz` arm entirely** and let it fall through to
the catch-all (which coerces via `String`). Worth a quick call —
flagging here rather than deciding unilaterally.

## Spec / ROADMAP conformance (best-effort assessment despite no-compile)

From [`ROADMAP.md`](../../ROADMAP.md) Phase 7 §SQL and the prompt
archive:

| Criterion                                                            | Status |
| ---                                                                  | ---    |
| `Implementation::name()` returns `"sql_postgres"` ([lib.rs:62](../../philharmonic-connector-impl-sql-postgres/src/lib.rs#L62)) | ✓ |
| Config shape `{connection_url, max_connections, default_timeout_ms, default_max_rows}` with `deny_unknown_fields` | ✓ |
| Request shape with optional `max_rows` / `timeout_ms`, downward-only clamping | ✓ |
| Response shape `{rows, row_count, columns, truncated}` | ✓ |
| `max_rows` truncation via streaming `take(max_rows+1)` + pop | ✓ |
| `timeout_ms` via `tokio::time::timeout` at effective timeout | ✓ |
| DECIMAL/NUMERIC as JSON string; i64-range overflow → `UpstreamError` 500 | ✓ |
| sqlx 0.8 + rustls + `default-features = false`; no native-tls | ✓ |
| Error mapping per doc 08 (syntax → InvalidRequest, parameter mismatch → InvalidRequest, connection → UpstreamUnreachable, timeout → UpstreamTimeout) ([error.rs](../../philharmonic-connector-impl-sql-postgres/src/error.rs)) | ✓ |
| SQLSTATE 42xxx → `InvalidSql`; other DB codes → `UpstreamDbError` | ✓ |
| No connector-level retry | ✓ |
| No panics in library `src/` (ctor `.unwrap()`-free; tests use `.unwrap()` only) | ✓ |
| `[0.0.0]` CHANGELOG entry dropped | ✓ |
| Crate-root rustdoc with usage snippet | ✓ |
| No workspace-root edits | ✓ (Cargo.lock dirtied, which is legitimate — new dep set) |
| **Crate compiles** | **✗ — B1, B2** |
| **Pre-landing green** | **✗ — never ran** |
| **Unit tests exercised** | **✗ — never ran** |

## Module inventory (what Codex produced)

```
philharmonic-connector-impl-sql-postgres/
├── Cargo.toml    — 0.1.0, sqlx+rustls+chrono+uuid+json features, async-trait, futures-util, thiserror, base64
├── CHANGELOG.md  — [0.1.0] - 2026-04-24 entry
├── README.md     — one-paragraph expansion
├── src/
│   ├── lib.rs       —  105 LOC — SqlPostgres type + Implementation impl + re-exports + rustdoc
│   ├── config.rs    —  151 LOC — SqlPostgresConfig + prepare() + 4 unit tests
│   ├── request.rs   —  141 LOC — SqlQueryRequest + effective_limits + 4 unit tests
│   ├── response.rs  —   63 LOC — SqlQueryResponse + Column + 1 unit test
│   ├── execute.rs   —  271 LOC — execute_query, bind, classify_sqlx_error + 2 unit tests
│   ├── types.rs     —  399 LOC — decode_cell + array_cell + helpers + 10 unit tests  ← COMPILE FAILS HERE
│   └── error.rs     —  142 LOC — Error enum + From<Error> for ImplementationError + 1 test
└── tests/
    ├── helpers.rs       —  120 LOC — testcontainers harness (see N1)
    ├── happy_path.rs    —  171 LOC — 2 #[ignore]d Docker tests
    ├── error_cases.rs   —  184 LOC — 6 #[ignore]d Docker tests
    └── types.rs         —   99 LOC — 1 #[ignore]d type-mapping Docker test
```

## Recommended next step

**Dispatch a narrow Codex round-02** with this scope:

1. Fix B1 by using `sqlx::postgres::types::Oid` for the `oid` arm.
2. Fix B2 — either drop the `timetz` arm (preferred, v1) or format
   `PgTimeTz` manually; rustdoc whichever is chosen.
3. Drop the unused `Column` import (B3).
4. Rename `tests/helpers.rs` → `tests/common/mod.rs` and update the
   three integration test files (N1).
5. Run `./scripts/pre-landing.sh
   philharmonic-connector-impl-sql-postgres` and return green before
   producing the structured output.

Round-02 should be able to ship in well under 10 minutes — the
round-01 scaffold is the hard part. Fmt fix I applied during this
review is already in the tree, so round-02 only needs to touch the
four things above.

Alternatively: **Claude applies B1/B2/B3/N1 as housekeeping** and
runs pre-landing itself (all four changes are small and mechanical).
Reasonable either way; mild preference for round-02 to keep the
"Codex owns the impl" boundary clean.

After a green round-02, write a follow-up review-delta
(`-02-review.md`) and then commit + publish via
`./scripts/commit-all.sh` + `./scripts/publish-crate.sh`.

## Files touched this session

- [`philharmonic-connector-impl-sql-postgres/src/*.rs`](../../philharmonic-connector-impl-sql-postgres/src/)
  — `cargo fmt -p philharmonic-connector-impl-sql-postgres` applied
  (minor whitespace layout in `types.rs`; rest untouched).
- [`docs/notes-to-humans/2026-04-24-0006-phase-7-sql-postgres-review.md`](2026-04-24-0006-phase-7-sql-postgres-review.md)
  — this file.
