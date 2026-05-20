# Entity list sort: optional `?sort=modified_desc` mode, WebUI opts in

**Date:** 2026-05-20 (JST)
**Slug:** `entity-list-sort-modified-opt-in`
**Round:** 01 — initial dispatch. Touches `philharmonic-api`,
`philharmonic-store`, `philharmonic-store-sqlx-mysql`, and the
`philharmonic` submodule's `webui/` source.
**Subagent:** `codex:rescue`

## Motivation

WebUI list pages currently surface items in creation-date-desc
order. For long-lived tenants, "most recently modified" is the
more useful default for human browsing — a template the admin
edited yesterday should bubble to the top, not get buried under
items created earlier.

Per Yuka 2026-05-20: add this as an **optional, opt-in sort
mode** on every entity list API endpoint. The default behaviour
stays `created_desc`; old API consumers (anything that doesn't
explicitly request `?sort=modified_desc`) see no change.
**Only the WebUI flips to the new mode.** The low-level schema
must stay as-is except for **one new index** on the existing
`entity_revision` table that keeps the per-entity
`MAX(created_at)` lookup cheap. No new columns, no new tables,
no per-kind denorm.

The implementation strategy is the "JOIN MAX(revision.created_at)"
approach the design discussion settled on: when sort=modified_desc,
the route fetches the latest revision timestamp per row and
uses that as the in-memory sort key for `paginate_items`. The
existing pagination shape (Rust-side sort over a fetched batch)
stays exactly the same — only the *sort key* changes.

## Why this is one of the lower-cost paths

The pagination here is **already in-memory in the route layer**
([`philharmonic-api/src/routes/identity.rs::paginate_items`](../../philharmonic-api/src/routes/identity.rs#L247-L278)
and two copies in `endpoints.rs` / `embed_datasets.rs`). The
route fetches the full result via `find_by_scalar_typed` /
`find_by_content_typed`, dedupes, then sorts and slices the
`Vec<(CursorKey, T)>` tuple by `(created_at DESC, id DESC)`.
There is **no SQL `ORDER BY` to change**. Switching sort keys
is a Rust-level edit at the tuple-build site for each list
endpoint, plus a small store helper that returns
`{entity_id → latest_modified_at}` for a batch of entity IDs.

## References (authoritative if anything in this prompt contradicts them)

1. [`philharmonic-api/src/pagination.rs`](../../philharmonic-api/src/pagination.rs)
   — the cursor types and helpers. `CursorKey` is currently
   `{ created_at: UnixMillis, id: Uuid }`; `CursorWire` is the
   base64url-JSON wire form.
2. [`philharmonic-api/src/routes/identity.rs`](../../philharmonic-api/src/routes/identity.rs#L247-L278)
   — the canonical `paginate_items` helper (`pub(super)`).
   Two more copies live in `routes/endpoints.rs:640` and
   `routes/embed_datasets.rs:773`; keep the three in lockstep
   or consolidate during this round (your judgment — see
   below).
3. [`philharmonic-store-sqlx-mysql/src/schema.rs`](../../philharmonic-store-sqlx-mysql/src/schema.rs)
   — `entity_revision` table is
   `PRIMARY KEY (entity_id, revision_seq)`. The new index goes
   in this file; add it to the `INDEX_MIGRATIONS` block so
   pre-existing deployments pick it up idempotently on next
   `apply_schema()`.
4. [`philharmonic-store/src/entity.rs`](../../philharmonic-store/src/entity.rs)
   — `EntityStore` trait. Add a new method (or extend an
   existing one) for the batch latest-revision-timestamp
   lookup.
5. [`docs/design/05-storage-substrate.md`](../design/05-storage-substrate.md)
   and [`docs/design/10-api-layer.md`](../design/10-api-layer.md)
   — touch only if the new query parameter is documented there
   (most likely 10-api-layer.md's pagination section); a
   one-line addition is enough.
6. [`CLAUDE.md`](../../CLAUDE.md) §"Hard rules vs. soft rules"
   and [`CONTRIBUTING.md`](../../CONTRIBUTING.md):
   - **§4** Git workflow — `./scripts/commit-all.sh` only;
     **you do not commit** (see Hand-off shape below).
   - **§5** Script wrappers — every cargo call routes via the
     wrappers (which set `CARGO_TARGET_DIR=target-main`).
   - **§10.3** No panics in library `src/` — no `.unwrap()` /
     `.expect()` on `Result` / `Option`, no `panic!` /
     `unreachable!` / `todo!` / `unimplemented!` on reachable
     paths. Tests exempt.
   - **§11** Pre-landing checks — `./scripts/pre-landing.sh`
     is mandatory before declaring done.

## Hard constraints (from Yuka, locked)

- **No low-level schema change except adding one new index** on
  the existing `entity_revision` table. No new columns, no new
  tables, no per-kind tables (the substrate is generic — one
  `entity` + one `entity_revision` keyed by `kind`). Migrations
  beyond the index addition are out of scope and would be
  rejected at review.
- **The new sort mode is optional**. Default for all list
  endpoints stays `created_desc`. Existing API consumers see
  no change unless they send `?sort=modified_desc`.
- **Old code continues to work as-is.** Cursors issued by the
  current build remain decodable; an old cursor decodes to
  `sort_mode = CreatedDesc` (default on missing field) and
  paginates the existing ordering correctly. No cursor-format
  break.
- **Only the WebUI switches to the new behaviour.** Every list
  page in `philharmonic/webui/src/pages/` that today calls a
  paginated list endpoint adds `?sort=modified_desc` to the
  request. No user-facing sort toggle, no dropdown — just
  flip the default on the WebUI side.
- **Audit log is exempt.** `routes/audit.rs::list_audit` sorts
  by event timestamp, not by revision; events are immutable;
  "modified" is meaningless. Leave it as-is (it ignores `?sort`
  entirely or errors if the param is passed — your call, but
  document the choice).
- **Cursor sort-mode mismatch is a 400, not a silent fallback.**
  If the caller sends `?sort=modified_desc` with a cursor that
  was issued under `created_desc` (or vice versa), the route
  returns `400 invalid pagination cursor` rather than
  surreptitiously switching mid-stream — the page would be
  nonsense across the boundary otherwise.

## Per-file scope (the full set of edits)

### `philharmonic-api/src/pagination.rs`

- Add `SortMode { CreatedDesc, ModifiedDesc }` enum;
  `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]`
  with `#[serde(rename_all = "snake_case")]` and
  `#[default]` on `CreatedDesc`.
- Extend `PaginationParams` with
  `pub sort: SortMode` (`#[serde(default)]`).
- Extend `CursorKey` so it can carry either timestamp under the
  active sort. Your judgment on the cleanest shape — preferred
  shape is: rename the field to `sort_key_value: UnixMillis`
  and add `sort_mode: SortMode`. Document the rename in the
  cursor module-level doc comment and update the three call
  sites (`identity.rs`, `endpoints.rs`, `embed_datasets.rs`)
  that construct it. Either way: keep the public-API-facing
  type names stable (`CursorKey`, `PaginationParams`,
  `PaginatedResponse`).
- Extend `CursorWire` with an optional `sort` field that
  defaults to `CreatedDesc` on decode (`#[serde(default)]`).
  Encoded cursors always include the field; old cursors that
  predate this change still decode (the missing field defaults
  to `CreatedDesc`).
- Update `decode_cursor` / `encode_cursor` accordingly.
- Tests:
  - Existing `cursor_round_trips` test still passes (now via
    explicit `SortMode::CreatedDesc`).
  - New round-trip test for `SortMode::ModifiedDesc`.
  - New test: decoding an old wire format (no `sort` field) →
    `SortMode::CreatedDesc`. Use a hand-built
    `serde_json::from_str` payload like
    `r#"{"created_at": 123, "id": "..."}"`# (or whatever the
    pre-change wire shape was) → confirm it decodes and
    defaults to `CreatedDesc`.
  - New test: `paginate_items` (whichever copy you keep)
    returns a 400-equivalent error when the cursor's
    `sort_mode` doesn't match the requested `params.sort`.

### `philharmonic-api/src/routes/identity.rs`, `endpoints.rs`, `embed_datasets.rs`

- All three `paginate_items` helpers become sort-mode aware.
  Strongly preferred: **consolidate** the three copies into
  one canonical `pagination::paginate_items_sorted(...)` (or
  similar name) and have the three route files import it.
  The duplication is a known wart and this dispatch is the
  natural moment to dedupe. If the consolidation is awkward
  (e.g., the `endpoints.rs` copy has a unique generic bound
  the others don't), document why in your session summary
  and leave them as three but keep them identical.
- Each tuple-build site (each `CursorKey::new(...)` call —
  see [`grep -n CursorKey::new philharmonic-api/src/routes`](../../philharmonic-api/src/routes/)
  for the inventory; roughly 10 sites across
  `authorities.rs`, `embed_datasets.rs`, `endpoints.rs`,
  `memberships.rs`, `principals.rs`, `roles.rs`,
  `workflows.rs` — plus `audit.rs` which is exempt) now
  takes a `sort_mode` and a `sort_key_value`. When
  `sort_mode == ModifiedDesc`, the `sort_key_value` is the
  entity's `latest_modified_at` (see below); otherwise it's
  the existing `row.created_at`.

### `philharmonic-store/src/entity.rs` (trait) + `philharmonic-store-sqlx-mysql/src/entity.rs` (impl) + the in-tree `MockEntityStore` (tests)

- Add a new `EntityStore` trait method:

  ```rust
  /// Return, for each entity_id in the input, the `created_at`
  /// of its highest-`revision_seq` revision. Entities with no
  /// revisions yet (created via `create_entity` but never
  /// appended-to) MUST NOT appear in the returned map — the
  /// caller falls back to `entity.created_at` for those.
  async fn latest_revision_timestamps(
      &self,
      entity_ids: &[Uuid],
  ) -> Result<HashMap<Uuid, UnixMillis>, StoreError>;
  ```

  Implementation in `sqlx-mysql/src/entity.rs`: one query of
  the form

  ```sql
  SELECT er.entity_id, er.created_at
  FROM entity_revision er
  INNER JOIN (
    SELECT entity_id, MAX(revision_seq) AS max_seq
    FROM entity_revision
    WHERE entity_id IN (?, ?, ...)
    GROUP BY entity_id
  ) latest
    ON latest.entity_id = er.entity_id
   AND latest.max_seq   = er.revision_seq
  ```

  Notes:
  - `revision_seq` is monotonically increasing per-entity
    (revisions are appended via `next_revision_seq(&latest)`),
    so `MAX(revision_seq)` selects the latest row and its
    `created_at` is the latest `created_at` per entity. Using
    `MAX(revision_seq)` is cheaper than `MAX(created_at)`
    because it rides the PK.
  - The new index (see schema.rs below) on
    `(entity_id, created_at)` is **not strictly required** for
    this query because the PK already handles
    `MAX(revision_seq) GROUP BY entity_id`. The index exists
    so the alternate "MAX(created_at) GROUP BY entity_id"
    formulation — and any future ad-hoc analytics — stays
    cheap. Add it anyway per Yuka's direction.
  - Use a parameterised `IN (...)` with the right number of
    `?` placeholders for the input length. Empty input →
    return empty map without querying.
  - The implementation must be `O(1)` round trips regardless
    of input length. **Do not** call `get_latest_revision` in a
    loop.
  - In-tree `MockEntityStore` (the one in
    [`philharmonic-store/src/entity.rs`'s tests
    module](../../philharmonic-store/src/entity.rs#L488))
    gains an implementation that iterates its in-memory
    revisions and returns the same shape.

- Existing `EntityStore` methods keep their signatures
  exactly. Anything that compiled before this round still
  compiles. New method is additive.

### `philharmonic-store-sqlx-mysql/src/schema.rs`

- Add one index migration to `INDEX_MIGRATIONS`:

  ```rust
  "ALTER TABLE entity_revision ADD INDEX ix_entity_revision_entity_created (entity_id, created_at)",
  ```

  Idempotent ALTER (the comment block above `INDEX_MIGRATIONS`
  spells out why — `CREATE TABLE IF NOT EXISTS` won't add
  indexes to pre-existing tables). No new columns; no
  table-shape change.

### Per-route tuple-build sites (the modified-mode path)

For each list endpoint that needs the new mode (everything
except `routes/audit.rs::list_audit`):

1. Decode the cursor and validate `cursor.sort_mode ==
   params.sort` (or no cursor at all). On mismatch return
   `400 invalid pagination cursor`.
2. Fetch the rows as today (`find_by_scalar_typed` /
   `find_by_content_typed`, dedupe).
3. If `params.sort == ModifiedDesc`, call
   `store.latest_revision_timestamps(&row_ids)` once,
   collecting the result into a `HashMap<Uuid, UnixMillis>`.
   Build each tuple's `CursorKey` with
   `sort_key_value = map.get(&row.identity.internal).copied()
   .unwrap_or(row.created_at)` (the `unwrap_or` covers the
   "no revisions yet" case — fall back to entity creation
   timestamp so newly-created-but-never-revised entities still
   sort somewhere sensible).
4. If `params.sort == CreatedDesc` (default), tuples are built
   exactly as today: `sort_key_value = row.created_at`,
   `sort_mode = CreatedDesc`.
5. Pass the tuples + params to the (now sort-mode-aware)
   `paginate_items`. Encoded next-cursor will carry the matching
   `sort_mode` discriminator so subsequent pages stay coherent.

Per-route inventory (touch each one):

- `routes/authorities.rs::list_authorities`
- `routes/embed_datasets.rs::list_datasets`
- `routes/endpoints.rs::list_endpoints`
- `routes/memberships.rs::list_memberships`
- `routes/principals.rs::list_principals`
- `routes/roles.rs::list_roles`
- `routes/workflows.rs::list_templates`
- `routes/workflows.rs::list_instances`
- `routes/workflows.rs::instance_history` — review whether
  this list is "instance + its revisions" (sortable by
  revision time, which IS the entity-modified time) or
  something else; preserve existing semantics if the modified
  sort doesn't apply meaningfully.
- `routes/audit.rs::list_audit` — **exempt**. Sort param is
  ignored or rejected; pick one and call it out.

The four other `CursorKey::new` sites that the grep
(`workflows.rs:1089`, the step record / revision listing
contexts) need a per-site decision: if the listed thing is
already a revision-shaped row whose "created_at" is itself
the modification time, then `created_desc` and `modified_desc`
collapse to the same answer — keep the existing behaviour and
just accept the param as a no-op for that endpoint (but still
honour the cursor-format change).

### WebUI side — `philharmonic/webui/`

For every page in `philharmonic/webui/src/pages/` that fetches
a paginated list (Templates, Instances, Endpoints, Principals,
Roles, Memberships, Authorities, EmbedDatasets), add
`sort=modified_desc` to the API call. Audit log stays as-is.

If the WebUI's API call sites go through a typed helper in
`philharmonic/webui/src/api/client.ts`, extend the helper's
parameter shape (`sort?: "created_desc" | "modified_desc"`).
If list calls are made ad-hoc via `apiCall<...>("foo?cursor=...")`
with hand-built query strings, add the `sort=modified_desc`
parameter at each site (and consolidate into a helper while
you're there if it's mechanical).

The WebUI is in the `philharmonic` submodule; that submodule's
working tree will be dirty alongside `philharmonic-api`,
`philharmonic-store`, `philharmonic-store-sqlx-mysql`, and the
parent (`Cargo.lock`).

After the WebUI source change, run
`./scripts/webui-build.sh --production` to regenerate
`philharmonic/webui/dist/{main.css,main.css.map,main.js,main.js.map}`
— those committed artefacts ship with the Rust binary that
embeds the WebUI, so they need to land in the same commit
Claude makes.

## Shape (locked)

### `CursorKey` (preferred shape — rename)

```rust
pub(crate) struct CursorKey {
    pub(crate) sort_key_value: UnixMillis,
    pub(crate) id: Uuid,
    pub(crate) sort_mode: SortMode,
}
```

### `CursorWire`

```rust
#[derive(Deserialize, Serialize)]
struct CursorWire {
    /// Sort-key timestamp at the encoded boundary. Was named
    /// `created_at` in the pre-change wire format; the rename
    /// is internal-only (the JSON field name stays
    /// `created_at` on the wire for backward compatibility).
    #[serde(rename = "created_at")]
    sort_key_value: i64,
    id: Uuid,
    #[serde(default)]
    sort: SortMode,
}
```

The wire-field name stays `created_at` so existing cursors in
the wild keep decoding. The internal Rust name changes for
clarity but the JSON shape is backward compatible.

### Query parameter

```
GET /v1/<resource>?cursor=<opaque>&limit=50&sort=modified_desc
```

`sort` is optional; default `created_desc`. Acceptable
values: `created_desc`, `modified_desc`. Unknown values →
serde-level deserialization error → 400 (existing
serde-error → API-error path covers this).

## Tests

Add coverage in (likely) `philharmonic-api/tests/` or extend
the existing route-level test files for each endpoint:

### `pagination.rs` unit tests

1. `SortMode` round-trips through serde with snake_case wire
   form.
2. `CursorKey` round-trips with `sort_mode = CreatedDesc`
   (existing test, updated).
3. `CursorKey` round-trips with `sort_mode = ModifiedDesc`.
4. **Backward compatibility**: hand-build a JSON payload
   matching the pre-change wire format (no `sort` field) →
   decodes to `CreatedDesc`.
5. **Cursor-mode mismatch rejected**: `paginate_items`
   (whichever consolidated helper you settle on) returns the
   400-equivalent error when `params.sort == ModifiedDesc`
   and the cursor's `sort_mode == CreatedDesc`. Mirror for
   the inverse.

### Per-endpoint integration tests (extend existing test files)

Pick **two representative endpoints** for the integration
matrix — `list_endpoints` and `list_templates` are good
candidates because they have the most revisions in typical
fixtures. For each:

6. **Default behaviour unchanged**: no `sort` query param →
   results sort exactly as before (created_desc).
7. **Modified-sort returns by latest revision**: create three
   entities A, B, C with creation times A < B < C. Append a
   revision to A so its latest_modified > B's, C's
   latest_modified. Call `?sort=modified_desc` → result order
   is A, C, B. Default sort (no param) → C, B, A.
8. **Modified-sort + cursor round-trip**: paginate a 5-entity
   set with `limit=2` under `?sort=modified_desc`, follow the
   `next_cursor`, get the next 2, etc.; assert correctness.
9. **Cursor mismatch is 400**: issue a cursor under
   `created_desc`, replay it with `?sort=modified_desc` →
   `400 invalid pagination cursor`.
10. **Entity with no revisions yet** (`create_entity` but no
    `append_revision`): falls back to `entity.created_at`
    under both sort modes; does not get filtered out, does
    not panic.

### `latest_revision_timestamps` store-level test

11. In `philharmonic-store-sqlx-mysql/tests/` (or the inline
    integration test pattern used by the crate), assert the
    new method:
    - Empty input → empty map, no SQL round trip.
    - Mixed input where some entity_ids have revisions and
      others don't → only the with-revision ones in the
      returned map; values match the highest-`revision_seq`
      row's `created_at`.
    - Inputs that aren't in the table at all → not in the
      returned map (silent — don't fail).

If the existing test layout doesn't admit a clean spot for
these, extend the closest existing harness rather than
inventing a new one.

## Doc updates (minimal)

- `docs/design/10-api-layer.md`'s pagination section: if it
  describes the cursor shape, add a one-line mention of the
  new `sort` query parameter and the cursor-mismatch 400.
- No other design docs need touching — the rest of the
  surface is unchanged at the contract level.
- No CHANGELOG entry on individual crates required for this
  round (it lands as `[Unreleased]` on whatever's already
  staged for the next bump); if the touched crates'
  CHANGELOG.md files already have a `## [Unreleased]`
  section, append a one-line entry. If they don't, leave
  CHANGELOG alone — Yuka cuts release notes at publish time.

## Verification (mandatory before declaring done)

Run, once, at the end:

```sh
./scripts/pre-landing.sh
```

Must print `=== pre-landing: all checks passed ===`. The
script auto-detects modified crates (`philharmonic-api`,
`philharmonic-store`, `philharmonic-store-sqlx-mysql` here,
plus whatever else cargo touches) and runs fmt + check +
clippy `-D warnings` + rustdoc + workspace test + per-crate
`--ignored` test phase.

For the WebUI side:

```sh
./scripts/webui-build.sh --production
```

Must exit 0. Pre-existing webpack asset-size warnings are OK;
no new TS errors or warnings introduced.

Do not run raw `cargo fmt` / `cargo check` / `cargo clippy` /
`cargo test` / `cargo doc` — these are soft-banned in
favour of the wrappers, and `pre-landing.sh` covers them with
the right `CARGO_TARGET_DIR`. Use `./scripts/rust-lint.sh
--phase check -p <crate>` mid-iteration if you need a fast
compile-only loop.

Do not run `./scripts/status.sh --diff` or any other status
script through `head` / `tail` — head/tail on every workspace
`scripts/*.sh` output is soft-banned. Redirect to a file and
`grep`/`Read` if you need to slice.

## Hand-off shape: Codex does not commit

**Leave the working tree dirty.** Claude commits via
`./scripts/commit-all.sh` after reviewing the diff. The
`codex-guard` (`scripts/lib/codex-guard.sh`) walks the
ancestor process chain and aborts if any process is named
`*codex*`; calling `commit-all.sh` from inside a Codex run
will hard-fail. Do not work around the guard.

Specifically:

- Do **not** run `./scripts/commit-all.sh` (any flags,
  including `--dry-run`, `--parent-only`, `--exclude`).
- Do **not** run raw `git commit` / `git push` / `git add`.
  The pre-commit hooks enforce signoff + signature +
  `Audit-Info:` trailer; the codex-guard fires from those
  hooks too. Read-only raw `git status` / `git diff` /
  `git log` are also soft-banned — use `./scripts/status.sh`
  (with `--diff` for diffs), `./scripts/heads.sh`, and
  `./scripts/log.sh` instead.
- Do **not** run `git commit --no-verify` / `--no-gpg-sign`.
- Do **not** run `git reset` / `git rebase` / `git amend`.
  History is append-only.
- Do **not** run `./scripts/push-all.sh`. Claude pushes.
- Do **not** run `./scripts/publish-crate.sh`. Yuka publishes.
- Do **not** edit `HUMANS.md`. Agent-readable,
  agent-writable forbidden.

Edits land in the working tree across:

- `philharmonic-api/` submodule — `src/pagination.rs`,
  `src/routes/*.rs` (~7 files), tests.
- `philharmonic-store/` submodule — `src/entity.rs` (trait
  + mock).
- `philharmonic-store-sqlx-mysql/` submodule — `src/entity.rs`
  (impl), `src/schema.rs` (one index migration), tests.
- `philharmonic/` submodule — `webui/src/pages/*.tsx` (~8
  list-page files), `webui/src/api/client.ts` if a typed
  helper is involved, and `webui/dist/{main.css,main.css.map,
  main.js,main.js.map}` regenerated by webui-build.sh.
- Parent repo — `Cargo.lock` (regenerated automatically if
  cargo build ran), possibly `docs/design/10-api-layer.md`,
  and this prompt-archive file's `## Outcome` section.

Codex's session summary should list which submodules + the
parent have dirty trees so Claude knows where to scope the
`commit-all.sh` run.

## Codex report (encouraged)

If anything non-obvious surfaced during this round — a design
call on `CursorKey` rename vs. additive field, a choice on
`paginate_items` consolidation vs. three-copy parity, an edge
case in `latest_revision_timestamps` for entities with zero
revisions, a tricky test seam — write a short report to

```
docs/codex-reports/2026-05-20-0001-entity-list-sort-modified-opt-in.md
```

per [`docs/codex-reports/README.md`](../codex-reports/README.md).
Routine specified-and-shipped work doesn't need one; the
session summary covers it. Leave the report **dirty** in the
working tree; Claude commits it alongside the implementation
diff.

If you skip the report, say so in the session summary.

## Outcome

Pending — will be updated after the Codex run.

---

<task>
Add an optional `?sort=modified_desc` mode to every entity
list API endpoint in `philharmonic-api`, plus the supporting
store-level batch helper and one index migration, and flip
the `philharmonic/webui/` list pages to use the new mode by
default. Existing API consumers (anything that doesn't
explicitly request `?sort=modified_desc`) see no change.

**Hard constraints (locked):**

- No low-level schema change except adding one new index on
  the existing `entity_revision` table
  (`(entity_id, created_at)`). No new columns, no new
  tables, no per-kind tables.
- New sort mode is **optional**. Default is `created_desc`
  for every list endpoint. Old API behaviour unchanged.
- **Old cursors keep decoding.** The cursor wire format
  gains a `sort` field that defaults to `created_desc` when
  missing on decode.
- **Cursor mode mismatch is 400.** If the caller sends
  `?sort=modified_desc` with a cursor issued under
  `created_desc` (or vice versa), respond
  `400 invalid pagination cursor`.
- **Only the WebUI flips to modified_desc.** No user-facing
  sort toggle.
- **Audit log is exempt** (events are immutable; modified
  has no meaning). `routes/audit.rs::list_audit` ignores
  or rejects the param — your call, document the choice.
- `CursorKey` rename to `sort_key_value` is preferred (with
  the JSON wire field staying `created_at` for backward
  compat); alternatively keep `created_at` and add an
  `Option<UnixMillis> modified_at` — justify whichever you
  pick in your session summary.
- No raw `cargo`, no raw `git`. Use `./scripts/*.sh`
  wrappers (`rust-lint.sh --phase check -p <crate>` for
  fast iteration; `pre-landing.sh` for final).
- No commits, no pushes. Leave dirty across multiple
  submodules + parent. Claude commits + pushes.

**Reference docs (authoritative if they contradict this prompt):**

1. `philharmonic-api/src/pagination.rs` — current cursor
   shape and helpers.
2. `philharmonic-api/src/routes/identity.rs::paginate_items`
   (the canonical helper; two copies in `endpoints.rs` and
   `embed_datasets.rs`).
3. `philharmonic-store/src/entity.rs` — `EntityStore` trait.
4. `philharmonic-store-sqlx-mysql/src/{entity.rs,schema.rs}`
   — sqlx-mysql impl + table definitions.
5. `CLAUDE.md` §"Hard rules vs. soft rules" and the relevant
   exec-summary bullets for raw cargo / raw git / head-tail.
6. `CONTRIBUTING.md` §§4, 5, 10.3, 11.
7. The full preamble above (this prompt's `## …` sections;
   especially "Shape (locked)", "Hard constraints",
   "Per-file scope", "Tests", "Verification").

**Per-file scope summary (full details in preamble):**

- `philharmonic-api/src/pagination.rs` — add `SortMode`,
  extend `PaginationParams.sort`, extend `CursorKey` with
  `sort_mode` + rename `created_at` → `sort_key_value`,
  extend `CursorWire` with `sort` field (backward-compat
  default), encode/decode updates, tests.
- `philharmonic-api/src/routes/{authorities,embed_datasets,
  endpoints,memberships,principals,roles,workflows}.rs`
  — each list endpoint accepts the sort param; tuple-build
  sites use the new key + mode. Consolidate the three
  `paginate_items` copies if mechanical (preferred);
  otherwise keep them in lockstep and document why.
- `philharmonic-api/src/routes/audit.rs` — exempt.
- `philharmonic-store/src/entity.rs` — add
  `latest_revision_timestamps(&[Uuid]) ->
  HashMap<Uuid, UnixMillis>` to `EntityStore` trait + mock.
- `philharmonic-store-sqlx-mysql/src/entity.rs` —
  implement the trait method (one SQL round trip via the
  `INNER JOIN (SELECT MAX(revision_seq))` pattern).
- `philharmonic-store-sqlx-mysql/src/schema.rs` — add
  `ix_entity_revision_entity_created` to
  `INDEX_MIGRATIONS`.
- `philharmonic/webui/src/pages/*.tsx` (list pages) —
  add `sort=modified_desc` to the API call.
- `philharmonic/webui/src/api/client.ts` — if a typed
  helper is involved, extend its parameter shape.
- `philharmonic/webui/dist/*` — regenerated via
  `./scripts/webui-build.sh --production`.
- Possibly `docs/design/10-api-layer.md` — one-line
  pagination-section addition.

**Verification (must run + pass before declaring done):**

- `./scripts/pre-landing.sh` — clean (`=== pre-landing: all
  checks passed ===`).
- `./scripts/webui-build.sh --production` — exit 0 (pre-
  existing webpack size warnings OK; no new TS errors /
  warnings).

That is the entire mandatory verification surface.

<default_follow_through_policy>
Codex is expected to land the pagination changes, store
trait + impl + mock, schema index migration, all touched
route files, the WebUI list-page changes, the regenerated
WebUI dist, and the tests in this single round. Partial
results (e.g., "pagination + routes done, store helper
pending") are NOT complete — keep going.

If a hard blocker surfaces (e.g., the existing
`paginate_items` copies have an incompatibility that
prevents clean consolidation without invasive refactoring),
**STOP and report the blocker before partial landing**.
A partial result that mixes a half-done sort-mode plumbing
with the WebUI flip is worse than a clean
"blocker found, here's what I'd recommend" report.

If `pre-landing.sh` fails on something orthogonal (a pre-
existing flake unrelated to this change), **fix forward**
only if the fix is mechanical and local. If it's
structural, **STOP and report**.
</default_follow_through_policy>

<completeness_contract>
"Complete" means:

1. `philharmonic-api/src/pagination.rs` has `SortMode`,
   extended `PaginationParams`, extended `CursorKey` + wire
   format, backward-compatible decode, all per the preamble.
2. The three `paginate_items` copies are either consolidated
   into one canonical helper (preferred) or kept in
   lockstep — both behaviours sort-mode-aware.
3. Every list route except `audit.rs::list_audit` accepts
   the optional `?sort=modified_desc`, fetches
   latest_revision_timestamps when needed, and builds
   tuples with the right sort key + mode.
4. `routes/audit.rs::list_audit` either ignores or
   explicitly rejects the param — call out the choice.
5. `EntityStore` trait has the new
   `latest_revision_timestamps` method; the sqlx-mysql impl
   uses one round-trip with the documented SQL shape; the
   in-tree `MockEntityStore` implements it; empty input
   short-circuits with no SQL.
6. `philharmonic-store-sqlx-mysql/src/schema.rs` has the
   new index in `INDEX_MIGRATIONS`.
7. Tests cover: cursor round-trip both modes, backward-
   compatible decode of old wire format, cursor-mode-mismatch
   400, two representative endpoints' default + modified
   behaviour, `latest_revision_timestamps` empty/mixed/none
   cases.
8. WebUI list pages send `sort=modified_desc`; typed helper
   (if any) extended; audit log page unchanged.
9. `philharmonic/webui/dist/{main.css,main.css.map,main.js,
   main.js.map}` regenerated via webui-build.sh --production.
10. `./scripts/pre-landing.sh` passes.
11. `./scripts/webui-build.sh --production` exits 0 with no
    new errors/warnings.
12. Working tree left dirty across `philharmonic-api/`,
    `philharmonic-store/`, `philharmonic-store-sqlx-mysql/`,
    `philharmonic/` (webui), and parent (`Cargo.lock` and
    possibly `docs/design/10-api-layer.md`). **No commits,
    no pushes** — Claude commits and pushes after reviewing.
13. Session summary lists which submodule + the parent have
    dirty trees + which `paginate_items` strategy you chose
    (consolidate vs. three-copy parity) + which approach
    you used for `CursorKey` (rename vs. additive field).
14. `## Outcome` section of this prompt file updated with:
    (a) list of files touched per submodule, (b) the
    `CursorKey` shape you settled on, (c) the
    `paginate_items` consolidation outcome, (d) the index
    addition confirmation, (e) verification results, (f)
    any blockers / residual risks, (g) submodule + parent
    head SHAs at hand-off.

If any of (1)–(13) is incomplete, the dispatch is
INCOMPLETE. Report INCOMPLETE clearly with what's done and
what's left, and STOP — don't synthesise a half-result.
</completeness_contract>

<verification_loop>
During implementation (between rounds of edits):

  ./scripts/rust-lint.sh --phase check -p philharmonic-api --quiet
  ./scripts/rust-lint.sh --phase check -p philharmonic-store --quiet
  ./scripts/rust-lint.sh --phase check -p philharmonic-store-sqlx-mysql --quiet

Per-crate tests (use the wrapper; do NOT raw `cargo test`):

  ./scripts/rust-test.sh philharmonic-api
  ./scripts/rust-test.sh philharmonic-store
  ./scripts/rust-test.sh philharmonic-store-sqlx-mysql

Final, single run:

  ./scripts/pre-landing.sh

If `pre-landing.sh` fails, read the failure carefully:

1. If a clippy / doctest / test in one of the touched
   crates caused it, that's a local fix — make the fix,
   re-run pre-landing.
2. If it's a workspace-wide failure (e.g., a downstream
   crate no longer compiles because of an unintended
   trait/type change), back the change out of that crate
   and re-run. The new `EntityStore` method is **additive**;
   it should not break downstream impls.
3. If you're tight-looping pre-landing.sh on a slow box,
   run `./scripts/xtask.sh resource-pressure` first to
   confirm the host has headroom; back off if it doesn't.

Do not run raw `cargo fmt` / `cargo check` / `cargo clippy` /
`cargo test` — soft-banned. The wrappers cover them.
</verification_loop>

<missing_context_gating>
Before you start editing, the workspace state must match
the prompt's claims:

  ./scripts/status.sh

Should print `(clean)` for the parent repo and all
submodules. If it doesn't, **STOP and report**. The prompt
assumes a clean starting tree — uncommitted changes in
unrelated submodules mean someone else is mid-edit; don't
conflict.

If `paginate_items` in `routes/identity.rs`,
`routes/endpoints.rs`, and `routes/embed_datasets.rs` have
genuinely diverged (different generic bounds, different
ApiError handling, etc.) such that consolidation requires
restructuring beyond this dispatch's scope, **STOP and
report**. Don't refactor the three of them as part of this
dispatch beyond what's mechanically clean — propose a
follow-up shape instead.

If the `EntityStore` trait already has a method whose name
collides with `latest_revision_timestamps`, **STOP and
report**.
</missing_context_gating>

<action_safety>
- **You do not commit.** Leave the working tree dirty across
  `philharmonic-api/`, `philharmonic-store/`,
  `philharmonic-store-sqlx-mysql/`, `philharmonic/`
  (webui), and parent. `./scripts/commit-all.sh` (any
  flags) and raw `git commit` / `git push` / `git add` /
  `git reset` / `git rebase` / `git amend` are all
  forbidden. The script's `codex-guard` will hard-abort if
  you try; the same guard fires from the pre-commit hooks.
  Claude commits + pushes after reviewing the diff.
- **Never** invoke `./scripts/push-all.sh`. Claude pushes.
- **Never** invoke `./scripts/publish-crate.sh`. Yuka
  publishes.
- **Never** edit `HUMANS.md`. Agent-readable, agent-writable
  forbidden.
- Every `cargo` invocation needs
  `CARGO_TARGET_DIR=target-main` (the wrappers in
  `scripts/` set this; if you call cargo directly, set it
  yourself — but prefer the wrappers, which are now
  soft-mandated).
- POSIX-ish host: no `bash`-only constructs in any shell
  you invoke. The wrappers are POSIX `#!/bin/sh`.
- The workspace's authoritative timezone is JST
  (Asia/Tokyo). Today is 2026-05-20 (Wed).
- No `head` / `tail` on any `scripts/*.sh` output —
  soft-banned. Redirect to a file and `grep`/`Read` if you
  need to slice the output of a verbose script.
</action_safety>

<structured_output_contract>
At the end of the dispatch, return:

1. **Summary** (3-4 sentences): what changed in
   pagination, the store layer, the routes, the schema,
   and the WebUI; the `CursorKey` shape you settled on
   (rename vs. additive); the `paginate_items` outcome
   (consolidated or three-copy parity).
2. **Touched files**: full list, grouped by submodule +
   parent.
3. **`CursorKey` + `CursorWire` diff**: paste the
   before/after of those two types so the reviewer can
   confirm the wire-format is backward compatible.
4. **`SortMode` definition + serde wire form**: paste the
   enum and its serde attributes.
5. **`latest_revision_timestamps` impl**: paste the SQL
   (sqlx-mysql impl) so the reviewer can eyeball the JOIN
   shape and parameter binding.
6. **Index migration**: paste the ALTER TABLE line.
7. **`paginate_items` strategy**: consolidated to one
   helper in `pagination.rs`? Or kept three copies in
   lockstep? Justify your choice.
8. **Route tuple-build sites**: list each route file +
   the line where `CursorKey::new` is called; confirm
   each one now carries the sort mode + sort key value.
9. **Audit log treatment**: describe what
   `routes/audit.rs::list_audit` does with the `?sort`
   param (ignore vs. reject).
10. **Test coverage**: list each new test by name and
    what it asserts. Confirm both the pagination unit
    tests and the two representative endpoint
    integration tests landed.
11. **WebUI changes**: list each list-page file that now
    sends `sort=modified_desc`; note whether the typed
    API helper (if any) was extended.
12. **`dist/` regeneration**: confirm
    `philharmonic/webui/dist/{main.css,main.css.map,
    main.js,main.js.map}` are present in the dirty tree
    and were produced by `webui-build.sh --production`.
13. **Verification results**:
    - `pre-landing.sh`: PASS / FAIL (with one-line
      summary if FAIL).
    - `webui-build.sh --production`: exit 0 / non-zero
      (with the bottom of the captured output redirected
      to file if non-zero).
14. **Working-tree state at hand-off**:
    - List which submodule + parent have dirty trees.
    - No commits expected from you. Claude commits +
      pushes after reviewing.
15. **Codex report**: if you wrote
    `docs/codex-reports/2026-05-20-0001-entity-list-sort-modified-opt-in.md`,
    note its presence (dirty in working tree; Claude
    commits it). If you skipped, say so.
16. **Residual risks**: anything you'd flag for Claude or
    Yuka before the WebUI flip ships (e.g., a downstream
    consumer that pages large result sets and might be
    sensitive to the cursor-mismatch 400 if it mixes
    sort modes; a slow-path query you noticed; a
    representative endpoint you didn't cover with
    integration tests because the test harness made it
    expensive).
17. **Outcome paragraph** for the prompt-archive file:
    4-6 sentences summarising the round for posterity,
    ready to drop into `## Outcome` of this file.
</structured_output_contract>
</task>
