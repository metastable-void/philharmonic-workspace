# Gate-1 proposal: lowerer ephemeral support for embedding-dataset embed jobs

**Date**: 2026-05-04
**Author**: Claude Code
**Status**: PENDING REVIEW (Yuka)
**Scope**: Post-v1 / post-MVP (embedding datasets feature, ROADMAP §9)
**Crypto-sensitive paths touched**: COSE_Sign1 token claims (`inst`, `step`),
COSE_Encrypt0 AEAD AAD, `payload_hash` binding.

---

## Summary

The post-MVP embedding-datasets feature
([`docs/design/16-embedding-datasets.md`](../../design/16-embedding-datasets.md))
introduces an **ephemeral embed job** — a one-shot mechanics
job dispatched by the API server when a dataset is created or
updated. It's not a workflow instance: there is no
`WorkflowTemplate`, no `WorkflowInstance`, no `StepRecord`.

The current `ConfigLowerer::lower` trait takes
`instance_id: EntityId<WorkflowInstance>` and `step_seq: u64`.
These flow into the COSE_Sign1 token's `inst` and `step` claims
and into the COSE_Encrypt0 AEAD AAD — both crypto-bound.

This proposal asks: **how do we lower a connector call from an
ephemeral job that has no workflow-instance identity, without
changing any cryptographic primitive or weakening the existing
binding?**

**Recommendation: Approach B (synthesized non-persisted instance
UUID).** No primitive change, no construction change, no public-
trait change, no wire-format change. The lowerer call site for
ephemeral jobs mints a fresh UUID per job and passes it as the
`instance_id`. Type-system muddling is the only cost; no
crypto cost.

**No new cryptographic primitives are introduced.** The
implementation calls already-Gate-2-approved library functions
identically to the v1 step lowerer, with a different *source*
for the `inst` UUID.

---

## Context

The embedding-datasets feature ships an ephemeral background
task that:

1. Reads a `TenantEndpointConfig` whose `implementation` is
   `embed`.
2. Lowers it (decrypt SCK blob, assemble `{realm, impl,
   config}`, COSE_Encrypt0 to realm KEM, mint COSE_Sign1).
3. Dispatches a `MechanicsJob` against the lowered config.
4. The mechanics worker runs a built-in JS embed script that
   calls the embed connector for each item and returns
   `CorpusItem[]`.

The job is **not** a workflow: no template, no instance, no
step record, no audit row. It runs once, and on success
appends a new revision to the `EmbeddingDataset` entity with
`status=Ready` + the corpus blob.

This means the existing v1 lowering path — built around
`WorkflowInstance.id` + `step_seq` — has no natural identity
to use for `inst` and `step` in the connector-authorization
token.

---

## Current shape

### Trait

`philharmonic-workflow/src/lowerer.rs:9-18`:

```rust
#[async_trait]
pub trait ConfigLowerer: Send + Sync {
    async fn lower(
        &self,
        abstract_config: &JsonValue,
        instance_id: EntityId<WorkflowInstance>,
        step_seq: u64,
        subject: &SubjectContext,
    ) -> Result<JsonValue, ConfigLoweringError>;
}
```

### Token claims (COSE_Sign1 payload)

`philharmonic-connector-common/src/lib.rs:13-39`:

```rust
pub struct ConnectorTokenClaims {
    pub iss: String,
    pub exp: UnixMillis,
    pub iat: UnixMillis,
    pub kid: String,
    pub realm: String,
    pub tenant: Uuid,
    pub inst: Uuid,        // ← WorkflowInstance UUID at the wire
    pub step: u64,         // ← step sequence within the instance
    pub config_uuid: Uuid,
    pub payload_hash: Sha256,
}
```

### AEAD AAD inputs (COSE_Encrypt0)

`philharmonic-connector-client/src/encrypt.rs:259-289` —
`AeadAadInputs` covers `realm, tenant, inst, step,
config_uuid, kid`. The CBOR-encoded struct is SHA-256-digested
to produce the 32-byte external AAD bound into the AES-256-GCM
ciphertext.

### Service-side delivery

`philharmonic-connector-common/src/lib.rs:41-56` —
`ConnectorCallContext` carries `instance_id: Uuid, step_seq:
u64` plus tenant/config_uuid/iat/exp. Implementations consume
this in `Implementation::execute(ctx, ...)`.

### Crucial property: `inst` is a wire-format `Uuid`, not an `EntityId`

The Rust trait uses the phantom-typed
`EntityId<WorkflowInstance>` for compile-time safety, but the
COSE_Sign1 claim, the AAD CBOR struct, and the
`ConnectorCallContext` all carry `Uuid` — connector-common is
deliberately ignorant of `philharmonic-workflow`'s entity
markers. **The wire format already has no type-level claim
that `inst` corresponds to a real `WorkflowInstance` row in
the substrate.**

The connector service does **not** validate `inst` against any
substrate registry. It is a binding identifier (signed claim +
AEAD-bound), not a foreign key. This is documented in
[`docs/design/16-embedding-datasets.md`](../../design/16-embedding-datasets.md)
§"Lowerer integration" and confirmed in this codebase by the
absence of any `inst`-lookup logic in
`philharmonic-connector-service/src/`.

---

## Approach A — `LowerScope` enum

### Shape

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

### What changes

- **Public trait**: `philharmonic-workflow` minor or major bump.
- **Connector-common** (optional): add a `scope` discriminator
  claim (e.g. `scope: ScopeKind` where `ScopeKind = Step |
  Ephemeral`) so the service can refuse a step token where an
  ephemeral one is expected (or vice versa).
- **Connector-service**: accept the new claim and either treat
  the two scopes identically or split delivery paths.
- **API server lowerer**: switch all v1 call sites from `(id,
  step)` to `LowerScope::Step{..}`; add ephemeral call site.

### What doesn't change

- Crypto primitives — same Ed25519 / ML-KEM-768 / X25519 /
  HKDF-SHA256 / AES-256-GCM stack.
- AAD construction order, HKDF info string, nonce scheme.
- Key material handling.

### Risks

- **Token-class confusion** if the discriminator is omitted
  or ignored at verification: a valid ephemeral token reused
  in a step context (or vice versa). Bound by `payload_hash`
  + `tenant` + `realm` + `config_uuid`, so the practical
  exploit surface is narrow, but the type-of-call ambiguity
  is real.
- **Public-trait churn**: every consumer of `ConfigLowerer`
  in the workspace updates simultaneously; published
  `philharmonic-workflow` minor bump.
- **Test-vector regeneration**: if a `scope` claim is added,
  every COSE_Sign1 test vector in connector-common /
  connector-client / connector-service is regenerated.

### Cost

Public-trait change. Discriminator-claim plumbing. Test-vector
regeneration. Connector-service verification update. Roughly
4–6 crates touched.

---

## Approach B — synthesized non-persisted instance UUID

### Shape

`ConfigLowerer` trait **unchanged**. `ConnectorTokenClaims`
**unchanged**. `AeadAadInputs` **unchanged**.
`ConnectorCallContext` **unchanged**. Connector-service
verification logic **unchanged**.

The embed-job dispatch site in `philharmonic-api-server`
mints a fresh UUID v4 per ephemeral job:

```rust
let ephemeral_inst: EntityId<WorkflowInstance> =
    EntityId::from_uuid(Uuid::new_v4());
let ephemeral_step: u64 = 0;
let lowered = lowerer
    .lower(&abstract_cfg, ephemeral_inst, ephemeral_step, &subject)
    .await?;
```

The lowerer pipes that UUID through to `ConnectorTokenClaims.inst`
and `AeadAadInputs.inst` exactly as it does for real workflow
steps. The connector service verifies the COSE_Sign1 signature,
verifies the COSE_Encrypt0 AEAD with the same AAD shape, and
delivers a `ConnectorCallContext` whose `instance_id` is the
synthesized UUID. The service has no way to tell the difference
— and no security-relevant reason to care.

### What changes

- **`bins/philharmonic-api-server/src/`** (only): the embed-job
  dispatch site mints a UUID v4 per job. One isolated module
  edit, ~10 lines plus a clearly-marked code comment.
- **`docs/design/16-embedding-datasets.md`**: update the
  "Lowerer integration" section to reflect Approach B as the
  chosen path (already drafted as one of the two approaches).

### What doesn't change

- `philharmonic-workflow` (no version bump).
- `philharmonic-connector-common` (no claim added).
- `philharmonic-connector-client` (no AAD change).
- `philharmonic-connector-service` (no verification change).
- COSE_Sign1 / COSE_Encrypt0 wire format.
- Any cryptographic primitive, library, or construction.
- Existing connector-client / connector-service test vectors.

### Risks

- **Type-system muddling**: at the Rust type level,
  `EntityId<WorkflowInstance>` for an embed job is a lie —
  the embed job has no row in the `workflow_instance`
  store. **Mitigation**: the wire format already carries
  `Uuid`, not `EntityId<WorkflowInstance>`; the phantom type
  is a compile-time hint, not a substrate guarantee. The lie
  is contained to one call site, documented in code with a
  multi-line comment, and consumed by APIs (`mint_token`,
  `encrypt_payload`) that themselves only see a `Uuid`.
- **Audit-trail attribution**: an embed job's connector call
  appears in connector-service logs with an `inst` UUID that
  does not exist in `philharmonic-policy`'s substrate.
  **Mitigation**: the `iss` (lowerer / API server identity)
  + `tenant` + `config_uuid` claims still attribute the call
  correctly; the embed-job code path is the only one that
  produces non-substrate `inst` UUIDs, and it is recorded as
  such in the audit-info trailer of the dataset's revision
  history.
- **Future regression risk**: if the connector service or any
  downstream code starts validating `inst` against the
  substrate registry, embed-job tokens would silently fail.
  **Mitigation**: a single integration test exercising the
  embed-job → connector path catches that immediately;
  proposed test plan below requires it.
- **Cross-instance replay**: `inst` is signed and AEAD-bound
  to the encrypted payload via the AAD digest, and the token
  is short-lived (`exp` claim, default 600s per the v1
  decision in
  [`2026-04-30-phase-9-config-lowerer.md`](2026-04-30-phase-9-config-lowerer.md)).
  A leaked embed-job token can be replayed only within its
  lifetime, against the same realm/config, to produce
  identical ciphertext bytes. **No new replay surface**
  compared to v1 step tokens.

### Cost

One isolated bin-level code change. Zero crypto-library
changes. Zero published-crate changes. Zero test-vector
regeneration.

---

## Recommendation: **Approach B for v1.**

### Rationale

1. **No crypto-shape change.** The cryptographic primitives,
   constructions, AAD inputs, claim set, and wire format are
   bit-for-bit identical to the v1 step path. Whatever we
   approved in Phase 5 Wave A (Gate-2 2026-04-22) and Wave B
   (Gate-2 2026-04-23) and Phase 9 ConfigLowerer (Gate-1
   2026-04-30) holds for embed jobs verbatim.
2. **No public-trait churn.** `philharmonic-workflow`'s
   surface is settled and consumed by the API server, all
   v1 call sites, and any future workflow consumer. A v1.x
   minor bump for one post-v1 feature trades broad churn for
   narrow benefit.
3. **No test-vector regeneration.** The committed COSE_Sign1
   and COSE_Encrypt0 vectors in connector-common / -client /
   -service stay valid.
4. **Reversibility.** If a second ephemeral source emerges
   (e.g. periodic key rotation jobs, deployment self-tests),
   the question of "do we want a typed `LowerScope`?"
   reopens cleanly. Approach B leaves Approach A available
   as a future migration; the reverse is not true.

### Trade-off accepted

The Rust type system says
`instance_id: EntityId<WorkflowInstance>` even for embed
jobs. This is a documented muddling, not a security weakness.
The wire format and AAD inputs use plain `Uuid`, which has
always been the source of truth.

---

## Implementation sketch (Approach B)

### File: `bins/philharmonic-api-server/src/embed_job.rs` (new)

```rust
//! Background tokio task that runs an embedding job for a dataset.
//!
//! NOTE: ephemeral jobs use a synthesized non-persisted UUID for
//! `instance_id` because the embed job is not a workflow instance.
//! This is documented in docs/crypto/proposals/2026-05-04-…
//! (Gate-1 approved). The connector service does not validate
//! `inst` against the substrate; the synthesized UUID is purely a
//! signed claim + AEAD AAD binding identifier.

pub async fn run_embed_job(
    api: &ApiState,
    dataset_id: EntityId<EmbeddingDataset>,
    embed_endpoint_id: EntityId<TenantEndpointConfig>,
    items: Vec<SourceItem>,
    max_batch_size: usize,
    tenant_id: EntityId<Tenant>,
) -> Result<Vec<CorpusItem>, EmbedJobError> {
    // Synthesize a non-persisted WorkflowInstance UUID.
    // This is intentional — see Gate-1 proposal cited above.
    // The UUID is unique per job and is not inserted into the
    // workflow_instance store.
    let ephemeral_inst: EntityId<WorkflowInstance> =
        EntityId::from_uuid(Uuid::new_v4());
    let ephemeral_step: u64 = 0;

    let abstract_cfg = build_abstract_cfg_for_endpoint(
        &api.policy,
        embed_endpoint_id,
    ).await?;

    let subject = SubjectContext::system_for_tenant(tenant_id);

    let lowered = api.lowerer
        .lower(&abstract_cfg, ephemeral_inst, ephemeral_step, &subject)
        .await?;

    let mech_job = MechanicsJob::new(
        EMBED_SCRIPT_SRC,
        json!({
            "items": items,
            "max_batch_size": max_batch_size,
        }),
        lowered_to_mechanics_config(lowered)?,
    ).with_run_timeout(Duration::from_secs(1800));  // D2

    let result = api.mechanics.run(mech_job).await?;
    parse_corpus_items(result)
}
```

### What this adds

- One module under `bins/philharmonic-api-server/src/`.
- One UUID v4 mint per embed-job dispatch.
- Existing lowerer trait, claims, AAD, and connector-service
  verification all unchanged.

### What this requires from D2 (mechanics per-job timeout)

`MechanicsJob::with_run_timeout(Duration)` is the per-job
timeout knob from D2 (ROADMAP §9 A.D2). D2 ships before D5;
the embed job sets `run_timeout` to the configured embed-job
default (default 30 minutes per design 16).

---

## Crypto primitives used

Identical to the v1 step lowerer (Phase 9 Gate-1 approved
2026-04-30). Reproduced for the audit trail:

| Primitive | Library | Version | Original gate |
|---|---|---|---|
| Ed25519 signing (COSE_Sign1) | `ed25519-dalek` | 2.x | Wave A Gate-2 ✅ |
| ML-KEM-768 encapsulation | `ml-kem` | 0.2 | Wave B Gate-2 ✅ |
| X25519 ECDH | `x25519-dalek` | 2.x | Wave B Gate-2 ✅ |
| HKDF-SHA256 | `hkdf` + `sha2` | 0.13 / 0.11 | Wave B Gate-2 ✅ |
| AES-256-GCM (COSE_Encrypt0) | `aes-gcm` | 0.10 | Wave B Gate-2 ✅ |
| SHA-256 (payload hash + AAD digest) | `sha2` | 0.11 | Phase 2 Gate-2 ✅ |
| UUID v4 (ephemeral inst) | `uuid` (`v4` feature) | 1.x | not crypto-bound; UUID v4 is randomly generated |

**No new primitives, no new constructions, no new library
dependencies, no `unsafe` blocks.**

---

## Key material handling

Unchanged from Phase 9. The lowerer's signing key
(`LowererSigningKey`, `Zeroizing<[u8; 32]>` wrapper) and the
realm public keys (`RealmPublicKey`, no zeroization needed
— public material) are loaded at API-server startup and
reused for every lowering call (step or ephemeral).

The synthesized UUID v4 is **not** key material; it is a
public binding identifier sourced from `OsRng` via `uuid`'s
`v4` feature.

---

## What this does NOT do

- Does **not** introduce any new cryptographic primitive.
- Does **not** modify `philharmonic-connector-client`,
  `philharmonic-connector-common`,
  `philharmonic-connector-service`, or `philharmonic-workflow`.
- Does **not** change the COSE_Sign1 claim set or the
  COSE_Encrypt0 AAD construction.
- Does **not** introduce `unsafe` code.
- Does **not** add a discriminator between step and
  ephemeral tokens at the protocol layer (Approach A's
  feature; deliberately deferred — see "Recommendation"
  rationale point 4).
- Does **not** persist the synthesized `inst` UUID to the
  `workflow_instance` substrate.

---

## Test plan

Since Approach B calls the **same** Gate-2-approved primitives
as the v1 step path with **identical** AAD shape, **no new
crypto test vectors are required**. Existing vectors in
`philharmonic-connector-client`,
`philharmonic-connector-service`, and
`philharmonic-connector-common` cover the primitives.

The embed-job code path needs:

1. **Unit test** — `bins/philharmonic-api-server/tests/embed_job_lowering.rs`:
   - Run `run_embed_job` against a stub mechanics + stub
     connector router.
   - Assert that the lowered config decodes via
     `verify_and_decrypt` against the test realm KEM key.
   - Assert `ConnectorCallContext.instance_id` is the
     synthesized UUID (not zero, not the dataset UUID).

2. **Integration test** — extend the existing e2e suite:
   - Create a dataset with one source item, observe the
     embed job complete, assert `status=Ready` and corpus
     length 1.
   - The connector service in this test must **not** look
     up `inst` in any substrate. If a future change adds
     such a lookup, this test fails — which is the desired
     regression signal per "Risks" above.

3. **Negative test** — assert that an embed-job token does
   not collide with any in-flight workflow-step token by
   construction (UUID v4 randomness; verify in a probabilistic
   property test that 1 000 ephemeral UUIDs don't collide with
   1 000 instance UUIDs).

No fresh COSE_Sign1 / COSE_Encrypt0 hex-byte vectors are
generated; the existing committed vectors continue to pin the
crypto contract.

---

## Hard constraints reaffirmed

Per [`crypto-review-protocol`](../../../.claude/skills/crypto-review-protocol/SKILL.md):

- **No `unsafe` blocks** in crypto code or its immediate
  dependents. Approach B touches only bin-level dispatch code;
  no `unsafe` introduced.
- **No custom primitives.** Only the RustCrypto suite already
  approved in earlier gates. Approach B introduces zero new
  primitives.
- **Key material is zeroized** — unchanged from Phase 9; the
  lowerer's signing key uses `Zeroizing` already.
- **Signatures over untrusted input** — token claims are
  built by the API server itself, not from user input. Source
  items, payloads, and dataset configurations are user input
  but they ride inside the encrypted COSE_Encrypt0 payload,
  not in the signed claims.

---

## Hybrid KEM construction restated (per skill discipline)

For the audit trail — Approach B uses this construction
unchanged from Wave B Gate-2:

- **KEM-then-ECDH IKM ordering** for HKDF input:
  `IKM = ML-KEM-768 shared secret || X25519 shared secret`.
- **HKDF-SHA256**: `salt = empty`,
  `info = "philharmonic-connector-v1/aes-256-gcm"`,
  `len = 32` (AES-256 key).
- **AEAD**: AES-256-GCM with 12-byte random nonce per
  encryption (sourced from `OsRng`).
- **AAD** = SHA-256 of CBOR-serialized
  `{realm, tenant, inst, step, config_uuid, kid}` —
  `inst` here is the synthesized UUID for embed jobs.
- **Token claim binding**: `payload_hash = SHA-256(COSE_Encrypt0
  bytes)` is part of the COSE_Sign1 signed claim set, binding
  the encrypted payload to the signed token.

---

## Decisions to confirm at Gate-1 review

1. **Approach B vs Approach A for v1.** Proposal: B. (See
   "Recommendation" above.)
2. **Embed-job token lifetime.** Proposal: same default as the
   v1 step token — 600 seconds (10 minutes) — per the Phase 9
   ConfigLowerer Gate-1 decision. Embed-job mechanics work can
   take longer than 600s, but the connector token is per
   *connector call* (per item batch); each call is sub-second
   typically. The 30-minute embed-job timeout (D2) and the
   600-second connector token are independent timers.
3. **`step` value for ephemeral jobs.** Proposal: `0`. An
   ephemeral job has one logical step; using `0` keeps the
   AAD-binding deterministic and avoids any need for a per-job
   step counter.
4. **Synthesized UUID source.** Proposal: `uuid::Uuid::new_v4()`
   (RFC 4122 random). UUID v4 collision probability is
   negligible at workspace scale; no central registry is
   required.
5. **Documentation.** Proposal: a multi-line code comment at
   the synthesis site citing this Gate-1 file by path; design
   16's "Lowerer integration" section updated to mark
   Approach B as chosen (cross-link this file).

---

## Approval

```
Gate-1 status: PENDING REVIEW

Reviewer: Yuka MORI
Date: ___________
Decision: ☐ Approved as proposed
          ☐ Approved with changes (see notes)
          ☐ Rejected — switch to Approach A
          ☐ Rejected — neither approach; redesign

Notes:
```

Once approved, this file is referenced by the embed-job code
comment, the embed-job Codex prompt (D5 in ROADMAP §9), and
the eventual Gate-2 code review of the embed-job
implementation.
