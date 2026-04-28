# Phase 8 sub-phase E endpoint-config audit

**Date:** 2026-04-28
**Prompt:** docs/codex-prompts/2026-04-28-0006-phase-8-sub-phase-e-endpoint-config.md
**Reviewed note:** docs/notes-to-humans/2026-04-28-0011-e-claude-code-review.md
**Reviewed implementation:** root commit `096ab2f`, `philharmonic-api` commit `84bee6f`

## Summary

The SCK encrypt/decrypt call sites themselves match the requested
binding shape: both pass tenant internal UUID, endpoint-config internal
UUID, and `key_version` into the `philharmonic-policy` SCK API. Decrypt
uses the revision's stored `key_version`, not the builder's current
version. The endpoint module does not log plaintext or ciphertext, and
metadata responses do not include the `encrypted_config` content hash or
config plaintext.

I found one correctness/security mismatch in the missing-SCK gate. The
Claude review note says "if SCK is not configured, all endpoint-config
routes return 500"; the prompt also says SCK is required for
endpoint-config routes and any hit without SCK should return 500
("SCK not configured"). The implementation only enforces that gate on
create, decrypted read, and rotate. List, metadata read, and retire still
execute without an SCK.

## Finding 1: missing-SCK gate is incomplete

Severity: medium.

Prompt requirement:

- `docs/codex-prompts/2026-04-28-0006-phase-8-sub-phase-e-endpoint-config.md:80`
  says SCK is required for endpoint-config routes.
- `docs/codex-prompts/2026-04-28-0006-phase-8-sub-phase-e-endpoint-config.md:88`
  says if SCK is absent and an endpoint-config route is hit, return 500
  with "SCK not configured".
- `docs/codex-prompts/2026-04-28-0006-phase-8-sub-phase-e-endpoint-config.md:114`
  repeats missing SCK -> 500.

Review note claim:

- `docs/notes-to-humans/2026-04-28-0011-e-claude-code-review.md:52`
  says all endpoint-config routes return 500 when SCK is not configured.

Implementation evidence:

- `philharmonic-api/src/routes/endpoints.rs:90` calls `require_sck`
  in `create_endpoint`.
- `philharmonic-api/src/routes/endpoints.rs:185` calls `require_sck`
  in `read_endpoint_decrypted`.
- `philharmonic-api/src/routes/endpoints.rs:216` calls `require_sck`
  in `rotate_endpoint`.
- `philharmonic-api/src/routes/endpoints.rs:132` starts
  `list_endpoints` and never calls `require_sck`.
- `philharmonic-api/src/routes/endpoints.rs:159` starts
  `read_endpoint_metadata` and never calls `require_sck`.
- `philharmonic-api/src/routes/endpoints.rs:273` starts
  `retire_endpoint` and never calls `require_sck`.

Impact:

An authorized caller can still enumerate endpoint-config metadata and
retire existing endpoint configs in a deployment whose API builder was
constructed without an SCK. That contradicts the intended "endpoint
config routes are unavailable unless SCK is configured" behavior. It
also means a deployment that omitted SCK because it "doesn't use endpoint
configs" can still expose metadata for any pre-existing configs and can
mutate retirement state if those routes are reachable and the caller has
the relevant permission.

Recommended fix:

Call `require_sck(&state)?` at the top of every endpoint-config handler,
including list, metadata read, and retire, before store reads or writes.
Expand `endpoint_routes_without_sck_return_internal_error` so it covers
all six routes, not only `POST /v1/endpoints`. For read/list/retire
coverage, seed a config directly into the mock store, build the router
without SCK, grant the matching permission, and assert 500 with
"SCK not configured".

## Crypto call-site checks that passed

- `encrypt_config` canonicalizes the request JSON and calls
  `sck_encrypt` with `(sck, canonical bytes, tenant internal UUID,
  endpoint-config internal UUID, current key_version)`. This matches the
  SCK AAD construction in `philharmonic-policy/src/sck.rs`.
- `read_endpoint_decrypted` reads `key_version` from the stored revision,
  loads the `encrypted_config` content, and calls `sck_decrypt` with the
  same tenant/config/key-version binding.
- Both SCK error paths collapse to generic 500 messages:
  "endpoint config encryption failed" and "endpoint config decryption
  failed".
- The endpoint module contains no `tracing::` calls, so it does not
  directly log plaintext or ciphertext.
- Metadata response construction reads `display_name`, revision numbers,
  timestamps, retirement status, and key version. It does not load or
  serialize the encrypted config content.

## Test coverage gap

`philharmonic-api/tests/endpoint_config.rs:514` defines
`endpoint_routes_without_sck_return_internal_error`, but that test only
exercises create with `endpoint:create`. It does not prove the
all-routes missing-SCK invariant stated in the prompt and review note.
