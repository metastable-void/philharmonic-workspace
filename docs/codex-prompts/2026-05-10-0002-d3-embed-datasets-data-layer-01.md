# D3 round 01 — embedding-datasets data layer (initial dispatch)

**Date:** 2026-05-10
**Slug:** `d3-embed-datasets-data-layer`
**Round:** 01 (initial dispatch — D3 split into round-01 data layer
and round-02 integration; this is round 01)
**Subagent:** `codex:codex-rescue`

## Motivation

D3 in [`docs/ROADMAP.md` §3.A](../ROADMAP.md#a-embedding-datasets-6-dispatches--1-gate-1)
("Embedding-datasets backend") covers the entire embedding-
datasets feature surface — entity, atoms, API CRUD, workflow
engine `data` assembly. Round 01 ships the **data-layer
foundation**: the `EmbeddingDataset` entity in
`philharmonic-policy`, four new permission atoms, the
`data_config` content slot on `WorkflowTemplate`, and the
deterministic-CBOR codec for source-items + corpus blobs. After
round 01 lands, the schema surface compiles and tests in
isolation, and round 02 (workflow engine integration + API
routes + bin wiring) bolts onto a stable foundation.

D1 (`MEDIUMBLOB → LONGBLOB`) and D2 (`MechanicsJob.run_timeout`
override) are already done (commit `ee2bd61`, 2026-05-02). Gate 1
for the lowerer ephemeral path is **APPROVED 2026-05-10** with
Approach B; D4/D5 are unblocked but not part of this round.

## References

- [`docs/ROADMAP.md` §3.A](../ROADMAP.md#a-embedding-datasets-6-dispatches--1-gate-1)
  (post-v1 dispatch plan, A. Embedding datasets).
- [`docs/design/16-embedding-datasets.md`](../design/16-embedding-datasets.md)
  — authoritative design. **If anything in this prompt
  contradicts design 16, design 16 wins; flag the contradiction
  in your structured output.**
- [`docs/crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md`](../crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md)
  — Gate-1 proposal (Approach B approved). Out of scope for
  round 01, but read the "Self-review feedback addressed"
  section's `Identity { internal: Uuid::now_v7(), public:
  Uuid::new_v4() }.typed::<WorkflowInstance>()` construction
  pattern — round 02 will use the same pattern.
- [RFC 8949](https://www.rfc-editor.org/rfc/rfc8949) §4.2.1
  Core Deterministic Encoding (storage encoding).
- [RFC 8746](https://www.rfc-editor.org/rfc/rfc8746) §3.2 typed-
  array tags — tag 81 is `IEEE 754 binary32, big endian, Typed
  Array`; we use it for the per-item `vector: Vec<f32>` payload.
- `philharmonic-policy/src/entity.rs:1-90` (existing entity-impl
  pattern, especially `Tenant` and `TenantStatus`).
- `philharmonic-policy/src/permission.rs:7-85` (atom constants
  + `ALL_ATOMS` array).
- `philharmonic-workflow/src/entities.rs:6-19` (existing
  `WorkflowTemplate` entity — new `data_config` slot lives here).

## Context files pointed at

- `philharmonic-policy/src/entity.rs`
- `philharmonic-policy/src/permission.rs`
- `philharmonic-policy/src/lib.rs`
- `philharmonic-policy/src/error.rs`
- `philharmonic-policy/Cargo.toml`
- `philharmonic-policy/CHANGELOG.md`
- `philharmonic-workflow/src/entities.rs`
- `philharmonic-workflow/Cargo.toml`
- `philharmonic-workflow/CHANGELOG.md`
- (new test files Codex creates) — see Verification block below

## Outcome

Pending — will be updated after Codex run.

---

## Prompt (verbatim)

<task>
Land the data-layer foundation for D3 (embedding-datasets
backend) across two submodules: `philharmonic-policy` and
`philharmonic-workflow`. Four deliverables, all in one round.
Each is small individually; they form a cohesive schema surface
that round 02 (workflow engine `data` assembly + API routes +
bin wiring) builds on.

If anything below contradicts
[`docs/design/16-embedding-datasets.md`](docs/design/16-embedding-datasets.md),
[`docs/ROADMAP.md`](docs/ROADMAP.md), or `CONTRIBUTING.md`, the
docs win — flag the contradiction in your structured output
instead of guessing.

If you hit scope limits, finish whichever deliverable is closest
to done and report what is left in the structured output — do
not silently abandon a half-done chunk, and do not bundle
unfinished or speculative work into a single confused dirty
tree. (Codex does not commit on this workspace — see the Git
rules block below.)

## Deliverable 1 — `EmbeddingDataset` entity in `philharmonic-policy`

**Crate:** `philharmonic-policy` (currently v0.2.1 on `main`).

**Why:** Design 16 §"Entity schema → `EmbeddingDataset`"
specifies a new entity kind in `philharmonic-policy` for tenant-
managed corpora used by `vector_search`-driven workflows.

**What changes:**

1. In `philharmonic-policy/src/entity.rs`, add a new
   `EmbeddingDataset` entity. Use the existing `uuid!("...")`
   construction pattern (see `Tenant`, `TenantEndpointConfig`).
   The KIND UUID is **already minted** — use exactly:

   ```rust
   const KIND: Uuid = uuid!("37aaccf5-a760-457d-879f-a48f6617ef33");
   ```

   Do **not** mint a new UUID; this one was generated via
   `./scripts/xtask.sh gen-uuid -- --v4` on 2026-05-10 and is
   committed to this prompt.

2. Slot declarations per design 16 §"Entity schema":
   - **Content slots** (4): `display_name`, `source_items`,
     `corpus`, `embed_endpoint_id`. `corpus` is absent on the
     first revision until embedding completes — but the slot is
     still declared on the entity (revisions are free to omit
     individual content slots; the slot list declares what
     content slots can appear).
   - **Entity slots** (1): `tenant` pointing at
     `Tenant`, `SlotPinning::Pinned`.
   - **Scalar slots** (3): `status` (i64, indexed),
     `is_retired` (bool, indexed), `item_count` (i64,
     **not** indexed — design 16 doesn't mark it queryable, and
     the `WorkflowTemplate.is_retired` precedent shows what
     gets the `true` flag).

3. Add an `EmbeddingDatasetStatus` enum, mirroring the
   `TenantStatus` pattern (i64 discriminants, `as_i64()`,
   `TryFrom<i64> for EmbeddingDatasetStatus` returning a new
   `PolicyError::InvalidEmbeddingDatasetStatusDiscriminant
   { value }` variant). Variants per design 16:

   | Variant | i64 discriminant |
   |---|---|
   | `Created` | 0 |
   | `Embedding` | 1 |
   | `Ready` | 2 |
   | `Failed` | 3 |

4. Re-export `EmbeddingDataset` and `EmbeddingDatasetStatus`
   from `philharmonic-policy/src/lib.rs`'s top-level `pub use`
   block (mirror the existing `Tenant` / `TenantStatus`
   re-export pattern — they appear together if they are
   re-exported together; otherwise add a new pair).

5. Add a `PolicyError::InvalidEmbeddingDatasetStatusDiscriminant
   { value: i64 }` variant in `philharmonic-policy/src/error.rs`
   with a `#[error(...)]` message following the existing
   `InvalidTenantStatusDiscriminant`'s wording shape.

**What stays / out of scope:**

- No CBOR encoding/decoding logic on the `EmbeddingDataset`
  type itself — that lives in deliverable 4.
- No revision-history convenience helpers (the carry-forward
  rule from design 16 §"Revision model" is engine-side, not
  entity-side).
- No `embed_job_inst: Uuid` scalar slot (design 16 §"Audit-
  correlation logging" notes this is optional and not required
  for v1; do **not** add it).
- No API routes (round 02).

**Verification specific to deliverable 1:**

- Add a unit test in `philharmonic-policy/src/entity.rs`'s
  existing `#[cfg(test)] mod tests` (or create one if absent)
  asserting `EmbeddingDataset::KIND` has version 4, `NAME ==
  "embedding_dataset"`, and the slot count tuple matches
  `(content=4, entity=1, scalar=3)` to catch accidental slot-
  list edits.
- Add a unit test for `EmbeddingDatasetStatus::try_from` round-
  tripping the four valid discriminants and rejecting an
  out-of-range value.

## Deliverable 2 — Permission atoms

**Crate:** `philharmonic-policy`.

**Why:** Design 16 §"Permission atoms" specifies four new
atoms gating embedding-dataset access. They form a new
`embed_dataset:` permission group (separate from
`workflow:` and `endpoint:`), and the WebUI's permission
grouping (`<prefix>:` split) picks them up automatically once
they exist.

**What changes:**

1. In `philharmonic-policy/src/permission.rs`, add four
   constants in `mod atom`:

   ```rust
   pub const EMBED_DATASET_CREATE: &str = "embed_dataset:create";
   pub const EMBED_DATASET_READ:   &str = "embed_dataset:read";
   pub const EMBED_DATASET_UPDATE: &str = "embed_dataset:update";
   pub const EMBED_DATASET_RETIRE: &str = "embed_dataset:retire";
   ```

   Doc-comments mirror the existing atoms (one-sentence each).

2. Append the four constants to `ALL_ATOMS`. Update the array
   length annotation on the `pub const ALL_ATOMS: [&str; 22]`
   line — new length is `26`. Place the four entries together
   as a group; ordering can mirror the source-of-truth ordering
   in design 16's table (create, read, update, retire). Pick
   any sensible position in the array (e.g. between the
   `endpoint:` group and the `tenant:` group); the consumers
   iterate the array, so absolute position doesn't matter for
   correctness, only for diff readability.

**What stays / out of scope:**

- No role-default updates (which atoms attach to which built-in
  roles is a deployment-config concern handled outside
  `philharmonic-policy`).
- No WebUI i18n labels (those live in
  `philharmonic/webui/src/i18n/en.ts`/`ja.ts` and are part of
  D6, not D3).

**Verification specific to deliverable 2:**

- Existing `permission.rs` tests likely include an array-length
  consistency check or a `parse_atom` round-trip — they should
  continue to pass against the new length / constants. If the
  tests use a hard-coded array length that conflicts with `26`,
  update the test to match.
- Add a unit test asserting all four new atom strings parse via
  whatever the existing `parse_atom`-or-equivalent helper is, if
  one exists. If no such helper exists, skip — don't invent one.

## Deliverable 3 — `data_config` content slot on `WorkflowTemplate`

**Crate:** `philharmonic-workflow` (currently v0.1.2 on `main`).

**Why:** Design 16 §"Template data bindings" introduces a new
optional content slot on `WorkflowTemplate` carrying the JSON
binding `{ "embed_datasets": { "<name>": "<dataset-uuid>" } }`.
Round 02 will read this slot in `engine.rs::execute_step` to
build the `data` field of `script_arg`. Round 01's job is just
to add the slot declaration so revisions can carry it.

**What changes:**

1. In `philharmonic-workflow/src/entities.rs`, the
   `WorkflowTemplate::CONTENT_SLOTS` array currently is:

   ```rust
   const CONTENT_SLOTS: &'static [ContentSlot] =
       &[ContentSlot::new("script"), ContentSlot::new("config")];
   ```

   Update it to:

   ```rust
   const CONTENT_SLOTS: &'static [ContentSlot] = &[
       ContentSlot::new("script"),
       ContentSlot::new("config"),
       ContentSlot::new("data_config"),
   ];
   ```

   No other slot or KIND change. The slot is **optional** —
   revisions that omit it correspond to templates without data
   bindings, which is the existing behaviour for every template
   in the wild today.

**What stays / out of scope:**

- No engine-side reading logic for `data_config` (round 02).
- No API exposure (round 02 — the API surfaces this slot as
  the `data_config` request/response field on template
  create/read/update endpoints, parallel to `abstract_config`).
- No validation of the JSON shape (`{ embed_datasets: { ... } }`)
  — that's an API-side concern in round 02.

**Verification specific to deliverable 3:**

- Existing tests in `philharmonic-workflow` (e.g. anything that
  asserts `WorkflowTemplate`'s slot count) may need updating to
  reflect three content slots instead of two; if they are slot-
  count-aware, fix them; if they are slot-name-aware (e.g.
  iterating `CONTENT_SLOTS` to look up something), they should
  pass unchanged.

## Deliverable 4 — Deterministic CBOR codec for source-items + corpus

**Crate:** `philharmonic-policy` (already depends on
`ciborium = "0.2"` per `Cargo.toml`; the codec lives alongside
`EmbeddingDataset`).

**Why:** Design 16 §"CBOR encoding profile" specifies the
storage encoding for the `source_items` and `corpus` content
blobs. Round 01 ships the codec so that round 02's API can
encode-on-write and decode-on-read deterministically. Round 02
also needs the codec for embed-job result assembly; landing it
now keeps round 02's diff focused on integration.

**What changes:**

1. New module `philharmonic-policy/src/embed_dataset_codec.rs`
   (or `embed_dataset.rs` if you prefer to colocate the entity
   helpers — pick whichever reads cleanest; explain the choice
   in the structured output if non-obvious).

2. Public API on the codec module:

   ```rust
   /// One source item: admin-supplied raw text + optional payload.
   #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
   pub struct SourceItem {
       pub id: String,
       pub text: String,
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub payload: Option<serde_json::Value>,
   }

   /// One corpus item: source id + embedding vector + optional
   /// payload (carried through from source).
   #[derive(Clone, Debug, PartialEq)]
   pub struct CorpusItem {
       pub id: String,
       pub vector: Vec<f32>,
       pub payload: Option<serde_json::Value>,
   }

   /// Encode a slice of source items as deterministic CBOR per
   /// the design-16 encoding profile. Result is the bytes
   /// stored in the `source_items` content blob.
   pub fn encode_source_items(items: &[SourceItem])
       -> Result<Vec<u8>, EmbedDatasetCodecError>;

   /// Decode the storage bytes of a `source_items` blob.
   pub fn decode_source_items(bytes: &[u8])
       -> Result<Vec<SourceItem>, EmbedDatasetCodecError>;

   /// Encode a slice of corpus items as deterministic CBOR per
   /// the design-16 encoding profile, with each `vector` as
   /// RFC 8746 tag 81 (IEEE 754 binary32, big endian, Typed
   /// Array). Result is the bytes stored in the `corpus`
   /// content blob.
   pub fn encode_corpus(items: &[CorpusItem])
       -> Result<Vec<u8>, EmbedDatasetCodecError>;

   /// Decode the storage bytes of a `corpus` blob.
   pub fn decode_corpus(bytes: &[u8])
       -> Result<Vec<CorpusItem>, EmbedDatasetCodecError>;

   #[derive(Debug, thiserror::Error)]
   pub enum EmbedDatasetCodecError {
       /// Storage bytes are not valid CBOR.
       #[error("invalid CBOR: {0}")]
       InvalidCbor(String),
       /// Storage bytes do not match the expected source-items shape.
       #[error("malformed source-items blob: {0}")]
       MalformedSourceItems(String),
       /// Storage bytes do not match the expected corpus shape.
       #[error("malformed corpus blob: {0}")]
       MalformedCorpus(String),
       /// A vector tag has an unexpected payload length.
       #[error("malformed RFC 8746 tag 81 payload: {0}")]
       MalformedVectorTag(String),
   }
   ```

   Re-export from `lib.rs`'s top-level `pub use` block alongside
   the rest of the `EmbeddingDataset` types.

3. Encoding rules (must be enforced by the encoders, must be
   tolerated by the decoders):

   - **Maps**: definite-length, keys sorted in
     **bytewise lexicographic order of their CBOR-encoded
     forms** (RFC 8949 §4.2.1, the "core deterministic" rule).
     `ciborium`'s default `Value::Map`/serde encoding is **not**
     guaranteed to do this, so encoder paths must do it
     explicitly. (Hint: encode each key separately, sort by the
     resulting bytes, then emit the map.)
   - **Arrays**: definite-length.
   - **Strings, byte strings**: definite-length.
   - **Integers**: smallest-form encoding (RFC 8949 §4.2.1
     rule (a)).
   - **No indefinite-length items, no undefined values, no
     NaN/Inf in floats**.
   - **`vector: Vec<f32>`**: encoded as **CBOR tag 81** wrapping
     a byte string whose contents are `vector.len() * 4` bytes
     of `f32` values laid out big-endian (network byte order),
     one after another. Length must be a multiple of 4.
   - **`payload: Option<serde_json::Value>`**: when present,
     re-encoded as CBOR using ciborium's serde-Value path with
     the deterministic-map rule applied. When absent, the field
     is **omitted** from the map (do not emit a CBOR null).

4. The decoder must:

   - Accept any well-formed CBOR that decodes to the expected
     shape (do not require the input to itself be deterministic
     — round-trip determinism is enforced on encode only).
   - Reject input whose `vector` tag-81 byte-string length is
     not a multiple of 4 with `MalformedVectorTag`.
   - Reject input whose top-level shape isn't an array of maps,
     or where a required key is missing, with the appropriate
     `Malformed*` variant.

**What stays / out of scope:**

- No JSON-transport conversion helpers (`SourceItem` /
  `CorpusItem` already implement `Serialize`/`Deserialize`
  for the API's JSON shape via their derives, except
  `CorpusItem` deliberately does not — round 02's API code
  will derive a transport struct or hand-write conversion).
  **Do not** add `Serialize`/`Deserialize` derives to
  `CorpusItem` for JSON in round 01 — round 02 owns transport.
- No size-cap enforcement (per-item bytes, total blob size) —
  that lives in the API layer in round 02.
- No async / streaming variants. Embedding-dataset blobs fit
  comfortably in a `Vec<u8>` per the design-16 caps.

**Verification specific to deliverable 4:**

Required tests, in `philharmonic-policy/tests/embed_dataset_codec.rs`
(integration test crate, not unit — these tests exercise public
API only):

1. **Round-trip determinism for source items:**
   - Encode a fixed `Vec<SourceItem>` with payloads that
     exercise nested maps, arrays, ints, strings, bools,
     nulls.
   - Encode a second time from the same input — assert the
     bytes are byte-for-byte identical.
   - Decode and assert structural equality with the input.

2. **Round-trip determinism for corpus:**
   - Encode a fixed `Vec<CorpusItem>` with vectors of size 1,
     1024, and 4096.
   - Encode a second time — assert byte-for-byte identical.
   - Decode and assert structural equality (vectors compared
     bit-exact via `f32::to_bits`).

3. **Known-vector tests** (commit at least 2 vectors per side,
   total 4):
   - Two source-items vectors: hex-encoded expected bytes
     committed inline as `hex!("...")`. Generate the expected
     bytes once, by running the encoder on the chosen input
     **and inspecting the output**, then commit the hex. The
     test re-encodes and asserts equality with the committed
     hex. Generation method goes in a code comment so the
     vector is reproducible.
   - Two corpus vectors: same approach, with one vector
     payload using a single small vector (e.g. `[1.0f32, 2.0,
     3.0]`) so the tag-81 layout is human-verifiable.

4. **RFC 8746 tag-81 layout test:**
   - Encode a corpus with a single item whose `vector ==
     [1.0f32]`.
   - Manually decode the output by skipping outer wrappers,
     find the tag-81 bytes, assert they are exactly the four
     bytes `0x3F 0x80 0x00 0x00` (big-endian IEEE 754 binary32
     of 1.0).

5. **Malformed input rejection:**
   - Truncated CBOR → `InvalidCbor`.
   - Tag-81 with payload length 5 (not a multiple of 4) →
     `MalformedVectorTag`.
   - Map missing required `id` key → `MalformedSourceItems`
     or `MalformedCorpus`.

6. **`payload` round-trip via JSON Value:**
   - Source item with payload `serde_json::json!({"a": 1, "b":
     [true, null, "x"]})` round-trips structurally.
   - Source item with `payload: None` does not emit a
     payload key in the encoded form — assert by decoding back
     and observing `payload.is_none()`.

The "Generate expected outputs once, by hand" rule from
`crypto-review-protocol` / Wave-A test discipline applies here
even though this is **not** a crypto codec — known-vector
tests catch silent encoder drift better than round-trip alone.

## Cross-deliverable: version bumps and CHANGELOGs

After all four deliverables are in place:

1. **`philharmonic-policy`**: bump from `0.2.1` → `0.2.2` in
   `Cargo.toml`. Add a `## [0.2.2] - 2026-05-10` block to
   `philharmonic-policy/CHANGELOG.md` listing:
   - `EmbeddingDataset` entity + `EmbeddingDatasetStatus` enum.
   - 4 new permission atoms (`embed_dataset:*`).
   - Deterministic-CBOR codec for source-items + corpus blobs.
   - The new error variant.

2. **`philharmonic-workflow`**: bump from `0.1.2` → `0.1.3` in
   `Cargo.toml`. Add a `## [0.1.3] - 2026-05-10` block to
   `philharmonic-workflow/CHANGELOG.md` listing:
   - `WorkflowTemplate.CONTENT_SLOTS` gains `data_config`.

3. Downstream pins of these crates in sibling crates use caret
   ranges (`"0.2"`, `"0.1"`) — the new patch versions satisfy
   them, no further edits needed.

4. Do **not** edit the workspace `[patch.crates-io]` block.

## Cross-deliverable: workspace verification

Run **`./scripts/pre-landing.sh`** before declaring done. The
script auto-detects modified crates and runs fmt + check +
clippy (`-D warnings`) + rustdoc + test, including `--ignored`
tests for any crate that has them. Both
`philharmonic-policy` and `philharmonic-workflow` will be
detected; that covers everything.

If `pre-landing.sh` reports failures, fix them and re-run.
**Do not** run raw `cargo test` / `cargo clippy` — the script
sets `CARGO_TARGET_DIR=target-main` to keep your build cache
out of `target/` (which `rust-analyzer` uses) per
`CONTRIBUTING.md §5`.

Run **`./scripts/test-scripts.sh`** — should pass clean (no
shell scripts touched in this round).

Do **not** run `cargo publish` (or `cargo publish --dry-run`
on `philharmonic-policy` — Yuka's two-gate review applies to
that crate even though round 01's changes are not crypto-
sensitive themselves; the publish gate is a separate decision
held by Yuka).

<structured_output_contract>
At the end of your run, produce a single structured report with
these sections:

- **Summary** — one paragraph: what landed, in which crates,
  and whether the run completed all four deliverables.
- **Touched files** — list every file added/edited/deleted with
  a one-line note per file. Distinguish `(new)`, `(edited)`,
  `(deleted)`.
- **Verification results** — exact commands run and their
  outcomes:
  - `./scripts/pre-landing.sh` — pass/fail/output excerpt
  - `./scripts/test-scripts.sh` — pass/fail
  - any per-crate command (e.g. a specific `cargo test ...`
    you ran for focused debugging)
- **Residual risks / known issues** — anything you could not
  resolve or that you want flagged for review (e.g. an
  ambiguity in design 16 you guessed past, a test you couldn't
  make deterministic, a clippy lint you suppressed and why).
- **Git state** — current `HEAD` SHAs in each touched submodule
  + parent. Which branch each submodule is on. Confirm whether
  any commits were made (Codex must not commit on this
  workspace; this section confirms that).
- **Open questions** — questions you'd like Yuka or Claude
  Code to resolve before round 02 dispatches.
</structured_output_contract>

<default_follow_through_policy>
- Finish each deliverable end-to-end (code + tests +
  versioning) before starting the next, unless they're
  inherently entangled (deliverables 1 and 2 share
  `philharmonic-policy/CHANGELOG.md`; sequence those naturally).
- Order recommendation: **D2 (atoms — smallest) → D1 (entity)
  → D4 (codec — largest) → D3 (slot — trivial)**. This lets you
  validate the file-edit cadence on the smallest item first.
- If a deliverable's tests fail, fix the implementation before
  moving on. Do not leave failing tests in place.
- If `pre-landing.sh` finds a clippy lint in code you didn't
  write but is in the modified-detect set, **fix it** if
  one-line; **flag it** in residual risks if it's larger than
  one line.
- If you discover that a slot name in design 16 conflicts with
  an existing slot name in `philharmonic-policy/src/entity.rs`,
  do **not** rename — flag the conflict and stop.
</default_follow_through_policy>

<completeness_contract>
"Done" means all four deliverables present, version bumps +
CHANGELOG entries in both crates, and `pre-landing.sh` clean.

Partial completion is acceptable if you hit a token limit or a
genuine blocker — but you must explicitly say so in the
structured report, listing which deliverables landed cleanly,
which are partial, and which are not started. Do not paper over
a partial run by claiming completion.
</completeness_contract>

<verification_loop>
For every deliverable:
1. Edit code.
2. Add/update tests.
3. Run `cargo test -p <crate> --lib --tests` (or whatever the
   crate's pre-landing-detected test set is) **inside the
   workspace target dir**: `CARGO_TARGET_DIR=target-main
   cargo test -p <crate>` if running cargo directly; otherwise
   trust `pre-landing.sh`.
4. If green, move on. If red, fix and re-run from step 3.
5. Once all deliverables are green individually, run
   `./scripts/pre-landing.sh` once for the workspace pass.
</verification_loop>

<missing_context_gating>
If you find yourself needing information that isn't in this
prompt or the cited authoritative docs (design 16, ROADMAP,
CONTRIBUTING.md, the Gate-1 proposal), **stop** and report
what's missing in the structured output's "Open questions"
section. Do not invent slot names, KIND UUIDs, atom strings,
or CBOR encoding details — the design and the prompt are the
source of truth, and where they disagree, design 16 wins.

Specifically: do **not** mint new UUIDs (the EmbeddingDataset
KIND is already minted in deliverable 1). Do **not** rename
existing slots, atoms, or types in either crate. Do **not**
remove existing functionality or rewrite unrelated code.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `philharmonic-policy/src/entity.rs` (edited)
- `philharmonic-policy/src/permission.rs` (edited)
- `philharmonic-policy/src/lib.rs` (edited — re-exports)
- `philharmonic-policy/src/error.rs` (edited — new variant)
- `philharmonic-policy/src/embed_dataset_codec.rs` **OR**
  `philharmonic-policy/src/embed_dataset.rs` (new — pick one)
- `philharmonic-policy/Cargo.toml` (edited — version bump)
- `philharmonic-policy/CHANGELOG.md` (edited — new entry)
- `philharmonic-policy/tests/embed_dataset_codec.rs` (new)
- `philharmonic-workflow/src/entities.rs` (edited)
- `philharmonic-workflow/Cargo.toml` (edited — version bump)
- `philharmonic-workflow/CHANGELOG.md` (edited — new entry)
- `Cargo.lock` (auto-regenerated — co-travels with version
  bumps; leave dirty for Claude Code to commit alongside the
  rest)

Files NOT to touch (flag if you find a reason to):

- The workspace `Cargo.toml` `[patch.crates-io]` block.
- `philharmonic-types`, `philharmonic-store`, `mechanics-*`,
  `philharmonic-connector-*`, `philharmonic-api`,
  `bins/philharmonic-api-server`, `philharmonic/webui` — all
  are out of scope for round 01.
- Any `.claude/` content, any `docs/` content (Claude Code
  updates docs separately), any `scripts/` content.

Do **not** run `git add`, `git commit`, `git push`,
`commit-all.sh`, or `push-all.sh`. Codex does not commit on
this workspace — Claude Code reviews your work and commits via
the workspace scripts. Leave the working tree dirty.
</action_safety>

## Git rules (workspace-specific, mandatory)

This workspace's git workflow:

- **Never** run `git commit` / `git push` / `git add` directly.
- **Never** invoke `scripts/commit-all.sh` or
  `scripts/push-all.sh` — Claude Code owns commits, not Codex.
- **Never** run `cargo publish` (even `--dry-run`).
- All cargo commands you run must use
  `CARGO_TARGET_DIR=target-main` (set by the wrapper scripts;
  if you run raw cargo, prefix yourself).
- Don't `--no-verify` around any hooks. The tracked Git hooks
  enforce signed commits + DCO sign-off; bypassing them is
  forbidden.

If you need to verify state, use read-only git commands
(`git status`, `git diff`, `git log`, `git show`,
`git branch`, `git submodule status`). All state-changing git
operations are Claude Code's responsibility.

## Verification commands (mandatory before declaring done)

1. `./scripts/pre-landing.sh` — full workspace pass (auto-
   detects modified crates).
2. `./scripts/test-scripts.sh` — POSIX shell-script syntax
   check (no scripts touched here, so this should be a no-op
   pass).

Optional, for focused debugging only:

- `CARGO_TARGET_DIR=target-main cargo test -p philharmonic-policy`
- `CARGO_TARGET_DIR=target-main cargo test -p philharmonic-workflow`
- `CARGO_TARGET_DIR=target-main cargo clippy -p philharmonic-policy --all-targets -- -D warnings`

</task>
