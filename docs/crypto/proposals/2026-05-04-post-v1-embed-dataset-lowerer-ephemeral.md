# Gate-1 proposal: lowerer ephemeral support for embedding-dataset embed jobs

**Date**: 2026-05-04 (revised 2026-05-10 — see "Pre-review feedback addressed" and "Self-review feedback addressed" below)
**Author**: Claude Code
**Status**: PENDING REVIEW (Yuka)
**Scope**: Post-v1 / post-MVP (embedding datasets feature, ROADMAP §3 — was §9 prior to the 2026-05-10 ROADMAP trim)
**Crypto-sensitive paths touched**: COSE_Sign1 token claims (`inst`, `step`),
COSE_Encrypt0 AEAD AAD, `payload_hash` binding.

---

## Pre-review feedback addressed (2026-05-10)

Codex produced a pre-review pass at
[`docs/codex-reports/2026-05-10-0001-embed-dataset-lowerer-ephemeral-pre-review.md`](../../codex-reports/2026-05-10-0001-embed-dataset-lowerer-ephemeral-pre-review.md)
with four findings. All four are addressed in this revision (no
wontfix). Summary of changes:

1. **Token-lifetime mismatch** (Finding 1) — the original
   decision 2 said the connector token is "per connector call
   (per item batch); each call is sub-second typically". The
   actual lowerer mints ONE token per `lower()` call (see
   `bins/philharmonic-api-server/src/lowerer.rs:251-266`); the
   embed script reuses the same `Authorization` /
   `X-Encrypted-Payload` headers across every batch. Decision 2
   below now explicitly sets the embed-job token lifetime to
   match the embed-job's `run_timeout` (default 30 minutes via
   D2), with the wider replay window called out in the threat
   model.
2. **Replay-impact understatement** (Finding 2) — the original
   "Cross-instance replay" risk said a leaked token only allows
   "produce identical ciphertext bytes". The encrypted payload
   is bound by `payload_hash`, but the cleartext request body
   is taken from the current HTTP request and forwarded to
   `Implementation::execute` (see
   `bins/philharmonic-connector/src/main.rs:388-426`). A leaked
   token + lowered config = bearer access to the configured
   tenant endpoint with attacker-chosen request bodies, until
   `exp`. The risk text now reflects this; this is the same
   model as v1 step tokens, but the wider lifetime from
   Finding 1 makes it materially worse, so it gets explicit
   threat-model treatment.
3. **Audit-correlation contradiction** (Finding 3) — the
   original mitigation said the synthesized `inst` UUID is
   "recorded in the audit-info trailer of the dataset's
   revision history", but the same proposal says the UUID is
   not persisted and design 16 has no corresponding storage
   field. Replaced with a structured-logging plan: the API
   server emits one log line at embed-job dispatch carrying
   `{tenant, dataset_id, revision_id, embed_endpoint_id,
   config_uuid, synthetic_inst}` so incident responders can
   join API logs with connector-service logs by SHA. Design 16
   §"Lowerer integration" gains the same note alongside this
   commit.
4. **Invented `SubjectContext::system_for_tenant`** (Finding 4)
   — the implementation sketch called a constructor that does
   not exist; `philharmonic-workflow/src/subject.rs` only
   exposes `SubjectKind::{Principal, Ephemeral}`. The sketch
   now constructs `SubjectContext` inline with the existing
   shape and notes that a proper system-actor model is a
   separate design question (deferred — the lowerer only
   consumes `subject.tenant_id` for tenant binding, and no
   `StepRecord` subject is produced because ephemeral jobs
   have no step records).

---

## Self-review feedback addressed (2026-05-10)

After Yuka's Gate-1 sign-off (committed in `e36cce2`, see
[`docs/crypto/approvals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md`](../approvals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md)),
Claude self-reviewed the revised proposal per the approval
condition and identified one further finding, addressed in this
revision:

5. **Synthesized `inst` UUID version + `EntityId`
   construction** — the original implementation sketch wrote
   `EntityId::from_uuid(Uuid::new_v4())` and
   `ephemeral_inst.as_uuid()`, neither of which exists on
   `EntityId<T>`; decision 4 also recorded `Uuid::new_v4()` as
   the synthesized inst source. The wire `inst` claim is
   sourced from `instance_id.internal().as_uuid()`
   (`bins/philharmonic-api-server/src/lowerer.rs:74`), where
   `InternalId<WorkflowInstance>` is UUID v7 — `InternalId::from_uuid`
   rejects anything that isn't v7
   (`philharmonic-types/src/id.rs:111-122`). The sketch now
   constructs the embed-job inst via `Identity { internal:
   Uuid::now_v7(), public: Uuid::new_v4() }.typed::<WorkflowInstance>()`
   so both halves' v7/v4 invariants hold and the wire `inst`
   matches the v1-step path bit-for-bit. Decision 4 and the
   crypto-primitives table are updated accordingly. The tracing
   line uses `.internal().as_uuid()` (the actual `EntityId`
   accessor). This is the same shape of error as Finding 4 —
   invented API; round-1 caught the SubjectContext call, this
   round catches the EntityId call.

   **Approval-state note**: Yuka's Gate-1 approval lives in the
   separate file linked above. The "PENDING REVIEW" status header
   and the Approval block at the bottom of this file are kept as
   the proposal's own artefact; the approval file is the
   authoritative sign-off record.

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
mints a fresh `EntityId<WorkflowInstance>` per ephemeral job
— a UUID v7 internal half (the wire `inst`) plus a UUID v4
public half (required by `Identity::typed` but never used
downstream):

```rust
let ephemeral_inst: EntityId<WorkflowInstance> = Identity {
    internal: Uuid::now_v7(),
    public: Uuid::new_v4(),
}
.typed()
.expect("freshly-minted v7+v4 pair satisfies Identity::typed");
let ephemeral_step: u64 = 0;
let lowered = lowerer
    .lower(&abstract_cfg, ephemeral_inst, ephemeral_step, &subject)
    .await?;
```

The lowerer extracts the wire `inst` from
`instance_id.internal().as_uuid()` (the v7 half) and pipes it
through to `ConnectorTokenClaims.inst` and `AeadAadInputs.inst`
exactly as it does for real workflow steps. The connector service
verifies the COSE_Sign1 signature, verifies the COSE_Encrypt0
AEAD with the same AAD shape, and delivers a
`ConnectorCallContext` whose `instance_id` is the synthesized
v7 UUID. The service has no way to tell the difference — and no
security-relevant reason to care.

### What changes

- **`bins/philharmonic-api-server/src/`** (only): the embed-job
  dispatch site mints a fresh `EntityId<WorkflowInstance>` per
  job (UUID v7 internal + UUID v4 public via `Identity::typed`).
  One isolated module edit, ~10 lines plus a clearly-marked
  code comment.
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
- **Audit-trail attribution** (revised per Finding 3): an
  embed job's connector call appears in connector-service
  logs with an `inst` UUID that does not exist in
  `philharmonic-policy`'s substrate. **Mitigation**: the
  `iss` (lowerer / API server identity) + `tenant` +
  `config_uuid` claims still attribute the call correctly at
  the connector-service side, and the API server emits one
  structured log line at embed-job dispatch carrying
  `{tenant, dataset_id, revision_id, embed_endpoint_id,
  config_uuid, synthetic_inst}` so an incident responder
  joins API-server logs (which carry the synthetic_inst at
  dispatch time) with connector-service logs (which carry
  the same UUID as `inst`) by SHA. **The synthesized UUID is
  not persisted into the workflow_instance substrate** (that
  is the "non-persisted" claim the proposal makes), but it
  is recorded in API-server logs and **may** also be added
  as an optional `embed_job_inst` field on
  `EmbeddingDatasetRevision` if Yuka wants substrate-level
  correlation; the proposal does not require this. Design 16
  §"Lowerer integration" gains the same logging note in the
  same commit.
- **Future regression risk**: if the connector service or any
  downstream code starts validating `inst` against the
  substrate registry, embed-job tokens would silently fail.
  **Mitigation**: a single integration test exercising the
  embed-job → connector path catches that immediately;
  proposed test plan below requires it.
- **Bearer-replay window** (revised per Finding 2):
  `inst` is signed and AEAD-bound to the encrypted lowered
  config via the AAD digest, and `payload_hash` binds the
  COSE_Encrypt0 ciphertext to the COSE_Sign1 claim set. **But
  the cleartext per-call request body is not part of either
  binding** — the connector service receives the lowered
  config (verified + decrypted via `verify_and_decrypt`) and
  the current HTTP request body separately, then passes both
  to `Implementation::execute(&payload.config, &request,
  &verified.context)` (see
  `bins/philharmonic-connector/src/main.rs:388-426`). So a
  leaked token + lowered-config pair gives the attacker bearer
  access to the configured tenant endpoint config — they can
  send arbitrary embed requests with attacker-chosen bodies,
  subject only to the connector implementation's normal
  request validation, until `exp`. This is the same model as
  v1 step tokens; Approach B does not introduce a new replay
  *capability*, but Finding 1's resolution (embed-job tokens
  matching the 30-minute embed-job `run_timeout` rather than
  the v1 600-second default) materially widens the
  *window*. **Mitigations**: the lowered config never leaves
  the API server's mechanics-job dispatch path (no on-disk
  cache, no log-line capture); per-step-call rate limits on
  the connector router (existing v1 control) bound the
  per-token request volume; structured logging at API-server
  dispatch (see Audit-trail attribution below) lets incident
  responders identify which dataset's job a leaked token
  belonged to.

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
    // Synthesize a non-persisted WorkflowInstance EntityId.
    // This is intentional — see Gate-1 proposal cited above.
    // The pair is unique per job and is not inserted into the
    // workflow_instance store.
    //
    // The wire `inst` claim is sourced from
    // `instance_id.internal().as_uuid()` in the lowerer
    // (`bins/philharmonic-api-server/src/lowerer.rs:74`), where
    // `InternalId<WorkflowInstance>` is UUID v7. We mint v7 for
    // the internal half (the half that reaches the wire) and v4
    // for the public half (required by the `Identity::typed`
    // v4-public-half invariant; never used downstream because
    // the embed job has no substrate row to externalise).
    // `Identity::typed` succeeds on a freshly-minted v7+v4 pair
    // by construction, so the `expect` is infallible.
    let ephemeral_inst: EntityId<WorkflowInstance> = Identity {
        internal: Uuid::now_v7(),
        public: Uuid::new_v4(),
    }
    .typed()
    .expect("freshly-minted v7+v4 pair satisfies Identity::typed");
    let ephemeral_step: u64 = 0;

    let abstract_cfg = build_abstract_cfg_for_endpoint(
        &api.policy,
        embed_endpoint_id,
    ).await?;

    // SubjectContext for embed-job dispatch.
    //
    // The lowerer reads only `subject.tenant_id` for tenant
    // binding (see `bins/philharmonic-api-server/src/lowerer.rs`
    // and `philharmonic-workflow/src/subject.rs`). Embed jobs
    // produce no StepRecord, so the other fields don't reach
    // the substrate either. We construct a minimal context
    // inline and use the existing `SubjectKind::Principal`
    // variant (the embed-job dispatcher is a persistent
    // codebase component, not an ephemeral-credential caller).
    //
    // A proper system-actor model — likely a third
    // `SubjectKind::System` variant — is a separate design
    // question deferred to a follow-up review (see Finding 4
    // resolution at the top of the Gate-1 proposal). Until
    // then, this stub carries forward the dataset's owning
    // tenant and a stable system identifier; nothing else
    // crosses the lowerer boundary.
    let subject = SubjectContext {
        kind: SubjectKind::Principal,
        id: format!("system:embed-job:{dataset_id}"),
        tenant_id,
        authority_id: None,
        claims: serde_json::Value::Null,
    };

    let lowered = api.lowerer
        .lower(&abstract_cfg, ephemeral_inst, ephemeral_step, &subject)
        .await?;

    // Structured log for incident-correlation (see Audit-trail
    // attribution risk + Finding 3 resolution at the top).
    // Emitted at INFO. Fields: tenant, dataset_id, revision_id
    // (resolved from `dataset_id` via the latest revision row),
    // embed_endpoint_id, config_uuid (from `lowered`), and
    // synthetic_inst (the UUID minted just above).
    tracing::info!(
        tenant = %tenant_id,
        dataset_id = %dataset_id,
        embed_endpoint_id = %embed_endpoint_id,
        synthetic_inst = %ephemeral_inst.internal().as_uuid(),
        "embed-job lowerer dispatch"
    );

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
- One fresh `EntityId<WorkflowInstance>` mint per embed-job
  dispatch (UUID v7 internal half = wire `inst`; UUID v4 public
  half = unused but required by `Identity::typed`).
- Existing lowerer trait, claims, AAD, and connector-service
  verification all unchanged.

### What this requires from D2 (mechanics per-job timeout)

`MechanicsJob::with_run_timeout(Duration)` is the per-job
timeout knob from D2 (ROADMAP §3 A.D2 — was §9 prior to the 2026-05-10 ROADMAP trim). D2 ships before D5;
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
| UUID v7 (ephemeral inst — wire `inst` half) | `uuid` (`v7` feature) | 1.x | not crypto-bound; v7 timestamp prefix + 62 random bits — collision-free at workspace scale |
| UUID v4 (ephemeral inst — `Identity` public half) | `uuid` (`v4` feature) | 1.x | not crypto-bound and not on the wire; minted only to satisfy `Identity::typed`'s v4-public-half invariant |

**No new primitives, no new constructions, no new library
dependencies, no `unsafe` blocks.**

---

## Key material handling

Unchanged from Phase 9. The lowerer's signing key
(`LowererSigningKey`, `Zeroizing<[u8; 32]>` wrapper) and the
realm public keys (`RealmPublicKey`, no zeroization needed
— public material) are loaded at API-server startup and
reused for every lowering call (step or ephemeral).

The synthesized inst (UUID v7 + UUID v4 pair) is **not** key
material; it is a public binding identifier sourced from
timestamp + `OsRng` via `uuid`'s `v7` and `v4` features. Only
the v7 internal half reaches the wire as `inst`; the v4 public
half is local to the API server.

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
   construction (UUID v7 timestamp + 62 random bits; verify in
   a probabilistic property test that 1 000 ephemeral v7 UUIDs
   don't collide with 1 000 v7 workflow-instance UUIDs).

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
2. **Embed-job token lifetime.** Proposal (revised per
   Finding 1): set the embed-job lowerer's token lifetime to
   match the embed-job's `MechanicsJob::run_timeout` —
   default 1 800 000 ms (30 minutes per design 16) — rather
   than the v1-step default of 600 000 ms (10 minutes per
   the Phase 9 ConfigLowerer Gate-1 decision in
   [`2026-04-30-phase-9-config-lowerer.md`](2026-04-30-phase-9-config-lowerer.md)).
   The original framing — "the connector token is per
   *connector call* (per item batch); each call is
   sub-second typically" — was wrong: the lowerer mints
   ONE COSE_Sign1 token per `lower()` call (see
   `bins/philharmonic-api-server/src/lowerer.rs:251-266`),
   embeds it into the `HttpEndpoint` headers of the lowered
   config, and hands that static lowered config to the
   mechanics worker. The built-in JS embed script reuses the
   same `Authorization` and `X-Encrypted-Payload` headers
   for every batch in the loop. With a 600-second token
   lifetime, any embed job whose later batches start more
   than 10 minutes after dispatch fails on `exp` even though
   the job's own `run_timeout` is 30 minutes.

   Three resolution paths considered:

   - **(a) Per-batch refresh** — the embed script obtains a
     fresh lowered config per batch via a mechanics→API
     callback. Requires a callback channel that does not
     exist today; large infra change. Rejected for v1.
   - **(b) Per-batch mechanics jobs** — split the embed job
     into N mechanics jobs (one per batch), each with its
     own short-lived token. Keeps the 600-second default
     but blows up coordination (N pool slots, partial
     failures, retries). Rejected for v1; reconsider if
     embed-job token replay risk turns out to dominate.
   - **(c) Lifetime = embed-job run_timeout** — single
     token lasts as long as the job. Minimum implementation
     change; widens the bearer-replay window from 10 to 30
     minutes. **Chosen.**

   Implementation note: the API server's lowerer currently
   takes a single `token_lifetime_ms` at construction
   (`bins/philharmonic-api-server/src/lowerer.rs:33`). D5
   chooses between (i) a second lowerer instance configured
   for embed jobs, or (ii) a per-call lifetime override —
   either is a one-line surface decision.

   Threat-model implication: embed-job tokens have a
   3× wider bearer-replay window than v1 step tokens. The
   replay capability (per Finding 2 / "Bearer-replay window"
   above) is unchanged in shape — same realm/config bound,
   same `payload_hash` binding, same `Implementation::execute`
   bypass for cleartext request bodies — but the time
   exposure tripling deserves explicit acknowledgment, and
   the connector-router rate limits should be sized with
   embed-job traffic in mind.
3. **`step` value for ephemeral jobs.** Proposal: `0`. An
   ephemeral job has one logical step; using `0` keeps the
   AAD-binding deterministic and avoids any need for a per-job
   step counter.
4. **Synthesized UUID source.** Proposal (revised per
   self-review Finding 5): `Identity { internal: Uuid::now_v7(),
   public: Uuid::new_v4() }.typed::<WorkflowInstance>()`. The
   wire `inst` is the v7 internal half (`bins/philharmonic-
   api-server/src/lowerer.rs:74` extracts it via
   `instance_id.internal().as_uuid()`); the v4 public half is
   minted only because `Identity::typed` validates both halves
   and is never used downstream. UUID v7 has a millisecond
   timestamp prefix + 62 random bits, providing collision-free
   identifiers at workspace scale with no central registry.
   (The original proposal said `uuid::Uuid::new_v4()`;
   `InternalId::from_uuid` rejects v4 with `IdKindError`, so
   that wording would have failed at construction.)
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
comment, the embed-job Codex prompt (D5 in ROADMAP §3 — was §9 prior to the 2026-05-10 ROADMAP trim), and
the eventual Gate-2 code review of the embed-job
implementation.
