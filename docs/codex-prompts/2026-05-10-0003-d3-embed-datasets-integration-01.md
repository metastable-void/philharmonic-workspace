# D3 round 02 — embedding-datasets integration (initial dispatch)

**Date:** 2026-05-10
**Slug:** `d3-embed-datasets-integration`
**Round:** 01 (initial dispatch — D3 round 02 is the integration
layer following round 01 which landed the data-layer foundation
on 2026-05-10 commit `bbc26f9`)
**Subagent:** `codex:codex-rescue`

## Motivation

D3 in [`docs/ROADMAP.md` §3.A](../ROADMAP.md#a-embedding-datasets-6-dispatches--1-gate-1)
("Embedding-datasets backend") was split into two rounds at
dispatch time. Round 01 (data layer) landed
2026-05-10 in commits `d017a36` (`philharmonic-policy` 0.2.1 →
0.2.2 — entity, atoms, codec) + `68b774c` (`philharmonic-workflow`
0.1.2 → 0.1.3 — `data_config` slot) + parent `bbc26f9`. **Round 02
ships the integration layer**: workflow-engine `data` assembly in
`execute_step`, the embed-datasets API CRUD surface, the
template-route `data_config` extension, server-side caps
enforcement, and bin-side route wiring.

After round 02 lands, the embedding-datasets feature is functional
end-to-end **except** for the embed-job dispatch itself (D5 —
gated on D4, the lowerer ephemeral support implementation per the
2026-05-10 Gate-1-approved Approach B). Newly-created datasets sit
at `status=Created` until D5 lands; the workflow-engine read path
still works whenever a dataset has a corpus blob (test fixtures
write one directly, bypassing the embed job). D6 (WebUI) and D5
both build on round 02's API surface.

## References

- [`docs/ROADMAP.md` §3.A](../ROADMAP.md#a-embedding-datasets-6-dispatches--1-gate-1).
- [`docs/design/16-embedding-datasets.md`](../design/16-embedding-datasets.md)
  — authoritative design. **If anything in this prompt
  contradicts design 16, design 16 wins; flag it in the
  structured output.**
- Round 01 prompt + outcome: [`docs/codex-prompts/2026-05-10-0002-d3-embed-datasets-data-layer-01.md`](2026-05-10-0002-d3-embed-datasets-data-layer-01.md)
  — context for what already exists in `philharmonic-policy`
  (`EmbeddingDataset`, `EmbeddingDatasetStatus`, codec API,
  `embed_dataset:*` atoms) and `philharmonic-workflow`
  (`WorkflowTemplate.data_config` slot).
- [`docs/crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md`](../crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md)
  — Gate-1 approved Approach B. Out of scope for round 02
  (the lowerer ephemeral path is D4); the proposal is referenced
  only because round 02's API stops short of dispatching embed
  jobs and round 03 will pick that up.

## Context files pointed at

`philharmonic-workflow`:

- `philharmonic-workflow/src/engine.rs` — `execute_step` at
  line 132–335; `script_arg` assembly point at line 241–246
  (the `json!({ "context": ..., "args": ..., "input": ...,
  "subject": ... })` macro). Round 02 adds a `"data"` field.
- `philharmonic-workflow/src/lib.rs` — re-exports.
- `philharmonic-workflow/Cargo.toml` — already depends on
  `philharmonic-policy = "0.2"`; the caret allows 0.2.2.
- `philharmonic-workflow/CHANGELOG.md`.

`philharmonic-api`:

- `philharmonic-api/src/routes/mod.rs` — module declarations
  (line 8–20) + `router()` merge (line 36–53). Round 02 adds
  a new `embed_datasets` module.
- `philharmonic-api/src/routes/workflows.rs` — `CreateTemplateRequest`
  / `UpdateTemplateRequest` / `TemplateResponse` shapes, the
  `validate_abstract_config` helper, the `protected()` wrapper,
  and the `create_template` / `read_template` / `update_template`
  / `retire_template` patterns Codex must mirror for embed-
  datasets.
- `philharmonic-api/src/routes/endpoints.rs` — `EndpointState`
  pattern (cloneable, holds an `ApiStoreHandle`); template for
  `EmbedDatasetState`.
- `philharmonic-api/src/lib.rs` — `ApiError`, `RequestContext`,
  `ApiStoreHandle`, `PaginationParams`, helpers like
  `paginate_items`, `dedupe_rows`, `latest_revision`,
  `resolve_public_id`, `ensure_revision_tenant`,
  `require_tenant_principal`, `bool_scalar`, `i64_scalar`,
  `required_content_hash`, `optional_content_hash`,
  `required_entity_ref`, `put_bytes`, `put_json`, `store_error`,
  `workflow_error`. Discover what's `pub` vs `pub(crate)` and
  match the existing visibility for new helpers if any.
- `philharmonic-api/Cargo.toml` (depends on `philharmonic-policy`
  and `philharmonic-workflow` — bump `philharmonic-policy` to
  the published `0.2.2` if Cargo.lock doesn't already pick it
  up).
- `philharmonic-api/CHANGELOG.md`.

`bins/philharmonic-api-server`:

- `bins/philharmonic-api-server/src/main.rs` — API server
  bootstrap; route table is built via
  `philharmonic_api::routes::router()`. **No bin-side wiring
  needed if `routes/mod.rs::router()` itself merges
  `embed_datasets::router()`** — the bin already consumes the
  merged router. Confirm by inspection; if a state-injection
  step is needed (e.g. an `EmbedDatasetState` Extension), wire
  it the same way `WorkflowState` / `EndpointState` are wired.

## Outcome

Pending — will be updated after Codex run.

---

## STRUCTURED-OUTPUT-CONTRACT — READ THIS FIRST

The round-01 dispatch (2026-05-10) emitted `task_complete`
without producing the prompt's structured-output report.
Claude had to reconstruct verification state by re-running
`pre-landing.sh` and inspecting every diff manually. **Do not
repeat that mistake.** Before issuing `task_complete` you MUST
emit a six-section report (described below). A run that lacks
this report is incomplete by definition, even if the code is
correct — Claude reviews this report to decide what to commit.

The contract is repeated at the end of the prompt for
reference; it is on you to actually emit it before
`task_complete`.

---

## Prompt (verbatim)

<task>
Land D3 round 02 — embedding-datasets integration. Four
deliverables across three crates: workflow-engine `data`
assembly (`philharmonic-workflow`), embed-datasets CRUD routes
+ caps + tenant scoping (`philharmonic-api`), template-route
`data_config` extension (also `philharmonic-api`), and any
bin-side wiring necessary in `bins/philharmonic-api-server`
(probably none — `routes/mod.rs::router()` is already consumed
by the bin).

Round 01 already landed the data layer (entity, atoms, codec,
`data_config` slot declaration). Round 02 wires it up.

If anything below contradicts
[`docs/design/16-embedding-datasets.md`](docs/design/16-embedding-datasets.md),
the design wins — flag the contradiction in your structured
output instead of guessing.

If you hit scope limits, finish whichever deliverable is closest
to done and report what is left in the structured output. Do
**not** silently abandon a half-done chunk and do **not** bundle
unfinished or speculative work into a confused dirty tree.

## Pre-landing-sh hygiene (avoid the round-01 retry loop)

The round-01 run hit `pre-landing.sh` red three times before
green:

1. rustfmt diffs in newly-written files.
2. rustdoc missing-docs warnings on new public items.
3. (clean run, eventually green.)

Save the cycles by running BEFORE invoking `pre-landing.sh`:

```bash
cargo fmt -p philharmonic-workflow
cargo fmt -p philharmonic-api
cargo fmt -p philharmonic-api-server  # the bin crate
```

(All cargo invocations must use `CARGO_TARGET_DIR=target-main`
per `CONTRIBUTING.md §5`. The wrapper scripts set this; raw
cargo invocations need it explicit.)

And ensure every new `pub` item, every new `pub` enum variant,
and every new `pub` struct field carries a one-sentence
doc-comment. The workspace's rustdoc gate is `-D rustdoc::missing_docs_in_lints`-equivalent.

## Deliverable A — Workflow-engine `data` assembly

**Crate:** `philharmonic-workflow` (currently v0.1.3 on
`main` as of round 01).

**Why:** Design 16 §"Script execution context" specifies that
`execute_step` builds a `data` field on the script's argument
JSON (alongside `context` / `args` / `input` / `subject`). The
`data` field carries decoded corpus arrays for every
embedding dataset bound to the template via `data_config`,
omitting datasets that are retired or have no corpus yet.

**What changes:**

1. In `philharmonic-workflow/src/engine.rs`, modify
   `execute_step` to add a `data` field to the `script_arg`
   JSON. The current assembly is at lines 241–246:

   ```rust
   let script_arg = json!({
       "context": context,
       "args": args,
       "input": input_json,
       "subject": subject_json,
   });
   ```

   Replace with:

   ```rust
   let data = build_script_data(
       self.store.as_ref(),
       &template_revision,
       &instance_tenant_ref,
   ).await?;

   let script_arg = json!({
       "context": context,
       "args": args,
       "input": input_json,
       "subject": subject_json,
       "data": data,
   });
   ```

2. Add a new private async function `build_script_data` (in
   `engine.rs` or a new sibling module — `engine/data.rs` is
   reasonable if you want isolation). Signature:

   ```rust
   async fn build_script_data(
       store: &dyn ApiStore,  // or whatever the engine's store trait is
       template_revision: &RevisionRow,
       template_tenant_ref: &EntityRefValue,  // see existing engine code
   ) -> Result<JsonValue, WorkflowError>
   ```

   Behaviour per design 16 §"Script execution context"
   (the numbered list there is normative):

   a. Try to read `data_config` content slot from
      `template_revision`. Use `optional_content_hash` (or
      whatever the existing pattern is for "this slot may or
      may not be present"). If absent, return `json!({})`.
   b. Decode the content blob as a JSON object. Expected
      shape: `{ "embed_datasets": { "<name>": "<dataset-uuid>" } }`.
      Other top-level keys are reserved for future data
      kinds; iterate only `embed_datasets` for v1. If the
      shape doesn't decode, return `WorkflowError::DataConfigInvalid`
      (a new variant) — do not silently default to `{}`.
   c. Build a `data` map with `embed_datasets: { ... }`.
      Initialize `embed_datasets` as `{}`.
   d. For each `embed_datasets.<name>: <public_uuid_string>`:
      - Parse the UUID. On parse failure: return
        `WorkflowError::DataConfigInvalid`.
      - Resolve public→internal via
        `IdentityStore::resolve_public(public_uuid)`. If
        `None`: log `tracing::warn!` with the dataset name +
        public UUID, omit from `data.embed_datasets`,
        continue. (Design 16 §"Script execution context"
        rule 2(d): "any data kind can be absent without
        causing a workflow failure".)
      - Get the entity row. If `kind != EmbeddingDataset::KIND`:
        return `WorkflowError::DataConfigInvalid` with a
        kind-mismatch detail. (This is a "user error" — the
        template authoring API validated the UUID, so a
        mismatch at execute time means substrate corruption
        or a deliberately-bad write; failing the workflow is
        right.)
      - Get the latest revision.
      - Verify the dataset's `tenant` entity-ref equals
        `template_tenant_ref.target_entity_id` (cross-tenant
        embed-dataset reference is a security violation —
        return `WorkflowError::DataConfigInvalid`).
      - Read the `is_retired` scalar. If `true`: omit, continue.
      - Try to read the `corpus` content slot. If absent:
        omit, continue.
      - Read the corpus content bytes; decode via
        `philharmonic_policy::decode_corpus`. On decode
        failure: return `WorkflowError::DataConfigInvalid`
        with a decode-failure detail.
      - Convert `Vec<CorpusItem>` to `JsonValue` via a new
        helper `corpus_items_to_json` (see deliverable A.3
        below).
      - Insert `data.embed_datasets[name] = corpus_items_json`.
   e. Return `data` as `JsonValue::Object(...)`.

3. Add `corpus_items_to_json(items: &[CorpusItem]) -> JsonValue`
   in `engine.rs` or wherever `build_script_data` lives. It
   converts each `CorpusItem` to a JSON object:

   ```json
   { "id": "...", "vector": [1.0, 2.0, ...], "payload": {...} }
   ```

   - `vector` is a JSON number array (NOT a base64 string,
     NOT the storage's tag-81 byte string — the script sees
     a normal float array).
   - `payload` is the JSON value as-is when present;
     **omitted** from the JSON object when `None` (matching
     the storage encoding rule).
   - For NaN/Inf in `vector` (shouldn't happen — the codec
     rejects them on encode — but defensive): use
     `serde_json::Number::from_f64`'s `Option` to detect
     and return an error (or rely on the codec's
     finite-only invariant and `unwrap` with a comment;
     pick whichever you prefer, but explain in code).

4. Add a new `WorkflowError::DataConfigInvalid { detail: String }`
   error variant (in whatever file `WorkflowError` lives —
   probably `philharmonic-workflow/src/error.rs`). Doc-
   comment + `#[error]` message in the existing style.

5. **Tests for deliverable A.** Add an integration test in
   `philharmonic-workflow/tests/data_assembly.rs` (or extend
   the existing engine test file if there's one — check
   `tests/engine_mock.rs`). Tests should:

   a. Build a tenant + a template with `data_config = { "embed_datasets":
      { "kb": "<public-uuid>" } }` + an `EmbeddingDataset`
      revision with `corpus` content (encoded via
      `philharmonic_policy::encode_corpus`) + the same tenant.
      Run `execute_step` and assert the script-executor stub
      received a `script_arg` with a non-empty
      `data.embed_datasets.kb` array whose first item's `id`
      matches the corpus item's `id`.
   b. Same setup but the dataset is `is_retired=true` →
      assert `data.embed_datasets.kb` is **absent** from
      `script_arg`.
   c. Same setup but the dataset's latest revision has no
      `corpus` slot → assert absence (this models a `Created`
      or `Embedding`-without-prior-Ready dataset).
   d. Cross-tenant case: dataset's tenant ≠ template's tenant →
      assert `WorkflowError::DataConfigInvalid`.
   e. Missing-public-UUID case (`resolve_public` → None) →
      assert absence (warn-and-skip).
   f. No-`data_config`-slot case → assert `data == {}` (still
      a top-level field, just empty).

   Use the `MockIdentityStore`-style pattern from
   `philharmonic-store/src/identity.rs::tests` for the
   identity layer; use whatever in-memory store mock the
   existing engine tests use for the entity/content layer.

6. Bump `philharmonic-workflow` 0.1.3 → 0.1.4. Add a
   `## [0.1.4] - 2026-05-10` entry to its CHANGELOG.

## Deliverable B — Embed-datasets API routes

**Crate:** `philharmonic-api` (currently the latest published
version per `Cargo.toml`).

**Why:** Design 16 §"API endpoints" specifies seven endpoints
under `/v1/embed-datasets`. Round 02 lands them.

**What changes:**

1. New module `philharmonic-api/src/routes/embed_datasets.rs`.
   Mirror the structure of `routes/endpoints.rs` (closer
   analogue to embed-datasets than `routes/workflows.rs`
   because it handles a single tenant resource type with
   CRUD + tenant scoping):

   ```rust
   #[derive(Clone)]
   pub(crate) struct EmbedDatasetState {
       store: ApiStoreHandle,
   }

   impl EmbedDatasetState {
       pub(crate) fn new(store: Arc<dyn ApiStore>) -> Self {
           Self {
               store: ApiStoreHandle::new(store),
           }
       }
   }

   pub fn router() -> Router {
       Router::new()
           .route("/v1/embed-datasets",
               protected(post(create_dataset), atom::EMBED_DATASET_CREATE))
           .route("/v1/embed-datasets",
               protected(get(list_datasets), atom::EMBED_DATASET_READ))
           .route("/v1/embed-datasets/{id}",
               protected(get(read_dataset), atom::EMBED_DATASET_READ))
           .route("/v1/embed-datasets/{id}/update",
               protected(post(update_dataset), atom::EMBED_DATASET_UPDATE))
           .route("/v1/embed-datasets/{id}/retire",
               protected(post(retire_dataset), atom::EMBED_DATASET_RETIRE))
           .route("/v1/embed-datasets/{id}/source-items",
               protected(get(read_source_items), atom::EMBED_DATASET_READ))
           .route("/v1/embed-datasets/{id}/corpus",
               protected(get(read_corpus), atom::EMBED_DATASET_READ))
   }
   ```

2. Endpoint behaviours per design 16 §"API endpoints":

   - **POST /v1/embed-datasets** (create): body
     `{display_name: String, embed_endpoint_id: Uuid, items: [{id, text, payload?}]}`.
     - Validate `embed_endpoint_id` references a non-retired
       `TenantEndpointConfig` within the tenant **and** that
       the endpoint's `implementation` content slot decodes
       to the literal string `"embed"`. (See `routes/endpoints.rs`
       for how `implementation` is stored — it's a JSON-
       encoded string in a content slot.)
     - Validate caps: `items.len() <= 10_000`, each item's
       `text.len() <= 65_536`, each item's `payload` JSON-
       encoded length `<= 65_536`, total CBOR-encoded
       `source_items` blob `<= 256 * 1024 * 1024` (256 MiB).
       On any cap failure: return `400 Bad Request` with the
       failing item index and which cap was violated.
     - Encode source-items via
       `philharmonic_policy::encode_source_items`.
     - Mint EmbeddingDataset, write rev 0 with
       `display_name` + `source_items` + `embed_endpoint_id`
       (JSON-string-encoded the same way endpoints store
       their `implementation` field) content slots, `tenant`
       entity ref pinned, scalars `status=Created (0)`,
       `is_retired=false`, `item_count=items.len()`.
     - Return 201 + `{dataset_id: <public-uuid>}`.
     - **Do NOT** dispatch the embed job (that's D5; embed-
       job dispatch is gated on D4's lowerer ephemeral
       support, which has cleared Gate 1 but not yet been
       implemented). Datasets sit at `Created` until D5
       lands. Document this in a TODO comment at the
       create_dataset handler with a pointer to D5 in
       ROADMAP §3.A.

   - **GET /v1/embed-datasets** (list): paginated, newest-
     first (use the cursor pattern from `routes/workflows.rs`
     `list_templates` / `list_instances`). Each item is a
     summary: `{dataset_id, display_name, status, item_count, embed_endpoint_id, created_at, updated_at, is_retired}`. Tenant-scoped.

   - **GET /v1/embed-datasets/{id}** (read): metadata only —
     same fields as the list summary plus any read-only
     deployment-config-derived limits if relevant. Does **not**
     return `source_items` or `corpus` content (those have
     dedicated endpoints).

   - **POST /v1/embed-datasets/{id}/update**: body
     `{items: [{id, text, payload?}]}`. Re-validates caps the
     same way as create. **If the latest revision has
     `status=Embedding`**: return `409 Conflict` with body
     `{error: "dataset is currently embedding"}`. Otherwise
     write a new revision with new `source_items`, carry
     forward the existing `corpus` content hash if the
     previous revision had one (read-then-rewrite), set
     `status=Embedding (1)` (round 02 leaves it at
     `Embedding` because the embed job won't actually run
     until D5 — TODO comment at the call site documenting
     this; alternatively, set it to `Created` to match the
     create-flow shape and let D5 transition it on first
     dispatch — pick whichever; explain the choice in code
     and surface in your structured output if non-obvious).
     Update `item_count` scalar. Returns 200 + the dataset
     summary.

   - **POST /v1/embed-datasets/{id}/retire**: set
     `is_retired=true` on the next revision. Standard pattern
     mirroring `retire_template`. Returns 200 +
     `{dataset_id, is_retired: true}`.

   - **GET /v1/embed-datasets/{id}/source-items**: read
     `source_items` content blob, decode via
     `philharmonic_policy::decode_source_items`, return
     `Json([{id, text, payload?}])`. Always present (even
     `Created` / `Embedding` datasets have source items).

   - **GET /v1/embed-datasets/{id}/corpus**: read `corpus`
     content blob; if present, decode via
     `philharmonic_policy::decode_corpus`, convert to JSON
     via the same `corpus_items_to_json` style as
     deliverable A.3 (or a sibling helper here — pick
     whichever — they don't have to share). If absent
     (dataset is `Created`, first-ever `Embedding` without
     prior corpus, or first-ever `Failed`): return **404 Not
     Found** with a `{error: "no corpus available"}` body.

3. **Tenant scoping**: every endpoint must verify the dataset's
   `tenant` entity ref equals the requesting principal's
   tenant. Reuse `ensure_revision_tenant` (the same helper
   used in `routes/workflows.rs`).

4. **Pagination caps**: same shape as the existing routes
   (cursor + page-size).

5. Wire into `routes/mod.rs`:
   - Add `pub mod embed_datasets;` to the module list.
   - Add `.merge(embed_datasets::router())` to `router()`.

6. **Tests for deliverable B.** Add integration tests in
   `philharmonic-api/tests/embed_datasets.rs` (or extend an
   existing equivalent if `routes/endpoints.rs` has tests).
   Cover:

   - Happy-path create → list → read → source-items → retire.
   - 409 on update-while-embedding.
   - 404 on corpus when no corpus has been written.
   - 400 on items-cap violation, text-cap violation, payload-
     cap violation.
   - 400 on `embed_endpoint_id` referencing a non-`embed`
     implementation or a non-existent endpoint.
   - Tenant-scope rejection: tenant A's principal cannot
     read tenant B's dataset (404 or 403 — match what the
     existing endpoints routes do for cross-tenant).
   - Permission-atom rejection: principal without
     `embed_dataset:create` cannot POST.

7. Bump `philharmonic-api` to the next patch version. Add a
   `## [0.x.y] - 2026-05-10` CHANGELOG entry listing the new
   route surface.

## Deliverable C — Template-route `data_config` extension

**Crate:** `philharmonic-api`.

**Why:** Design 16 §"Template data bindings" specifies that
`WorkflowTemplate` create/update endpoints accept an optional
`data_config` field with shape
`{ "embed_datasets": { "<name>": "<dataset-uuid>" } }`. The
API validates that every UUID references an existing,
non-retired `EmbeddingDataset` within the tenant.

**What changes:**

1. In `philharmonic-api/src/routes/workflows.rs`:

   - Add `data_config: Option<JsonValue>` to
     `CreateTemplateRequest` and `UpdateTemplateRequest`
     (each with `#[serde(default, skip_serializing_if = "Option::is_none")]` if needed).
   - Add `data_config: Option<JsonValue>` to `TemplateResponse`
     (so reads round-trip the bound config).
   - Add a new helper `validate_data_config(store, tenant,
     data_config) -> Result<(), ApiError>`. The helper:
     - Verifies the top-level shape is `{ "embed_datasets":
       Object }` (and that `embed_datasets` is itself a JSON
       object, not array). Other top-level keys are accepted
       but ignored for v1 — round 02 only validates
       `embed_datasets`.
     - For each `embed_datasets.<name>: <uuid_string>`:
       - Validate `<name>` is a non-empty string of
         reasonable length (e.g. ≤ 64 chars; pick a sensible
         cap and document inline). The script accesses this
         as `data.embed_datasets.<name>` so it should be
         valid JS-property-name-ish.
       - Parse the UUID string. On parse failure: 400.
       - Resolve public→internal. If absent: 400 ("dataset
         <uuid> not found").
       - Get the entity. Verify `kind == EmbeddingDataset::KIND`
         (else 400 "<uuid> is not an embedding dataset").
       - Get the latest revision. Verify the dataset's
         `tenant` entity ref equals the requester's tenant
         (else 404 — same shape the existing endpoint-
         validation does for cross-tenant).
       - Verify `is_retired == false` (else 400
         "dataset <uuid> is retired").

2. In the `create_template` handler:
   - After `validate_abstract_config`, also call
     `validate_data_config(&state.store, tenant, &request.data_config)`
     when `request.data_config.is_some()`.
   - When writing the revision, add `with_content("data_config",
     data_config_hash)` if `data_config` was provided.

3. In the `update_template` handler:
   - Same validation when `request.data_config.is_some()`.
   - When writing the next revision: if
     `request.data_config.is_some()`, write the new value;
     otherwise carry-forward the previous revision's
     `data_config` hash if it had one (use
     `optional_content_hash`, since `data_config` is
     optional).

4. In `template_response` (the helper that builds
   `TemplateResponse` from a row + revision): read
   `data_config` content (optional) and include it in the
   response.

5. **Tests for deliverable C.** Extend the workflows-route
   tests:

   - Create a dataset, then create a template with
     `data_config = { embed_datasets: { kb: <dataset-pubid> } }`.
     Assert 201, then GET the template and assert
     `data_config` round-trips.
   - Update the template to remove `data_config` (omit the
     field) → assert the previous `data_config` is preserved
     (carry-forward semantics).
   - Update the template with `data_config = { embed_datasets: {} }`
     (explicit-empty) → assert the empty value is stored.
   - Create-with-bad-UUID (parse failure) → 400.
   - Create-with-non-existent UUID → 400.
   - Create-with-retired-dataset UUID → 400.
   - Create-with-wrong-tenant UUID → 404 (or whichever the
     existing tenant-mismatch-validation does for endpoint
     IDs; pick the matching shape).

## Deliverable D — Bin wiring sanity check

**Crate:** `bins/philharmonic-api-server`.

**Why:** The bin already consumes the merged
`philharmonic_api::routes::router()` so adding to that merge
in deliverable B should auto-wire. But the bin may need an
extension layer for `EmbedDatasetState`.

**What changes:**

1. Read `bins/philharmonic-api-server/src/main.rs` and find
   where `WorkflowState` and `EndpointState` Extensions are
   inserted into the router (look for `.layer(Extension(...))`
   chains around the router build). Add `EmbedDatasetState`
   the same way.

2. **Tests for deliverable D**: a smoke test that the bin
   compiles and serves the new routes (the bin's existing
   integration test set should already exercise route
   mounting; add `embed_datasets` to whatever covers
   `workflows` / `endpoints` if the pattern is per-route).

3. No version bump on the bin (bins are not published).

## Cross-deliverable notes

### Caps as a deployment-config concern

Design 16 says "Operators can adjust these limits via
deployment config." For round 02, hardcode the caps as `pub
const`s in `philharmonic-api/src/routes/embed_datasets.rs` (or
a sibling `caps.rs` module). Wire-up to `ApiConfig` is
deferred — flag in the structured output as an open question
for round 03 (D5) or a separate cleanup dispatch.

### Carry-forward of `corpus` on update

Design 16 §"Revision model": "On item update with a
previously-Ready dataset: new revision with updated
`source_items`, status=Embedding, the previous revision's
corpus content hash carried forward." The update handler
must read the previous revision's `corpus` content hash via
`optional_content_hash(&latest, "corpus")` and re-include it
in the new revision via `with_content("corpus", hash)` if
present. **This is the carry-forward rule the engine relies
on** to keep workflows reading a usable corpus during re-
embed.

### Ordering of the deliverables (suggested)

A → C → B → D (most contained → most diff-impactful):

- A is workflow-internal; small surface, high-value
  (everything else relies on the engine reading
  `data_config`).
- C is small additions to existing template routes; touches
  request/response shapes carefully.
- B is the largest deliverable — seven new route handlers,
  caps enforcement, decode/encode plumbing, integration
  tests. Save it for last so you've cycled through fmt /
  rustdoc / clippy on the smaller changes first.
- D is sanity check + small wiring.

### Version bumps and CHANGELOGs

After all four deliverables are complete:

1. **`philharmonic-workflow`**: 0.1.3 → 0.1.4 in `Cargo.toml`.
   `## [0.1.4] - 2026-05-10` block listing the engine
   `data` assembly + `WorkflowError::DataConfigInvalid`
   variant.

2. **`philharmonic-api`**: bump to next patch version per
   the existing CHANGELOG cadence. `## [next] - 2026-05-10`
   block listing the embed-datasets route surface + the
   `data_config` template-route extension.

3. **Bins**: no version bump.

4. Downstream pins on `"0.1"` / `"0.2"` are caret ranges and
   pick up the new patches automatically. Do **not** edit the
   workspace `[patch.crates-io]` block.

### Workspace verification (mandatory)

Run **`./scripts/pre-landing.sh`** before declaring done. It
auto-detects modified crates and runs fmt + check + clippy
(`-D warnings`) + rustdoc + workspace-test + per-crate
`--ignored` test phase.

Run **`./scripts/test-scripts.sh`** — should pass clean (no
shell scripts touched in this round).

Do **not** run `cargo publish` (or `cargo publish --dry-run`)
on any crate. Publishing is Yuka's gate.

<structured_output_contract>
**Critical: emit this report before `task_complete`. Round 01
skipped this and the run was effectively half-finished. Don't
repeat that.**

Six sections, in this order:

1. **Summary** — one paragraph: which deliverables landed
   cleanly (A/B/C/D), which are partial, which are not
   started. Include the verbatim string "RUN STATUS:
   COMPLETE" or "RUN STATUS: PARTIAL — <one-line reason>" so
   Claude can grep for it.

2. **Touched files** — exhaustive list, one line per file:
   `(new|edited|deleted) <path> — <one-line note>`. Include
   `Cargo.lock` if it changed (it should — version bumps
   co-travel).

3. **Verification results** — exact commands run + outcomes:
   - `./scripts/pre-landing.sh` — pass/fail/exit-code/last-
     20-lines if failed.
   - `./scripts/test-scripts.sh` — pass/fail.
   - Any per-crate command you ran for focused debugging.

4. **Residual risks / known issues** — any guess-pasts,
   suppressed clippy lints (and why), tests you couldn't make
   deterministic, ambiguities in design 16 you resolved by
   judgement call.

5. **Git state** — current `HEAD` SHAs in each touched
   submodule + parent. Confirm whether any commits were made
   (Codex must NOT commit on this workspace; this section
   confirms that). Confirm working tree is dirty with the
   expected changes.

6. **Open questions** — questions for Yuka or Claude Code to
   resolve before round 03 (D5 — embed-job dispatch + lowerer
   ephemeral D4) dispatches.
</structured_output_contract>

<default_follow_through_policy>
- Suggested order: A → C → B → D. Smallest-blast-radius
  first; the largest (B) last so you've debugged your fmt /
  rustdoc / clippy hygiene on smaller diffs first.
- Run `cargo fmt -p <crate>` and ensure rustdoc on every new
  `pub` item BEFORE running `pre-landing.sh`. The wrapper
  enforces both; iterating inside the wrapper wastes time.
- If a deliverable's tests fail, fix the implementation
  before moving on. Do not leave failing tests in place.
- If you encounter a slot-name or atom-name conflict with
  round 01's data layer, do **not** rename round 01 — flag
  the conflict and stop. Round 01 is the source of truth for
  shapes.
</default_follow_through_policy>

<completeness_contract>
"Done" means all four deliverables present, version bumps +
CHANGELOG entries, `pre-landing.sh` clean, AND the structured
output report emitted before `task_complete`.

Partial completion is acceptable if you hit a token limit or
a genuine blocker — but you must say so explicitly in the
structured report (use "RUN STATUS: PARTIAL — <reason>"),
listing which deliverables landed cleanly, which are partial,
and which are not started. Do not paper over a partial run by
claiming completion.

A run without the structured-output-contract report is
**incomplete**, even if all four deliverables landed.
</completeness_contract>

<verification_loop>
For every deliverable:
1. Edit code.
2. Add/update tests.
3. Run `cargo fmt -p <crate>`.
4. Add field-level rustdoc on new `pub` items.
5. Run `CARGO_TARGET_DIR=target-main cargo test -p <crate>`
   (or trust `pre-landing.sh`).
6. If green, move on. If red, fix and re-run from step 5.
7. Once all deliverables are green individually, run
   `./scripts/pre-landing.sh` once for the workspace pass.
8. Emit the structured output report.
9. Then `task_complete`.
</verification_loop>

<missing_context_gating>
If you find yourself needing information that isn't in this
prompt or the cited authoritative docs (design 16, ROADMAP,
CONTRIBUTING.md, the round 01 prompt + outcome), **stop** and
report what's missing in the structured output's "Open
questions" section. Do not invent slot names, atom names,
route paths, or decode shapes — round 01's data layer + the
design are the source of truth.

Specifically: do **not** rename existing slots, atoms, or
types in `philharmonic-policy` or `philharmonic-workflow`. Do
**not** add a new permission atom (round 01 settled the four).
Do **not** mint new UUIDs — public UUIDs come from
`IdentityStore::mint`, internal UUIDs are derived.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `philharmonic-workflow/src/engine.rs` (edited — add `data`
  assembly, `corpus_items_to_json`, `build_script_data`).
- `philharmonic-workflow/src/error.rs` (edited — new
  variant).
- `philharmonic-workflow/src/lib.rs` (edited — re-exports if
  any new public types).
- `philharmonic-workflow/Cargo.toml` (edited — version bump).
- `philharmonic-workflow/CHANGELOG.md` (edited — new entry).
- `philharmonic-workflow/tests/data_assembly.rs` (new — or
  extend an existing engine test file).
- `philharmonic-api/src/routes/embed_datasets.rs` (new — the
  full route module).
- `philharmonic-api/src/routes/mod.rs` (edited — module
  declaration + merge).
- `philharmonic-api/src/routes/workflows.rs` (edited —
  template-route `data_config` extension).
- `philharmonic-api/src/lib.rs` (edited only if needed for
  re-exports / new helpers — minimize).
- `philharmonic-api/Cargo.toml` (edited — version bump).
- `philharmonic-api/CHANGELOG.md` (edited — new entry).
- `philharmonic-api/tests/embed_datasets.rs` (new) +
  possibly `philharmonic-api/tests/workflows_data_config.rs`
  (new) for deliverable C tests.
- `bins/philharmonic-api-server/src/main.rs` (edited — only
  if `EmbedDatasetState` extension wiring is needed).
- `Cargo.lock` (auto-regenerated — leave dirty for Claude to
  commit alongside).

Files NOT to touch (flag if you find a reason to):

- The workspace `Cargo.toml` `[patch.crates-io]` block.
- `philharmonic-policy` source (round 01 settled the data
  layer; only consume it via re-exports). The codec functions
  + entity types + atoms are imported from the published
  `0.2.2`.
- `philharmonic-types`, `philharmonic-store`, `mechanics-*`,
  `philharmonic-connector-*` — all untouched.
- Any `.claude/`, `docs/`, `scripts/` content (Claude owns
  those edits).

Do **not** run `git add`, `git commit`, `git push`,
`commit-all.sh`, `push-all.sh`, or `cargo publish`. Codex
does not commit on this workspace — Claude reviews and commits
via the workspace scripts. Leave the working tree dirty.
</action_safety>

## Git rules (workspace-specific, mandatory)

- **Never** run `git commit` / `git push` / `git add`
  directly.
- **Never** invoke `scripts/commit-all.sh` or
  `scripts/push-all.sh` — Claude owns commits.
- **Never** run `cargo publish` (even `--dry-run`).
- All cargo commands you run must use
  `CARGO_TARGET_DIR=target-main` (set by the wrapper scripts;
  if you run raw cargo, prefix yourself).
- Don't `--no-verify` around any hooks. The tracked Git
  hooks enforce signed commits + DCO sign-off; bypassing
  them is forbidden.

Read-only git is fine: `git status`, `git diff`, `git log`,
`git show`, `git branch`, `git submodule status`.

## Verification commands (mandatory before declaring done)

1. `./scripts/pre-landing.sh` — full workspace pass.
2. `./scripts/test-scripts.sh` — POSIX shell-script syntax
   check (no scripts touched here, no-op pass).

Optional, for focused debugging:

- `CARGO_TARGET_DIR=target-main cargo test -p philharmonic-workflow`
- `CARGO_TARGET_DIR=target-main cargo test -p philharmonic-api`
- `CARGO_TARGET_DIR=target-main cargo clippy -p philharmonic-api --all-targets -- -D warnings`

</task>
