# Phase 8 B1 auth middleware audit

**Date:** 2026-04-28
**Prompt:** docs/codex-prompts/2026-04-28-0003-phase-8-sub-phase-b1-auth-middleware.md
**Claude review:** docs/notes-to-humans/2026-04-28-0007-b1-claude-code-review.md

## Scope

I reviewed the landed B1 commits in:

- `philharmonic-api` `0fd2bcd`
- `philharmonic-store` `3e0ab7c`
- `philharmonic-store-sqlx-mysql` `a88ccdb`

The review focused on authentication correctness, authority and tenant binding,
external error collapse, substrate lookup semantics, MySQL query behavior, and
test coverage. B1 is crypto-touching because it calls
`verify_ephemeral_api_token`; it does not add new cryptographic primitives.

## Verdict

The central B1 authentication path is directionally correct. I agree with
Claude that the implemented middleware performs the required authority lookup,
authority-tenant binding, authority-epoch comparison, retired checks, active
tenant checks, and generic external 401 collapse.

I found no direct external auth-detail leak in HTTP responses. I did find
several follow-up issues worth fixing or explicitly accepting before merge:
three medium operational/security hardening issues in the ephemeral-token input
and MySQL migration/testing surface, plus lower-risk type-shape and secret
lifetime concerns.

## Findings

### Medium: ephemeral bearer values are base64-decoded before any encoded-size cap

`philharmonic-api/src/middleware/auth.rs:162` decodes every non-`pht_` bearer
string with `URL_SAFE_NO_PAD.decode(token)` before calling
`verify_ephemeral_api_token`. The policy primitive enforces `MAX_TOKEN_BYTES`
only after it receives decoded COSE bytes (`philharmonic-policy/src/api_token.rs:558`).

That means the API middleware can allocate decoded bytes for an oversized
attacker-controlled Authorization value before the primitive's size check has a
chance to reject it. Many deployments will have HTTP header limits, but the
auth layer should not rely on ingress defaults for the token-format cap it
already has in the policy crate.

Recommendation: reject overlong ephemeral bearer strings before decoding. The
constant should be derived from `philharmonic_policy::MAX_TOKEN_BYTES` and the
chosen base64url-no-padding encoding. Add a negative test with an oversized
base64url-looking bearer and assert the generic 401 envelope.

### Medium: MySQL schema migration does not add the new index to existing tables

`philharmonic-store-sqlx-mysql/src/schema.rs:39-47` adds
`KEY ix_attr_content_hash (attribute_name, content_hash)` inline inside
`CREATE TABLE IF NOT EXISTS attribute_content`. For a database whose
`attribute_content` table already exists, `migrate()` will skip the table
definition and will not add the new index.

This does not change auth correctness, but it can turn long-lived token lookup
into an avoidable scan on existing deployments, because B1's hot path now
depends on `find_by_content`. The prompt asked for an index migration; the
current implementation only affects fresh schemas.

Recommendation: add an idempotent index creation step after table creation,
or handle duplicate-index errors around an `ALTER TABLE ... ADD INDEX`
statement if MySQL syntax support is limited. Keep the inline key for fresh
schemas if desired, but do not rely on it as the migration.

### Medium: MySQL `find_by_content` has no integration coverage

The production implementation is in
`philharmonic-store-sqlx-mysql/src/entity.rs:364-407`, but
`philharmonic-store-sqlx-mysql/tests/integration.rs` has no
`find_by_content` test. The only new store test I found is the mock-level
typed-wrapper delegation test in `philharmonic-store/src/entity.rs:728-760`.

This leaves the real SQL query unexercised for the exact lookup now used by
long-lived API-token authentication. The default MySQL crate test run also
skips all 28 testcontainer tests, so CI-style default tests do not exercise
this path.

Recommendation: add ignored MySQL integration tests for `find_by_content`:
happy path, wrong kind returns empty, latest-revision semantics reject a stale
credential hash from an older revision, and missing latest content attr returns
empty.

### Low: long-lived minting-authority auth is typed as `EntityId<Principal>`

The B1 prompt explicitly says minting authorities authenticate through
`AuthContext::Principal`, so this is not a contract violation. Still, the
implementation deserves a design follow-up.

`authenticate_long_lived` first looks for a `Principal`, then for a
`MintingAuthority` (`philharmonic-api/src/middleware/auth.rs:123-136`). Both
paths call `long_lived_principal_context`, which promotes the row identity to
`EntityId<Principal>` (`philharmonic-api/src/middleware/auth.rs:147-150`).
`Identity::typed::<T>` only validates UUID versions, not the substrate kind
(`philharmonic-types/src/entity.rs:158-169`), so a minting-authority UUID can
be wrapped as `EntityId<Principal>`.

That can surprise sub-phase C/G code if it later assumes
`AuthContext::Principal.principal_id` always names a row of kind `Principal`.
It may be better to introduce a persistent-principal enum or a distinct
minting-authority auth variant before downstream authorization depends on this
shape.

### Low: bearer secrets are copied into ordinary heap buffers

`bearer_token` returns an owned `String`
(`philharmonic-api/src/middleware/auth.rs:99-112`), so both long-lived and
ephemeral bearer values are copied out of the header into non-zeroizing heap
storage. The ephemeral path also stores decoded token bytes in a normal `Vec`
(`philharmonic-api/src/middleware/auth.rs:162-165`).

This is not a remote exploit by itself, and the header storage is already
outside the middleware's control. However, `philharmonic-policy` treats
generated long-lived tokens as `Zeroizing<String>`, which signals that token
material deserves tighter memory hygiene. The API boundary should avoid extra
owned copies where possible, or use `Zeroizing` for unavoidable owned bearer
material.

Recommendation: parse the header as a borrowed `&str` for the long-lived path,
and consider wrapping decoded ephemeral bytes in `Zeroizing<Vec<u8>>` after
verifying the policy API accepts a slice from that wrapper.

### Low: documentation drift remains after B1

`philharmonic-api/src/lib.rs:59-66` still describes the API crate in
"Sub-phase A scope" terms and says authentication/authorization is a
placeholder layer, even though authentication is now real and only authz is
placeholder. `philharmonic-api/README.md:9` also still says placeholder
auth/authz. Separately, `docs/design/10-api-layer.md:132` says ephemeral tokens
are presented as `Authorization: Bearer <COSE_Sign1>`, while the B1 prompt and
tests use base64url-encoded COSE bytes.

Recommendation: update the API crate docs and design text in the same review
cycle so future prompts do not inherit stale auth-state or token-encoding
language.

## Positive checks

- External auth failures from the middleware collapse to one HTTP 401 envelope
  via `unauthenticated_response` (`philharmonic-api/src/middleware/auth.rs:269-277`).
- The negative auth tests assert `code = unauthenticated`, `message = invalid
  token`, `details = null`, and no `kid`, `signature`, `expiry`, or `epoch`
  substrings in the response body
  (`philharmonic-api/tests/auth_middleware.rs:261-275`).
- Authority-tenant binding is present before epoch acceptance
  (`philharmonic-api/src/middleware/auth.rs:180-190`), matching the B1 handoff
  contract.
- Negative authority-tenant, epoch mismatch, and negative-epoch tests exist
  (`philharmonic-api/tests/auth_middleware.rs:453-508`).
- The long-lived credential lookup computes the content address of the
  `TokenHash` bytes, matching the prompt's storage model
  (`philharmonic-api/src/middleware/auth.rs:118-132`).
- The MySQL `find_by_content` query checks the matched content attribute on
  the latest revision sequence only
  (`philharmonic-store-sqlx-mysql/src/entity.rs:374-386`).

## Verification run

I ran:

- `./scripts/rust-lint.sh philharmonic-api` — clean.
- `./scripts/rust-lint.sh philharmonic-store` — clean.
- `./scripts/rust-lint.sh philharmonic-store-sqlx-mysql` — clean.
- `./scripts/rust-test.sh philharmonic-api` — 32 tests plus 1 doctest passed.
- `./scripts/rust-test.sh philharmonic-store` — 22 tests passed; 1 doctest ignored.
- `./scripts/rust-test.sh philharmonic-store-sqlx-mysql` — crate compiled; 0 unit tests ran; 28 integration tests and 5 doctests ignored as testcontainer/doc examples.

I did not run `--ignored` MySQL tests. The existing ignored suite does not
appear to include `find_by_content` coverage.
