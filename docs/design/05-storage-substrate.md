# Storage Substrate

Two crates: `philharmonic-store` (trait definitions) and
`philharmonic-store-sqlx-mysql` (SQL implementation). Both published,
both in active use. The trait/impl split forces honesty about what's
actually backend-neutral.

## Three traits

### `ContentStore`

Bytes keyed by SHA-256 hash. Write-once, read-by-hash, idempotent
puts.

Methods:
- `put(&ContentValue)` — idempotent; same bytes twice is a no-op.
- `get(Sha256) -> Option<ContentValue>`.
- `exists(Sha256) -> bool`.

### `IdentityStore`

Minting and resolving `(internal: UUIDv7, public: UUIDv4)` pairs.
Append-only registry.

Methods:
- `mint() -> Identity` — generates a fresh pair.
- `resolve_public(Uuid) -> Option<Identity>`.
- `resolve_internal(Uuid) -> Option<Identity>`.

### `EntityStore`

Entities (kind UUID + identity) with append-only revision logs.

Methods:
- `create_entity(Identity, kind: Uuid)` — no `RevisionInput`;
  revision 0 is appended separately.
- `get_entity(Uuid) -> Option<EntityRow>`.
- `append_revision(entity_id, revision_seq, &RevisionInput)` —
  atomic multi-row insert.
- `get_revision(entity_id, revision_seq) -> Option<RevisionRow>`.
- `get_latest_revision(entity_id) -> Option<RevisionRow>`.
- `list_revisions_referencing(target, attr) -> Vec<RevisionRef>`.
- `find_by_scalar(kind, attr, value) -> Vec<EntityRow>`.

## Typed extension traits

`ContentStoreExt`, `IdentityStoreExt`, `EntityStoreExt` are blanket-
implemented over the base traits, providing typed methods that take
`EntityId<T>` and `T: Entity` parameters. Consumers get type safety
for free; backend implementors only implement the untyped base.

`StoreExt` is the umbrella trait for cross-concern conveniences like
`create_entity_minting::<T>()` (mints identity + creates entity in
one call).

## Error model

`StoreError` partitions by what the caller does:

- **Semantic violations** (not retryable): `KindMismatch`, `Decode`,
  `IdKind`, `IdentityKind`, `ScalarTypeMismatch`.
- **Concurrency outcomes** (retryable): `RevisionConflict`,
  `IdentityCollision`.
- **Missing references**: `EntityNotFound` (reads return
  `Option::None`; this error is for writes referencing non-existent
  entities).
- **Backend failures**: `Backend(BackendError)` with `retryable: bool`
  hint.

Method: `is_retryable()` gives a uniform check.

## Append-only and optimistic concurrency

No `update`, no `delete`, no `upsert`. Primary key on
`(entity_id, revision_seq)` in the revision table. Two writers
attempting the same `(entity_id, next_seq)` both try to insert; one
succeeds, the other gets `RevisionConflict`. Standard retry loop:

```rust
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

No transactions in the trait surface. `append_revision` is
internally atomic (entity existence check + revision row + attribute
rows in one DB transaction); no cross-method atomicity.

## SQL implementation details

`philharmonic-store-sqlx-mysql` uses sqlx 0.8 with `mysql` and
`runtime-tokio` features.

### Seven-table schema

1. `identity` — `(internal BINARY(16) PK, public BINARY(16) UNIQUE)`.
2. `content` — `(content_hash BINARY(32) PK, content_bytes MEDIUMBLOB)`.
3. `entity` — `(id BINARY(16) PK, kind BINARY(16), created_at BIGINT,
   KEY ix_entity_kind (kind))`.
4. `entity_revision` — `(entity_id BINARY(16), revision_seq BIGINT UNSIGNED,
   created_at BIGINT, PK (entity_id, revision_seq))`.
5. `attribute_content` — per-revision content-hash attrs.
6. `attribute_entity` — per-revision entity-ref attrs, with
   `target_revision_seq` nullable (NULL = track latest).
7. `attribute_scalar` — per-revision scalars with `value_kind`
   discriminator plus `value_bool` and `value_i64` columns.

LCD MySQL discipline: no JSON columns, no declared FKs, `BIGINT`
for timestamps, `BINARY` for hashes/UUIDs, InnoDB engine explicit.

### `ConnectionProvider` trait

Backend-specific abstraction in the SQL crate. `SqlStore` holds a
`ConnectionProvider` rather than a pool directly, enabling custom
routing (read replicas, sharded deployments).

Methods:
- `acquire_read(&self) -> Result<PoolConnection<MySql>, StoreError>`.
- `acquire_write(&self) -> Result<PoolConnection<MySql>, StoreError>`.

Default implementation `SinglePool` routes both through one pool.

Consistency contract: read-your-own-writes within a single
`SqlStore` instance. The optimistic-concurrency pattern depends on
this.

## Test infrastructure

28 integration tests via testcontainers MySQL 8, all passing.
Cleanly isolated per-test containers with a global async mutex
preventing parallel container startup contention.

## Status

Both crates published. The substrate is functionally complete;
future work is optional backends (in-memory for testing, possibly
Postgres) not changes to the trait surface.

## What the substrate doesn't know

The substrate stores bytes, hashes, UUIDs, and integers. It doesn't
know:

- Which entity kinds exist (`Entity::KIND` is just a UUID).
- What scalar attributes mean.
- What content blobs contain.
- About workflow lifecycle.
- About tenants or policy.
- About transactions across trait methods.

Application semantics live in application code. The substrate is
reusable beyond its current consumer; a policy system, an audit log,
a configuration system could all share the same substrate with their
own entity kinds.
