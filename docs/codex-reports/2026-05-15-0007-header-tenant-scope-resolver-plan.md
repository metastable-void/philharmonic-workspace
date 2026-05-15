# Audit refactor: header tenant scope resolver plan

**Date:** 2026-05-15
**Prompt:** Direct follow-up to HUMANS.md §Priority: Audit & refactor; Yuka note: avoid unnecessary dependency-graph bloat.

The next non-crypto refactor slice I would choose is extracting the API server's header-based tenant scope resolver into `philharmonic-api` as a generic helper:

```rust
HeaderTenantScopeResolver<S: IdentityStore>
```

This should be a generic resolver over `philharmonic_store::IdentityStore`, not a MySQL- or sqlx-specific type. The important dependency rule is that `philharmonic-api` must not gain `philharmonic-store-sqlx-mysql`, `sqlx`, or any deployment-database dependency for this extraction. The API server can keep constructing `SqlStore::from_pool(pool)` locally and pass that store into the generic resolver.

The helper can live beside the existing `RequestScopeResolver` trait in `philharmonic-api/src/scope.rs` and use only dependencies that `philharmonic-api` already has: `http`, `philharmonic-policy`, `philharmonic-store`, and `philharmonic-types`. No new feature flag should be needed if the implementation stays within that existing dependency set. A feature would only be justified if the extraction introduced a dependency that many `philharmonic-api` consumers should be able to avoid.

The intended behavior should exactly match `bins/philharmonic-api-server/src/scope.rs` today:

- missing tenant header resolves to `RequestScope::Operator`;
- invalid header bytes, invalid UUIDs, and unknown public IDs resolve to `ResolverError::Unscoped`;
- identity-store errors and typed-ID conversion errors resolve to `ResolverError::Internal`;
- the default header name remains `x-tenant-id`.

The bin-side result should be small: delete or reduce the local `scope.rs`, instantiate `HeaderTenantScopeResolver::new(SqlStore::from_pool(pool))`, and leave all MySQL/sqlx ownership in the API server crate. This continues the thin-bin direction without broadening the public API crate's dependency graph.

Recommended tests for the extraction are library-local unit tests using a tiny fake `IdentityStore`, not MySQL integration tests. Cover the operator fallback, invalid UUID, missing identity, successful tenant resolution, and store-error-to-internal mapping. Existing API server tests should remain the regression check for deployment wiring.
