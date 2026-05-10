# Embed dataset lowerer ephemeral pre-review

**Date:** 2026-05-10
**Prompt:** direct Codex request to pre-review `docs/crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md`

I reviewed the Gate-1 proposal for using synthesized, non-persisted workflow-instance UUIDs when lowering embedding-dataset embed jobs. I did not find a cryptographic primitive-level objection to Approach B itself: the current verifier derives `ConnectorCallContext` directly from signed claims, and neither `philharmonic-connector-service` nor the AEAD AAD construction looks up `inst` in the substrate.

I did find several surrounding design/security issues that should be resolved before approval or before the D5 implementation prompt is issued.

## Findings

### 1. Token lifetime is described as per-call, but the proposed lowerer path mints per-job/per-config tokens

The proposal's Gate-1 decision 2 says embed-job mechanics work can take longer than 600 seconds because "the connector token is per connector call (per item batch)." That does not match the current lowerer or connector architecture.

`bins/philharmonic-api-server/src/lowerer.rs` mints the COSE_Sign1 token and encrypted payload during `ConfigLowerer::lower`, embeds them into the returned `HttpEndpoint` headers, and hands that static lowered config to mechanics. The design docs also say one connector token is minted per config per step, not per script request. The proposed embed-job sketch lowers once before constructing the `MechanicsJob`; a built-in script that loops over batches would reuse the same `Authorization` and `X-Encrypted-Payload` headers for every batch.

With the current default `lowerer_token_lifetime_ms = 600_000`, any embed job whose later connector calls begin after expiry will fail even though the job's own run timeout is 30 minutes. This is primarily an availability/design bug, but it also matters for security because the obvious workaround, extending embed-job token lifetime to the full job timeout, expands the bearer replay window.

Recommended resolution: choose one explicitly before Gate-1 approval:

- make the embed script genuinely obtain a fresh lowered config per batch/call, which would require a mechanics/API callback or another refresh mechanism;
- lower per chunk by splitting the embed job into smaller mechanics jobs whose expected duration fits the existing 600-second token lifetime;
- or accept a longer embed-job connector token lifetime and update the threat model, rate-limit requirement, and proposal text accordingly.

### 2. Replay impact is understated because the token binds the encrypted config, not the cleartext connector request

The proposal says a leaked embed-job token can be replayed only "to produce identical ciphertext bytes" and claims no new replay surface compared to v1. The ciphertext/payload bytes are indeed bound by `payload_hash`, but the cleartext HTTP request body is not part of the signed claims or AEAD AAD. `bins/philharmonic-connector/src/main.rs` verifies/decrypts the static payload and then passes the request body from the current HTTP request to `Implementation::execute`.

So a leaked token plus encrypted payload is bearer access to the same tenant endpoint config until `exp`, with attacker-chosen request bodies accepted by the connector implementation's normal request validation. For an embed endpoint, that means arbitrary text embedding requests against the configured external provider during the token lifetime, not merely replay of one identical batch.

This is not necessarily a reason to reject Approach B, because the same model exists for v1 workflow-step connector tokens. But the proposal should state the actual replay capability, especially if finding 1 is resolved by lengthening the token lifetime for long embed jobs.

### 3. The audit-correlation mitigation contradicts the "do not persist the synthesized inst UUID" design

The proposal's Approach B risk section says the embed-job path records the non-substrate `inst` UUID "in the audit-info trailer of the dataset's revision history." I found no corresponding storage field or design text in `docs/design/16-embedding-datasets.md`, and the same proposal also says it does not persist the synthesized `inst` UUID.

If connector-service logs contain only `tenant`, `config_uuid`, `inst`, and `step`, an incident responder cannot reliably correlate a synthetic `inst` back to the embedding dataset/job unless the API side records that UUID somewhere durable or emits structured job logs with dataset ID, revision, config UUID, and synthetic inst together.

Recommended resolution: either persist the synthetic `inst` UUID in the embedding revision/job metadata, or remove the audit-trail mitigation claim and replace it with the actual logging/correlation plan.

### 4. The implementation sketch invents `SubjectContext::system_for_tenant`

The proposal sketch calls `SubjectContext::system_for_tenant(tenant_id)`, but `philharmonic-workflow/src/subject.rs` currently has only `Principal` and `Ephemeral` subject kinds and no system subject constructor. The current lowerer only consumes `subject.tenant_id`, so this is not a crypto bug by itself, but it hides a real design choice: who is the actor for a background embed job, and how should it appear in future audit or policy decisions?

Recommended resolution: either add and design a real system subject model before D5, or have the embed job carry forward the authenticated subject that initiated the dataset create/update, or state explicitly that the lowerer receives a minimal internal `SubjectContext` only for tenant binding and that no workflow `StepRecord` subject is produced.

## Overall recommendation

Approach B can probably be approved after tightening the proposal, but I would not approve the current text as-is. The core UUID-as-binding-identifier argument matches the current code. The token lifetime, replay semantics, and audit-correlation claims need to be corrected so the implementation prompt does not bake in false assumptions.
