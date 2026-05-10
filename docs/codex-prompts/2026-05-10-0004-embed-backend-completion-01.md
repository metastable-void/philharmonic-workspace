# Embedding-datasets backend completion (initial dispatch)

**Date:** 2026-05-10
**Slug:** `embed-backend-completion`
**Round:** 01 (initial dispatch — bundles D4 + D5 + two
round-02 follow-ups in one Codex run, per Yuka's "batch as
much as feasible" directive)
**Subagent:** `codex:codex-rescue`

## Motivation

D3 round 02 (`b134d44`, 2026-05-10) shipped the embedding-
datasets backend's API surface and workflow-engine `data`
assembly, but newly-created datasets sit at `status=Created`
forever because no embed-job dispatcher actually runs. This
batch closes the backend feature loop:

- **D4** — Lowerer ephemeral support per the **Gate-1-approved
  Approach B** (proposal `2026-05-04-...`, approval `0772184`,
  re-approval `e36cce2` after Claude's self-review revision
  `81936f2`). Synthesized non-persisted `EntityId<WorkflowInstance>`
  per embed job. **Crypto-touching, but no primitive or
  construction change** — calls the existing v1 step lowerer
  identically. Round 01's Finding 5 caught the
  `EntityId::from_uuid(Uuid::new_v4())` invented-API bug; the
  approved construction is `Identity { internal: Uuid::now_v7(),
  public: Uuid::new_v4() }.typed::<WorkflowInstance>()`.
- **D5** — Ephemeral embed job: built-in JS embed script
  (Codex-authored, compiled into the API binary as a static
  string) plus the background tokio task in
  `philharmonic-api-server` that lowers the embed endpoint via
  D4's path, dispatches the mechanics job with a 30-minute
  `MechanicsJob::run_timeout` (D2), receives the corpus result,
  and appends `status=Ready` + corpus / `status=Failed` +
  carry-forward revisions. Includes the audit-correlation
  `tracing::info!` per Gate-1 Finding 3.
- **Round-02 follow-up F1** — Wire the four embedding-dataset
  caps from `pub const`s through `ApiConfig` so deployments can
  override them (Codex round-02 open question 1).
- **Round-02 follow-up F2** — Add a standard `ApiError::Conflict`
  envelope variant; retrofit the `update_dataset` 409 emission
  to use it (Codex round-02 open question 2).

After this batch lands, the embedding-datasets backend is
functional end-to-end. Only D6 (WebUI) remains for the full
embedding-datasets feature, plus D7-9 (Tier 2/3 connectors)
and D11 (doc rewrite) for the broader post-v1 plan.

## References

- [`docs/ROADMAP.md` §3.A](../ROADMAP.md#a-embedding-datasets-6-dispatches--1-gate-1)
  — D4, D5 specs + sequencing.
- [`docs/design/16-embedding-datasets.md`](../design/16-embedding-datasets.md)
  §"Ephemeral embed job" + §"Audit-correlation logging" +
  §"Embed-job token lifetime" + §"Built-in embed script" —
  authoritative spec for all of section A.
- [`docs/crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md`](../crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md)
  — **Gate-1-approved.** Approach B (synthesized non-persisted
  `EntityId<WorkflowInstance>`, no public-trait change, no
  crypto-shape change). The implementation sketch in §"Implementation
  sketch (Approach B)" is the canonical reference for the
  embed-job tokio task structure including the
  audit-correlation log line and the SubjectContext
  construction. **Do NOT deviate from the Approach B
  construction without flagging in the structured output.**
- [`docs/crypto/approvals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md`](../crypto/approvals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md)
  — Yuka's re-approval of Approach B after Claude's self-review
  revision (commit `0772184`).
- Round 02 prompt + outcome: [`docs/codex-prompts/2026-05-10-0003-d3-embed-datasets-integration-01.md`](2026-05-10-0003-d3-embed-datasets-integration-01.md)
  — context for what already exists in `philharmonic-api`
  (the seven embed-dataset routes, `EmbedDatasetState`, the
  four `pub const` caps, the `update_dataset` 409-on-Embedding
  emission), `philharmonic-workflow` (engine `data` assembly),
  and `philharmonic-policy` (entity, atoms, codec).
- [`.claude/skills/crypto-review-protocol/SKILL.md`](../../.claude/skills/crypto-review-protocol/SKILL.md)
  — Gate-2-style review applies to D4's diff. The protocol's
  hard constraints (no `unsafe`, no custom primitives, key
  material zeroized, signatures over untrusted input flagged)
  apply.

## Context files pointed at

`bins/philharmonic-api-server`:

- `src/config.rs` — `ApiConfig` struct (`pub(crate)`); add the
  four embed-dataset caps + an embed-job token lifetime
  override here.
- `src/main.rs` — server bootstrap; `tokio::spawn` patterns at
  lines 760, 785, 796; lowerer construction around line 539.
- `src/lowerer.rs` — `ConnectorConfigLowerer::new` at line 33
  takes a single `token_lifetime_ms: u64`. D4/D5 either:
  (a) construct a second lowerer instance for embed jobs with
  a longer lifetime, or (b) add a per-call lifetime override
  to the lowerer. The Gate-1 proposal flagged either as
  acceptable; pick whichever reads cleanest, document the
  choice in code, and surface in residual risks if non-obvious.
  **Do not change any primitive or construction order in
  `lowerer.rs`'s `lower()` method body.** That code path is
  the v1 step lowerer; D4 reuses it identically.

`philharmonic-api`:

- `src/lib.rs` — `ApiError` enum (where the `Conflict` variant
  goes for F2); `EmbedDatasetState` (round 02's builder layer
  — F1 threads caps through here); `ApiBuilder` (where the
  bin-side state injection happens).
- `src/routes/embed_datasets.rs` — round-02 file. F1 replaces
  `pub const` reads with `state.caps.<field>` reads; F2 swaps
  the ad-hoc 409 emission for `ApiError::Conflict`; A.D5 gains
  an embed-job dispatch call from `create_dataset` and
  `update_dataset` after the synchronous revision write.
- `src/error.rs` (or wherever `ApiError` lives) — F2 site.

`philharmonic-workflow`:

- `src/lowerer.rs` — `ConfigLowerer` trait (UNTOUCHED — Gate-1
  Approach B's whole point is no public-trait churn).
- `src/engine.rs` — the `instance_id.internal().as_uuid()` line
  at the wire boundary; D4 mints an `EntityId<WorkflowInstance>`
  whose `internal()` UUID v7 reaches the wire identically.

`mechanics-config` / `mechanics-core`:

- `MechanicsJob::with_run_timeout(Duration)` from D2
  (`ee2bd61`) — D5 sets this to 30 minutes for embed jobs.
- The `MechanicsJob` invocation shape from API server side —
  Codex reads existing usage in `philharmonic-api` /
  `bins/philharmonic-api-server` to discover the pool API
  shape.

`philharmonic-policy`:

- `EmbeddingDataset`, `EmbeddingDatasetStatus`, `decode_corpus`,
  `encode_corpus`, `CorpusItem` — round 01's data layer.
  **Read-only consumption; do NOT edit `philharmonic-policy`
  source.**

## Outcome

**Completed 2026-05-10** — all three deliverables landed;
commits `e37f956` (parent) + `cbb19f6` (`philharmonic-api`
0.1.4 → 0.1.5). `philharmonic-workflow` and
`philharmonic-policy` source untouched per the prompt's
discipline.

`./scripts/pre-landing.sh` re-run by Claude post-Codex was
green end-to-end including the workspace-test phase + per-
crate `--ignored` testcontainers MySQL phase. Codex's own
pre-landing pass was green on first attempt. 12 new/updated
tests in `philharmonic-api/tests/embed_datasets.rs` cover
the Conflict envelope shape, per-cap rejection at custom
values, and the dispatch-call assertion.

**Crypto-discipline confirmed (D4)**:

- `bins/philharmonic-api-server/src/lowerer.rs::lower()` body
  bit-for-bit unchanged — verified by `git diff` returning
  no entries for that file.
- D4 synthesized-inst construction matches the Gate-1-
  approved Approach B verbatim — `embed_job.rs:98-103`:
  `Identity { internal: Uuid::now_v7(), public: Uuid::new_v4()
  }.typed::<WorkflowInstance>().expect(...)`.
- `SubjectContext { kind: Principal, id:
  format!("system:embed-job:{dataset_id}"), tenant_id, .. }`
  matches the proposal's §"Implementation sketch" pattern.
- Audit-correlation `tracing::info!` with `synthetic_inst`
  field present at `embed_job.rs:123`, matching Gate-1
  Finding 3 resolution.
- No COSE test vectors regenerated, no new `unsafe`, no
  zeroization gaps noticed in neighbouring code.
- Yuka Gate-2-style review on the embed-job dispatcher diff
  is the remaining gate.

**Codex's deliverable choices** (per the prompt's residual-
risks request):

- **Embed-job lowerer instance**: option (i) — a second
  `ConnectorConfigLowerer` instance held alongside the
  existing one, with a 30-minute lifetime via the new
  `embed_job_token_lifetime_ms` field on `ApiConfig`. This
  keeps `lower()`'s crypto body untouched (the alternative
  per-call override would have required edits inside the
  method body).
- **JS embed script location**: `bins/philharmonic-api-server/src/embed_script.js`
  compiled via `include_str!`. Keeps server-only mechanics
  code out of `philharmonic-api`. 84 LOC, defensive parsing
  for both `arg` shape and embed-response shape (handles
  `vectors` / `embeddings` / `data[].vector` /
  `data[].embedding` variants).
- **Tests scope**: API-handler dispatch tests + caps tests +
  conflict tests; did NOT add a full mechanics/lowerer/
  revision integration test or audit-log assertion. Surfaced
  in residual risks; tractable but lower priority — the
  unit-style coverage exercises the dispatch path's
  per-deliverable shape.
- **`MAX_*` route-module constants**: kept as `pub(crate)`
  defaults exposed through `EmbedDatasetCaps`'s associated
  default constants. The `Caps` struct is the runtime source
  of truth; the `pub(crate)` constants survive as the
  default value providers and as a cross-reference for
  reviewers.
- **`executor.rs` change**: clean refactor that extracted
  `execute_job(... timeout: Option<Duration>)`, kept the
  `StepExecutor::execute` trait impl delegating with
  `None`, added `execute_with_run_timeout` for the embed-
  job path. Trait surface preserved; not crypto-touching.

**Structured-output-contract honored** for the second
consecutive round. The `RUN STATUS: COMPLETE` token + six
required sections (Summary / Touched files / Verification
results / Residual risks / Git state / Open questions) all
emitted before `task_complete`. Codex also flagged its own
off-hours-session note ("JST now 18:07 Sunday — out-of-
hours/weekend session; proceeded per workspace rules.")
matching the workspace's coupled-obligation rule.

**Open questions Codex surfaced**:

1. **Yuka Gate-2 review** on D4's synthesized-inst
   construction + dispatcher diff before considering D4
   fully done. The proposal-then-self-review-then-Approach-B
   chain has cleared Gate 1; Gate 2 is the post-implementation
   line-by-line review. No `cargo publish` of any crate
   pending Gate 2.
2. **Restart-loss recovery**: design-16 v1 limitation — a
   server restart leaves a dataset in `status=Embedding`;
   admin re-submits via the update endpoint. Persistent
   job/retry semantics deferred to D6 / future operator
   tooling.

**Round 03+ readiness**: D6 (WebUI) is the natural next
dispatch for the embedding-datasets feature; D7-9 (Tier
2/3 connectors — SMTP, Anthropic, Gemini) and D11
(workflow authoring guide rewrite) are independent
batches. Each is a candidate for the next aggregate
dispatch.

---

## STRUCTURED-OUTPUT-CONTRACT — READ THIS FIRST

Round 02's dispatch (2026-05-10, commit `b134d44`) honored the
structured-output-contract correctly: emitted the six-section
report with `RUN STATUS: COMPLETE` token before `task_complete`.
**Round 03 (this dispatch) maintains the bar.** A run without
the report is incomplete by definition, even if the code
compiles and tests pass — Claude reviews this report to decide
what to commit.

The contract is repeated at the end of the prompt for
reference; it is on you to actually emit it before
`task_complete`.

---

## Pre-landing-sh hygiene (apply early — avoid retry loops)

Round 01 hit `pre-landing.sh` red three times; round 02 was
green on first try after this preamble was added. Keep doing
the same:

```bash
cargo fmt -p philharmonic-api
cargo fmt -p philharmonic-api-server  # the bin crate
```

(Use `CARGO_TARGET_DIR=target-main` per `CONTRIBUTING.md §5`
when running raw cargo. The wrapper scripts handle this.)

Add field-level rustdoc on every new `pub` item (struct
fields, enum variants, fn signatures). The workspace's
rustdoc gate is strict.

---

## Prompt (verbatim)

<task>
Land the embedding-datasets backend completion: D4 + D5 +
round-02 follow-ups F1 + F2. Three deliverable groups (with
D4 fused into D5's section since D4 is shape-of-D5 rather
than separate code):

- **A** — D5 embed-job dispatcher (with D4's synthesized-inst
  construction inside).
- **B** — F1 caps wire-up to `ApiConfig`.
- **C** — F2 `ApiError::Conflict` envelope variant + retrofit.

Suggested order: **C → B → A** (smallest blast radius first;
A is the largest because D5 includes the JS embed script + the
tokio task + lowerer integration + audit log + revision
writes).

If anything below contradicts
[`docs/design/16-embedding-datasets.md`](docs/design/16-embedding-datasets.md)
or the **Gate-1-approved**
[`docs/crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md`](docs/crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md),
the docs win — flag the contradiction in your structured
output instead of guessing.

If you hit scope limits, finish whichever deliverable is
closest to done and report what is left in the structured
output. Do **not** silently abandon a half-done chunk and do
**not** bundle unfinished or speculative work into a confused
dirty tree.

## CRYPTO REVIEW DISCIPLINE — applies to deliverable A only

D4's lowerer ephemeral support is crypto-touching, but the
Gate-1 review (proposal at the link above, approved
2026-05-10) settled the construction. **Do not deviate from
Approach B without flagging.** Specifically:

- **Approved construction** (verbatim from the proposal's
  §"Implementation sketch"):

  ```rust
  let ephemeral_inst: EntityId<WorkflowInstance> = Identity {
      internal: Uuid::now_v7(),
      public: Uuid::new_v4(),
  }
  .typed()
  .expect("freshly-minted v7+v4 pair satisfies Identity::typed");
  let ephemeral_step: u64 = 0;
  ```

  The wire `inst` claim is sourced from
  `instance_id.internal().as_uuid()` in
  `bins/philharmonic-api-server/src/lowerer.rs:74`, which
  expects UUID v7. Do **not** mint v4 for the internal half
  (`InternalId::from_uuid` rejects v4 with `IdKindError`); do
  **not** invent an `EntityId::from_uuid` constructor (the
  type has no single-UUID constructor; only
  `Identity::typed::<T>()` works).

- **Construction order**: KEM-then-ECDH IKM ordering, HKDF
  info string `"philharmonic-connector-v1/aes-256-gcm"`, AAD
  is SHA-256 of CBOR-serialized
  `{realm, tenant, inst, step, config_uuid, kid}`. **All of
  these are unchanged by D4** — Approach B reuses them
  identically. If you find yourself touching `lowerer.rs`'s
  `lower()` method body in any way that affects the
  primitives, the AAD construction, the HKDF inputs, or the
  signed claim set, **stop and flag in the structured output**
  rather than landing the change.

- **No `unsafe`, no custom primitives, key material stays
  zeroized.** If you encounter a zeroization gap or `unsafe`
  block in neighboring code (in `lowerer.rs`, in
  `philharmonic-connector-client`, etc.), **flag it** in your
  structured output's "Residual risks" section — do **not**
  fix it as part of this dispatch. That's separate Gate-1
  territory.

- **Test vector regeneration**: not required. The Gate-1
  proposal explicitly says "no new crypto test vectors are
  required... Existing vectors in `philharmonic-connector-client`,
  `philharmonic-connector-service`, and
  `philharmonic-connector-common` cover the primitives." Do
  **not** regenerate any committed COSE_Sign1 / COSE_Encrypt0
  test vectors.

- **Yuka does a Gate-2-style review on the diff before
  considering D4 fully done.** No `cargo publish` of any
  crate (the API server bin is not published anyway). If
  Yuka surfaces review comments after this dispatch lands,
  they get a follow-up Codex prompt; D4 is not "done"
  until Yuka clears Gate 2.

## Deliverable C — `ApiError::Conflict` envelope variant + retrofit

**Crate:** `philharmonic-api`.

**Why:** Round 02's `update_dataset` handler emits 409 ad-hoc;
Codex's round-02 open question flagged that adding a standard
`Conflict` envelope variant before more 409-producing routes
land is the right call. C is the smallest deliverable in this
batch and unblocks A's update-flow status transitions.

**What changes:**

1. In `philharmonic-api/src/lib.rs` (or wherever `ApiError`
   lives — discover by inspection), add a `Conflict(String)`
   variant. The doc-comment should mirror the existing
   variants' style (one sentence, e.g. "Resource is in a
   state that prevents the requested operation"). The
   `IntoResponse` impl (or whatever maps `ApiError` to an HTTP
   response) maps `Conflict` to **HTTP 409** with whichever
   error-envelope shape the existing variants use (look at
   `ApiError::InvalidRequest` or `NotFound` for the pattern;
   match it).

2. In `philharmonic-api/src/routes/embed_datasets.rs::update_dataset`,
   replace the existing ad-hoc 409 emission (the
   `status=Embedding` rejection path from round 02) with
   `Err(ApiError::Conflict("dataset is currently embedding".to_string()))`.

3. **Tests for deliverable C**: extend the existing
   `tests/embed_datasets.rs` test for the 409-on-Embedding
   case to assert the response envelope shape now matches the
   standard `ApiError::Conflict` shape (whatever code field
   that maps to in the envelope). If the test was already
   asserting envelope shape, just update the assertion.

## Deliverable B — Caps wire-up to `ApiConfig`

**Crate:** `philharmonic-api` + `bins/philharmonic-api-server`.

**Why:** Round 02 hardcoded the four embedding-dataset caps as
`pub const`s in `routes/embed_datasets.rs`. Codex's open
question flagged deployment-config wire-up. B threads them
through `ApiConfig` with the existing `pub const`s as defaults.

**What changes:**

1. In `bins/philharmonic-api-server/src/config.rs` `ApiConfig`,
   add a new section (group together):

   ```rust
   #[serde(default = "default_embed_dataset_max_items")]
   pub(crate) embed_dataset_max_items: usize,
   #[serde(default = "default_embed_dataset_max_text_bytes")]
   pub(crate) embed_dataset_max_text_bytes: usize,
   #[serde(default = "default_embed_dataset_max_payload_bytes")]
   pub(crate) embed_dataset_max_payload_bytes: usize,
   #[serde(default = "default_embed_dataset_max_source_items_blob_bytes")]
   pub(crate) embed_dataset_max_source_items_blob_bytes: usize,
   ```

   Plus the four `default_*` const fns returning the values
   currently defined in `routes/embed_datasets.rs::MAX_*`.
   Update the `Default for ApiConfig` impl to call them. The
   four `pub const`s in the route module become unused once
   you thread the config through; keep them as
   `pub(crate) const`s for now (or delete; pick whichever
   reads cleanest — flag the choice in residual risks).

2. In `philharmonic-api/src/lib.rs`, extend the
   `EmbedDatasetState` builder to take a `Caps` struct (a new
   public type with the same four `usize` fields). Add it to
   the `EmbedDatasetState`. Update the `ApiBuilder` to plumb
   caps from `ApiConfig` through to `EmbedDatasetState::new`.

3. In `philharmonic-api/src/routes/embed_datasets.rs`, replace
   each `MAX_*` reference with `state.caps.<field>` access.

4. **Tests for deliverable B**: extend the existing
   `tests/embed_datasets.rs` cap-violation tests to construct
   the `EmbedDatasetState` with smaller-than-default caps and
   assert the rejection happens at the configured value, not
   the hardcoded default. (One test per cap; pick narrow
   custom values like 5 items / 100 bytes / etc.)

## Deliverable A — D5 embed-job dispatcher (with D4 inside)

**Crate:** `philharmonic-api` + `bins/philharmonic-api-server`.

**Why:** This is the loop-closer. Currently a created or
updated dataset sits at `status=Created` or
`status=Embedding` permanently because no dispatcher runs.
D5 implements the dispatcher per design 16 §"Ephemeral embed
job", and D4 is the synthesized-inst construction the
dispatcher needs (per the Gate-1 proposal).

**Architecture per design 16:**

The flow is:

1. POST `/v1/embed-datasets` (or `/update`) writes the
   synchronous revision (rev 0 with `status=Created`, or
   the next revision with `status=Embedding` for updates).
2. After the synchronous write, the handler **spawns a
   tokio task** (`tokio::spawn`) that:
   - For first-embed (rev 0 was `Created`): appends a new
     revision with `status=Embedding`.
   - Reads the encrypted endpoint config server-side (the
     API server has the SCK), extracts `max_batch_size` from
     the decrypted `embed`-implementation config (default 8
     if absent).
   - Mints an ephemeral `EntityId<WorkflowInstance>` via the
     **Gate-1-approved Approach B construction** (see
     CRYPTO REVIEW DISCIPLINE block above for the verbatim
     construction).
   - Constructs an internal `SubjectContext` via the
     proposal's §"Implementation sketch" pattern:

     ```rust
     let subject = SubjectContext {
         kind: SubjectKind::Principal,
         id: format!("system:embed-job:{dataset_id}"),
         tenant_id,
         authority_id: None,
         claims: serde_json::Value::Null,
     };
     ```

     (Multi-line code comment explaining: lowerer reads only
     `subject.tenant_id`; embed jobs produce no `StepRecord`;
     proper `SubjectKind::System` is deferred to a follow-up
     review.)
   - Lowers the embed-endpoint abstract config via the
     existing `ConfigLowerer::lower(...)` call (no trait
     change; pass the synthesized `ephemeral_inst` and
     `step_seq=0`).
   - Emits the **audit-correlation log line per Gate-1
     Finding 3 resolution**:

     ```rust
     tracing::info!(
         tenant = %tenant_id,
         dataset_id = %dataset_id,
         embed_endpoint_id = %embed_endpoint_id,
         synthetic_inst = %ephemeral_inst.internal().as_uuid(),
         "embed-job lowerer dispatch"
     );
     ```

     (The `revision_id` and `config_uuid` fields from the
     proposal can also be added if convenient; they're
     mentioned in the Finding 3 resolution. At minimum the
     four above are required.)
   - Constructs a `MechanicsJob` with:
     - `module_source` = the built-in JS embed script (see
       below).
     - `arg` = `{ "items": [...], "max_batch_size": <N> }`
       per design 16's §"Built-in embed script" spec.
     - `config` = the lowered `MechanicsConfig`.
     - `run_timeout` = 30 minutes via D2's
       `MechanicsJob::with_run_timeout(Duration::from_secs(1800))`.
       This matches the Gate-1 Finding 1 resolution: the
       embed-job lowerer's token lifetime tracks the job's
       run_timeout (3× wider than the v1 600-second default;
       wider bearer-replay window documented in the proposal).
   - Sends to mechanics worker, awaits the result.
   - On success: parses the result as `Vec<CorpusItem>`,
     encodes via `philharmonic_policy::encode_corpus`,
     appends a new revision with `status=Ready` + the new
     corpus content slot.
   - On failure (mechanics error, script error, decode
     error): appends a new revision with `status=Failed`,
     **carrying forward the previous revision's corpus
     content hash** if it had one (read-then-rewrite per
     design 16's revision model — workflows continue
     reading the old corpus during a re-embed-failure).

3. **Restart-loss caveat**: per design 16 §"Flow" rule 4, "If
   the API server restarts mid-embed, the task is lost. The
   dataset is left in `status=Embedding`. Recovery: admin
   re-submits items via the update endpoint." This is
   acceptable for v1; document with a multi-line code comment
   referencing the design doc's wording.

**Embed-job lowerer instance** (the D4 surface choice from
the Gate-1 proposal): the existing `ConnectorConfigLowerer`
takes a single `token_lifetime_ms` at construction. Two
options the proposal flagged:

- **(i)** Construct a second lowerer instance for embed jobs
  with a 30-minute lifetime, held alongside the existing one.
- **(ii)** Add a per-call lifetime override on
  `ConnectorConfigLowerer` (a new method or a new field on a
  call-site struct) that the embed-job dispatcher uses.

**Pick whichever reads cleanest.** Both are mentioned in the
proposal as one-line surface decisions. Document the choice
inline; surface in residual risks.

**Built-in JS embed script:**

Authored as a static string compiled into the API binary
(`include_str!` from a separate `.js` file is fine, or inline
as a `pub const` — pick whichever reads cleanest). Live
location options:

- `bins/philharmonic-api-server/src/embed_script.js` +
  `include_str!`
- `philharmonic-api/src/embed_script.js` + `include_str!` +
  expose as a public const

Per design 16 §"Built-in embed script", the script must:

- Receive `arg = { items: [{id, text, payload?}],
  max_batch_size: N }`.
- Batch texts into groups of `max_batch_size`.
- For each batch, call `endpoint("embed", { body: { texts:
  [...] } })` (the existing `embed`-connector contract per
  the `philharmonic-connector-impl-embed` spec).
- Map embed responses back to source item IDs.
- Assemble `CorpusItem[]` with `{id, vector, payload}`.
- Error handling: if any batch fails, the entire job fails
  (return an error envelope or throw — match whatever the
  workflow JS contract expects for failure).
- Return: the script's return shape needs to match what the
  Rust side decodes. Per design 16 the script "assembles
  CorpusItem[]"; check the existing
  `philharmonic-workflow/src/engine.rs::parse_executor_result`
  or equivalent for the workflow-script return shape and
  pick whichever fits — `{ output: corpus_items, context:
  {}, done: true }` is the v1 workflow-step return shape and
  may work directly.

The script should be ≤ 100 lines, well-commented, and
defensively-coded (guard against missing `texts` field, empty
`items`, embed-response shape drift, etc.). Author it as if
Yuka will read it line-by-line — this is user-invisible code
shipping inside the API binary.

**Tests for deliverable A**: this is the hardest deliverable
to test in isolation because it spans HTTP handler →
tokio::spawn → lowerer → mechanics-worker → CBOR codec →
revision write. Two test layers are reasonable:

- **Unit-ish test (preferred)**: extract the embed-job
  background-task logic into a testable helper that takes a
  mock `ConfigLowerer`, a mock mechanics executor, and a
  store handle. Exercise:
  - Happy path: items → lowered config → mechanics returns
    a corpus → revision written with `status=Ready` +
    encoded corpus.
  - Mechanics returns an error → revision written with
    `status=Failed` + carried-forward corpus (if any).
  - First embed (rev 0 was `Created`): assert the
    intermediate `status=Embedding` revision is written
    before the final one.
  - Audit-log line is emitted (use `tracing-test` or a
    similar crate; if the workspace doesn't already have a
    tracing-test pattern, skip this assertion and surface in
    residual risks).

- **Integration test (optional, surface in residual risks if
  not feasible)**: full HTTP handler test that POSTs a
  dataset, polls until status leaves `Embedding`, and
  asserts the corpus endpoint returns the expected items.
  This requires either a real mechanics worker (testcontainers)
  or a stub mechanics endpoint — pick whichever the existing
  e2e test infrastructure supports and surface the choice.

## Cross-deliverable: version bumps and CHANGELOGs

After all three deliverables (A + B + C) are in place:

1. **`philharmonic-api`**: bump 0.1.4 → 0.1.5. CHANGELOG entry
   `## [0.1.5] - 2026-05-10` listing:
   - `ApiError::Conflict` variant.
   - Embedding-dataset caps wired through `EmbedDatasetState`
     (with `Caps` struct).
   - Embed-job dispatch (D5) integration in `update_dataset`
     and `create_dataset` handlers.

2. **`bins/philharmonic-api-server`**: bins are not published
   so no version bump. CHANGELOG entry not required (most
   bins don't have CHANGELOGs anyway; check for an existing
   one and follow the pattern).

3. **`philharmonic-workflow`** and **`philharmonic-policy`**:
   no edits. Round 01 + 02 settled their surfaces.

4. Downstream pins are caret ranges; the new patches satisfy
   them. Do **not** edit the workspace `[patch.crates-io]`
   block.

## Cross-deliverable: workspace verification

Run **`./scripts/pre-landing.sh`** before declaring done. It
auto-detects modified crates and runs fmt + check + clippy
(`-D warnings`) + rustdoc + workspace-test + per-crate
`--ignored` test phase.

Run **`./scripts/test-scripts.sh`** — should pass clean (no
shell scripts touched in this round).

Do **not** run `cargo publish` (or `cargo publish --dry-run`)
on any crate. Publishing is Yuka's gate.

<structured_output_contract>
**Critical: emit this report before `task_complete`. Round 02
honored this; round 03 maintains the bar.**

Six sections, in this order:

1. **Summary** — one paragraph: which deliverables landed
   cleanly (A/B/C), which are partial, which are not started.
   Include the verbatim string "RUN STATUS: COMPLETE" or
   "RUN STATUS: PARTIAL — <one-line reason>" so Claude can
   grep for it.

2. **Touched files** — exhaustive list, one line per file:
   `(new|edited|deleted) <path> — <one-line note>`. Include
   `Cargo.lock` if it changed.

3. **Verification results** — exact commands run + outcomes:
   - `./scripts/pre-landing.sh` — pass/fail/exit-code.
   - `./scripts/test-scripts.sh` — pass/fail.
   - Any per-crate cargo command for focused debugging.

4. **Residual risks / known issues** — including specifically:
   - Which surface choice you made for the embed-job lowerer
     instance (option (i) second instance vs (ii) per-call
     override), and why.
   - Where the JS embed script lives (`include_str!` vs
     inline `pub const`), and why.
   - Whether you implemented the integration-style test for
     deliverable A or only the unit-style test, and why.
   - Any zeroization or `unsafe` flags you noticed in
     neighboring code — **flag, not fix**.
   - `embed_dataset_max_*` caps: kept the route-module
     `pub const`s as defaults / deleted them / made them
     `pub(crate)` — which and why.

5. **Git state** — current `HEAD` SHAs in each touched
   submodule + parent. Confirm no commits were made (Codex
   must NOT commit on this workspace).

6. **Open questions** — questions for Yuka or Claude Code to
   resolve. Specifically:
   - The Gate-2 review of D4's diff (Yuka reviews the
     synthesized-inst construction line-by-line).
   - The restart-loss recovery story (per design 16 v1
     limitation; document but flag for D6 / future work).
</structured_output_contract>

<default_follow_through_policy>
- Suggested order: C → B → A. C is tiny (one variant + one
  retrofit + one test update). B is small (config plumbing).
  A is the largest, save it for last so your fmt / rustdoc /
  clippy hygiene is settled on smaller diffs first.
- Run `cargo fmt -p <crate>` and ensure rustdoc on every new
  `pub` item BEFORE running `pre-landing.sh`.
- If a deliverable's tests fail, fix the implementation
  before moving on.
- If you discover that deliverable A needs to touch
  `philharmonic-workflow` (the public `ConfigLowerer` trait
  or `engine.rs`), STOP. Approach B's whole point is no
  public-trait churn. Surface in your structured output as a
  blocker — Claude reviews before any such change lands.
</default_follow_through_policy>

<completeness_contract>
"Done" means:

- All three deliverables (A + B + C) present.
- `philharmonic-api` 0.1.4 → 0.1.5 + CHANGELOG entry.
- `pre-landing.sh` clean.
- Structured output report emitted before `task_complete`.

Partial completion is acceptable if you hit a token limit or a
genuine blocker — but you must say so explicitly with
"RUN STATUS: PARTIAL — <reason>" + which deliverables landed
cleanly, which are partial, and which are not started.

A run without the structured-output report is **incomplete**,
even if all three deliverables landed.
</completeness_contract>

<verification_loop>
For every deliverable:
1. Edit code.
2. Add/update tests.
3. Run `cargo fmt -p <crate>`.
4. Add field-level rustdoc on new `pub` items.
5. Run `CARGO_TARGET_DIR=target-main cargo test -p <crate>`
   (or trust `pre-landing.sh`).
6. If green, move on. If red, fix and re-run.
7. Once all deliverables are green individually, run
   `./scripts/pre-landing.sh` once for the workspace pass.
8. Emit the structured output report.
9. Then `task_complete`.
</verification_loop>

<missing_context_gating>
If you find yourself needing information not in this prompt or
the cited authoritative docs (design 16, Gate-1 proposal +
approval, ROADMAP, CONTRIBUTING.md, round 02 prompt + outcome),
**stop** and report what's missing in the structured output's
"Open questions" section. Do not invent slot names, atom
names, route paths, or decode shapes.

Specifically: do **not**:

- Mint new UUIDs (the synthesized-inst construction is
  per-job at runtime, not author-time).
- Change the `ConfigLowerer` trait signature, the `lower()`
  method body, or any primitive / construction-order /
  HKDF-input / AAD-shape / signed-claim-set choice.
- Edit `philharmonic-policy` source (round 01 settled it).
- Edit `philharmonic-workflow/src/lowerer.rs` or
  `engine.rs` (the workflow-engine `data` assembly is
  round-02-settled; the public lowerer trait is settled).
- Add new permission atoms (round 01 settled the four).
- Regenerate any committed COSE_Sign1 / COSE_Encrypt0 test
  vectors.
- Add `unsafe` blocks anywhere.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `bins/philharmonic-api-server/src/config.rs` (edited — add
  embed-dataset cap fields + defaults).
- `bins/philharmonic-api-server/src/main.rs` (edited — wire
  caps from config to ApiBuilder; possibly add the embed-
  job-lowerer construction if you pick option (i)).
- `bins/philharmonic-api-server/src/lowerer.rs` (edited
  ONLY for the per-call override surface if you pick option
  (ii); **never touch the `lower()` method body's primitive
  / AAD / claim logic**).
- `bins/philharmonic-api-server/src/embed_job.rs` (new —
  the tokio task implementation).
- `bins/philharmonic-api-server/src/embed_script.js` (new
  — the JS embed script if you pick the `include_str!` path).
- `bins/philharmonic-api-server/src/main.rs` (edited — wire
  the embed-job dispatcher into the API server's state).
- `philharmonic-api/src/lib.rs` (edited — `ApiError::Conflict`
  variant, `EmbedDatasetState::Caps` struct, builder plumbing).
- `philharmonic-api/src/error.rs` (edited if `ApiError` lives
  there).
- `philharmonic-api/src/routes/embed_datasets.rs` (edited —
  retrofit 409 to use Conflict; replace MAX_* reads with
  state.caps reads; spawn embed-job tokio task on create +
  update).
- `philharmonic-api/Cargo.toml` (edited — version bump).
- `philharmonic-api/CHANGELOG.md` (edited — new entry).
- `philharmonic-api/tests/embed_datasets.rs` (extended).
- `Cargo.lock` (auto-regenerated — leave dirty for Claude to
  commit alongside).

Files NOT to touch (flag if you find a reason to):

- The workspace `Cargo.toml` `[patch.crates-io]` block.
- `philharmonic-policy` source (round 01 settled).
- `philharmonic-workflow` source (round 02 settled).
- `philharmonic-types`, `philharmonic-store`, `mechanics-*`,
  `philharmonic-connector-*` — all untouched.
- `bins/philharmonic-api-server/src/lowerer.rs::lower()`
  method body.
- Any `.claude/`, `docs/`, `scripts/` content.

Do **not** run `git add`, `git commit`, `git push`,
`commit-all.sh`, `push-all.sh`, or `cargo publish`. Codex
does not commit on this workspace — Claude reviews and commits
via the workspace scripts. Leave the working tree dirty.
</action_safety>

## Git rules (workspace-specific, mandatory)

- **Never** run `git commit` / `git push` / `git add`.
- **Never** invoke `scripts/commit-all.sh` or
  `scripts/push-all.sh`.
- **Never** run `cargo publish` (even `--dry-run`).
- All cargo commands must use `CARGO_TARGET_DIR=target-main`.
- Don't `--no-verify` around any hooks.

Read-only git is fine: `git status`, `git diff`, `git log`,
`git show`, `git branch`, `git submodule status`.

## Verification commands (mandatory before declaring done)

1. `./scripts/pre-landing.sh`.
2. `./scripts/test-scripts.sh`.

Optional, for focused debugging:

- `CARGO_TARGET_DIR=target-main cargo test -p philharmonic-api`
- `CARGO_TARGET_DIR=target-main cargo test -p philharmonic-api-server`
- `CARGO_TARGET_DIR=target-main cargo clippy -p philharmonic-api -p philharmonic-api-server --all-targets -- -D warnings`

</task>
