# Sub-phase B1 — Claude assessment of Codex code-audit findings

**Author:** Claude Code · **Audience:** Yuka ·
**Date:** 2026-04-28 (Tue) JST afternoon

Codex audit report:
[`docs/codex-reports/2026-04-28-0003-phase-8-b1-auth-middleware-audit.md`](../codex-reports/2026-04-28-0003-phase-8-b1-auth-middleware-audit.md).
Five findings. Three fixed now; two deferred.

## Finding 1: Oversize bearer decoded before size cap

**Severity:** Medium. **Disposition:** Fixed.

`authenticate_ephemeral` base64-decoded the bearer string
before the B0 primitive's `MAX_TOKEN_BYTES` check. An
attacker-controlled oversized `Authorization` header would
allocate decoded bytes for no reason.

**Fix:** Added `MAX_BEARER_ENCODED_LEN = MAX_TOKEN_BYTES *
4 / 3 + 4` constant. Bearer strings exceeding this are
rejected with `AuthFailure::MalformedBearer` before any
base64 decode. The constant is a generous ceiling derived
from the primitive's already-pinned `MAX_TOKEN_BYTES` (16
KiB decoded → ~21 KiB encoded).

## Finding 2: MySQL index migration skips existing tables

**Severity:** Medium. **Disposition:** Fixed.

The `ix_attr_content_hash` index was inline in `CREATE
TABLE IF NOT EXISTS`, which only fires on fresh schemas.
Existing deployments upgrading from pre-B1 would miss the
index and hit full scans on `find_by_content`.

**Fix:** Added `INDEX_MIGRATIONS` array with a separate
`ALTER TABLE ... ADD INDEX` statement that runs after table
creation. The `migrate()` function now catches MySQL error
1061 ("Duplicate key name") for idempotency — fresh schemas
get the inline key from the `CREATE TABLE` and then silently
skip the `ALTER TABLE`; existing schemas get the index added
by the `ALTER TABLE`. Pattern is extensible for future
index migrations.

## Finding 3: MySQL find_by_content no integration test

**Severity:** Medium. **Disposition:** Deferred.

The MySQL `find_by_content` implementation has no
testcontainer integration test. The mock-level test covers
the typed wrapper delegation; the SQL mirrors the
already-tested `find_by_scalar` pattern.

**Why deferred:** Adding testcontainer integration tests
for the store crate is valuable but is a ~30 min piece of
work that doesn't block B1's correctness (the SQL is
structurally identical to `find_by_scalar` which IS
integration-tested). With the 5/2 target, I'm prioritizing
forward progress through C→H. The test should land before
Phase 8 close (sub-phase I), ideally alongside the other
store-level improvements that sub-phases D-F will introduce.

## Finding 4: Minting-authority typed as EntityId<Principal>

**Severity:** Low. **Disposition:** Deferred (design
follow-up).

The B1 prompt explicitly says minting authorities
authenticate as `AuthContext::Principal`. The type mismatch
(`EntityId<Principal>` wrapping a `MintingAuthority` UUID)
is a known compromise: `Identity::typed::<T>` validates UUID
versions but not entity kind.

Sub-phase G (token minting endpoint) is the first consumer
that will need to distinguish "is this principal actually a
minting authority?" — that's where the design decision
should land. Options: a `PersistentCaller` enum wrapping
either type, or an `AuthContext::MintingAuthority` variant.
Flagged for the G prompt.

## Finding 5: Bearer secrets in ordinary heap buffers

**Severity:** Low. **Disposition:** Deferred (hygiene
improvement).

The bearer string is already in the HTTP request's
header map (outside our control). The middleware copies it
into an owned `String` and then (for ephemeral) into a
`Vec<u8>`. Neither is `Zeroizing`.

This is a heap-residency hygiene concern, not a remote
exploit. The bearer's lifetime is bounded by the request
handler (dropped after the async handler returns). The
improvement would be to parse as `&str` (long-lived path)
and wrap decoded bytes in `Zeroizing<Vec<u8>>` (ephemeral
path). Low priority relative to forward progress.

## Finding 6: Documentation drift

**Severity:** Low. **Disposition:** Fixed.

- `src/lib.rs` §"Sub-phase A scope" renamed to §"Current
  scope (sub-phase B)" and updated to say auth is real,
  only authz is placeholder.
- `README.md` updated to match.
