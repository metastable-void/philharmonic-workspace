# Embed job security pre-review

**Date:** 2026-05-10
**Prompt:** direct Codex request to pre-review `bins/philharmonic-api-server/src/embed_job.rs` for blocking security problems

I reviewed `bins/philharmonic-api-server/src/embed_job.rs` as a focused security/audit pass, with surrounding checks in the embedding-dataset routes, the API-server lowerer, the mechanics executor, and Design 16. I did not edit code or run tests.

The module touches SCK decryption and the lowerer path for connector-token minting, so the findings below are crypto-sensitive in the workspace sense and should be included in Yuka's review gate for the embed-job dispatcher.

## Findings

### 1. Blocking: corpus output is effectively unbounded before storage

`parse_corpus_output` accepts the mechanics worker's `output` array and allocates a `Vec<CorpusItem>` plus one `Vec<f32>` per returned vector:

- `bins/philharmonic-api-server/src/embed_job.rs:361` parses `output`.
- `bins/philharmonic-api-server/src/embed_job.rs:368` preallocates the corpus from the returned array length.
- `bins/philharmonic-api-server/src/embed_job.rs:382` preallocates each vector from the returned vector length.
- `bins/philharmonic-api-server/src/embed_job.rs:213` then encodes the corpus.
- `bins/philharmonic-api-server/src/embed_job.rs:216` stores the encoded bytes as the dataset `corpus` content blob.

There is no cap on returned item count, vector dimension, encoded corpus byte length, or parsed JSON response size at this layer. The source-items API enforces input caps, but the embed endpoint or mechanics worker controls output vector shape and can return far larger data than the original source input implies.

Design 16 says the API enforces a total corpus blob default cap of 1 GiB: `docs/design/16-embedding-datasets.md:129`. I found no implementation of that cap in the embed-job path.

Security impact: a tenant-controlled endpoint configuration, compromised connector/provider, or compromised mechanics worker can induce API-server memory pressure and very large content-store writes during background embedding. This is a resource-exhaustion boundary and should block landing until the dispatcher enforces the corpus cap.

Recommended resolution:

- Add an embed-dataset corpus cap to the runtime configuration passed into `EmbedJobDispatcher`.
- Enforce it after `encode_corpus`, before `put_bytes`.
- Also consider parser-side limits before allocating, such as maximum output item count matching source item count, maximum vector length, and rejecting duplicate or unknown item IDs.
- Ensure the mechanics/executor HTTP response path has a response-size bound appropriate for embed jobs; this module should not rely only on downstream storage limits.

### 2. Tenant for crypto binding is trusted from the dispatcher call instead of checked against the dataset revision

`EmbedJobDispatcher::run` receives `tenant_id` from the caller and uses it for:

- SCK decrypt of the endpoint config in `read_max_batch_size` (`bins/philharmonic-api-server/src/embed_job.rs:242` through `:248`).
- Lowerer subject construction (`bins/philharmonic-api-server/src/embed_job.rs:109` through `:115`).
- Connector-token/AAD tenant binding through the lowerer (`bins/philharmonic-api-server/src/lowerer.rs:163` through `:169`).

The current HTTP route call sites appear to pass this argument only after tenant checks, so I did not find a normal-route exploit. However, `embed_job.rs` already has the dataset revision in hand and copies the `tenant` entity reference into appended revisions (`bins/philharmonic-api-server/src/embed_job.rs:202`). It should not need to trust an external tenant argument for a crypto-sensitive operation.

Security impact: this is a defense-in-depth and future-call-site risk around tenant isolation and SCK-bound decrypt/lowerer behavior. A wrong tenant argument currently causes decryption failure for correctly-bound endpoint configs, but that failure mode should not be the only boundary. If future maintenance introduces a mismatched dataset/tenant dispatch path, this module would attempt the crypto operation before detecting the mismatch.

Recommended resolution:

- At the start of `run`, compare `required_entity_ref(&latest, "tenant")?.target_entity_id` to `tenant_id.internal().as_uuid()` and fail before decrypting/lowering if they differ.
- Alternatively, derive the tenant from the dataset revision and remove the tenant argument from the dispatcher interface, if that fits the API crate boundary.

### 3. Endpoint state is validated at API request time, but not at job execution time

The create route validates that `embed_endpoint_id` references the same tenant, is not retired, and has implementation `embed`:

- `philharmonic-api/src/routes/embed_datasets.rs:412` through `:446`.

The embed job later re-resolves the public endpoint ID and reads the latest endpoint revision:

- `bins/philharmonic-api-server/src/embed_job.rs:92` resolves the stored public endpoint UUID.
- `bins/philharmonic-api-server/src/embed_job.rs:233` reads the latest endpoint revision for `max_batch_size`.
- `bins/philharmonic-api-server/src/embed_job.rs:116` through `:121` asks the lowerer to lower the same public endpoint.

Neither `embed_job.rs` nor `bins/philharmonic-api-server/src/lowerer.rs` re-checks the endpoint revision's owning tenant, `is_retired`, or `implementation` before job execution. The lowerer also decrypts the latest endpoint config without those semantic checks.

Security impact: a queued/racing embed job can use an endpoint after it has been retired or changed away from `embed`. This is likely an authorization/lifecycle violation rather than direct credential leakage, but it weakens the intended endpoint-control boundary.

Recommended resolution:

- Revalidate the endpoint revision in `embed_job.rs` before decrypting/lowering: same tenant, not retired, implementation exactly `embed`.
- Consider whether `ConnectorConfigLowerer` itself should reject retired or cross-tenant endpoint configs for all lowerer callers, not only embed jobs.
- Decide whether the job should use the latest endpoint revision by design, or pin the endpoint revision captured at dataset create/update time. The current code uses latest-at-job-time semantics.

## Notes

I did not flag the synthesized `WorkflowInstance` ID pattern as a new issue in this pass. It matches the Gate-1-approved Approach B shape described in the prior lowerer-ephemeral review, including the non-persisted UUID and structured log line with `synthetic_inst`.

I also did not flag `expect("freshly-minted v7+v4 pair satisfies Identity::typed")` as a blocker. The Rust convention allows narrow justified exceptions, and this one is tied to freshly minted UUID versions. If Yuka wants zero panics even for this invariant in the bin, it can be converted to an error path, but I do not consider that the blocking security issue here.

## Overall recommendation

Block landing on the missing corpus-size/resource cap. Treat the tenant derivation check and endpoint revalidation as strongly recommended security hardening before or with the same patch, because this code path crosses the SCK decrypt and connector-token lowerer boundary.
