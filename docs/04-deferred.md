# Philharmonic — Deferred Decisions

This document enumerates features that are deliberately not in the
system, why each is deferred, and how the current design keeps the
door open for adding them later. The goal is to make the deferral
list short and visible, not hidden — when someone asks "why doesn't
philharmonic do X?", the answer should be findable in one place.

The principles document covers "defer until concrete" abstractly.
This document is the concrete list.

A note on framing: nothing here is rejected. Each feature was
considered and judged not yet worth implementing, with a path for
adding it when the use case demands. Calling these "deferred" rather
than "out of scope" or "not supported" reflects the intent: the
system is shaped to *not foreclose* them.

## Determinism in execution

**What it would be.** Reproducible JS execution: seed `Math.random`
from a job parameter, freeze `Date.now` to a job-supplied value,
intercept all external I/O so it can be recorded and replayed.
Combined with the substrate's append-only revision log, this would
enable replaying a workflow from any past state and getting
identical results.

**Why deferred.** Determinism requires interceptable host functions
for every source of non-determinism (clock, RNG, all I/O), and
implementing it correctly is more work than current needs justify.
The substrate's append-only log already captures every state
transition, so post-hoc diagnosis ("what was the context at step
5?") doesn't require replay — the history is already there. Replay
would matter for use cases like deterministic testing of workflow
logic, A/B comparison of script changes, or forensic recreation of
production behavior. None of these are pressing yet.

**How the door stays open.** The `mechanics-core` API can grow
optional job parameters (`rand_seed: Option<u64>`,
`time_freeze: Option<UnixMillis>`) without breaking existing
callers. The orchestrator can record those parameters in step
records when present, and replay tooling can re-run jobs with the
recorded values. The substrate doesn't need to change at all — the
revision log already preserves what's needed.

## Multi-step orchestration

**What it would be.** A way for the workflow engine to run multiple
steps automatically, without external triggering between them. A
script signals "advance me with this input," and the engine executes
the next step without the caller's intervention.

**Why deferred.** The current design has each `execute_step` call
triggered externally — by an API request, a scheduler tick, a queue
message. The trigger source is where decisions about timing,
backpressure, and concurrency naturally live. Embedding a "run to
completion" loop in the engine would require the engine to make
those decisions, which couples the engine to scheduling concerns.

The deferral keeps the engine narrow. Workflows that need
auto-advancement can implement a loop in the caller's code (the
shape was sketched in the workflow doc); the engine just processes
the calls.

**How the door stays open.** A future scheduler crate can sit above
the workflow engine and drive `execute_step` based on whatever
auto-advancement convention the system adopts (a `next_input` field
in step output, a `done` flag in context, etc.). The convention is
between the scheduler and the script; the engine remains agnostic.
Adding the scheduler doesn't require changes to the workflow engine
or the substrate.

## Retry policy

**What it would be.** Automatic retry of failed steps with
configurable backoff, max-attempts, and conditional logic. The
workflow engine would notice a step failure and retry instead of
returning the error to the caller.

**Why deferred.** Retry policy is highly context-dependent. The
right policy depends on the failure mode (transient network error
vs. script bug), the workflow's domain (idempotent vs. side-effecting
operations), and operational constraints (rate limits, downstream
load). Baking one policy into the engine would force every consumer
to use it; making it configurable would push complexity into the
engine without obvious benefit.

The current design treats retry as a caller concern. The caller
(or the layer that wraps the engine — an API handler, a scheduler)
decides whether and how to retry based on its own context.

**How the door stays open.** The engine's `execute_step` returns
errors with enough context for callers to make retry decisions
(transport failures vs. script errors, instance status, step
sequence). A future policy crate or wrapper crate can implement
retry policies that consult workflow state and decide when to
re-call `execute_step`. No engine changes needed.

If a use case ever justifies engine-level retry, the addition would
be a new method (`execute_step_with_retry`) or a configuration
parameter, not a behavioral change to the existing method.

## Tenant scoping in the substrate

**What it would be.** First-class tenant awareness in the storage
substrate: every entity belongs to a tenant, queries filter by
tenant, the substrate enforces tenant isolation.

**Why deferred.** Tenancy is a policy concern, and the system
doesn't have a policy layer yet. Adding tenant fields to substrate
entities would either require all consumers to pass tenant context
through every call (intrusive) or assume an ambient tenant context
(magical). Neither is appealing without concrete policy
requirements driving the design.

The current substrate is tenant-neutral: entities live in a flat
namespace, and any tenancy lives at the layer above the substrate.

**How the door stays open.** When `philharmonic-policy` exists with
a `Tenant` entity kind, workflow entities can gain a `tenant`
entity-slot reference (`EntitySlot::of::<Tenant>("tenant", ...)`).
Queries can filter on this slot via `find_by_scalar` if tenant IDs
are scalars, or via `list_revisions_referencing` if they're entity
references. The substrate already supports both patterns
generically — no substrate changes needed.

The workflow engine will gain tenant-aware methods at that point
(probably via extension traits, parallel to how the substrate's
typed extensions work). Existing tenant-naive code would continue
to work; tenant-aware code uses the new surface.

## Authorization and policy enforcement

**What it would be.** A way to determine whether a caller is
allowed to perform a workflow operation: create an instance, read a
template, execute a step. Includes principals (who's calling),
permissions (what they can do), and policy evaluation logic.

**Why deferred.** Same reasoning as tenancy: this is a policy
layer, and the policy crate doesn't exist yet. Designing
authorization speculatively would produce APIs shaped by imagined
threats and imagined user models, which usually don't match real
ones.

The current workflow engine assumes the caller has been authorized
by something upstream. The caller is whoever calls `execute_step`;
the engine doesn't ask who they are or what they're allowed to do.

**How the door stays open.** A future policy crate will likely
provide a gating layer that wraps the workflow engine: the gate
checks the caller's principal against a policy, and either calls
through to the engine or returns an authorization error. The
workflow engine itself doesn't need to change — it gets called
either by the gate (in policy-enabled deployments) or directly (in
deployments that don't use the policy layer).

If integration patterns demand engine-level awareness later (e.g.,
"the engine should only return entities the caller can read"),
that's a new method or extension trait, not a behavioral change.

## Connector routing in the orchestrator

**What it would be.** The orchestrator picks which executor
instance to send each job to based on tenant, realm, allowlist, or
other policy. Different jobs go to different executor pools with
different host configurations.

**Why deferred.** The current design treats the executor as a
single endpoint (whatever the load balancer in front of the worker
fleet presents). The orchestrator submits jobs against that
endpoint; routing decisions happen below it.

This works because all current jobs share the same execution
model: stateless workers, opaque configs, no per-request runtime
selection. If a deployment needs tenant-specific worker pools or
realm-specific networking, that's currently solved at the
load-balancer layer (different DNS names, different ingress rules)
rather than at the orchestrator layer.

A future system with truly heterogeneous executor needs (some jobs
needing GPU workers, some needing geo-restricted execution, some
needing isolated tenant pools) might need orchestrator-level
routing. None of these needs are concrete yet.

**How the door stays open.** The `StepExecutor` trait is per-engine
(the engine holds one). A future engine could hold multiple
executors and route between them based on per-step decisions; or a
single executor implementation could internally route to different
backends based on inspection of the job's config. Both patterns are
additive — the existing single-executor case continues to work.

## In-memory storage backend

**What it would be.** `philharmonic-store-mem`, a substrate
implementation backed by `Arc<Mutex<HashMap>>` collections. Useful
for testing downstream crates without spinning up MySQL, for
single-process embedded deployments, and for documentation
examples.

**Why deferred.** The SQL backend exists and works; testing
against it via testcontainers is functional, if slow. The
in-memory backend is a quality-of-life improvement, not a blocker
for any current work. Implementing it properly (including the
optimistic-concurrency semantics, the read-your-own-writes
contract, the error variants) is real work — maybe 500 lines and a
test suite.

The deferral is about prioritization, not design. The crate's
shape is well-understood; it would be a faithful in-memory
implementation of the substrate traits.

**How the door stays open.** The substrate's trait surface is
backend-agnostic. An in-memory backend is just another
implementation, sibling to the SQL backend. Crate name is already
claimed (`philharmonic-store-mem` is reserved). Implementation can
happen whenever the testing-ergonomics cost of testcontainers
becomes annoying enough to motivate it.

## Postgres backend

**What it would be.** `philharmonic-store-sqlx-pgsql`, a substrate
implementation backed by Postgres via sqlx. Would have its own LCD
discipline (Postgres-compatible features only, supporting standard
Postgres plus likely Aurora Postgres and CockroachDB).

**Why deferred.** No current need. The MySQL ecosystem covers
most of the deployment scenarios the system targets, and the LCD
MySQL discipline already supports four MySQL-compatible targets
(MySQL 8, MariaDB, Aurora MySQL, TiDB). Adding Postgres would
double the maintenance surface for marginal additional reach.

If a deployment ever requires Postgres specifically (organizational
preference, compliance constraints, integration with existing
Postgres infrastructure), the implementation path is clear: the
trait surface is unchanged, the implementation differs in error
codes, schema syntax for a few features (e.g., `BYTEA` instead of
`BINARY`), and column type choices. The bulk of the work is
translating the existing SQL crate's logic to Postgres equivalents.

**How the door stays open.** The substrate trait surface doesn't
mention MySQL, sqlx, or any backend specifics. A Postgres backend
is a sibling crate to the MySQL backend, with no changes needed in
the trait crate or in consumers. If demand materializes, it can
happen as a standalone effort.

## Universal tombstone flag in entities

**What it would be.** A built-in `is_deleted` (or similar) scalar
on every entity, with substrate-level semantics: deleted entities
are filtered from queries, cascading deletions propagate through
references, etc.

**Why deferred.** Each entity kind has different deletion
semantics. Workflow templates can be retired (no new instances)
but historical instances should still resolve their template
reference. Workflow instances might be cancelled, archived, or
purged with different policies. A universal flag would impose one
deletion model on all kinds, which would be wrong for at least
some of them.

The current design has each kind define its own deletion scalar
(`is_retired` on templates, `status: Cancelled` on instances) with
the semantics that fit. Consumers query active entities by
filtering on these scalars; the substrate stays neutral.

**How the door stays open.** If a pattern emerges where many kinds
share the same deletion semantics, a convenience trait or shared
scalar declaration could be added at the workflow-layer or
cornerstone level — not in the substrate. The substrate's neutrality
is preserved by keeping deletion conventions at the consumer layer.

## Read replicas and connection routing

**What it would be.** First-class support for read replicas in
the SQL backend: reads route to replicas (with replication-lag
awareness), writes route to the primary, with session affinity for
read-your-own-writes.

**Why deferred.** The `ConnectionProvider` trait already supports
this pattern in principle: a custom provider could implement
`acquire_read` against a replica pool and `acquire_write` against
a primary pool. But implementing it correctly (especially the
session affinity required for the read-your-own-writes contract)
requires deployment-specific logic that the substrate can't
predict.

The default `SinglePool` provider routes both reads and writes
through one pool, which is correct for primary-only deployments.
Deployments that need read scaling can implement custom providers
when they need them.

**How the door stays open.** The trait split (`acquire_read` vs.
`acquire_write`) is the extension point. A future
`philharmonic-store-sqlx-mysql-replicated` crate could provide a
`ReplicatedPool: ConnectionProvider` implementation that handles
the routing. Or deployment-specific logic can implement the trait
inline in the application binary.

The substrate's neutrality means this is purely an extension
concern — no substrate changes needed.

## Pagination on substrate queries

**What it would be.** Substrate query methods that return paginated
results: `find_by_scalar_paginated(kind, attr, value, cursor,
limit)` returning a page plus a continuation cursor.

**Why deferred.** Current query methods return full vectors. This
works because current queries are scoped (find templates, find
instances for a tenant, find step records for an instance) and
return small result sets. Workloads that would produce
huge result sets (millions of step records, thousands of
templates) aren't current targets.

If admin tooling or analytics use cases ever produce queries
returning large result sets, pagination becomes important. The
right move at that point is new methods (`*_paginated` variants),
not retrofitting cursor parameters onto existing methods. The
narrow methods stay narrow.

**How the door stays open.** The substrate trait surface is
extensible. New methods can be added without breaking existing
ones; pagination support is a pure addition. Consumers that don't
need pagination continue using the simple methods; consumers that
need it use the paginated versions.

## Streaming results

**What it would be.** Query methods that return streams (`impl
Stream<Item = Result<EntityRow, StoreError>>`) rather than
vectors. Useful for processing large result sets without
materializing them in memory.

**Why deferred.** Same reasoning as pagination: result sets are
small enough to materialize. Streaming adds complexity (lifetime
management, backpressure, error handling mid-stream) without
solving a current problem.

**How the door stays open.** Same as pagination: new streaming
methods are pure additions. The trait surface accommodates them
when they're needed.

## Garbage collection of orphaned data

**What it would be.** Periodic cleanup of substrate data that no
longer has live references: identities minted but never used for
entities, content blobs not referenced by any revision, possibly
old revisions of entities marked for retention-period expiry.

**Why deferred.** The substrate is append-only by design, and
garbage collection is structurally at odds with append-only
semantics. Adding GC would require deciding what counts as
"orphaned" (the substrate doesn't track references in a way that
makes this query efficient), implementing safe collection (without
deleting data still in use), and handling the operational
complexity of a long-running collector process.

For current workloads, storage growth is acceptable. Reading old
data is rare; storage is cheap; the audit-trail benefit of
retaining everything is real.

**How the door stays open.** GC is fundamentally an operational
concern, not a substrate concern. Deployments that need it can
implement out-of-band collectors that scan the substrate and
delete what they identify as safe to remove. The substrate would
need to gain `delete_*` methods if it were to participate in GC,
but those methods aren't on the current roadmap and adding them
deserves serious design work (they'd be the first non-additive
methods in the substrate, breaking the append-only discipline).

## Cross-instance coordination primitives

**What it would be.** Workflow-engine support for "wait for
another instance to complete," "fan out to N child workflows,"
"join on the results of multiple workflows." Process-orchestration
primitives that span multiple instances.

**Why deferred.** Cross-instance coordination is workflow-pattern
territory, and patterns vary. Some systems want strict
parent-child relationships; others want event-bus coordination;
others want explicit dependency declaration. Designing one
coordination model speculatively would constrain consumers; making
it configurable adds engine complexity for unclear benefit.

The current design treats each instance as independent.
Coordination across instances, if needed, happens at the layer
above the engine: a parent workflow can call out to a connector
that creates and monitors child workflows; an external scheduler
can sequence workflows based on completion events.

**How the door stays open.** The substrate already supports
cross-instance entity references. A workflow can reference another
workflow via an entity-slot in its context (or in scalar
attributes). A future coordination layer or convention can use
these references without engine changes.

If coordination becomes a first-class concern, it probably
deserves its own crate (`philharmonic-orchestration` or similar)
that sits above the workflow engine and provides the primitives,
rather than baking them into the engine.

## What this list is not

**Not a roadmap.** The deferred features aren't promised to be
implemented in any particular order, or at all. Implementation
happens when use cases justify it, and use cases come from
real deployments. A feature on this list might never be built if
the need never materializes.

**Not a "nice to have" list.** Each entry was considered seriously
enough to think through the deferral and the future-proofing. The
features are real possibilities, not aspirational additions.

**Not a complete enumeration.** Other features will surely come up.
This list captures the ones that came up during initial design and
were explicitly deferred. New deferrals get added when they're
made.

**Not a substitute for issue tracking.** When someone wants a
specific deferred feature, that's the right time to file an issue
and gather requirements. This document explains why the feature
isn't in the system; an issue tracks the specific use case driving
the request.

## How to use this list

When evaluating philharmonic for a use case, this document
answers "does the system support X?" for cases where the answer is
"not currently, and here's why and how it could." If a feature you
need is here, you have three options:

**Wait for it.** If the use case is shared (e.g., many deployments
will eventually want determinism), the feature is more likely to
be implemented. Filing an issue with concrete requirements helps.

**Implement it yourself.** The future-proofing notes explain how
each feature could be added. For features that are pure additions
to existing crates, contributing the implementation is
straightforward. For features that need new crates (in-memory
backend, scheduler), the work is more substantial but well-scoped.

**Layer it above.** Many deferred features can be implemented as
wrapper crates that consume the existing system. Retry policy can
wrap the engine; cross-instance coordination can wrap the engine;
a Postgres backend is a sibling to the MySQL one. The layered
design makes this kind of extension natural.

If a feature you need *isn't* on this list and isn't in the
system, that's the most interesting case: it means the design
hasn't considered it. Filing an issue or starting a discussion is
the right move; it might lead to a deferral being documented here,
or to the feature being added.
