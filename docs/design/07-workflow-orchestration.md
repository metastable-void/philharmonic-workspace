# Workflow Orchestration

`philharmonic-workflow` — the orchestration layer. Not yet
implemented; design is substantially complete. Depends on
`philharmonic-policy` for the `Tenant` entity marker (workflow
templates and instances are tenant-scoped).

## Three entity kinds

### `WorkflowTemplate`

The reusable definition. Tenant-scoped.

- Content slot `script` — JavaScript module source.
- Content slot `config` — abstract endpoint configuration
  (JSON). A `{script_name: config_uuid}` map binding the
  script's local endpoint names to `TenantEndpointConfig`
  entity UUIDs. Opaque to the workflow crate; interpreted by
  the lowerer.
- Entity slot `tenant` (pinned) — the owning tenant.
- Scalar `is_retired` (bool, indexed) — soft-delete flag.

Templates belong to one tenant. Cross-tenant sharing would
require explicit design; not in v1.

Template updates (new script, new abstract config, or both)
append a new revision to the same template UUID. Running
instances stay bound to their pinned template revision;
newly-created instances use whatever revision is current at
creation time. This matches how `TenantEndpointConfig` itself
evolves — same UUID, new revisions, pinned references from
dependents.

### `WorkflowInstance`

A running (or completed) execution. Tenant-scoped.

- Content slot `context` — current workflow state as canonical
  JSON.
- Content slot `args` — per-instance arguments, set at
  revision 0.
- Entity slot `template` (pinned) — reference to the template
  revision this instance was created against.
- Entity slot `tenant` (pinned) — the owning tenant. Inherited
  from the template at creation but stored explicitly for
  efficient tenant-filtered queries.
- Scalar `status` (i64, indexed) — lifecycle status enum.

Each revision of an instance is a state transition. The revision
log is the instance's lifecycle history.

### `StepRecord`

Immutable execution record.

- Content slot `input` — per-step input.
- Content slot `output` — present on success.
- Content slot `error` — present on failure (stringified error
  wrapped in JSON envelope).
- Content slot `subject` — structured caller context (see
  "Subject context" below). Records who authorized the step.
- Entity slot `instance` (pinned) — reference to the instance
  revision current when the step started.
- Scalar `step_seq` (i64, indexed) — step number within the
  instance.
- Scalar `outcome` (i64, indexed) — 0=Success, 1=Failure.

Step records are separate entities, not revisions on the
instance. This separates state history (the instance's revision
log) from execution history (queryable across all steps).

Subject content includes the subject kind (principal vs.
ephemeral), the subject identifier, and the minting authority
ID (for ephemeral subjects). Full injected claims are **never**
persisted, regardless of tenant — committed as a
privacy-by-default choice with no per-tenant knob. See
`09-policy-and-tenancy.md`.

## Status enum

Stable i64 values:

- 0 `Pending` — created, no steps run.
- 1 `Running` — at least one step executed, non-terminal.
- 2 `Completed` — finished successfully. Terminal.
- 3 `Failed` — step error transitioned to failure. Terminal.
- 4 `Cancelled` — caller-cancelled. Terminal.

## Status transitions

```
Pending → Running   (first step starts and succeeds)
Pending → Completed (caller marks complete before any step)
Pending → Cancelled (cancel before any step)
Pending → Failed    (first step errors, including malformed output)
Running → Running   (step succeeds, no completion signal)
Running → Completed (script signals done=true, or caller completes)
Running → Failed    (step errors, including malformed output)
Running → Cancelled (cancel during execution)
```

The engine writes one instance revision per step execution, so a
first-step failure has to transition directly from `Pending`
(revision 0) to `Failed` — there is no intermediate Running
revision. Earlier drafts of this doc omitted `Pending → Failed`;
the transition was added when Phase 4's Codex-authored
implementation flagged the inconsistency between this diagram
and §Execution sequence step 8.

Terminal states have no outgoing transitions. The engine refuses
operations on terminal instances with `InstanceTerminal` error.

## Pinning conventions

- **Instance → template is pinned.** Template updates don't
  affect running instances.
- **Instance → tenant is pinned.** Tenant settings changes don't
  affect the instance's identity.
- **Step record → instance is pinned** to the pre-step revision.
  Step records are self-contained snapshots.
- **Latest references are not currently used** by the workflow
  layer. The substrate supports them if future needs arise.

## Subject context

Every engine operation that mutates state (executing a step,
completing, cancelling) carries a `SubjectContext` describing
the caller that authorized the operation. This threads through
to step records for audit and to the workflow script as an
additional argument.

```rust
pub struct SubjectContext {
    pub kind: SubjectKind,
    pub id: String,
    pub tenant_id: EntityId<Tenant>,
    pub authority_id: Option<EntityId<MintingAuthority>>,
    pub claims: JsonValue,
}

pub enum SubjectKind {
    /// Authenticated as a persistent principal (tenant admin,
    /// service account, etc.).
    Principal,
    /// Authenticated via an ephemeral token minted by a
    /// minting authority on behalf of an end user.
    Ephemeral,
}
```

For `Principal` subjects: `id` is the principal's entity ID,
`authority_id` is `None`, `claims` is typically empty or carries
minimal metadata.

For `Ephemeral` subjects: `id` is the opaque subject identifier
the minting authority asserted, `authority_id` is the minting
authority's entity ID, `claims` is the free-form injected
metadata (user ID in the tenant's system, account tier, locale,
whatever the minting authority chose to assert).

The API layer constructs `SubjectContext` from the authenticated
request context and passes it to the engine. The engine doesn't
perform authorization — that already happened at the API layer —
but uses the subject for step-record attribution and for
passing to the workflow script.

## The engine

```rust
pub struct WorkflowEngine<S, E, L> {
    store: S,
    executor: E,
    lowerer: L,
}

impl<S, E, L> WorkflowEngine<S, E, L>
where
    S: ContentStore + IdentityStore + EntityStore,
    E: StepExecutor,
    L: ConfigLowerer,
{
    pub async fn create_instance(
        &self,
        template_id: EntityId<WorkflowTemplate>,
        args: CanonicalJson,
        subject: SubjectContext,
    ) -> Result<EntityId<WorkflowInstance>, WorkflowError>;

    pub async fn execute_step(
        &self,
        instance_id: EntityId<WorkflowInstance>,
        input: CanonicalJson,
        subject: SubjectContext,
    ) -> Result<StepResult, WorkflowError>;

    pub async fn complete(
        &self,
        instance_id: EntityId<WorkflowInstance>,
        subject: SubjectContext,
    ) -> Result<(), WorkflowError>;

    pub async fn cancel(
        &self,
        instance_id: EntityId<WorkflowInstance>,
        subject: SubjectContext,
    ) -> Result<(), WorkflowError>;
}
```

Three type parameters for the three dependencies. All plug in at
construction time. Engine stays policy-naive (it doesn't
evaluate permissions; the API layer did), backend-naive (via
the store trait), and transport-naive (via the executor trait).

The instance's tenant is inherited from the template at creation
time; the engine reads the template's tenant slot and sets the
instance's tenant slot to match. The API layer is responsible
for checking the subject's tenant against the template's tenant
before calling `create_instance`.

## `StepExecutor` trait

```rust
#[async_trait]
pub trait StepExecutor: Send + Sync {
    async fn execute(
        &self,
        script: &str,
        arg: &JsonValue,
        config: &JsonValue,
    ) -> Result<JsonValue, StepExecutionError>;
}

pub enum StepExecutionError {
    Transport(String),
    ScriptError(String),
}
```

The `arg` parameter is the full script argument:
`{context, args, input, subject}`. The engine assembles this
before calling the executor.

Two error variants distinguish retryable-inconclusive
(`Transport`) from recorded-failure (`ScriptError`).

## `ConfigLowerer` trait

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

Takes abstract config (opaque to the engine), instance/step
context for token binding, and subject context. Produces
concrete config (also opaque to the engine, forwarded to the
executor).

Subject context is passed for two reasons: (1) future capability
implementations may want to include subject information in
encrypted payloads (e.g., for per-subject rate limiting at the
connector service); (2) audit trails in the lowerer's own
substrate writes want to attribute correctly. For v1, most
lowerers will not use the subject context for routing or
credential selection — credentials are per-tenant, not per-
subject.

The engine doesn't know what's in either config. The lowerer
handles all capability-specific and policy-specific concerns.

## Script argument

The workflow script receives a four-field argument:

```javascript
export default async function main({context, args, input, subject}) {
    // context: threaded state, evolves step by step (content-
    //          addressed across revisions)
    // args:    per-instance arguments, set at creation,
    //          immutable
    // input:   per-step input value, varies per invocation
    // subject: authenticated caller context (see below)
    return {context: newContext, output: stepOutput, done: true};
}
```

The `subject` field shape matches `SubjectContext` serialized to
JSON:

```javascript
{
    kind: "ephemeral",  // or "principal"
    id: "opaque-subject-id",
    tenant: "tenant_abc",
    authority: "auth_xyz",  // null for kind="principal"
    claims: {
        // Free-form, tenant-defined for ephemeral subjects;
        // typically empty or minimal for principals.
        user_id: "u_12345",
        locale: "ja-JP"
    }
}
```

Scripts that don't care about the caller can ignore the
`subject` field. Scripts that do (for example, passing the
injected user ID into a SQL query) destructure it directly.

Claims are free-form for v1 — the minting authority and the
script coordinate the schema within the tenant's own
application. Philharmonic doesn't validate claim shape.

## Execution sequence

1. Read instance's latest revision. Verify non-terminal. Compute
   next revision sequence as `latest_seq + 1`.
2. Read template (pinned revision). Verify the template's
   tenant matches the instance's tenant (sanity check;
   mismatch indicates data corruption). Extract script and
   abstract config content hashes.
3. Fetch script bytes and abstract config bytes from content
   store.
4. Read latest context and args from content store.
5. Assemble executor arg: `{context, args, input, subject}`.
6. Call lowerer with abstract config, instance/step context,
   and subject; get concrete config.
7. Submit job to executor with script, arg, and concrete
   config.
8. Process result:
   - **Transport failure** → `ExecutorUnreachable`, no records
     written. Caller may retry.
   - **Script error** → record failed step (with subject),
     transition instance to `Failed`.
   - **Malformed result** (missing `context` or `output`) →
     treat as script error.
   - **Valid result** → record successful step (with subject),
     check top-level `done` flag, transition instance to
     `Running` or `Completed`.
9. Create step record entity first (references current instance
   revision; carries subject content). Then append new instance
   revision.

## Completion mechanisms

Two ways to reach `Completed`:

1. **`complete()` engine method** — caller-driven. Explicit
   terminal transition. Required primitive.
2. **`done: true` in script return** — script-driven. Bounded
   workflows signal termination. Convenience built on top of
   the same internal logic.

Either mechanism produces a Completed-status revision.

## Single-step orchestration

The engine executes one step per call. Caller drives the loop
for multi-step workflows:

```rust
loop {
    let result = engine.execute_step(
        instance_id,
        input,
        subject.clone(),
    ).await?;
    if result.is_terminal() { break; }
    input = compute_next_input(result);
}
```

Embedded run-to-completion logic would require the engine to
decide when to stop, which couples to workflow-specific
semantics. Left out.

For browser-driven use cases (like the chat-app pattern), each
user message is a separate API call that results in one
`execute_step`; the caller is the browser, and multi-step
orchestration is per-user-turn rather than per-session.

## Workflow authoring patterns

The workflow primitives (templates, instances, steps,
`execute_step`) are general. Common patterns that fit them
cleanly:

### Chat session as workflow instance

One chat session maps to one `WorkflowInstance`. Each user
message is one `execute_step` call. The instance's `context`
threads conversation state; `args` set at creation hold session-
level parameters; each step's `subject` identifies the end
user.

Validates the existing design: the generic primitives fit this
specific use case without modification.

### Multi-step inference with a small LLM

For deployments using small (~4B) local models, workflow
authors combine structured-output LLM calls with deterministic
JavaScript to get reliable behavior:

- Vector-similarity classification for broad intent matching.
- Structured-output LLM call for parameter extraction once the
  class is narrowed.
- Validation in JavaScript (parameters match expected shape,
  constraints satisfied).
- Branch on validated result to SQL/HTTP connector calls.
- Template responses rendered from results.

Fallback patterns (try local model; on validation failure or
low confidence, escalate to a larger model) are implemented in
the script, not in the engine.

These patterns don't require engine changes — they're authoring
guidance. Formal authoring documentation is a separate piece of
work.

## What the workflow crate doesn't do

- **Retry policy** — caller decides.
- **Authorization** — the API layer checks permissions before
  calling the engine.
- **Scheduling** — external infrastructure. The engine provides
  `execute_step` as the primitive; schedulers or event-driven
  callers drive it.
- **Multi-instance coordination** — scripts compose via
  connector calls if coordination is needed.
- **Tenant administration** — the policy layer owns tenants.
- **Subject authentication** — the API layer authenticates; the
  engine trusts the `SubjectContext` it receives.

## Dependencies

- `philharmonic-types` — cornerstone.
- `philharmonic-store` — substrate traits.
- `philharmonic-policy` — for the `Tenant` entity marker (and
  `MintingAuthority` for step record entity slots if that
  optional reference is added).

The workflow crate does **not** depend on `mechanics-core` or
the connector crates directly. Communication with the executor
is through the `StepExecutor` trait; communication with the
lowerer is through the `ConfigLowerer` trait. Concrete
implementations plug in at construction time (typically in the
API layer binary).

## Status

Design complete in outline, with the following refinements
from earlier drafts:

- Tenant entity slots on `WorkflowTemplate` and
  `WorkflowInstance` (making the workflow crate dependent on
  `philharmonic-policy`).
- `SubjectContext` parameter added to mutating engine methods.
- Script argument expanded from `{context, args, input}` to
  `{context, args, input, subject}`.
- Step records carry subject content for audit attribution.
- `ConfigLowerer` trait takes subject context (for future
  capability implementations that may consume it).

Implementation not started. Dependent on the connector layer
design being settled (the `ConfigLowerer` contract has to be
stable before the workflow implementation can plug in a
concrete lowerer).
