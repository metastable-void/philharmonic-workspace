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
- `payload` — optional free-form JSON echoed through to
  `vector_search` results (e.g. the full answer text, a URL,
  metadata).

Source items are stored as a JSON content blob on the dataset
entity. They are **not** encrypted — they contain corpus
content, not credentials or capability-bearing URLs. If a
future use case requires encrypted source items, that would be
a separate content slot with SCK, not a change to this design.

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
JSON content blob on the dataset entity.

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

## Entity schema

### `EmbeddingDataset`

New entity kind in `philharmonic-policy`.

**Content slots:**

| Slot | Type | Description |
|------|------|-------------|
| `display_name` | JSON string | Admin-visible name |
| `source_items` | JSON array | `[{id, text, payload?}]` — the raw input. Plaintext (not SCK-encrypted); same model as workflow inputs. |
| `corpus` | JSON array | `CorpusItem[]` — the embedded output. Absent on the very first revision until the first embed completes. **Carried forward** to subsequent revisions during re-embeds so the engine can keep reading the latest revision. |
| `embed_endpoint_id` | JSON string | Public UUID of the `TenantEndpointConfig` to use for embedding. Stored as a content slot (not entity ref) for consistency with `WorkflowTemplate.config` and to give latest-revision behavior — endpoint rotations propagate to the next re-embed automatically. |

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
  const embedResult = await endpoint("embed", {
    body: { texts: [input.text] }
  });
  const queryVector = embedResult.embeddings[0];

  // Search the pre-embedded corpus
  const searchResult = await endpoint("vector_search", {
    body: { query_vector: queryVector, corpus, top_k: 5 }
  });

  // Use search results to build LLM context
  const relevantDocs = searchResult.results.map(r => r.payload?.text).join("\n");
  const llmResult = await endpoint("llm", {
    body: {
      model: "gpt-5.5",
      messages: [
        { role: "system", content: `Answer using: ${relevantDocs}` },
        { role: "user", content: input.text }
      ]
    }
  });

  return { output: { text: llmResult.output.text }, context, done: true };
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

- Batching texts into groups respecting the embed connector's
  `max_batch_size` (read from the lowered config or defaulting
  to a sensible batch size).
- Mapping embed responses back to source item IDs.
- Assembling `CorpusItem[]` with `{id, vector, payload}`.
- Error handling: if any batch fails, the entire job fails.

### Lowerer integration — the `instance_id` problem

The `ConfigLowerer` trait currently requires
`instance_id: EntityId<WorkflowInstance>` (used in the
COSE_Sign1 token's `inst` claim and in AEAD AAD bindings). The
embed job has no workflow instance — it's not a workflow.

**Resolution**: extend the lowerer trait to accept either a
step scope or an ephemeral scope:

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

The `inst` claim in the COSE_Sign1 token is filled with either
the workflow instance UUID or a freshly-minted ephemeral job
UUID. The connector service doesn't validate `inst` against any
substrate registry — it's a binding identifier, not a foreign
key — so an unrecorded UUID is acceptable.

This is a breaking change to `philharmonic-workflow`'s public
trait. Bump major or minor version accordingly. The existing
`StepRecord` schema is unaffected.

**Alternative considered**: synthesize a non-persisted
`EntityId<WorkflowInstance>` for embed jobs without changing
the trait. Rejected because it muddles the semantics — the
type says "this is a workflow instance ID" and the embed job
isn't a workflow.

### Timeout

Embed jobs use a long timeout (configurable, default 30
minutes) because embedding large corpora involves many
sequential connector calls. The timeout is set on the
`MechanicsJob`, not on the mechanics worker's global config.

### Concurrency

One embed job per dataset at a time. If an update arrives
while an embed job is running, the API queues the update —
the new source items are stored, and a new embed job starts
after the current one completes (or is abandoned on timeout).

Implementation note: the simplest v1 approach is to reject
updates while `status=Embedding` with a 409 Conflict. More
sophisticated queuing can come later.

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
  Returns metadata + status. Does not include corpus (use
  the corpus endpoint for that).

- `POST /v1/embed-datasets/{id}/update` — update items.
  Requires `embed_dataset:update`.
  Body: `{items: [{id, text, payload?}]}`.
  Replaces all source items and triggers re-embed.
  Returns 409 if `status=Embedding`.

- `POST /v1/embed-datasets/{id}/retire` — retire dataset.
  Requires `embed_dataset:retire`.

- `GET /v1/embed-datasets/{id}/corpus` — read corpus.
  Requires `embed_dataset:read`.
  Returns the `CorpusItem[]` JSON array (the embedded output).
  Returns 404 if no corpus is available yet.

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
| `embed_dataset:read` | List/read datasets and corpora |
| `embed_dataset:update` | Update dataset items (triggers re-embed) |
| `embed_dataset:retire` | Retire datasets |

These go in the `deployment` permission group in the WebUI
(alongside the existing deployment atoms), or a new `data`
group if the UI benefits from the separation.

## WebUI

New "Embedding Datasets" navigation item between "Authorities"
and "Audit" in the sidebar.

### List page

Table columns: ID, Display Name, Status (badge), Item Count,
Updated. Status badges: Created (info), Embedding (warning
with spinner text), Ready (good), Failed (bad).

### Create form

Fields: Display Name (text), Embed Endpoint (dropdown of
active `embed`-implementation endpoints), Items (JSON editor
for the `[{id, text, payload?}]` array — or a structured
table editor if UX warrants it).

### Detail page

Metadata grid (ID, display name, status, item count, embed
endpoint, timestamps). Tabs: Source Items (JSON viewer),
Corpus (JSON viewer — `CorpusItem[]`), with a refresh button
for polling status during embedding.

### i18n

Both `en.ts` and `ja.ts` need the `embedDatasets` translation
section.

## What doesn't change

- **Connector layer**: no changes. The `embed` and
  `vector_search` connectors already support the needed
  request/response formats.
- **Mechanics worker**: no changes. It runs the built-in embed
  script the same way it runs user scripts.
- **Lowerer**: no changes. The embed job uses the same
  lowering pipeline as regular workflow steps.
- **Encrypted config model**: datasets are not encrypted.
  Source items and corpora are plaintext content blobs.
  The `embed_endpoint` reference points at an encrypted
  `TenantEndpointConfig` which the lowerer decrypts at
  job time — the dataset itself never holds credentials.

## Implementation order

1. Entity schema (`EmbeddingDataset` in `philharmonic-policy`).
2. Permission atoms.
3. API routes (CRUD + corpus read).
4. Template `data_config` content slot + API validation.
5. Workflow engine `data` assembly in `execute_step`.
6. **Lowerer trait change**: introduce `LowerScope` enum;
   migrate `ConnectorConfigLowerer` and `StubLowerer` to the
   new signature; update the workflow engine's call site to
   pass `LowerScope::Step{...}`. This is a breaking change to
   `philharmonic-workflow` — bump version and update the
   `philharmonic-api-server` lowerer accordingly.
7. Ephemeral embed job (built-in script + background tokio task
   in API server, calling the lowerer with
   `LowerScope::Ephemeral{...}` and the executor directly).
8. WebUI pages + i18n.

Steps 1–5 can land without the embed job — datasets would
accept admin-supplied corpus directly (a development-only API
mode) for testing the read path. Step 6 lands as a separable
refactor. Step 7 closes the loop. Step 8 is parallel-safe
after step 3.
