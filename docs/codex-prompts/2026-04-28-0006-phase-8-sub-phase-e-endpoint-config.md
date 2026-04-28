# Phase 8 sub-phase E — endpoint config management (crypto-touching)

**Date:** 2026-04-28
**Slug:** `phase-8-sub-phase-e-endpoint-config`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Sub-phases A–D landed skeleton, auth, authz, and workflow
endpoints. **This dispatch adds endpoint-config CRUD** per
doc 10 §"Endpoint config management". It's the first
endpoint family that touches the substrate credential key
(SCK) — the API encrypts config blobs on submit and decrypts
on authorized read.

Crypto-touching: calls `sck_encrypt` and `sck_decrypt` from
`philharmonic-policy`. Code-review gate fires before merge.

## References

- [`docs/design/10-api-layer.md`](../design/10-api-layer.md)
  §"Endpoint config management" (lines 270-307).
- [`docs/design/11-security-and-cryptography.md`](../design/11-security-and-cryptography.md)
  §"Substrate at-rest endpoint config encryption" —
  SCK encrypt/decrypt flow.
- `philharmonic-policy` — `Sck`, `sck_encrypt(sck,
  plaintext, tenant_id, config_uuid, key_version)`,
  `sck_decrypt(sck, wire, tenant_id, config_uuid,
  key_version)`.
- `philharmonic-policy` — `TenantEndpointConfig` entity
  (content slot `encrypted_config`, scalar `is_retired`,
  scalar `key_version`, entity ref `tenant`).
- `philharmonic-api/src/routes/workflows.rs` —
  `validate_abstract_config` already references
  `TenantEndpointConfig`.
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md) — §10.3, §10.4,
  §11.

## Scope

### In scope

#### 1. Route module (`src/routes/endpoints.rs`)

6 endpoint handlers per doc 10:

- `POST /v1/endpoints` — create. Requires `endpoint:create`.
  Body: `{display_name, config}` where `config` is free-form
  JSON. Encrypt with SCK before storage. Returns 201 +
  endpoint UUID.
- `GET /v1/endpoints` — list (metadata only). Requires
  `endpoint:read_metadata`. Paginated. Returns display names,
  UUIDs, created_at, is_retired.
- `GET /v1/endpoints/{id}` — read metadata. Requires
  `endpoint:read_metadata`.
- `GET /v1/endpoints/{id}/decrypted` — read decrypted config.
  Requires `endpoint:read_decrypted`. Decrypts with SCK and
  returns the plaintext JSON blob.
- `POST /v1/endpoints/{id}/rotate` — new revision (new
  credentials, new config). Requires `endpoint:rotate`.
  Encrypts new blob, appends revision. Same UUID.
- `POST /v1/endpoints/{id}/retire` — retire. Requires
  `endpoint:retire`.

**No plaintext config in metadata reads.** Only the
`/decrypted` path returns the plaintext blob. Metadata reads
return `display_name`, `is_retired`, `key_version`, creation
times.

**No ciphertext in any response.** The raw `encrypted_config`
content blob is never returned to callers — either they get
the decrypted plaintext (via `/decrypted` with the right
permission) or they get metadata only.

**No ciphertext logged.** `tracing::warn!` and
`tracing::info!` calls in the endpoint handlers must never
include ciphertext or plaintext config bytes.

#### 2. SCK as a builder dependency

The builder gains an `sck: Option<Sck>` field. The `Sck` is
required for endpoint-config routes. Pass it via:
```rust
pub fn sck(mut self, sck: philharmonic_policy::Sck) -> Self
```

If absent and an endpoint-config route is hit, return 500
("SCK not configured"). This allows deployments that don't
use endpoint configs to omit the key.

**IMPORTANT:** `Sck` does not implement `Clone`. The builder
wraps it in `Arc<Sck>` for sharing across handlers.

#### 3. `key_version` handling

Each `TenantEndpointConfig` revision carries a `key_version`
scalar (`I64`). The builder also accepts a `key_version: i64`
indicating the deployment's current SCK version. On create /
rotate, the handler stores the current `key_version`. On
decrypt, the handler reads the stored `key_version` and
passes it to `sck_decrypt`.

For sub-phase E, the builder accepts `key_version` as a
simple `i64` parameter. Key rotation (re-encrypting existing
configs with a new SCK) is a deployment-ops procedure
documented in doc 11, not an API endpoint.

#### 4. Error handling

- `sck_encrypt` / `sck_decrypt` failures → 500 (internal
  error). **Do not leak the specific crypto error** to
  external responses.
- Missing SCK → 500 ("SCK not configured").
- Content/entity not found → 404.
- Wrong tenant → 404.
- Retired config → 400 (for rotate on a retired config).

#### 5. Tests (`tests/endpoint_config.rs`)

Using mock stores:

- Create endpoint → read metadata → read decrypted →
  assert plaintext matches.
- Create → rotate → read decrypted → assert new plaintext.
- Create → retire → assert is_retired.
- List endpoints → paginated response.
- Read decrypted without permission → 403.
- Metadata read does NOT contain plaintext or ciphertext.
- Cross-tenant read → 404.

### Out of scope

- **Principal/role/authority CRUD** — sub-phase F.
- **Token minting** — sub-phase G.
- **Audit + rate limit** — sub-phase H.
- **Key rotation re-encryption** — deployment ops.
- **Schema validation of config blob** — deferred per doc 10
  ("no schema validation on the blob contents in v1").
- **`cargo publish`** — sub-phase I.

## Workspace conventions

- **No panics in library `src/`** (§10.3).
- **Library takes bytes, not file paths** (§10.4) — the
  builder accepts `Sck` (which itself can be constructed
  from bytes); no file path in the API crate.
- **No `unsafe`**.
- **No ciphertext or plaintext in logs.**

## Pre-landing

```sh
./scripts/pre-landing.sh philharmonic-api
```

## Git

Do NOT commit, push, branch, tag, or publish.

## Verification loop

```sh
./scripts/pre-landing.sh philharmonic-api
cargo test -p philharmonic-api --all-targets
cargo doc -p philharmonic-api --no-deps
git -C philharmonic-api status --short
git -C . status --short
```

## Action safety

- Edits only in `philharmonic-api/` + `Cargo.lock`.
- No new crypto — calls existing SCK primitives only.

## Deliverables

1. `src/routes/endpoints.rs` — 6 endpoint handlers.
2. `src/routes/mod.rs` — wire endpoint routes.
3. `src/lib.rs` — builder gains `sck` + `key_version` deps.
4. `tests/endpoint_config.rs` — integration tests (7+).
5. `tests/common/mod.rs` — any mock extensions needed.

Working tree: dirty. Do not commit.

---

## Outcome

**Status:** Landed clean 2026-04-28. Awaiting Yuka crypto
call-site review.
**Claude review:** PASSES — see
[`docs/notes-to-humans/2026-04-28-0011-e-claude-code-review.md`](../notes-to-humans/2026-04-28-0011-e-claude-code-review.md).

Files: `src/routes/endpoints.rs` (624 lines, 6 handlers),
`src/routes/mod.rs`, `src/lib.rs` (builder gains sck +
key_version), `tests/endpoint_config.rs` (8 tests).

59 tests green. Clippy clean.
