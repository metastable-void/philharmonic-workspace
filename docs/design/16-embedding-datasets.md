# Embedding Datasets

Post-MVP extension. Adds pre-computed vector-embedding datasets
to the workflow execution model, enabling JS scripts to perform
similarity search against tenant-managed corpora without embedding
at query time.

## Motivation

The `vector_search` connector requires a `corpus: CorpusItem[]`
in every request — each item carries a pre-computed embedding
vector. For use cases like support chat with a knowledge base,
the corpus is static between updates and expensive to compute
(one embed-connector call per item). Embedding datasets let
tenants manage corpora as first-class API resources, embed them
asynchronously via the `embed` connector, and bind them to
workflow templates so scripts receive ready-to-search corpora
in their execution context.

## Concepts

### Source items

Admin-submitted raw data. Each item has:

- `id` — stable string identifier (tenant-chosen).
- `text` — the text to embed.
- `payload` — optional JSON-compatible value echoed through to
  `vector_search` results (e.g. the full answer text, a URL,
  metadata). It is encoded inside the CBOR storage blob, but the
  public connector/API contract remains JSON-compatible because
  `vector_search` uses `serde_json::Value`.

Source items are stored as a deterministic CBOR (RFC 8949
§4.2.1 "Core Deterministic Encoding") content blob on the
dataset entity. They are **not** encrypted — they contain corpus
content, not credentials or capability-bearing URLs. If a future
use case requires encrypted source items, that would be a
separate content slot with SCK, not a change to this design.

### Corpus

The output of the embed process: an array of `CorpusItem` values
as defined by `philharmonic-connector-impl-vector-search`:

```rust
pub struct CorpusItem {
    pub id: String,
    pub vector: Vec<f32>,
    pub payload: Option<JsonValue>,
}
```

Each corpus item corresponds to one source item. The `id` and
`payload` are carried through from the source; the `vector` is
the embedding produced by the `embed` connector. Stored as a
deterministic CBOR content blob on the dataset entity.

### CBOR encoding profile

Embedding dataset content slots use this project-specific
deterministic CBOR profile:

- Maps, arrays, strings, integers, booleans, nulls, and generic
  JSON-compatible payload values follow RFC 8949 §4.2.1 Core
  Deterministic Encoding.
- Corpus vectors use RFC 8746 typed-array tags, specifically
  tag 81 (`IEEE 754 binary32, big endian, Typed Array`) for each
  `Vec<f32>`. This makes the density claim concrete instead of
  relying on generic CBOR float arrays, and it gives deterministic
  encoders exactly one vector representation.
- API responses decode storage blobs back to JSON-compatible
  transport shapes. For example, `GET .../corpus` returns
  `vector` as a JSON number array even though storage uses the
  compact typed-array representation.

### Dataset lifecycle

```
Created ──→ Embedding ──→ Ready
                │            │
                ↓            ↓ (update items)
             Failed      Embedding ──→ Ready
```

- **Created**: source items stored, no corpus yet.
- **Embedding**: ephemeral embed job running. Previous corpus
  (if any) remains available to workflows.
- **Ready**: corpus available. Source items + corpus both stored.
- **Failed**: embed job failed. Previous corpus (if any) still
  available. Admin can retry by re-submitting items.

Retired datasets are excluded from all queries and are not
served to workflows.

## Storage substrate prerequisites

### Content blob size

Embedding datasets store noticeably larger content blobs than
other entities. A dataset with 10,000 items and 1,024-dimension
`f32` vectors is ~40 MB of vector bytes alone before CBOR item
overhead, plus payloads. The current MySQL substrate stores content blobs
in `MEDIUMBLOB`, capped at 16 MB per blob — too small.

**Required substrate change** (prerequisite for this feature):

- Migrate the `content` table's blob column from `MEDIUMBLOB`
  to `LONGBLOB` (cap 4 GB per blob) in
  `philharmonic-store-sqlx-mysql`.
- The migration runs automatically on startup if the column
  type does not match. Idempotent.
- Any future SQL backend (e.g. a SQLite substrate, not in
  v1) is responsible for selecting an equivalently-sized
  blob column type at first migration; the workspace ships
  only the MySQL backend today.

### Hard limits enforced by the API

Even with `LONGBLOB`, runaway dataset sizes hurt API memory
use, transfer time, and the workflow engine. The API enforces:

| Limit | Default | Notes |
|------|---------|-------|
| Items per dataset | 10,000 | Server-side cap; rejected at create/update with 400. |
| Bytes per `text` | 64 KiB | Per source item. |
| Bytes per `payload` (JSON) | 64 KiB | Per source item. |
| Total source-items blob | 256 MiB | After CBOR encoding. |
| Total corpus blob | 1 GiB | After CBOR encoding; bounds vector dim × item count. |

Operators can adjust these limits via deployment config; the
defaults exist so a misconfigured tenant cannot fill the
substrate.

### Read-path paging

The corpus is delivered to workflow scripts in a single
`data.embed_datasets.<name>` array — no in-engine paging.
Operators sizing for very large corpora (>100k items) should
either chunk corpora across multiple datasets or wait for a
future paged read API; v1 keeps the read shape simple.

The corpus REST endpoint (`GET .../corpus`) is also
single-response v1; if it becomes a problem we add cursor
paging later without breaking the workflow read path.

## Entity schema

### `EmbeddingDataset`

New entity kind in `philharmonic-policy`.

**Content slots:**

| Slot | Type | Description |
|------|------|-------------|
| `display_name` | UTF-8 text | Admin-visible name. |
| `source_items` | CBOR array | `[{id, text, payload?}]` — the raw input. Deterministic CBOR per the encoding profile above. `payload` is JSON-compatible. Plaintext (not SCK-encrypted); same model as workflow inputs. |
| `corpus` | CBOR array | `CorpusItem[]` — the embedded output. Deterministic CBOR per the encoding profile above; each vector uses RFC 8746 tag 81. Absent on the very first revision until the first embed completes. **Carried forward** to subsequent revisions during re-embeds so the engine can keep reading the latest revision. |
| `embed_endpoint_id` | UTF-8 text | Public UUID of the `TenantEndpointConfig` to use for embedding. Stored as a content slot (not entity ref) for consistency with `WorkflowTemplate.config` and to give latest-revision behavior — endpoint rotations propagate to the next re-embed automatically. |

**Scalar slots:**

| Slot | Type | Description |
|------|------|-------------|
| `status` | i64 | 0=Created, 1=Embedding, 2=Ready, 3=Failed |
| `is_retired` | bool | Standard retirement flag |
| `item_count` | i64 | Number of source items in this revision |

**Entity refs:**

| Slot | Pinning | Description |
|------|---------|-------------|
| `tenant` | Pinned | Owning tenant |

**Revision model:**

The carry-forward rule keeps the engine simple: it always reads
the latest non-retired revision; if `corpus` is present it
serves it; if absent it omits the dataset. Status transitions
never erase a known-good corpus.

- Rev 0: created with `source_items`, `status=Created`. No
  `corpus` slot. Embed job starts immediately after creation.
- Rev 1: `status=Embedding` (set when the embed job begins).
  No `corpus` slot (this is the first embed).
- Rev 2: `status=Ready` + `corpus` content (on success), or
  `status=Failed` (no `corpus` slot, since we have no prior
  good corpus to fall back to).
- On item update with a previously-Ready dataset: new revision
  with updated `source_items`, `status=Embedding`, **the
  previous revision's `corpus` content hash carried forward**.
  Workflows continue receiving the old corpus during re-embed.
- On embed completion after re-embed: new revision with
  `status=Ready` + new `corpus`. On embed failure: new revision
  with `status=Failed` + previous `corpus` carried forward (so
  the dataset stays usable while admin retries).

### Template data bindings

`WorkflowTemplate` gains a new optional content slot named
`data_config`:

| Slot | Type | Description |
|------|------|-------------|
| `data_config` | JSON object | `{ "embed_datasets": { "<name>": "<dataset-uuid>" } }` |

This parallels the existing `config` slot (which holds the
abstract endpoint config and is exposed to the API as
`abstract_config`). The template author assigns script-local
names to dataset UUIDs. The `data_config` slot is optional —
templates without data bindings omit it entirely, and the
`data` field in the script argument is `{}`.

The API surfaces this slot as the `data_config` request/response
field on template create/read/update endpoints (parallel to
`abstract_config`).

Future data kinds beyond `embed_datasets` (e.g. key-value
stores, static file references) add new top-level keys to
the `data_config` JSON without changing the schema shape.

## Script execution context

The workflow engine's `script_arg` changes from:

```json
{"context": {}, "args": {}, "input": {}, "subject": {}}
```

to:

```json
{"context": {}, "args": {}, "input": {}, "subject": {}, "data": {}}
```

`data` is always present (defaults to `{}`). The engine builds
it at step-execution time:

1. Read the template's `data_config` content slot. If absent,
   `data = {}`.
2. For each entry in `data_config.embed_datasets`:
   a. Look up the `EmbeddingDataset` by public UUID; read its
      latest revision.
   b. If the dataset is retired, omit the entry.
   c. If the latest revision has a `corpus` content slot, read
      it and add it to `data.embed_datasets.<name>`. (Thanks to
      the carry-forward rule, this works for `Ready`, `Embedding`
      with prior corpus, and `Failed` with prior corpus.)
   d. If the latest revision has no `corpus` slot (`Created`,
      first-ever `Embedding`, or first-ever `Failed`), omit the
      entry.
3. Any data kind can be absent without causing a workflow
   failure. Scripts that require a dataset should check for its
   presence.

### Script usage example

```js
import endpoint from "mechanics:endpoint";

export default async function({context, args, input, subject, data}) {
  const corpus = data.embed_datasets?.knowledge_base;
  if (!corpus) {
    return { output: { error: "knowledge base not ready" }, context, done: true };
  }

  // Embed the user's query
  const embedResponse = await endpoint("embed", {
    body: { texts: [input.text] }
  });
  const queryVector = embedResponse.body.embeddings[0];

  // Search the pre-embedded corpus
  const searchResponse = await endpoint("vector_search", {
    body: { query_vector: queryVector, corpus, top_k: 5 }
  });

  // Use search results to build LLM context
  const relevantDocs = searchResponse.body.results
    .map(r => r.payload?.text)
    .join("\n");

  const llmResponse = await endpoint("llm", {
    body: {
      model: "gpt-5.5",
      messages: [
        { role: "system", content: `Answer using: ${relevantDocs}` },
        { role: "user", content: input.text }
      ],
      output_schema: {
        type: "object",
        properties: { text: { type: "string" } },
        required: ["text"],
        additionalProperties: false
      }
    }
  });

  return {
    output: { text: llmResponse.body.output.text },
    context,
    done: true
  };
}
```

## Ephemeral embed job

When a dataset is created or updated, the API server runs an
embed job asynchronously. The job is **not** a workflow
instance — it uses the lowerer + executor pipeline directly
without persisting step records.

### Flow

The embed job is asynchronous from the user's perspective. The
API write request returns immediately with the dataset ID; the
embed runs as a background tokio task, and the user polls
`GET /v1/embed-datasets/{id}` to observe status transitions.

1. API receives create/update request with source items.
2. API stores the new revision synchronously (`status=Created`
   on first create, otherwise `status=Embedding` with
   carried-forward `corpus`). Returns 201/202 with the dataset
   ID.
3. API spawns a tokio task that:
   a. If the revision was `Created` (first embed), append a new
      revision with `status=Embedding`.
   b. Lowers the `embed_endpoint_id` config (produces the
      `MechanicsConfig` with the embed connector's
      `HttpEndpoint`).
   c. Sends a `MechanicsJob` to the mechanics worker:
      - `module_source`: a built-in embed script (compiled into
        the API binary, not user-authored).
      - `arg`: `{ items: [{id, text, payload?}] }`
      - `config`: the lowered `MechanicsConfig`.
   d. The built-in script iterates over items, batches texts,
      calls the embed connector, assembles `CorpusItem[]` from
      the responses.
   e. On success: appends a new revision with `status=Ready`
      and the `corpus` content blob.
   f. On failure: appends a new revision with `status=Failed`
      and the previous `corpus` carried forward (if any).
4. If the API server restarts mid-embed, the task is lost. The
   dataset is left in `status=Embedding`. Recovery: admin
   re-submits items via the update endpoint. (More robust
   recovery — persistent job queue, restart detection — is out
   of scope for v1; document the limitation in operator docs.)

### Built-in embed script

The script is authored by Codex, compiled into the API binary
as a static string, and not exposed to users. It handles:

- Batching texts into groups according to a `max_batch_size`
  value passed in `arg` (see below).
- Mapping embed responses back to source item IDs.
- Assembling `CorpusItem[]` with `{id, vector, payload}`.
- Error handling: if any batch fails, the entire job fails.

**`max_batch_size` is not visible to JS via lowered
`MechanicsConfig`** — the connector implementation's private
config (`philharmonic-connector-impl-embed/src/config.rs`) sits
inside the COSE_Encrypt0 payload and is not exposed to
mechanics. Instead, the API server reads `max_batch_size` from
the decrypted endpoint config server-side (it has the SCK), and
includes it in the embed job's `arg`:

```json
{ "items": [{"id": "...", "text": "...", "payload": ...}],
  "max_batch_size": 32 }
```

If `max_batch_size` is absent in the endpoint config, the API
server passes a conservative default (e.g. 8) so the script
never has to guess.

### Lowerer integration — the `instance_id` problem

The `ConfigLowerer` trait currently requires
`instance_id: EntityId<WorkflowInstance>` and `step_seq: u64`
(used in the COSE_Sign1 token's `inst` and `step` claims and in
COSE_Encrypt0 AEAD AAD bindings — see
`philharmonic-workflow/src/lowerer.rs`,
`philharmonic-connector-common/src/lib.rs`,
`bins/philharmonic-api-server/src/lowerer.rs`). The embed job
has no workflow instance — it's not a workflow.

This intersects with crypto-bound semantics: `inst`, `step`,
`config_uuid`, and `payload_hash` are signed claims and AEAD
AAD inputs. Changing what `inst` means — even just relaxing
"this is always a workflow instance UUID" — is a
crypto-sensitive design change and **must clear Yuka's two-gate
crypto review (`crypto-review-protocol`) before
implementation**. Two approaches are on the table; the choice
is up to Gate 1.

**Approach A (preferred long-term): `LowerScope` enum.**

```rust
pub enum LowerScope {
    Step { instance_id: EntityId<WorkflowInstance>, step_seq: u64 },
    Ephemeral { job_id: Uuid },
}

#[async_trait]
pub trait ConfigLowerer: Send + Sync {
    async fn lower(
        &self,
        abstract_config: &JsonValue,
        scope: LowerScope,
        subject: &SubjectContext,
    ) -> Result<JsonValue, ConfigLoweringError>;
}
```

The `inst` claim is filled with either the workflow instance
UUID or a freshly-minted ephemeral job UUID, and a new claim or
discriminator distinguishes the two so the connector service
cannot be confused into accepting an ephemeral token where a
step token is expected (or vice versa). `philharmonic-workflow`
gets a minor-or-major version bump; existing `StepRecord`
schema is unaffected. Connector-common's
`ConnectorCallContext.instance_id` and the corresponding token
claim documentation must be updated to reflect the dual
meaning.

**Approach B (v1-fallback): synthesize a non-persisted instance UUID.**

Mint a fresh `EntityId<WorkflowInstance>` per embed job, do
not insert it into the substrate, and pass it through the
existing trait. The connector service does not validate `inst`
against any registry, so an unrecorded UUID is accepted.
Trait, AEAD, and signed-claim shape are unchanged — Gate 1
becomes a "we are reusing the existing claim shape; here is
why that is safe" memo rather than a structural change.
Tradeoff: the type system says "workflow instance" but for
embed jobs that is no longer literally true.

**Recommendation**: ship Approach B for v1 (no crypto-shape
change, lowest risk), and revisit Approach A when a second
ephemeral job source appears. Either way, **Gate 1 must clear
before any code lands**.

The Gate-1 proposal recommending Approach B is filed at
[`docs/crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md`](../crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md);
update this section's wording once Yuka signs off.

### Timeout

Embed jobs need a long timeout (default 30 minutes) because
embedding large corpora involves many sequential connector
calls. `MechanicsJob` and `MechanicsPool` today expose only a
pool-wide `run_timeout` (`mechanics-core/src/internal/pool/`),
which is sized for ordinary user steps. Two options:

1. **Per-job timeout on `MechanicsJob`** — extend
   `MechanicsJob` with an optional `run_timeout` override that
   `MechanicsPool` honors when set. Backward-compatible
   `Option<Duration>`. This is a small, contained
   `mechanics-core` change.
2. **Dedicated long-job pool** — the API server holds a
   second `MechanicsPool` instance configured with a long
   `run_timeout`, used only for embed jobs. No mechanics
   change; more deployment configuration.

V1 picks (1): one pool, per-job overrides. The "no mechanics
worker change" claim in earlier drafts of this doc was
incorrect — mechanics-core gains the per-job timeout knob.

### Concurrency

One embed job per dataset at a time. If an update arrives
while an embed job is running, the API rejects it with **409
Conflict** until `status` leaves `Embedding`. Queuing updates
across embed jobs is a future enhancement; v1 keeps the model
simple so the revision log stays linear.

## API endpoints

All require `Principal` authentication and tenant scope.

### Dataset CRUD

- `POST /v1/embed-datasets` — create dataset.
  Requires `embed_dataset:create`.
  Body: `{display_name, embed_endpoint_id, items: [{id, text, payload?}]}`.
  `embed_endpoint_id` is the public UUID of an active
  `TenantEndpointConfig` whose implementation is `embed`; the
  API validates this at request time.
  Returns: `{dataset_id}`. Triggers embed job (background task).

- `GET /v1/embed-datasets` — list datasets.
  Requires `embed_dataset:read`.
  Cursor-paginated, newest-first.
  Returns: `{items: [{dataset_id, display_name, status, item_count, embed_endpoint_id, created_at, updated_at, is_retired}], next_cursor}`.

- `GET /v1/embed-datasets/{id}` — read dataset detail.
  Requires `embed_dataset:read`.
  Returns metadata + status only. Does **not** include source
  items or corpus (use the dedicated endpoints below).

- `POST /v1/embed-datasets/{id}/update` — update items.
  Requires `embed_dataset:update`.
  Body: `{items: [{id, text, payload?}]}`.
  Replaces all source items and triggers re-embed.
  Returns 409 if `status=Embedding`.

- `POST /v1/embed-datasets/{id}/retire` — retire dataset.
  Requires `embed_dataset:retire`.

- `GET /v1/embed-datasets/{id}/source-items` — read raw source
  items for the WebUI Source Items tab and for admin export.
  Requires `embed_dataset:read`.
  Returns `[{id, text, payload?}]` (CBOR decoded to JSON for
  transport). Always present once the dataset is created.

- `GET /v1/embed-datasets/{id}/corpus` — read corpus.
  Requires `embed_dataset:read`.
  Returns the `CorpusItem[]` JSON array (the embedded output —
  CBOR decoded to JSON for transport).
  Returns 404 if no corpus is available yet (dataset is
  `Created`, first-ever `Embedding`, or first-ever `Failed`).

### Template data config

The existing template create/update endpoints gain an optional
`data_config` field:

- `POST /v1/workflows/templates` — add optional `data_config`
  to request body.
- `PATCH /v1/workflows/templates/{id}` — add optional
  `data_config` to request body.

The API validates that every UUID in
`data_config.embed_datasets` references an existing, non-retired
`EmbeddingDataset` within the tenant.

## Permission atoms

New atoms added to `ALL_ATOMS` in `philharmonic-policy`:

| Atom | Description |
|------|-------------|
| `embed_dataset:create` | Create embedding datasets |
| `embed_dataset:read` | List/read datasets, source items, and corpora |
| `embed_dataset:update` | Update dataset items (triggers re-embed) |
| `embed_dataset:retire` | Retire datasets |

The `embed_dataset` prefix forms its own permission group in
the WebUI — embedding datasets are tenant resources, not
operator-scope deployment plumbing, so they should not share a
group with `deployment:*`. The WebUI permission grouping logic
(`philharmonic/webui/src/components/permissions.ts`) already
groups by `<prefix>:` so a new "Embedding Datasets" group
falls out of the existing logic without code changes; only the
i18n labels and any group ordering need updating.

## WebUI

New "Embedding Datasets" navigation item between "Authorities"
and "Audit" in the sidebar.

**Friendly UI, not raw JSON.** Per workspace direction
(HUMANS.md), embedding datasets get a structured editor — not
the raw JSON textarea pattern used elsewhere in the v0 admin
UI. Datasets are larger and more repetitive than typical
endpoint configs, and a raw JSON editor for `[{id, text,
payload?}]` is unusable in practice.

### List page

Table columns: ID, Display Name, Status (badge), Item Count,
Updated. Status badges: Created (info), Embedding (warning
with spinner text), Ready (good), Failed (bad).

### Create form

Fields:

- **Display Name** — text input.
- **Embed Endpoint** — dropdown of active
  `embed`-implementation endpoints.
- **Items** — structured table editor with one row per item:
  - `id` — text input.
  - `text` — multiline input.
  - `payload` — structured key/value editor for simple flat
    payloads, with the option to expand a row into a small
    JSON-aware editor component (a maintained code-editor
    dependency — see HUMANS.md "Code editor" item — with
    JSON syntax highlighting and validation, **not** a raw
    textarea) for nested payloads. Defaults to empty.

Add / remove / reorder rows; per-row schema validation with
the failing item index surfaced in errors.

**Bulk import** is a separate affordance: an Import modal
that accepts pasted CSV or JSON, runs validation, and
populates the structured table. After import the user
edits via the structured table; there is no persistent
raw-JSON view of the dataset itself. This matches the
HUMANS.md erratum "No raw JSON editor for Embedding DB:
please add a friendly UI."

### Detail page

Metadata grid (ID, display name, status, item count, embed
endpoint, timestamps). Tabs:

- **Source Items** — same structured table as Create
  (read-only when not `Ready`/`Failed`; editable triggers an
  Update call). Backed by
  `GET /v1/embed-datasets/{id}/source-items`.
- **Corpus** — table view with one row per item. Columns: `id`,
  `payload` (collapsed/expandable), and a "vector"
  expand-on-click showing dimensionality and the first few
  components. The full `f32` array stays collapsed by default
  because rendering a 1024-element float array per row crashes
  the page. Backed by `GET /v1/embed-datasets/{id}/corpus`.

A polling refresh button is available on the detail page for
observing `status` transitions during embedding. Clear UI
states for: "first embed in progress" (no fallback corpus
visible), "re-embed in progress with previous corpus served",
"failed with previous corpus served", and "failed without
fallback corpus".

Server-side limits (item count, per-item bytes, blob size —
see "Storage substrate prerequisites") are surfaced inline in
the editor so the user sees caps before submission, not after
the API rejects.

### i18n

Both `en.ts` and `ja.ts` need the `embedDatasets` translation
section.

## What does and doesn't change

- **Connector layer**: no changes. The `embed` and
  `vector_search` connectors already support the needed
  request/response formats.
- **Mechanics worker (`mechanics` bin)**: no changes. It runs
  the built-in embed script the same way it runs user scripts.
- **Mechanics core (`mechanics-core` crate)**: small change —
  add an optional per-job `run_timeout` override on
  `MechanicsJob` so embed jobs can run longer than the pool
  default (see "Timeout"). Backward-compatible
  `Option<Duration>`.
- **Lowerer (`philharmonic-workflow` + `philharmonic-api-server`
  lowerer)**: small change — accommodate ephemeral jobs (see
  "Lowerer integration"). Approach A is a public-trait change
  on `philharmonic-workflow`; Approach B is implementation-only
  inside the API server. **Either approach is crypto-sensitive
  and gated on Yuka's two-gate review (`crypto-review-protocol`)
  before any code lands.**
- **Storage substrate**: small change — `philharmonic-store-sqlx-mysql`
  migrates the content blob column from `MEDIUMBLOB` to
  `LONGBLOB` (auto-applied on startup). The MySQL backend is
  the only SQL backend shipped today; a future SQLite (or
  other) backend would need to pick an equivalent blob column
  at its first migration.
- **Encrypted config model**: datasets are not encrypted.
  Source items and corpora are plaintext content blobs.
  `embed_endpoint_id` is a plaintext content slot pointing at
  an encrypted `TenantEndpointConfig` which the lowerer
  decrypts at job time — the dataset itself never holds
  credentials.

## Implementation order

0. **Crypto Gate 1 proposal** for the lowerer ephemeral-job
   approach (A or B — see "Lowerer integration"). No code
   touching the lowerer or connector token lands before sign-off.
1. **Substrate migration**: `MEDIUMBLOB` → `LONGBLOB` in
   `philharmonic-store-sqlx-mysql` with idempotent startup
   migration. Land first; everything else stores blobs that
   may exceed the old cap.
2. **Mechanics per-job timeout**: optional `run_timeout`
   override on `MechanicsJob`, honored by `MechanicsPool`.
3. Entity schema (`EmbeddingDataset` in `philharmonic-policy`)
   with deterministic-CBOR-encoded content slots.
4. Permission atoms (`embed_dataset:*`).
5. API routes (CRUD + source-items read + corpus read), with
   server-side caps enforced.
6. Template `data_config` content slot + API validation.
7. Workflow engine `data` assembly in `execute_step`.
8. **Lowerer ephemeral support** (per Gate 1 outcome): either
   the `LowerScope` enum (Approach A) or the synthesized-UUID
   path (Approach B). Migrate `ConnectorConfigLowerer` and
   `StubLowerer`; if Approach A, bump
   `philharmonic-workflow` and update the `philharmonic-api-server`
   lowerer accordingly.
9. Ephemeral embed job (built-in script + background tokio task
   in API server, using the chosen lowerer path and the
   executor directly).
10. WebUI pages + i18n (structured editor; no raw-JSON-only
    dataset UI).

Steps 3–7 can land without the embed job — datasets would
accept admin-supplied corpus directly (a development-only API
mode) for testing the read path. Steps 8 and 9 close the loop.
Step 10 is parallel-safe after step 5.

## Crypto-review note

Approaches A and B both touch crypto-bound semantics:

- COSE_Sign1 token's `inst` and `step` claims (signed).
- COSE_Encrypt0 AEAD AAD (which binds those claims into the
  encrypted payload).
- `payload_hash` claim (binds COSE_Encrypt0 ciphertext bytes
  into the signed envelope).
- Connector-common documentation of `ConnectorCallContext`
  semantics.

Per `CLAUDE.md` and `crypto-review-protocol`, this is on
Yuka's personal review path. The Gate 1 proposal must state:
which approach (A or B), what `inst`/`step` mean for ephemeral
jobs, whether a discriminator is added to distinguish ephemeral
vs step tokens, and how connector-service verification handles
the new shape. No implementation prompts (Codex or otherwise)
should be written before Gate 1 clears.
