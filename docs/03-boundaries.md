# Philharmonic — Layer Boundaries

This document is about the seams between the system's layers: what
crosses each boundary, what doesn't, and why the boundary is drawn
where it is. Each prior document covered one layer and included a
section on what that layer doesn't know about the others. This
document covers the boundaries themselves, from the outside.

The aim is to make the layering navigable. When a contributor asks
"where does this feature belong?", the boundaries are the answer.
A feature lives on one side of a boundary or the other; placing it
on the wrong side is the failure mode this document tries to
prevent.

## The boundaries

There are six boundaries worth being explicit about. Five exist in
the current system; one is forward-looking (the policy boundary,
not yet realized). They are:

1. **Cornerstone ↔ everything else.** What's shared vocabulary vs.
   what's domain-specific.
2. **Storage trait ↔ storage backend.** What the substrate's trait
   surface owns vs. what implementations own.
3. **Storage substrate ↔ orchestration layer.** What the substrate
   stores vs. what the orchestrator interprets.
4. **Orchestration ↔ execution.** What the workflow engine
   coordinates vs. what the executor computes.
5. **Execution library ↔ execution service.** What `mechanics-core`
   provides vs. what `mechanics` (the HTTP wrapper) adds.
6. **Workflow ↔ policy** *(forward-looking)*. What the workflow
   layer handles vs. what the future policy layer will handle.

Each section below describes one boundary along the same axes:
what crosses it, what doesn't, and why it's drawn there.

## Cornerstone ↔ everything else

The cornerstone (`philharmonic-types`) sits below every other
crate. The boundary between it and its consumers is about scope:
what counts as workspace-wide vocabulary vs. what counts as
specific to a layer.

**What crosses.** Types that appear in multiple crates' public
APIs. `Uuid`, `JsonValue`, `Sha256`, `ContentHash<T>`, `Identity`,
`EntityId<T>`, the `Entity` trait and its associated slot
declarations, `UnixMillis`, `CanonicalJson`. These are the lingua
franca that lets crates compose without redefining shared types.

**What doesn't.** Specific entity kinds (`WorkflowInstance`,
`StepRecord`, future `Tenant`, etc.). These are owned by the
crates that define them. Domain logic of any kind. Storage
interfaces, executor interfaces, transport details. Convenience
utilities that aren't shared types.

**Why here.** The cornerstone earns its load-bearing role by
staying small and stable. Every type added to the cornerstone is a
type that downstream crates have to update for when it changes.
The bar for inclusion is "this type would otherwise be redefined
in multiple crates." The bar for exclusion is "this type is
specific to one crate's domain." Both bars are conservative; the
cornerstone is meant to be the slowest-moving crate in the
workspace.

A type that's borderline ("might appear in multiple APIs
eventually") starts in the crate that needs it, and migrates to
the cornerstone when the second consumer appears. Pulling it up
proactively risks designing for use cases that never materialize.

## Storage trait ↔ storage backend

The substrate split: `philharmonic-store` defines traits;
`philharmonic-store-sqlx-mysql` (and future sibling crates)
implements them. The boundary is the trait surface itself.

**What crosses.** The trait methods (`put`, `mint`,
`create_entity`, `append_revision`, etc.), their signatures, and
the `StoreError` variants they return. The data types passed
across (UUIDs, hashes, byte vectors, scalar values, revision
inputs/rows) are all defined in the trait crate or the cornerstone.

**What doesn't.** SQL syntax, sqlx types, connection pools,
database-specific error codes, schema migration logic, transaction
mechanics. None of these appear in the trait surface; they're all
implementation concerns.

The `ConnectionProvider` trait is itself an interesting case.
It's a backend-specific abstraction (lives in the SQL crate,
returns sqlx connection types), but it serves the same role for
backend implementations that the substrate traits serve for
consumers: an extension point that lets deployments customize
behavior without modifying the implementation crate.

**Why here.** Putting the trait surface in a separate crate from
the implementation forces honesty about what's actually
backend-neutral. If trait and impl lived in one crate, it would be
too easy to write a method whose documentation mentions
"the MySQL backend will optimize this," or to add a method that
only one backend can support. The split makes those couplings
visible: any change to the trait crate is a change every
implementation must respond to.

The split also enables in-memory backends (planned but not yet
implemented as `philharmonic-store-mem`) to coexist with SQL
backends without either depending on the other. Consumers depend
on the trait crate; deployment chooses which implementation to
plug in.

## Storage substrate ↔ orchestration layer

This is the boundary the storage doc and the workflow doc both
described from their respective sides. Here it gets the
cross-cutting treatment.

**What crosses.** The substrate trait surface: a workflow engine
holding `S: ContentStore + IdentityStore + EntityStore` calls
substrate methods to read and write entities. Data crossing the
boundary is in substrate-defined shapes: `EntityRow`,
`RevisionRow`, `RevisionInput`, content blobs as bytes,
`ScalarValue`, `EntityRefValue`. Errors cross as `StoreError`
variants.

**What doesn't.** The substrate doesn't know what the data means.
It doesn't know that an entity with kind UUID `0x...` is a
`WorkflowInstance`. It doesn't know that `is_retired = true`
means a template shouldn't be used for new instances. It doesn't
know about the lifecycle state machine, the pinning conventions,
the JSON contract for scripts, or anything else that's
workflow-specific. From the substrate's view, every entity is
just bytes-and-references-and-scalars.

The orchestration layer, conversely, doesn't know how the
substrate stores data. It doesn't know about `BINARY(16)` columns,
about `MEDIUMBLOB` size limits, about the seven-table EAV-shaped
schema. It works through the trait surface and trusts the
implementation to handle persistence.

**Why here.** Putting workflow semantics in the substrate would
couple the substrate to one consumer's domain. A future policy
layer or audit-log layer wanting to use the substrate would either
have to share the workflow layer's conventions (forcing them all
to evolve together) or fork the substrate (defeating reuse). By
keeping semantics out of the substrate, multiple consumers can
share the same storage layer with their own conventions.

The cost is that the orchestration layer carries the
interpretation logic. Every read assembles meaning from
substrate-neutral data; every write decomposes meaning into
substrate-neutral data. The orchestrator's code is the thicker
layer, by design.

## Orchestration ↔ execution

The workflow engine reaches the JS executor through the
`StepExecutor` trait. This boundary is between the layer that
coordinates persistence (workflow) and the layer that runs
JavaScript (executor).

**What crosses.** The `StepExecutor::execute` call, with three
parameters: a script string, a JSON arg, a JSON config. The
return: a JSON value or a `StepExecutionError`. That's the entire
interface.

**What doesn't.** The executor has no idea what workflow it's
running. It doesn't see the instance ID, the template ID, the
step number, or the orchestrator's persistence state. The
orchestrator, conversely, has no idea what the executor does
internally — which Boa version, which worker, which host
configuration. It sees a JSON-in/JSON-out service.

Within this trait surface, three sub-boundaries are worth noting:

The script is passed as a string, not as a content hash. The
executor doesn't fetch from the content store; the orchestrator
fetches and ships the bytes. This keeps the executor stateless
(no content-store access) at the cost of slightly more bytes
crossing the wire per step.

The config is opaque to the orchestrator. The orchestrator
doesn't validate it, doesn't interpret it, doesn't even know its
schema beyond "it's a JSON value." The executor consumes it; if
it's malformed, the executor reports an error.

The error type is split into `Transport` and `ScriptError`. This
distinction matters at the workflow layer (transport errors don't
record a step; script errors do), so it's part of the trait
surface. Other distinctions (JS exception vs. malformed return,
HTTP timeout vs. connection refused) aren't workflow-relevant and
get collapsed into the two variants.

**Why here.** The trait abstraction lets the workflow crate be
testable without a running executor (mock implementations are
trivial), and lets the executor be swappable (different
transports, different runtime versions, different deployments) at
the application binary's choice. The narrow surface — one method,
three params, two error variants — keeps the contract small
enough to maintain across versions.

A wider trait surface (multiple methods for different job types,
typed config parameters, structured error variants) was
considered and rejected. Each addition would couple the workflow
crate to executor specifics. Keeping the trait minimal forces the
two crates to evolve independently.

## Execution library ↔ execution service

The split between `mechanics-core` (the Rust library wrapping
Boa) and `mechanics` (the HTTP service wrapping the library)
mirrors the storage substrate's trait/backend split, but for a
different reason.

**What crosses.** The library's API is what the service consumes.
Job submission (`MechanicsPool::run` or equivalent), result
delivery (a JSON value or error), pool management
(`MechanicsPoolConfig`, `MechanicsPoolStats`).

**What doesn't.** HTTP semantics, request routing, network
configuration, TLS, authentication. The library doesn't know it's
being driven by HTTP; it would work identically in a CLI tool or
an in-process embedding. The service doesn't know how the library
manages workers internally; it submits jobs and awaits results.

**Why here.** Two reasons.

First: deployments that want to embed JS execution in-process
(perhaps a single-node deployment or a test harness) can depend
on `mechanics-core` directly without dragging in HTTP
infrastructure. The library is reusable across deployment models.

Second: the HTTP service is the natural place for cross-cutting
operational concerns — metrics, tracing, request logging, rate
limiting, authentication. Putting these in the library would
force every consumer (including in-process ones) to deal with
them. Putting them in the service confines them to network
deployments, where they belong.

The orchestrator never depends on the library directly. It talks
to the service over HTTP via its `StepExecutor` implementation.
The library is the service's internals; from the orchestrator's
view, only the service exists.

## Workflow ↔ policy *(forward-looking)*

This boundary doesn't exist yet — `philharmonic-policy` is a
defensive name claim, not an implementation. But the design has
shape, and getting the boundary right early prevents trouble
later.

**What will cross.** Likely an entity-slot reference from
workflow entities to policy entities. `WorkflowInstance` would
gain a `tenant` entity-slot pointing at a `Tenant` entity in the
policy crate. The workflow engine's methods would gain a
tenant-context parameter (or accept it via some ambient context
mechanism), and queries would filter by tenant.

The substrate already supports cross-crate entity references —
`EntitySlot::of::<T>` takes any `T: Entity`, regardless of which
crate `T` lives in. A `WorkflowInstance` referencing a `Tenant`
is the same shape as a `WorkflowInstance` referencing a
`WorkflowTemplate`; the substrate doesn't care.

**What won't cross.** Authorization decisions, permission
evaluation, principal authentication. These will live in the
policy crate. The workflow engine won't decide whether a caller
is allowed to create an instance; it will assume the caller has
been authorized by something upstream (the API layer, a policy
gate). This keeps the workflow engine free of policy-specific
logic.

**Why here, when the time comes.** The discipline is the same as
every other boundary: the workflow crate doesn't know about
policy specifics; the policy crate doesn't know about workflow
internals. Each defines its own entity kinds and lets references
cross via the substrate's neutral entity-reference machinery.

The current design intentionally leaves room for this. The
workflow crate's engine doesn't have hardcoded "no tenants" logic
that would need to be removed; it just doesn't have tenant logic
at all. Adding tenant-awareness will be additive: new methods,
new entity slots, new query filters, no removal of existing
behavior.

## What the boundaries are trying to prevent

The principles document describes "layered ignorance" abstractly.
The boundaries above are the concrete realizations. The failure
mode each boundary prevents is roughly the same shape:

**Premature coupling.** A method gets added to layer A that uses
layer B's specifics for "convenience." Now A depends on B in a
way the trait surface doesn't capture. Replacing B becomes
impossible without rewriting A's coupled methods. The boundary
prevents this by keeping B's specifics out of A's source code.

**Spreading semantics.** Layer-specific meaning leaks into
neutral infrastructure. The substrate gains a method that only
makes sense for workflows; the executor gains an option that
only makes sense for one orchestrator. These leaks are
seductive — they "improve performance" or "simplify the common
case" — but they accumulate, and eventually the neutral
infrastructure isn't neutral anymore. The boundary prevents this
by keeping the infrastructure ignorant of what's above it.

**Versioning lock-in.** Two crates end up tightly coupled
because their internal types appear in each other's APIs. Now
neither can release without coordinating. The boundary prevents
this by keeping API surfaces narrow and using cornerstone-level
or layer-defined types at the seams.

The boundaries above are prophylactic. They cost some convenience
(the substrate could be slightly faster for workflow queries if it
knew about workflow semantics; the executor could optimize
specific orchestrator patterns if it knew about them). The
trade is that the system stays modular: pieces can be swapped,
upgraded, replaced, or reused in other contexts.

## When boundaries should move

Boundaries aren't immutable. Sometimes the right answer is to
move one — to pull a concern down into a lower layer, or push it
up into a higher layer.

The signs that a boundary should move:

**Repeated workarounds.** If multiple consumers of layer A
implement the same workaround for something layer A doesn't
provide, the workaround probably belongs in A. The duplication is
a signal.

**Frequent crossings.** If a method on layer A always gets
called immediately followed by a method on layer B in a specific
combination, the combination might belong in a higher-level
extension trait or in a convenience method that wraps both.

**Awkward extension surfaces.** If extending the system requires
plumbing the same thing through multiple layers, the layer
structure may need adjustment.

The signs that a boundary is correctly placed:

**Independent evolution.** Each side of the boundary evolves
without forcing changes on the other. New entity kinds in the
workflow crate don't require substrate changes; new sqlx versions
don't require workflow crate changes.

**Substitutability.** Each side has multiple plausible
implementations, even if only one exists currently. The substrate
trait surface admits an in-memory backend; the executor trait
admits mock implementations.

**Reusability.** The lower layer has uses outside its current
consumer. The substrate could host a policy system or an audit
log; the executor could run JavaScript for reasons other than
workflow steps.

When a boundary moves, the move should be deliberate: a recognized
pattern, not a one-off optimization. The principles document's
"defer until concrete" applies to boundary changes too. Moving a
boundary speculatively is just as risky as adding speculative
features.

## Reading map

The boundaries described here are covered from each layer's side
in the per-layer documents. If you want the inside view of a
specific layer, read its document:

- Cornerstone: `02-01-cornerstone.md`
- Storage: `02-02-storage.md`
- Execution: `02-03-execution.md`
- Workflow: `02-05-workflow.md`

This document is the cross-cutting view. Use it when the question
is "where does X belong?" — a feature, a method, a piece of
configuration. The boundaries above are the answer.
