# Phase 7 Tier 1 — `sql_mysql` Codex output review (round 01)

**Date:** 2026-04-24 (金), JST ~21:00
**Reviewer:** Claude Code
**Subject:** submodule `philharmonic-connector-impl-sql-mysql`,
  working tree dirty (not yet committed)
**Prompt archive:**
  [`docs/codex-prompts/2026-04-24-0004-phase-7-sql-mysql.md`](../codex-prompts/2026-04-24-0004-phase-7-sql-mysql.md)
**Codex session:**
  `019dbf1f-5bc4-7d10-8ff3-2a162c5568ab` (~29 min, 10:53→11:22 UTC;
  clean `task_complete` + structured summary returned).
**Verdict:** **Ready to commit + publish** as `0.1.0`. Pre-landing
  workspace-wide is blocked only by the unrelated postgres-crate
  compile failure (see the sibling review `-0006-*`); crate-scoped
  lint and unit-plus-non-ignored-integration tests are green. Four
  non-blocking flags below.

## TL;DR

Codex delivered a complete `philharmonic-connector-impl-sql-mysql`
`0.1.0`: 7 `src/` modules + 3 `tests/` integration suites sharing a
`tests/common/mod.rs` harness. Crate-scoped verification is green
without any fix from Claude. Codex's own ignore-gated Docker run
(per its final structured report) passed `happy_path` 2/2,
`error_cases` 1/1, `types` 1/1 against `mysql:8.0`; I did **not**
re-run those under Docker this session. Four non-blocking flags,
none of which gate `0.1.0`.

## Verification this session (what actually ran)

```
$ ./scripts/rust-lint.sh philharmonic-connector-impl-sql-mysql
  ↳ EXIT=0
  ↳ cargo fmt -p <crate> --check             : clean
  ↳ cargo check -p <crate>                    : clean
  ↳ cargo clippy -p <crate> --all-targets
                -- -D warnings                 : clean
  ↳ === rust-lint: clean ===

$ ./scripts/rust-test.sh philharmonic-connector-impl-sql-mysql
  ↳ EXIT=0
  ↳ lib unit:       18 passed / 0 failed / 0 ignored
  ↳ tests/error_cases.rs:  2 passed / 0 failed / 1 ignored (Docker)
  ↳ tests/happy_path.rs:   1 passed / 0 failed / 2 ignored (Docker)
  ↳ tests/types.rs:        1 passed / 0 failed / 1 ignored (Docker)
  ↳ doctests:              0 passed / 0 failed / 0 ignored
  ↳ Totals: 22 passed / 0 failed / 4 ignored (Docker-gated)

Codex-reported (Docker-present sandbox, not re-verified this session):
  $ ./scripts/rust-test.sh --ignored philharmonic-connector-impl-sql-mysql
  ↳ error_cases: 1/1 pass, happy_path: 2/2 pass, types: 1/1 pass
```

Workspace-scoped pre-landing still fails at `cargo fmt --all
--check` because of the sibling postgres tree being mid-fix. Not a
mysql issue; this review is for the mysql crate in isolation.

## Conformance with spec + ROADMAP

From the prompt archive and
[`docs/design/08-connector-architecture.md`](../design/08-connector-architecture.md)
§"SQL":

| Criterion                                                            | Status |
| ---                                                                  | ---    |
| `Implementation::name()` returns `"sql_mysql"` ([lib.rs:36](../../philharmonic-connector-impl-sql-mysql/src/lib.rs#L36)) | ✓ |
| Config shape `{connection_url, max_connections, default_timeout_ms, default_max_rows}` + `deny_unknown_fields` | ✓ |
| `connection_url` accepts both `mysql://` and `mariadb://`, rejects others at `prepare()` ([config.rs:97](../../philharmonic-connector-impl-sql-mysql/src/config.rs#L97)) | ✓ |
| Request shape `{sql, params, max_rows?, timeout_ms?}` + downward-only clamp helpers ([request.rs:23](../../philharmonic-connector-impl-sql-mysql/src/request.rs#L23)) | ✓ |
| Response shape `{rows, row_count, columns, truncated}` ([response.rs:8](../../philharmonic-connector-impl-sql-mysql/src/response.rs#L8)) | ✓ |
| MySQL `?` positional placeholders — bespoke state-machine parser handles quote / identifier / comment contexts ([execute.rs:132](../../philharmonic-connector-impl-sql-mysql/src/execute.rs#L132)) | ✓ |
| `max_rows` via `fetch(...).take(max_rows + 1)` with post-stream truncate + `truncated: true` ([execute.rs:64](../../philharmonic-connector-impl-sql-mysql/src/execute.rs#L64)) | ✓ |
| `timeout_ms` via `tokio::time::timeout` on the whole describe+execute closure ([execute.rs:92](../../philharmonic-connector-impl-sql-mysql/src/execute.rs#L92)) | ✓ |
| Unsigned > `i64::MAX` → `IntegerOverflow` → `UpstreamError{500, ...}` ([types.rs:193](../../philharmonic-connector-impl-sql-mysql/src/types.rs#L193)) | ✓ |
| DECIMAL/NUMERIC → JSON string (precision preserved) ([types.rs:168](../../philharmonic-connector-impl-sql-mysql/src/types.rs#L168)) | ✓ |
| DATETIME zone-naive, no suffix; TIMESTAMP UTC, `Z` suffix ([types.rs:176-180](../../philharmonic-connector-impl-sql-mysql/src/types.rs#L176-L180)) | ✓ |
| MySQL JSON type → verbatim `JsonValue` ([types.rs:91](../../philharmonic-connector-impl-sql-mysql/src/types.rs#L91)) | ✓ |
| Top-level JSON-array params rejected as `InvalidRequest` ([execute.rs:120](../../philharmonic-connector-impl-sql-mysql/src/execute.rs#L120)) | ✓ |
| Syntax errors → `InvalidSql → InvalidRequest`; SQLSTATE `42xxx` + MySQL codes `1064 / 1054 / 1146 / 1149` detected ([error.rs:67](../../philharmonic-connector-impl-sql-mysql/src/error.rs#L67)) | ✓ |
| Connection IO / pool / protocol → `UpstreamUnreachable` ([error.rs:43](../../philharmonic-connector-impl-sql-mysql/src/error.rs#L43)) | ✓ |
| sqlx 0.8 + rustls + `default-features = false`; no native-tls | ✓ |
| No connector-level retry | ✓ |
| No panics in library `src/`; no `unsafe`; `thiserror` for errors | ✓ |
| `[0.0.0]` CHANGELOG entry dropped; `[0.1.0] - 2026-04-24` present | ✓ |
| Crate-root rustdoc w/ MySQL-specific quirks summary | ✓ |
| No workspace-root edits; no commit / push by Codex | ✓ |
| Working tree dirty, per prompt | ✓ |

## Module inventory

```
philharmonic-connector-impl-sql-mysql/
├── Cargo.toml    — 0.1.0, sqlx+rustls+mysql+chrono+uuid+json, chrono direct dep, async-trait, base64, futures-util, thiserror
├── CHANGELOG.md  — [0.1.0] - 2026-04-24 entry; [0.0.0] dropped
├── README.md     — one-paragraph expansion
├── src/
│   ├── lib.rs       —  89 LOC — SqlMysql + Implementation impl + re-exports + name test
│   ├── config.rs    — 177 LOC — SqlMysqlConfig + validate_connection_url_scheme + 4 tests (incl. async scheme test)
│   ├── request.rs   —  91 LOC — SqlQueryRequest + effective_* clamp helpers + 3 tests
│   ├── response.rs  —  53 LOC — SqlQueryResponse + Column + 1 test
│   ├── execute.rs   — 268 LOC — execute_sql_query, bind_params, count_mysql_placeholders state machine + 2 tests
│   ├── types.rs     — 345 LOC — DecodedValue enum + per-type `is_*` discriminators + json_from_decoded + 5 tests
│   └── error.rs     — 186 LOC — Error enum + classify_database_error + From<Error> for ImplementationError + 1 test
└── tests/
    ├── common/mod.rs     — shared testcontainers harness (proper cargo layout, no orphan test binary)
    ├── happy_path.rs     — 3 tests (2 #[ignore]d Docker + 1 non-ignored type-accessibility smoke)
    ├── error_cases.rs    — 3 tests (1 #[ignore]d Docker + 2 non-ignored: parameter mismatch, connection refused)
    └── types.rs          — 2 tests (1 #[ignore]d Docker + 1 non-ignored file-present)
```

Codex also added **non-ignored** integration smokes where they
didn't need a DB (parameter-mismatch — which short-circuits in
`count_mysql_placeholders` before touching sqlx, and
connection-refused — which only needs a bogus URL). Nice
Docker-absent coverage that the prompt didn't ask for.

## Non-blocking flags (numbered for reference)

1. **`try_get_unchecked` used for `DECIMAL` and `TIMESTAMP` decode
   paths** ([types.rs:72](../../philharmonic-connector-impl-sql-mysql/src/types.rs#L72),
   [types.rs:128](../../philharmonic-connector-impl-sql-mysql/src/types.rs#L128)).
   `try_get_unchecked` skips sqlx's runtime compile-time-derived
   type check, which Codex flagged in its own Residual risks as a
   workaround for sqlx-mysql's type-compat behavior around those
   two columns. `_unchecked` here is not `unsafe` — the underlying
   decode still returns `Result` — but it does lose the type-system
   assertion that the SQL column's declared type matches what we're
   asking for. The unit-test matrix covers these paths end-to-end
   under Docker; any regression would surface as a decode error in
   integration, not silent corruption. Worth a second look if we
   ever bump sqlx to 0.9+.

2. **MySQL `BOOLEAN` vs `TINYINT(1)` classification is name-based**
   ([types.rs:225](../../philharmonic-connector-impl-sql-mysql/src/types.rs#L225)).
   MySQL aliases `BOOLEAN` to `TINYINT(1)` at schema level, but the
   sqlx type-info name reported for a boolean column can vary
   (`boolean` on some configs, `tinyint` on others). The current
   code maps `bool | boolean` → `Bool`, anything else starting
   `tinyint` → `Signed(i64)`. A column declared `BOOLEAN` that sqlx
   reports as `tinyint` will surface as `0`/`1` integers rather than
   `false`/`true`. Low severity; scripts can coerce; flagged in
   Codex's own Residual risks. Easy follow-up: special-case
   `tinyint(1)` in the discriminator or offer a config knob.

3. **`value as i64` cast in `unsigned_to_json`** ([types.rs:201](../../philharmonic-connector-impl-sql-mysql/src/types.rs#L201))
   is guarded by a `value > i64::MAX as u64` early return two lines
   above, so the cast is provably non-lossy. However
   `CONTRIBUTING.md §10.3` prefers avoiding `as` casts even when
   the bounds are hand-verified. Swap for
   `i64::try_from(value).map_err(...)` and drop the explicit
   comparison, or add a short `// safe: checked above` comment —
   either works. Nitpick.

4. **`is_unsigned_integer_type` uses `contains("unsigned")` +
   individual substring checks** ([types.rs:204–212](../../philharmonic-connector-impl-sql-mysql/src/types.rs#L204-L212)).
   The current predicate would also accept pathological strings
   like `"bigint unsigned zerofill"`, which is actually the correct
   behavior for MySQL type names with modifiers. But a column typed
   `tinyint unsigned zerofill` would collapse to "unsigned
   integer" and go through `Unsigned(u64)` — fine, but not something
   the unit tests directly cover. The substring match is pragmatic;
   flagging only for future awareness.

## Recommendation

**Commit + publish 0.1.0**, gated only on the sibling postgres
tree becoming clean so the workspace-scoped pre-landing passes.
Concretely:

1. Sort out the postgres compile failure (see `-0006-*` review);
   that unblocks workspace-wide pre-landing.
2. Run `./scripts/commit-all.sh` — both submodule commits land
   together; parent pointer bumps carry `Cargo.lock`.
3. Run `./scripts/publish-crate.sh
   philharmonic-connector-impl-sql-mysql` once the commit lands.

The four flags above are all follow-up material; none affects the
0.1.0 wire contract.

## Files touched this session

- [`docs/notes-to-humans/2026-04-24-0007-phase-7-sql-mysql-review.md`](2026-04-24-0007-phase-7-sql-mysql-review.md)
  — this file.

No edits to the submodule tree itself during this review pass —
what Codex wrote is what will ship.
