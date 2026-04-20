# Philharmonic — Design Principles

This document captures the design commitments shared across the system.
Each principle constrains decisions across multiple crates and is the
reason a particular kind of choice keeps coming up in the same way.
Subsequent documents reference these principles rather than restating
them.

## Append-only

Storage operations add data; they never modify or delete existing rows.
Every entity, every revision, every content blob, every identity row
exists from the moment it's written until the database is destroyed.

This is the system's most consequential commitment. It collapses or
eliminates several categories of complexity:

**Concurrency.** Two writers cannot conflict on the same row, because
no row is ever updated. The only conflict shape is two writers
attempting to insert the same primary key, which the database rejects
deterministically and the substrate translates to a semantic error
(`RevisionConflict`, `IdentityCollision`). No locks, no compare-and-swap,
no read-modify-write loops in the substrate's API.

**Caching.** Once a row is read, it can be cached forever — the data
will not change. The substrate doesn't provide caching itself, but
consumers that want to cache reads have a trivial correctness story.

**Auditability.** Every entity has a complete history of state
transitions. Workflow instances, in particular, carry their entire
lifecycle in the revision log. Diagnosis after the fact is reading the
log; there's no question of "what did this look like an hour ago"
because the answer is right there.

**Replication.** Append-only data is trivial to replicate. Logical
replication, snapshot-and-WAL replay, event-sourced rebuilds — all
work without coordination, because there's no in-place mutation to
coordinate.

**What's lost.** Storage usage grows monotonically. Soft-delete
semantics (retiring a workflow template, marking an instance as
cancelled) must be expressed as new revisions with status fields, not
as removals. Garbage collection of genuinely orphaned data (orphaned
identities, content not referenced by any revision) is an out-of-band
operational concern; the substrate doesn't perform it.

The trade is deliberate. The complexity that append-only eliminates is
the complexity that distributed databases spend most of their
engineering effort on. By refusing to mutate, the substrate makes its
own consistency story trivial and pushes operational concerns
(retention, GC) to where they belong (deployment-specific tooling).

## Content-addressed

Anything storable as bytes and identifiable by its content is keyed
by its SHA-256 hash. Two writers producing the same bytes produce the
same hash and dedupe naturally. References to content travel as hashes,
not as paths or names.

The substrate has a `ContentStore` for arbitrary bytes (typically
JSON-shaped, but the substrate doesn't care) and per-revision
content-hash attributes that point into it. Workflow scripts, workflow
configurations, workflow contexts, step inputs, and step outputs are
all content-addressed: they are stored once per unique value, and
revisions point at them by hash.

For JSON content specifically, canonicalization (RFC 8785, JCS) runs
before hashing so that semantically-equal JSON produces equal hashes.
`{"a":1,"b":2}` and `{"b":2,"a":1}` hash identically. This means
deduplication holds even when callers serialize JSON with different
key orderings.

The benefits cascade:

**Deduplication.** A workflow template's script is stored once,
regardless of how many templates use it. A workflow context unchanged
across two steps is stored once. Common shapes (empty objects, common
configurations) collapse to single rows.

**Integrity.** A reference and its target cannot mismatch. If a
revision says it points at hash `0xabc...`, the bytes at that hash
are the bytes the revision references, by mathematical definition.
There is no "stale pointer" failure mode.

**Caching.** Content keyed by immutable hash is trivially cacheable
at every layer (process memory, CDN, reverse proxy). The cached value
cannot become stale.

**Diff and comparison.** Two revisions referencing the same content
hash are known to have identical content without reading the bytes.
This makes "did anything change?" queries cheap.

The substrate doesn't compress content blobs (that's an engine-level
concern at the storage layer) and doesn't enforce a maximum size
beyond MySQL's `MEDIUMBLOB` limit (16 MB). Content blobs in this
system are typically small JSON documents and JS scripts; if a use
case ever needs multi-megabyte blobs, the contract is worth revisiting.

## Backend-agnostic interfaces

The storage substrate is defined as traits, not as a concrete
implementation. The `philharmonic-store` crate exports
`ContentStore`, `IdentityStore`, `EntityStore`; the
`philharmonic-store-sqlx-mysql` crate provides one implementation;
nothing in the trait surface mentions SQL, MySQL, sqlx, or any
specific database technology.

The same principle applies one layer up: the workflow orchestrator
will reach the executor through a `StepExecutor` trait, not through a
concrete HTTP client. Multiple implementations can coexist —
production HTTP, in-process for testing, mock for unit tests — and
consumers select the implementation at construction time.

This isn't about portability for its own sake. It's about keeping
abstractions honest. A trait that's "secretly" coupled to one
implementation produces specific kinds of bugs: tests that work
against the real implementation but not against mocks, methods whose
documentation mentions database error codes, traits that grow
methods only one backend can provide. By keeping the trait surface
backend-free, those couplings don't accumulate.

The construction site (typically the application binary) is where
concrete types come together: a `SqlStore` from one crate, an
`HttpStepExecutor` from another, a `WorkflowEngine` that takes both
generically. Each piece is testable in isolation; the wiring is
deployment-specific.

## Vocabulary collapses misuse paths

Types in the cornerstone are deliberately narrow. They expose what
the system needs and refuse to expose what would lead to bad designs,
even when the broader type would be technically useful.

The canonical example: `ScalarType` (the type of a scalar attribute
on an entity revision) has variants `Bool` and `I64` only. There is
no `Str` variant. Strings are not a scalar type in this system.

The reasoning isn't that strings are forbidden — content blobs hold
plenty of text. The reasoning is that *scalar* strings, sitting
alongside booleans and integers as a queryable column on an entity
revision, almost always indicate a misuse:

- Names, titles, descriptions belong in content blobs (where they
  benefit from deduplication and content-addressing).
- Status values belong in `i64` enum encodings (where the variants
  are defined in Rust and can't drift).
- Foreign keys belong in entity references (where the relationship
  is typed).
- Tags and categories belong in dedicated tables if they need
  querying, or in content blobs otherwise.

Every "but I have a string!" case turns out to be one of these in
disguise. Providing `ScalarType::Str` would invite each consumer to
reach for it without confronting which of the four right answers
their case actually is. The cornerstone refuses the invitation.

Same reasoning shows up elsewhere: `Identity` has exactly two UUIDs
(internal v7, public v4) — no third "alternative ID" variant. The
`Entity` trait declares slot collections as static slices — no
runtime registry that would invite "register your kind at startup"
patterns. `OpaqueError` in the executor contract is a string — no
structured error type that would invite the workflow layer to
introspect script errors and act on their internals.

The principle generalizes: when a vocabulary type would have
multiple plausible uses, but only some of them are appropriate, the
type should expose only the appropriate ones. The inappropriate
uses get pushed to other constructs that fit them better. The
substrate gets a smaller, more opinionated, harder-to-misuse API.

## LCD MySQL

The SQL implementation targets the lowest common denominator across
MySQL 8, MariaDB 10.5+, Amazon Aurora MySQL, and TiDB. Features
present in only some of those, or with subtle behavioral differences,
are avoided.

Concretely:

**No JSON columns.** MySQL's `JSON` type, MariaDB's `JSON` (which is
an alias for `LONGTEXT`), and TiDB's `JSON` have different storage,
different query semantics, and different index support. JSON content
is stored as `MEDIUMBLOB` and parsed in application code.

**No declared foreign keys.** TiDB's FK enforcement varies by version
and configuration. The substrate's writer enforces relational
integrity in code, declaring no FKs in the schema. This means a
deployment to a target that does enforce FKs gains nothing from the
substrate's writes (because the writer already maintains integrity)
but doesn't lose anything either.

**No vendor-specific operators.** `JSON_EXTRACT`, `MATCH AGAINST`,
window functions with vendor-specific syntax — all avoided. Queries
use standard SQL that all four targets accept.

**`BIGINT` for timestamps.** Stored as milliseconds since the Unix
epoch. The native `TIMESTAMP` and `DATETIME` types have subtle
differences across versions (timezone handling, range, precision).
`BIGINT` is unambiguous and the application converts at the boundary.

**`BINARY(16)` and `BINARY(32)` for UUIDs and hashes.** Compact, sortable,
and supported identically across all targets. UUID-as-string would
work but takes 2.25× the space and produces ambiguous comparisons
(case sensitivity, hyphen handling).

**`InnoDB` engine, explicitly specified.** Some deployments still
default to `MyISAM` or have unusual defaults; specifying `InnoDB`
guarantees row-level locking and transactional support that the
substrate's `append_revision` relies on.

The benefit is portability. A deployment can choose its
MySQL-compatible target based on operational needs (managed MySQL,
TiDB for horizontal scaling, MariaDB for licensing, Aurora for AWS
integration) without changing the application. The cost is that
features specific to one target — TiDB's distributed transactions,
MySQL's JSON path expressions, MariaDB's sequences — aren't
available to the substrate.

This trade has a self-reinforcing benefit: it forces the substrate's
SQL to be simple. Complex queries that depend on advanced features
get pushed up into application code, where they're easier to read,
test, and maintain. The schema stays understandable, the queries
stay grep-able, and "what does this index do" has obvious answers.

## Statelessness in execution

JavaScript workers maintain no state across jobs. Each job runs in an
isolated Boa realm; `globalThis` mutations don't persist; in-process
caches don't either. Workers are fungible: any worker can run any
job, no worker affinity is required for correctness.

This commitment shapes both the executor's API and the deployment
model. The executor takes everything it needs as job inputs (script,
arg, config) and returns everything it produces as job outputs
(JSON value or error string). It cannot accumulate state between
jobs because there's no place to put it.

Why:

**Horizontal scaling.** A worker fleet scales by adding workers.
Routing decisions don't depend on "where the relevant state is."
Load balancers can use simple algorithms (round-robin,
least-connections) without affinity hashing.

**Failure recovery.** A crashed worker loses nothing. The next
request goes to a different worker; the failed request is retried
or surfaced as an error to the caller. There's no recovery
protocol, no state replay, no warm-up.

**Determinism.** With no implicit state, a job's output depends only
on its inputs (and on host calls to non-deterministic services like
the clock and RNG, which are separate concerns covered in the
deferred-decisions document). This makes reasoning about jobs
local: read the script, read the inputs, predict the output. No
"what's in the cache" or "which worker has the warm runtime" to
account for.

**Testing.** Jobs are reproducible. Run the same job against the
same script with the same arg, get the same execution path. This
makes test fixtures stable and CI deterministic.

The cost is that any caching has to live outside the worker
process — in the orchestrator, in a shared cache (Redis, etc.), or
in the content store itself (which already deduplicates by hash).
For the workloads this system targets, the cost is small: scripts
are usually small, contexts are usually small, and the I/O cost of
fetching them per-job is dwarfed by the JS execution cost.

If a use case ever needs warm caches inside workers (large JIT'd
modules, expensive precomputation), the right move is a separate
caching layer that workers query, not stateful workers.

## Cornerstone as the workspace anchor

`philharmonic-types` is the source of truth for shared vocabulary.
Types that appear across crate boundaries (`Uuid`, `JsonValue`,
`Sha256`, the various ID types, `Entity` and its slot declarations)
live there. Other crates re-export the cornerstone's definitions
rather than depending on upstream sources directly.

This isn't about reducing dependency count. It's about preventing
version skew. If `philharmonic-store` depended on `uuid` directly
and `philharmonic-types` also depended on `uuid` directly, the two
could pin different major versions, and `Uuid` from one crate would
be a different type from `Uuid` in the other. Function signatures
written in terms of "the workspace's `Uuid`" would silently become
incompatible.

Pinning the cornerstone's version of upstream types and re-exporting
them ensures that everyone who follows the convention sees the same
types. The cornerstone takes one version per upstream crate; downstream
crates inherit it transitively. Version updates happen once, in the
cornerstone, and propagate via the dependency graph.

The convention applies whenever an upstream type appears in a
substrate-or-orchestration-layer API surface. Implementation details
that don't cross trait boundaries (like sqlx types inside the SQL
backend) don't need to flow through the cornerstone.

## Errors carry meaning, not stack traces

Error types in this system are designed to be matched on, not just
displayed. Each variant represents a specific failure mode that a
caller might want to react to differently.

In the storage substrate, `StoreError` distinguishes:

- Semantic violations (`KindMismatch`, `Decode`, `IdKind`,
  `IdentityKind`, `ScalarTypeMismatch`) — bugs or data corruption,
  not retryable.
- Concurrency outcomes (`RevisionConflict`, `IdentityCollision`) —
  expected races, retryable.
- Reference errors (`EntityNotFound`) — caller wrote against a
  parent that doesn't exist, indicates application logic error.
- Backend failures (`Backend(BackendError)`) — wrapped underlying
  errors, retryability indicated by a flag on the wrapper.

The variants partition the failure space by *what the caller should
do*, not by *what went wrong technically*. A consumer writing retry
logic can check `is_retryable` and act; a consumer writing diagnostic
logging can match on the specific variant for context-aware messages;
a consumer writing a high-level API can map variants to user-facing
status codes.

Errors do not carry stack traces. Rust's `std::error::Error::source`
chain is sufficient for tracing causes when needed; full backtraces
are an observability concern, attached at logging time by tools like
`tracing`, not baked into the error type.

The principle: errors are part of the API. They get the same design
attention as the success types. A poorly-designed error type forces
every consumer to parse strings or downcast, which propagates
fragility throughout the system. A well-designed error type makes
the right responses obvious at the call site.

## Ergonomic typed surfaces, object-safe base traits

The substrate traits (`ContentStore`, `IdentityStore`, `EntityStore`)
are object-safe: methods take untyped UUIDs and bytes, and
implementations can be held as `&dyn ContentStore`. This supports
runtime backend selection and easy mocking.

But raw UUIDs and bytes are an awkward API for application code that
knows the entity kinds it's working with. So each base trait has an
extension trait (`ContentStoreExt`, `IdentityStoreExt`,
`EntityStoreExt`) providing typed methods via blanket impls. A
consumer holding `impl EntityStore` automatically has
`EntityStoreExt`'s typed methods available; the typed methods take
`EntityId<T>` and `T: Entity` parameters and dispatch through to the
untyped base.

The pattern keeps both audiences happy: backend implementations need
implement only the untyped base trait (small surface, easy to get
right), and consumers get type-safe operations for free
(`get_entity_typed::<WorkflowInstance>(id)` returns either the
correctly-typed entity or a `KindMismatch` error).

This is also why `StoreExt` exists as an umbrella trait: convenience
methods that span multiple substrate concerns (`create_entity_minting`
mints an identity and creates an entity in one call) live there,
auto-implemented for any combination of base traits that supports
them.

The principle generalizes: when a trait has both ergonomic and
object-safety constraints that pull in opposite directions, split
them. The base trait optimizes for implementability; the extension
trait optimizes for callability. Blanket impls connect them.

## Layered ignorance

Each layer of the system is deliberately ignorant of layers above it.
The storage substrate doesn't know about workflows. The execution
substrate doesn't know about persistence. The cornerstone vocabulary
doesn't know about either substrate.

This isn't about hiding information for its own sake. It's about
preventing the wrong kinds of dependencies from forming. If the
storage substrate knew that workflow instances exist, someone would
eventually add a substrate method like `find_running_instances` that
uses substrate internals to optimize the query — and now the
substrate is coupled to workflow semantics, the workflow layer can't
swap in a different storage backend without re-implementing that
optimization, and the substrate's API has grown a method that only
makes sense for one consumer.

By keeping each layer ignorant of layers above, the failure mode
(adding inappropriate methods) becomes structurally impossible.
The substrate has no language to describe a "running instance," so
no substrate method can mention one.

This is the principle the document `06-boundaries.md` is dedicated to
making concrete. Each layer's "what it doesn't know about" gets
explicit articulation: the substrate doesn't know about workflow
state; the workflow doesn't know about HTTP transport; the executor
doesn't know about persistence. The boundaries are the layering, and
the layering is what makes the system maintainable.

## Defer until concrete

Many features are deliberately not in the system: tenant scoping,
read-replica routing, retry policies, multi-step orchestration,
universal tombstone flags, in-process caches, replay tapes for
deterministic re-execution. Each was considered and deferred.

The reasoning isn't that these features are bad. It's that designing
them well requires concrete use cases, and concrete use cases haven't
materialized yet. Designing speculatively produces APIs shaped by
imagined needs, which usually don't match real needs. When the real
needs appear, the speculative API has to be redesigned anyway, and
the original design becomes either dead code or a compatibility
burden.

The discipline: a feature lands when there's a real consumer that
needs it. Until then, the system is shaped to *not foreclose* the
feature (the substrate is append-only so replay is possible; the
executor is stateless so determinism is achievable; the storage
trait is generic so caching layers can wrap it) without *implementing*
the feature.

Document `07-deferred.md` enumerates the specific deferred features
and the future-proofing for each. The principle is to keep the
deferral list short by being conservative about additions, not by
hiding deferred features from view.
