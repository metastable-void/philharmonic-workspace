# Phase 9 — full-pipeline e2e test (API → lowerer → connector service)

**Date:** 2026-04-30
**Slug:** `phase-9-e2e-full-pipeline`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

The e2e_mysql tests exercise CRUD + auth against a real MySQL
database, but use `StubExecutor`/`StubLowerer`. The real
`ConnectorConfigLowerer` and `HttpStepExecutor` were just
landed. We need a test that proves the full crypto pipeline
works: API lowerer encrypts → executor POSTs → connector
service verifies + decrypts → impl dispatches → response
flows back.

## References

- `philharmonic-api/tests/e2e_mysql.rs` — existing e2e pattern.
- `philharmonic/src/bin/philharmonic_api/lowerer.rs` —
  `ConnectorConfigLowerer`.
- `philharmonic/src/bin/philharmonic_api/executor.rs` —
  `HttpStepExecutor`.
- `philharmonic/src/bin/philharmonic_connector/main.rs` —
  connector service HTTP handler pattern.
- `philharmonic-connector-client/tests/encryption_vectors.rs`
  — test key generation patterns.
- `philharmonic-connector-service/src/verify.rs` —
  `verify_and_decrypt`.

## Scope

### In scope

Create `philharmonic-api/tests/e2e_full_pipeline.rs`. This
test spins up BOTH the API server and a connector service,
with matching cryptographic key material, and exercises a
step execution that flows through the real crypto pipeline.

#### Test setup

1. **MySQL testcontainer** (same pattern as e2e_mysql.rs).
2. **Generate test key material**:
   - Ed25519 keypair for the lowerer signing key
     (`ed25519_dalek::SigningKey::generate(&mut OsRng)`).
   - ML-KEM-768 keypair
     (`MlKem768::generate(&mut OsRng)` → `(dk, ek)`).
   - X25519 keypair
     (`StaticSecret::random_from_rng(OsRng)` → secret,
     `PublicKey::from(&secret)` → public).
3. **Start the connector service** (in-process, NOT the bin):
   - Build a minimal axum router that mimics
     `philharmonic-connector`'s `POST /` handler:
     extract `Authorization` header + `X-Encrypted-Payload`
     header, call `verify_and_decrypt(...)`, look up the impl,
     call `impl.execute(...)`, return the JSON response.
   - Register `VectorSearch::new()` as the only impl.
   - Set up `MintingKeyRegistry` with the Ed25519 verifying
     key.
   - Set up `RealmPrivateKeyRegistry` with the ML-KEM
     decapsulation key + X25519 static secret.
   - Bind to `127.0.0.1:0`.
4. **Start the API server** (same pattern as e2e_mysql.rs but
   with real lowerer + executor):
   - `ConnectorConfigLowerer` with the Ed25519 signing key +
     realm public key (ML-KEM encapsulation key + X25519
     public key).
   - `HttpStepExecutor` pointing at the connector service's
     address.
   - `PhilharmonicApiBuilder` with real store + real lowerer +
     real executor.
   - Bind to `127.0.0.1:0`.

#### Test case

One test: `full_pipeline_step_execution`

1. Seed tenant, principal with token + role (same as
   e2e_mysql.rs).
2. Create a workflow template whose abstract config contains:
   ```json
   {
     "realm": "test-realm",
     "impl": "vector_search",
     "config_uuid": "<some-uuid>",
     "config": { "documents": [...], "top_k": 2 }
   }
   ```
3. Create a workflow instance from the template.
4. Execute a step: the workflow engine calls
   `ConnectorConfigLowerer::lower()` to produce
   `{token, encrypted_payload}`, then `HttpStepExecutor`
   POSTs to the connector service.
5. The connector service verifies the COSE_Sign1 token,
   decrypts the COSE_Encrypt0 payload, dispatches to
   `VectorSearch::execute()`, and returns the result.
6. Assert the response contains vector search results.

This proves the full crypto round-trip: Ed25519 sign →
verify, ML-KEM-768 + X25519 + HKDF → AES-256-GCM encrypt →
decrypt.

#### Dependencies

Add to `philharmonic-api/Cargo.toml` `[dev-dependencies]`:
```toml
ed25519-dalek = { version = "2", features = ["rand_core"] }
ml-kem = "0.2"
x25519-dalek = { version = "2", features = ["static_secrets"] }
rand_core = { version = "0.6", features = ["getrandom"] }
philharmonic-connector-service = "0.1.0"
philharmonic-connector-client = "0.1.0"
philharmonic-connector-common = "0.2.0"
philharmonic-connector-impl-vector-search = "0.1.0"
philharmonic-connector-impl-api = "0.1.0"
axum = "0.8"
hex = "0.4"
coset = "0.4"
```

Check which are already present before adding.

### Out of scope

- Testing all connector impls (just vector_search for now).
- Browser/WebUI testing.
- Multi-realm routing.

## Outcome

Pending — will be updated after Codex run.

---

## Prompt (verbatim)

<task>
You are working inside the `philharmonic-workspace` Cargo workspace.
Your target crate is `philharmonic-api` at `philharmonic-api/`.

## IMPORTANT: Do NOT commit or push

**Do NOT run `./scripts/commit-all.sh`.** Do NOT run any git
commit command. Do NOT run `./scripts/push-all.sh`. Leave the
working tree dirty.

## What to build

Create `philharmonic-api/tests/e2e_full_pipeline.rs` — an
end-to-end test that exercises the complete crypto pipeline:
API server with real `ConnectorConfigLowerer` +
`HttpStepExecutor` → connector service with real
`verify_and_decrypt` + `VectorSearch` impl.

**Read these files first:**

- `philharmonic-api/tests/e2e_mysql.rs` — existing e2e test
  pattern (testcontainer setup, auth seeding, API builder).
  Follow this pattern for MySQL + auth setup.
- `philharmonic-api/tests/common/mod.rs` — test helpers.
- `philharmonic/src/bin/philharmonic_api/lowerer.rs` —
  `ConnectorConfigLowerer` constructor and `lower()` impl.
- `philharmonic/src/bin/philharmonic_api/executor.rs` —
  `HttpStepExecutor` constructor.
- `philharmonic/src/bin/philharmonic_connector/main.rs` —
  the connector service's HTTP handler (how it extracts
  headers, calls `verify_and_decrypt`, dispatches to impl).
  **Replicate this handler in the test** as a minimal axum
  route.
- `philharmonic-connector-service/src/verify.rs` —
  `verify_and_decrypt` function signature.
- `philharmonic-connector-service/src/lib.rs` — re-exports
  (`MintingKeyRegistry`, `MintingKeyEntry`,
  `RealmPrivateKeyRegistry`, `RealmPrivateKeyEntry`,
  `VerifyingKey`).
- `philharmonic-connector-common/src/lib.rs` —
  `RealmPublicKey`, `RealmId`, `MLKEM768_PUBLIC_KEY_LEN`.
- `philharmonic-connector-impl-vector-search/src/lib.rs` —
  `VectorSearch::new()`, `name() -> "vector_search"`.
- `philharmonic-connector-client/src/signing.rs` —
  `LowererSigningKey::from_seed`.
- `philharmonic-connector-client/tests/encryption_vectors.rs`
  — how test keypairs are generated (ML-KEM, X25519, Ed25519).
- `CONTRIBUTING.md`

### Test file structure

```rust
// philharmonic-api/tests/e2e_full_pipeline.rs

// #[tokio::test(flavor = "multi_thread")]
// #[ignore = "requires MySQL testcontainer"]
// #[serial_test::file_serial(docker)]
// async fn full_pipeline_step_execution()
```

### Key generation in the test

Generate all keys at test time using OsRng:

```rust
// Ed25519 for lowerer signing
let ed_signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
let ed_verifying_key = ed_signing_key.verifying_key();
let lowerer_kid = "test-lowerer-kid";

// ML-KEM-768 for payload encryption
let (dk, ek) = ml_kem::MlKem768::generate(&mut OsRng);

// X25519 for hybrid KEM
let x25519_sk = x25519_dalek::StaticSecret::random_from_rng(OsRng);
let x25519_pk = x25519_dalek::PublicKey::from(&x25519_sk);
```

### Connector service (in-process)

Build a minimal axum `Router` that replicates the connector
bin's `POST /` handler:

1. Extract `Authorization: Bearer <hex>` → decode hex →
   COSE_Sign1 token bytes.
2. Extract `X-Encrypted-Payload: <hex>` → decode hex →
   COSE_Encrypt0 bytes.
3. Parse body as JSON request.
4. Call `verify_and_decrypt(token_bytes, encrypted_bytes,
   realm, &minting_registry, &realm_registry, now)`.
5. Deserialize the decrypted plaintext to get `impl` name +
   `config`.
6. Look up the impl in a `HashMap<String, Box<dyn Implementation>>`.
7. Call `impl.execute(&config, &request, &context).await`.
8. Return the JSON result.

Read the connector bin's `handle_connector_request_inner` in
`philharmonic/src/bin/philharmonic_connector/main.rs` — it
does exactly this. Simplify for the test (fewer error cases)
but keep the same wire protocol.

Bind to `127.0.0.1:0` and capture the port.

### API server setup

Same MySQL testcontainer + auth seeding as `e2e_mysql.rs`, but:

- Instead of `StubLowerer`, use `ConnectorConfigLowerer::new(
    lowerer_signing_key, realm_keys, issuer, 600_000)`.
  - `lowerer_signing_key` = `LowererSigningKey::from_seed(
    Zeroizing::new(ed_signing_key.to_bytes()), lowerer_kid)`
  - `realm_keys` = map with one entry: realm "test-realm" →
    `RealmPublicKey::new(realm_kid, RealmId::new("test-realm"),
    ek_bytes, x25519_pk_bytes, not_before, not_after)`.
- Instead of `StubExecutor`, use
  `HttpStepExecutor::new(connector_url)`.

The `ConfigLowerer` and `StepExecutor` are trait objects
passed to `PhilharmonicApiBuilder` via `.config_lowerer()`
and `.step_executor()`. But wait — the builder takes
`Arc<dyn ConfigLowerer>` and `Arc<dyn StepExecutor>`, and
`ConnectorConfigLowerer`/`HttpStepExecutor` live in the bin
crate (`philharmonic/src/bin/`), not in a library. So you
CANNOT import them from a test in `philharmonic-api`.

**Solution**: reimplement the lowerer and executor inline in
the test file (they're ~50 lines each and the logic is
straightforward), OR build the pipeline differently:
call the lowerer function directly, then make the HTTP
request manually in the test. The simplest approach:

1. Build the API with `StubLowerer` + `StubExecutor` (same
   as existing e2e tests).
2. In the test, manually call the lowerer logic to produce
   `{token, encrypted_payload}`.
3. POST those directly to the connector service.
4. Verify the connector returns a valid response.

This proves the crypto round-trip without needing to import
bin-internal types. The test becomes:
"lowerer produces valid output that the connector service
accepts and dispatches correctly."

### Test flow

```
1. Generate keys
2. Start connector service (axum, with VectorSearch impl)
3. Start MySQL testcontainer + migrate
4. Build ConnectorTokenClaims manually
5. Call LowererSigningKey::mint_token → COSE_Sign1 bytes
6. Call encrypt_payload → COSE_Encrypt0 bytes
7. POST to connector service with the token + payload
8. Assert: 200 response, valid vector search results
```

This is cleaner than trying to import bin-internal types.

### Assertions

- Connector returns 200.
- Response body is valid JSON with vector search results.
- The test proves: Ed25519 sign ✓, ML-KEM-768 + X25519 +
  HKDF + AES-256-GCM encrypt ✓, verify + decrypt ✓,
  impl dispatch ✓.

## Rules

- **Do NOT commit, push, or publish.**
- Use `CARGO_TARGET_DIR=target-main` for raw cargo commands.
- You MAY modify `philharmonic-api/Cargo.toml` dev-deps.
- Do NOT modify files outside `philharmonic-api/` except
  `Cargo.lock`.
- Tests may use `.unwrap()` / `.expect()` freely.
</task>

<structured_output_contract>
When you are done, produce a summary listing:
1. Files created or modified.
2. All verification commands run and their pass/fail status.
3. Any design decisions.
4. Confirmation that you did NOT commit or push.
</structured_output_contract>

<completeness_contract>
The test must compile and, when run with Docker available,
exercise the full crypto round-trip. No TODO stubs.
</completeness_contract>

<verification_loop>
Before finishing:
1. `CARGO_TARGET_DIR=target-main cargo check -p philharmonic-api
   --tests` — must compile.
2. If Docker available: `CARGO_TARGET_DIR=target-main cargo test
   -p philharmonic-api --test e2e_full_pipeline -- --ignored
   --test-threads=1` — run the test.
</verification_loop>

<missing_context_gating>
If a library API doesn't match expectations, describe the gap.
Trust the library code over this prompt.
</missing_context_gating>

<action_safety>
- Do NOT commit. Do NOT push. Do NOT publish.
- Do NOT modify files outside philharmonic-api/ except Cargo.lock.
</action_safety>
