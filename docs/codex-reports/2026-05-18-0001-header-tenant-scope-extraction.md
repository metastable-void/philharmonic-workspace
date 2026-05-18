# Header tenant scope resolver extraction

**Date:** 2026-05-18
**Prompt:** Direct Yuka request: plan and implement the non-crypto `scope.rs` Audit & refactor slice, then update this report with the landed changes.

## Plan

The behavior-preserving extraction is to move the API server's header-based tenant scope resolver out of the unpublished `philharmonic-api-server` bin and into `philharmonic-api` as a reusable generic helper over `philharmonic_store::IdentityStore`.

Implementation steps:

1. Add `HeaderTenantScopeResolver<S>` to `philharmonic-api/src/scope.rs`, beside `RequestScopeResolver`.
2. Keep the resolver generic over the supplied store and avoid adding `sqlx`, `philharmonic-store-sqlx-mysql`, or any deployment-database dependency to `philharmonic-api`.
3. Preserve the existing API server behavior exactly:
   - missing `x-tenant-id` header resolves to `RequestScope::Operator`;
   - invalid header bytes, invalid UUIDs, and unknown public IDs resolve to `ResolverError::Unscoped`;
   - identity-store errors and typed-ID conversion errors resolve to `ResolverError::Internal`;
   - the default header name remains `x-tenant-id`.
4. Re-export the helper from `philharmonic-api`, and update `philharmonic-api-server` to construct `SqlStore::from_pool(...)` locally before passing it to the generic resolver.
5. Delete the now-empty bin-local resolver module.
6. Add library-local unit tests with a small fake `IdentityStore`, covering the behavior bullets above.
7. Run the required workspace checks and record the results here.

This slice deliberately avoids `lowerer.rs` and `embed_job.rs`; those touch SCK / endpoint-payload handling and remain deferred for a crypto-review-aware Gate protocol slice.

## Landed Changes

The non-crypto extraction landed as planned.

- `philharmonic-api/src/scope.rs` now defines `HeaderTenantScopeResolver<S>` over `S: IdentityStore`.
- `philharmonic-api/src/lib.rs` re-exports `HeaderTenantScopeResolver` with the existing scope types.
- `bins/philharmonic-api-server/src/main.rs` now constructs `SqlStore::from_pool(...)` locally and passes it to `HeaderTenantScopeResolver::new(...)`.
- `bins/philharmonic-api-server/src/scope.rs` was deleted; the unpublished bin no longer owns request-scope lookup logic.
- Library-local unit tests cover operator fallback, invalid header bytes, invalid UUID, unknown public ID, store-error mapping, typed-identity error mapping, and successful tenant resolution.
- `docs/ROADMAP.md` now marks the `scope.rs` extraction done and records `lowerer.rs` / `embed_job.rs` as deferred out of K for a crypto-review-aware Gate protocol slice.

The behavior is intended to remain identical to the deleted bin-local resolver: the header is still `x-tenant-id`; missing headers still produce operator scope; malformed or unknown tenant IDs still produce `ResolverError::Unscoped`; store and typed-ID failures still produce `ResolverError::Internal`.

## Validation

Focused validation:

- `./scripts/rust-lint.sh philharmonic-api` — passed.
- `./scripts/rust-test.sh philharmonic-api` — passed.
- `./scripts/rust-lint.sh philharmonic-api-server` — passed.

Workspace/supporting checks:

- `./scripts/check-md-bloat.sh` — final total Markdown line count reported as `96267`; this report was `60` lines.
- `./scripts/tokei.sh` — final report was `984` files, `194008` total lines, `92718` code lines, `76331` comments/docs lines, and `24959` blanks.
- `./scripts/xtask.sh resource-pressure` before pre-landing — `cpu 0.3% | load1/cpus 0.06 | mem 47.21G/188.52G avail (25.0%) | swap 6.86G/141.39G used (4.9%)`.

Canonical validation:

- `./scripts/pre-landing.sh` initially reached workspace rustdoc and then failed on the known Cargo output filename collision between the `philharmonic-api` library docs and the `philharmonic-api-server` bin target named `philharmonic-api`; this same collision is already noted in `docs/codex-reports/2026-05-15-0002-doc-warnings-build-status.md`.
- `CARGO_BUILD_JOBS=1 ./scripts/pre-landing.sh` — passed. Serializing jobs avoided the rustdoc output race while keeping the canonical wrapper path. The sandbox also caused `rustup check` to print a non-fatal read-only `~/.rustup/tmp` temp-file error during `check-toolchain.sh`.

## Residuals

The remaining `docs/ROADMAP.md` §3.K candidate, `lowerer.rs` / parts of `embed_job.rs`, is still deferred because it touches SCK decrypt/encrypt and endpoint-payload handling and should be handled as its own crypto-review-aware Gate protocol slice.
