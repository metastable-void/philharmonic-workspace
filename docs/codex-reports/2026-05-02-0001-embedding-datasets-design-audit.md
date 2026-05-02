# Embedding Datasets Design Audit

**Date:** 2026-05-02
**Prompt:** Direct chat request to review `docs/design/16-embedding-datasets.md`.

I reviewed `docs/design/16-embedding-datasets.md` against the existing
design docs and current implementation surfaces in the workflow, policy,
API, mechanics, connector, and WebUI crates. The new component is a
post-MVP extension, but several parts are inconsistent with current
substrate and execution contracts in ways that should be resolved before
implementation prompts are written.

## High-severity findings

### Single-blob dataset storage can exceed substrate limits

The design stores both `source_items` and `corpus` as single JSON content
blobs on `EmbeddingDataset` revisions
(`docs/design/16-embedding-datasets.md:81-88`). The MySQL substrate stores
content in `MEDIUMBLOB`
(`philharmonic-store-sqlx-mysql/src/schema.rs:15-20`), which caps one
content blob at the MySQL `MEDIUMBLOB` limit. A `CorpusItem[]` containing
float vectors plus payload JSON can reach that limit quickly, especially
with bge-m3-scale embeddings and support knowledge-base payloads.

This is not just an operator sizing concern because the public API exposes
`GET /v1/embed-datasets/{id}/corpus` as a single `CorpusItem[]` response
(`docs/design/16-embedding-datasets.md:372-375`) and the workflow engine
injects available corpora directly into `script_arg.data`
(`docs/design/16-embedding-datasets.md:165-180`). Before implementation,
the design needs explicit item-count and byte caps, chunked content slots,
a paging/read model, or a different storage shape for large corpora.

### Per-job timeout is described on a surface that does not exist

The design says embed jobs use a long timeout set on `MechanicsJob`, not on
the mechanics worker's global config
(`docs/design/16-embedding-datasets.md:321-326`). Current `MechanicsJob`
has only `module_source`, `arg`, and `config`
(`mechanics-core/src/internal/job.rs:31-44`). Run timeout and execution
limits are configured on `MechanicsPoolConfig`, not per job
(`mechanics-core/src/internal/pool/config.rs:21-24`,
`mechanics-core/src/internal/pool/config.rs:129-139`), and `MechanicsPool`
applies the pool run timeout while waiting for worker replies
(`mechanics-core/src/internal/pool/api.rs:199-275`).

This conflicts with the same document's "Mechanics worker: no changes"
claim (`docs/design/16-embedding-datasets.md:441-442`). The design either
needs to add a mechanics/API executor surface for per-job limits, route
embed jobs to a separate worker pool with long limits, or remove the
`MechanicsJob` timeout claim.

### Lowerer changes are both required and denied, and touch crypto-bound
semantics

The lowerer section proposes replacing the current `ConfigLowerer::lower`
signature with a `LowerScope` enum so embed jobs can pass
`LowerScope::Ephemeral { job_id }`
(`docs/design/16-embedding-datasets.md:278-313`). The current trait takes
`instance_id: EntityId<WorkflowInstance>` and `step_seq: u64`
(`philharmonic-workflow/src/lowerer.rs:7-18`). The implementation in the
API server also assumes a real workflow instance ID when minting connector
token claims and AEAD AAD (`bins/philharmonic-api-server/src/lowerer.rs:52-57`,
`bins/philharmonic-api-server/src/lowerer.rs:222-264`).

The same design later says "Lowerer: no changes"
(`docs/design/16-embedding-datasets.md:443-444`), which is directly
contradictory.

More importantly, the proposed ephemeral UUID changes the semantics of the
connector token's `inst` claim and the `ConnectorCallContext.instance_id`.
Current connector common types document both as workflow instance UUIDs
(`philharmonic-connector-common/src/lib.rs:31-36`,
`philharmonic-connector-common/src/lib.rs:43-51`). Since `inst`, `step`,
`config_uuid`, and `payload_hash` are part of signed token claims and AEAD
AAD, this is crypto-sensitive and should be explicitly routed through
Yuka's two-gate crypto review.

## Medium-severity findings

### Built-in embed script cannot read `max_batch_size` from lowered config

The design says the built-in embed script batches texts according to the
embed connector's `max_batch_size`, "read from the lowered config"
(`docs/design/16-embedding-datasets.md:266-273`). The lowered config
visible to mechanics is a `MechanicsConfig` endpoint map; the decrypted
connector config remains inside a COSE payload header and is not visible to
JavaScript. The API server lowerer constructs mechanics endpoint entries
with URL, headers, request/response body types, and limits
(`bins/philharmonic-api-server/src/lowerer.rs:280-291`), not the
connector implementation's private config.

The embed implementation's `max_batch_size` is an implementation config
field (`philharmonic-connector-impl-embed/src/config.rs:8-11`) enforced by
the connector request validator
(`philharmonic-connector-impl-embed/src/request.rs:13-29`). The built-in
script can only avoid overlarge batches if the API passes a separate batch
size alongside the items, uses a conservative fixed batch size, or calls
one item per request.

### Permission atom grouping does not match existing policy/WebUI grouping

The design introduces `embed_dataset:create`, `embed_dataset:read`,
`embed_dataset:update`, and `embed_dataset:retire`
(`docs/design/16-embedding-datasets.md:391-400`). It then suggests placing
them in the `deployment` permission group, or in a new `data` group
(`docs/design/16-embedding-datasets.md:402-404`).

Existing deployment atoms are operator-scope permissions such as
`deployment:tenant_manage`, `deployment:realm_manage`, and
`deployment:audit_read`
(`philharmonic-policy/src/permission.rs:53-58`). Embedding datasets are
tenant resources, so grouping them under deployment would blur the
operator-vs-tenant boundary. Existing WebUI grouping derives membership by
permission-string prefix
(`philharmonic/webui/src/components/permissions.ts:26-37`), so
`embed_dataset:*` will not appear under a `data` group unless the grouping
logic is changed. A new prefix such as `data:embed_dataset_create` or a
grouping override is needed if the UI should present these permissions
cleanly.

### API detail shape does not provide the WebUI Source Items tab

The design says `GET /v1/embed-datasets/{id}` returns metadata and status
only, and explicitly does not include corpus
(`docs/design/16-embedding-datasets.md:358-361`). It adds a separate corpus
endpoint (`docs/design/16-embedding-datasets.md:372-375`). The WebUI detail
page, however, requires tabs for both Source Items and Corpus
(`docs/design/16-embedding-datasets.md:424-429`).

There is no documented source-items endpoint and no statement that dataset
detail includes source items. As written, the WebUI cannot render the
Source Items tab. The design should either include source items in the
detail response, add `GET /v1/embed-datasets/{id}/source-items`, or remove
that tab from the v1 WebUI plan.

## Low-severity finding

### Update concurrency behavior is inconsistent

The concurrency section first says that if an update arrives while an embed
job is running, the API queues the update and starts a new embed job after
the current one completes (`docs/design/16-embedding-datasets.md:328-333`).
It then says the simplest v1 approach is to reject updates while
`status=Embedding` with `409 Conflict`
(`docs/design/16-embedding-datasets.md:335-337`). The API endpoint section
also codifies 409 (`docs/design/16-embedding-datasets.md:363-367`).

The document should pick one v1 behavior. Rejecting with 409 is simpler and
fits the endpoint contract; queuing can remain a future enhancement if the
revision/lost-background-task model is later hardened.

## WebUI usability concerns

The proposed WebUI relies on JSON editors/viewers for both source items and
corpus (`docs/design/16-embedding-datasets.md:417-429`). This is consistent
with the current v0 admin UI style for endpoint configs, but embedding
datasets are likely to involve larger and more repetitive data than endpoint
configs. A raw JSON array editor for `[{id, text, payload?}]` will be
error-prone for nontrivial corpora, and a raw `CorpusItem[]` viewer can
become unusable once vectors are present because each item contains a long
float array.

Minimum usability additions before implementation should include:

- JSON parse/schema validation before submit, with item index in errors.
- A table-style source-item editor or paste/import affordance for ordinary
  admin use, even if raw JSON remains available.
- Corpus preview that hides or collapses vectors by default and prioritizes
  `id` plus payload metadata.
- Clear polling/retry states for first embed, re-embed with old corpus
  available, and failed-with-fallback versus failed-without-corpus.
- Explicit response-size and item-count limits surfaced in the UI, matching
  whatever backend caps the design adopts.

## Crypto-review note

Any implementation of the `LowerScope::Ephemeral` path or equivalent
change to connector token/AAD semantics touches COSE_Sign1 claims,
COSE_Encrypt0 AAD, payload-hash binding, and lowerer behavior. Per
`AGENTS.md`, this needs Yuka's personal two-gate crypto review. This report
flags the issue only; no code was changed.
