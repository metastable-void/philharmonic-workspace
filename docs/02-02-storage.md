# Philharmonic — Storage Substrate

This document covers the storage substrate: the trait crate
`philharmonic-store`, the canonical implementation
`philharmonic-store-sqlx-mysql`, and the shape of the data they manage.
The substrate is what every persistent thing in the system sits on.

## What the substrate is

Three concerns, three traits. Each handles one aspect of persistence
with a minimal API surface; together they support the entity-centric
data model the rest of the system uses.

**`ContentStore`** — bytes keyed by SHA-256 hash. Write-once,
read-by-hash, idempotent puts. The simplest of the three.

**`IdentityStore`** — minting and resolving `(internal: UUIDv7,
public: UUIDv4)` pairs. Append-only registry of identities that
entities will hang off of.

**`EntityStore`** — entities (typed by a kind UUID) with append-only
revision logs. Each revision carries content-hash references,
entity-reference attributes, and small typed scalar attributes. The
substrate's most substantial trait.

These three sit alongside each other; none depends on the others at
the trait level. Implementations typically provide all three (the
SQL backend does), but consumers can hold and use them independently.

## Why three traits, not one

The simplest possible substrate would have one trait: "store and
retrieve typed entities." But the three concerns are genuinely
independent in the domain.

A workflow template's script is a content blob — bytes, hashed,
stored, fetched. It has no identity in the entity sense; it's not
a thing with a lifecycle, just a value that exists. Content-addressing
gives it deduplication for free: a script shared across templates
exists once.

A workflow instance's identity is minted before any of its data
exists. The identity is a long-lived handle that external systems
reference (the public UUID is the API-facing identifier; the internal
UUID is what the database joins on). The identity persists even if
the entity it identifies is somehow garbage-collected.

A workflow instance itself is an entity: a kind, a creation
timestamp, and an append-only log of revisions, each carrying the
state at that point in the instance's lifecycle.

Collapsing these into one trait would obscure the distinctions.
Content would gain implicit identity it doesn't need; identity would
gain entity machinery it doesn't use; entities would have to know
about content addressing internally rather than referencing it.
Three traits keep each concern's surface area focused.

## Append-only and what it implies

The append-only property is established in the principles document;
this section covers its concrete implications for the substrate's API.

Every write either creates a new row or no-ops. There is no `update`,
no `delete`, no `upsert`. The API surface reflects this: methods are
named `put`, `mint`, `create_entity`, `append_revision` — all
additive verbs. There is no `update_entity`, `set_revision_attr`, or
similar.

This means the substrate has no transaction API at the trait level.
Each write method is independently atomic at the backend's
single-statement-or-single-transaction level, and there's no need for
the trait to expose `begin`/`commit`. Backend implementations may
use transactions internally for multi-row writes (the SQL backend's
`append_revision` uses one to insert the revision row plus all its
attribute rows atomically), but this is an implementation detail.

Consumers wanting cross-method atomicity (mint identity, then create
entity, then append first revision, all-or-nothing) cannot get it
from the substrate. The trade is intentional: the orphan-identity
case (mint succeeds, create fails, identity row exists with no
entity referencing it) is acknowledged as harmless. Cleaning up
orphans is an operational concern, handled out-of-band, not at the
substrate's API.

The append-only property also dictates the concurrency model. Two
writers cannot conflict on an existing row because no row is ever
modified. The only conflict shape is two writers attempting to
insert the same primary key, which the database rejects and the
substrate translates to a semantic error variant
(`RevisionConflict`, `IdentityCollision`). This is what makes
optimistic concurrency on revision sequences work: writers compute
`next_seq = latest_seq + 1`, attempt the insert, and retry on
conflict. No locks, no compare-and-swap, no read-modify-write.

## Object-safe traits, typed extension traits

The base traits (`ContentStore`, `IdentityStore`, `EntityStore`) are
object-safe: their methods take untyped UUIDs and bytes, and
implementations can be held as `&dyn ContentStore`. This supports
runtime backend selection (a deployment chooses its store
implementation at startup) and easy mocking (tests can use trivial
mock implementations without compile-time gymnastics).

But raw UUIDs and bytes are an awkward API for application code that
knows the entity kinds it's working with. So each base trait has an
extension trait — `ContentStoreExt`, `IdentityStoreExt`,
`EntityStoreExt` — providing typed methods via blanket impls. A
consumer holding `impl EntityStore` automatically has the
`EntityStoreExt` typed methods available; the typed methods take
`EntityId<T>` and `T: Entity` parameters and dispatch through to the
untyped base.

The pattern keeps both audiences happy. Backend implementations
implement only the untyped base trait (small surface, easy to get
right). Consumers get type-safe operations through the extension
traits (`get_entity_typed::<WorkflowInstance>(id)` returns either
the correctly-typed entity or a `KindMismatch` error).

The `StoreExt` umbrella trait adds cross-concern conveniences. It's
the right home for methods that span multiple substrate concerns —
`create_entity_minting<T>()` mints an identity and creates an entity
in one call, requiring both `IdentityStore` and `EntityStore`. The
umbrella trait is auto-implemented for any type that implements both
required base traits.

This three-tier shape (object-safe base, typed extension, umbrella
convenience) is the substrate's pattern for balancing implementability
against ergonomics. New cross-concern conveniences land on `StoreExt`
when the same multi-step pattern keeps showing up; new typed
ergonomics land on the per-concern extension traits.

## The error model

`StoreError` partitions the failure space by what the caller should
do, not by what went wrong technically. Three groups:

**Semantic violations.** `KindMismatch`, `Decode`, `IdKind`,
`IdentityKind`, `ScalarTypeMismatch`. The data on disk doesn't match
what the caller expected. Indicates a bug at the call site (asking
for the wrong type), schema drift (data written by an incompatible
version), or corruption. Not retryable; the caller should treat
these as evidence of a problem to investigate.

**Concurrency outcomes.** `RevisionConflict`, `IdentityCollision`.
The requested operation lost a race. The caller should re-read
state and retry with adjusted parameters (incremented revision
sequence, freshly-minted identity). These are expected outcomes in
a concurrent system, not errors per se — the variant just makes the
race outcome visible to the caller.

**Backend failures.** `Backend(BackendError)`. The storage backend
reported an error that doesn't map to a substrate-level semantic
(database unreachable, deadlock, schema constraint other than the
ones the substrate enumerates). The wrapped `BackendError` carries
a human-readable message and a `retryable: bool` flag set by the
backend's translator.

Plus one outlier: `EntityNotFound`. Returned by writes that require
a parent entity to exist (e.g., `append_revision` against an
entity that was never created). This is distinct from "row not
found on read" — read methods return `Option<T>` for that, because
absence is a normal outcome callers handle via pattern-matching, not
an exception.

The `is_retryable()` method on `StoreError` returns the
classification: concurrency outcomes are retryable, backend errors
defer to their internal flag, semantic violations and missing
entities are not retryable. Callers writing retry loops use this as
a single uniform check rather than matching every variant.

What's deliberately absent: stack traces, structured failure
contexts beyond the variants, and serialization derives. Errors are
matched on by application code; they're not transmitted over the
wire as part of public APIs.

## The connection-provider abstraction

The SQL backend doesn't hold a `MySqlPool` directly. It holds a
`ConnectionProvider`, which is a trait abstracting connection
acquisition. The default implementation (`SinglePool`) wraps a
single pool and routes both reads and writes through it; a custom
implementation can route differently.

The trait has two methods:

```
async fn acquire_read(&self) -> Result<PoolConnection<MySql>, StoreError>;
async fn acquire_write(&self) -> Result<PoolConnection<MySql>, StoreError>;
```

The split exists so that future deployments can route reads and
writes differently — a deployment with a primary-replica setup
might send writes to the primary and reads to a replica pool, with
session affinity guaranteeing read-your-own-writes within a single
`SqlStore` instance.

The trait is a backend-specific abstraction — it lives in the SQL
backend crate, not in the trait crate, and it returns sqlx
connection types. A future Postgres backend would have its own
provider abstraction, similarly shaped but using Postgres types.
The substrate trait crate doesn't know about connection providers
at all; the abstraction exists at the backend layer where it
matters.

The consistency contract that providers must honor: read-your-own-writes
within a single store instance. If a write commits a row, a
subsequent read on the same store must observe it. The substrate's
optimistic-concurrency pattern depends on this — without it, a writer
could compute `next_seq = latest_seq + 1` from a stale read and
insert what looks like a fresh sequence but actually conflicts with
a recent write. Documented in the trait's doc comment because it's
a property the trait can't enforce structurally.

The default `SinglePool` satisfies the contract trivially. Custom
providers using replication need to handle it via session affinity
or by routing follow-up reads to the writer connection.

## The schema

Seven tables, designed for the data model and for the LCD MySQL
discipline.

**`identity`** — `(internal BINARY(16) PK, public BINARY(16) UNIQUE)`.
The identity registry. Internal IDs are UUIDv7 stored as bytes;
public IDs are UUIDv4 stored as bytes. The unique constraint on
`public` lets `resolve_public` use an index lookup.

**`content`** — `(content_hash BINARY(32) PK, content_bytes MEDIUMBLOB)`.
The content store. SHA-256 hash as primary key; bytes as
`MEDIUMBLOB` (16 MB max), which is generous for the JSON documents
and JS scripts the system stores.

**`entity`** — `(id BINARY(16) PK, kind BINARY(16), created_at BIGINT,
KEY ix_entity_kind (kind))`. The entity registry. The `kind` index
supports queries like "find all entities of kind X."

**`entity_revision`** — `(entity_id BINARY(16), revision_seq
BIGINT UNSIGNED, created_at BIGINT, PK (entity_id, revision_seq))`.
The revision log. Compound primary key gives optimistic concurrency:
two writers attempting to insert revision N+1 hit the constraint;
exactly one succeeds.

**`attribute_content`**, **`attribute_entity`**, **`attribute_scalar`**
— per-revision attribute tables, each with PK
`(entity_id, revision_seq, attribute_name)`. Schema details:

- `attribute_content` adds `content_hash BINARY(32)`.
- `attribute_entity` adds `target_entity_id BINARY(16)` and
  `target_revision_seq BIGINT UNSIGNED NULL` (NULL means "track
  latest"; non-NULL means pinned to that revision). Has a secondary
  index `ix_target (target_entity_id, attribute_name)` for
  reverse-lookup queries ("which revisions reference this entity via
  this attribute").
- `attribute_scalar` adds `value_kind TINYINT UNSIGNED`,
  `value_bool TINYINT UNSIGNED NULL`, `value_i64 BIGINT NULL`. The
  discriminator (`value_kind`) plus type-specific value columns
  represents the `ScalarValue` enum on disk. Secondary indexes
  `ix_attr_scalar_bool` and `ix_attr_scalar_i64` support `find_by_scalar`
  queries efficiently.

This shape — entity registry plus revision log plus per-revision
attribute tables, organized by attribute kind — is an EAV
(entity-attribute-value) variant. The justification for the EAV
shape over a per-kind table approach is that the substrate doesn't
know about specific entity kinds; it can't define columns for slots
it doesn't know exist. The EAV shape lets new entity kinds appear
without schema migration, at the cost of slightly more complex
queries when assembling a revision's attributes.

The cost is paid at read time (`get_revision` issues four queries:
header plus three attribute tables) and on the index size (each
attribute is its own row). The benefit is that adding new entity
kinds is a code-only operation; the database doesn't need to know
about them.

**No declared foreign keys.** Per the LCD discipline: TiDB's FK
support varies. The writer enforces the conceptual relationships
(entity exists before revisions reference it, content exists before
attributes reference it) in code. Backends that want FK declarations
can add them as ALTER TABLE statements after the substrate's
migration runs; the substrate doesn't depend on them.

## Read patterns

The substrate's read API is shaped for the patterns the workflow
layer actually uses, plus a few admin-shaped queries.

**Look up an entity by internal ID** — `get_entity(uuid)`. Returns
`EntityRow` with kind and creation time. Used when a caller has an
entity ID and needs basic metadata.

**Look up an identity** — `resolve_public(uuid)` and
`resolve_internal(uuid)`. Returns `Identity` if found. Used at API
boundaries (resolve a public UUID to an internal ID for further
queries) and in reverse-mapping (resolve an internal ID to its
public counterpart for rendering).

**Read a specific revision** — `get_revision(entity_id, revision_seq)`.
Returns `RevisionRow` with all three attribute maps. Used for
historical queries ("what did this look like at revision 5") and
for reading specific revisions referenced from elsewhere.

**Read the latest revision** — `get_latest_revision(entity_id)`.
Convenience for the common case of "what's the current state."
Internally fetches the maximum revision sequence and then assembles
the row.

**Reverse-lookup by entity reference** — `list_revisions_referencing(target, attr)`.
Returns the list of revisions that reference the target entity via
the named attribute. Used for queries like "what workflow templates
reference this configuration" or "what step records belong to this
instance."

**Find by scalar value** — `find_by_scalar(kind, attr, value)`.
Returns entities of the given kind whose latest revision has the
given scalar value. Used for queries like "find active templates"
(`is_retired = false`) or "find failed instances" (`status = 3`).

What's deliberately absent: pagination (queries return full vectors;
result sets are expected to be small enough to fit in memory),
streaming (same reason), full-text search (not the substrate's
job), and complex multi-attribute queries (composite filters are
done in application code by combining narrower queries).

If a use case ever needs pagination — admin views over millions of
instances, perhaps — the right move is to add explicit paginated
methods (`find_by_scalar_paginated`) rather than retrofitting
paginate-or-not flags onto the existing methods. The narrow
methods stay narrow.

## Write patterns

Three flavors of write, each independently atomic.

**Content puts** — `put(content_value)`. Idempotent: same bytes
twice is a no-op (`INSERT IGNORE`). The atomicity is per-statement;
no transaction needed.

**Identity mints** — `mint()` returns a fresh `Identity`. Generates
UUIDs in Rust, inserts the pair, returns. Atomic per-statement.
Translates duplicate-key errors (effectively impossible) to
`IdentityCollision`.

**Entity creates** — `create_entity(identity, kind)`. Inserts a
single row in the `entity` table. The identity must have been
minted first; the substrate doesn't enforce this via FK
(per LCD discipline) but does check via `EntityNotFound` on
subsequent operations that depend on entity existence.

**Revision appends** — `append_revision(entity_id, seq, input)`.
The only multi-row write. Inside a backend transaction:

1. Verify the entity exists. Returns `EntityNotFound` if not.
2. Insert the revision-log row. Returns `RevisionConflict` if the
   `(entity_id, seq)` PK collides.
3. Insert one row per attribute in the input, across the three
   attribute tables.
4. Commit.

Translation: any duplicate-key error on step 2 becomes
`RevisionConflict`. Errors on attribute inserts (which shouldn't
happen if the input is well-formed and the revision-log insert
succeeded) become `Backend` errors.

The optimistic-concurrency loop a caller writes around this:

```
loop {
    let latest = store.get_latest_revision(entity_id).await?;
    let next_seq = latest.map(|r| r.revision_seq + 1).unwrap_or(0);
    let input = build_revision_input(...);
    match store.append_revision(entity_id, next_seq, &input).await {
        Ok(()) => break,
        Err(StoreError::RevisionConflict { .. }) => continue,
        Err(other) => return Err(other),
    }
}
```

The substrate doesn't provide this loop as a method because the
caller usually has logic between the read and the write (computing
the new state, validating preconditions) that can't be hidden inside
a generic helper. The pattern is short enough to write inline.

## Backend implementations

The trait crate (`philharmonic-store`) defines the interface. The
SQL crate (`philharmonic-store-sqlx-mysql`) implements it. Other
backends would be sibling crates implementing the same traits.

Currently planned but not yet implemented:

**`philharmonic-store-mem`** — in-memory backend for testing and
local development. `Arc<Mutex<HashMap>>` based, no external
dependencies, useful for downstream crates that want to test
without spinning up MySQL.

Possible but not committed:

**`philharmonic-store-sqlx-pgsql`** — Postgres backend. Would have
its own LCD discipline (Postgres-compatible features only). The
trait surface is unchanged; the implementation differs in error
codes, SQL syntax for a few features, and column type choices.

The substrate's value depends on the trait surface staying
backend-neutral. Each new backend forces the question "does this
change need to be on every backend, or only this one?" — and the
answer should usually be "only this one." Backend-specific behavior
that consumers care about (caching, replication, sharding) lives
inside the backend crate's `ConnectionProvider` implementations,
not in the substrate trait surface.

## What the substrate doesn't know

The substrate stores bytes, hashes, UUIDs, and integers. It doesn't
know what those values *mean*.

It doesn't know which entity kinds exist. The `Entity::KIND` UUID
is just a 128-bit number to the substrate; the trait crate has no
list of valid kinds. Backends store whatever kind UUID they're
given.

It doesn't know what scalar attributes mean. `is_retired`,
`status`, `priority` — these are application concepts. The substrate
sees `(attribute_name: "is_retired", value_kind: 0, value_bool: 1)`
and stores it.

It doesn't know what content blobs contain. They're bytes. The
substrate doesn't validate that they're JSON, doesn't parse them,
doesn't enforce a schema.

It doesn't know about workflow lifecycle. Status transitions,
terminal states, retry policies — none of these are substrate
concepts. The substrate provides storage; the workflow layer
provides semantics.

It doesn't know about tenants, principals, or permissions. If
those concepts are added later (in `philharmonic-policy`), they
become entity kinds and scalar attributes, modeled in the same
shape as everything else.

It doesn't know about transactions across operations. Each method
is independently atomic; cross-method atomicity is impossible at
the trait level and intentionally so.

The discipline is: anything the substrate would need application
context to understand, the substrate refuses to understand.
Application context lives in application code, layered on top of
the substrate's neutral storage.

This is what makes the substrate reusable. A workflow system, a
policy system, a configuration system, an audit log — all could
sit on the same substrate, each defining their own entity kinds
and attributes, none requiring substrate changes for their own
features. The substrate is the durable layer that doesn't change
when the layers above it do.
