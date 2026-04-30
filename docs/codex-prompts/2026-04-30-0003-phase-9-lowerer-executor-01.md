# Phase 9 tasks 7–8 — real ConfigLowerer + StepExecutor

**Date:** 2026-04-30
**Slug:** `phase-9-lowerer-executor`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Replace `StubLowerer` and `StubExecutor` in the
`philharmonic-api` bin with real implementations that wire
the connector-client crypto pipeline and HTTP dispatch to
upstream workers/connectors. Gate-1 approved 2026-04-30 at
`docs/design/crypto-proposals/2026-04-30-phase-9-config-lowerer.md`.

## References

- `docs/design/crypto-proposals/2026-04-30-phase-9-config-lowerer.md`
  — Gate-1 approved proposal.
- `philharmonic-connector-client/src/signing.rs` —
  `LowererSigningKey::mint_token`.
- `philharmonic-connector-client/src/encrypt.rs` —
  `encrypt_payload`, `AeadAadInputs`.
- `philharmonic-connector-common/src/lib.rs` —
  `ConnectorTokenClaims`, `RealmPublicKey`.
- `philharmonic-workflow/src/lowerer.rs` — `ConfigLowerer` trait.
- `philharmonic-workflow/src/executor.rs` — `StepExecutor` trait.
- `philharmonic/src/bin/philharmonic_api/main.rs` — current
  bin using `StubLowerer` + `StubExecutor`.

## Context files pointed at

- `philharmonic/src/bin/philharmonic_api/`
- `philharmonic-connector-client/src/`
- `philharmonic-connector-common/src/lib.rs`
- `philharmonic-workflow/src/lowerer.rs`
- `philharmonic-workflow/src/executor.rs`

## Scope

### In scope

#### 1. `ConnectorConfigLowerer` (`philharmonic/src/bin/philharmonic_api/lowerer.rs`)

Read the Gate-1 proposal for the full design. Summary:

```rust
pub struct ConnectorConfigLowerer {
    signing_key: LowererSigningKey,
    realm_keys: HashMap<String, RealmPublicKey>,
    issuer: String,
    token_lifetime_ms: u64,  // default 600_000 (10 min)
}
```

Implements `ConfigLowerer`:
1. Parse `abstract_config` JSON: extract `realm`, `config_uuid`,
   `impl` (connector implementation name), `config` (endpoint
   config content).
2. Look up `realm_keys[&realm]`.
3. Serialize plaintext payload: `{"realm", "impl", "config"}`.
4. Call `encrypt_payload(plaintext, &realm_key, aad_inputs, &mut OsRng)`.
5. Compute `payload_hash` = SHA-256 of encrypted bytes.
6. Build `ConnectorTokenClaims` with `exp = now + 600_000ms`.
7. `signing_key.mint_token(&claims)`.
8. Return JSON: `{"token": "<hex>", "encrypted_payload": "<hex>"}`.

Read `ConnectorTokenClaims`, `AeadAadInputs`, `encrypt_payload`,
and `LowererSigningKey::mint_token` to match exact API signatures.

#### 2. `HttpStepExecutor` (`philharmonic/src/bin/philharmonic_api/executor.rs`)

Implements `StepExecutor`. Sends the lowered config to the
appropriate upstream service via HTTP.

Read `philharmonic-workflow/src/executor.rs` for the trait:

```rust
#[async_trait]
pub trait StepExecutor: Send + Sync {
    async fn execute(
        &self,
        lowered_config: &JsonValue,
        instance_id: EntityId<WorkflowInstance>,
        step_seq: u64,
        subject: &SubjectContext,
    ) -> Result<JsonValue, StepExecutionError>;
}
```

Implementation:
- The `lowered_config` JSON contains `token` and
  `encrypted_payload` (hex strings from the lowerer).
- Send a POST to the connector service URL (from config)
  with:
  - `Authorization: Bearer <token-hex>`
  - `X-Encrypted-Payload: <encrypted-payload-hex>`
  - Body: the step request JSON
- Parse the response JSON and return it.
- Use `reqwest::Client` with rustls.

```rust
pub struct HttpStepExecutor {
    client: reqwest::Client,
    connector_url: String,  // e.g. "http://127.0.0.1:3002"
}
```

#### 3. Config additions (`config.rs`)

Add to `ApiConfig`:
```rust
pub lowerer_signing_key_path: Option<PathBuf>,
pub lowerer_signing_key_kid: Option<String>,
pub lowerer_issuer: Option<String>,  // defaults to config.issuer
pub lowerer_token_lifetime_ms: u64,  // default 600_000
pub realm_public_keys: Vec<RealmPublicKeyConfig>,
pub connector_service_url: Option<String>,  // default "http://127.0.0.1:3002"
```

`RealmPublicKeyConfig`:
```rust
pub struct RealmPublicKeyConfig {
    pub kid: String,
    pub realm_id: String,
    pub mlkem_public_key_path: PathBuf,
    pub x25519_public_key_path: PathBuf,
    pub not_before: UnixMillis,
    pub not_after: UnixMillis,
}
```

#### 4. Wire into `main.rs`

Replace `StubLowerer` and `StubExecutor` with the real
implementations when the config has the necessary fields
(signing key + realm keys for lowerer, connector URL for
executor). Fall back to stubs when config fields are missing
(so the bin still starts without connector config).

### Out of scope

- Modifying any library crate.
- New crypto primitives (we call existing ones only).
- Tests beyond compilation (the e2e testcontainer suite will
  test the full pipeline once both lowerer and executor work).

## Outcome

Pending — will be updated after Codex run.

---

## Prompt (verbatim)

<task>
You are working inside the `philharmonic-workspace` Cargo workspace.
Your target is `philharmonic/src/bin/philharmonic_api/` — bin
target code inside the `philharmonic` meta-crate submodule.

## IMPORTANT: Do NOT commit or push

**Do NOT run `./scripts/commit-all.sh`.** Do NOT run any git
commit command. Do NOT run `./scripts/push-all.sh`. Leave the
working tree dirty.

## What to build

Replace `StubLowerer` and `StubExecutor` with real
implementations. **Read these files first:**

- `docs/design/crypto-proposals/2026-04-30-phase-9-config-lowerer.md`
  — the approved design.
- `philharmonic-connector-client/src/signing.rs` —
  `LowererSigningKey` API.
- `philharmonic-connector-client/src/encrypt.rs` —
  `encrypt_payload`, `AeadAadInputs`.
- `philharmonic-connector-common/src/lib.rs` —
  `ConnectorTokenClaims`, `RealmPublicKey`,
  `ConnectorSignedToken`, `ConnectorEncryptedPayload`.
- `philharmonic-workflow/src/lowerer.rs` — `ConfigLowerer` trait.
- `philharmonic-workflow/src/executor.rs` — `StepExecutor` trait.
- `philharmonic/src/bin/philharmonic_api/main.rs`
- `philharmonic/src/bin/philharmonic_api/config.rs`
- `philharmonic/src/bin/philharmonic_connector/main.rs` —
  reference for key loading + hex file reading patterns.
- `CONTRIBUTING.md`

### Files to create/modify

1. **`philharmonic/src/bin/philharmonic_api/lowerer.rs`** (new):
   `ConnectorConfigLowerer` implementing `ConfigLowerer`.

2. **`philharmonic/src/bin/philharmonic_api/executor.rs`** (new):
   `HttpStepExecutor` implementing `StepExecutor`.

3. **`philharmonic/src/bin/philharmonic_api/config.rs`** (modify):
   Add lowerer + executor config fields.

4. **`philharmonic/src/bin/philharmonic_api/main.rs`** (modify):
   Wire real implementations when config is present; fall back
   to stubs when fields are missing.

5. **`philharmonic/Cargo.toml`** (modify if needed):
   Add `reqwest` with `rustls` feature if not already present,
   `sha2`, `rand` for OsRng. Check existing deps first.

### `ConnectorConfigLowerer` design

The `lower()` method:
1. Parse `abstract_config`: expect JSON object with keys
   `realm` (string), `config_uuid` (UUID string), `impl`
   (string), `config` (object — the endpoint config content).
   If any are missing → `ConfigLoweringError::InvalidConfig`.
2. Look up realm public key by `realm` in `self.realm_keys`.
   Missing → `ConfigLoweringError::InvalidConfig`.
3. Serialize plaintext: `{"realm": "...", "impl": "...",
   "config": {...}}` as JSON bytes.
4. Build `AeadAadInputs` from claims fields. Read the struct
   to see what fields it needs.
5. Call `encrypt_payload(plaintext_bytes, &realm_key,
   aad_inputs, &mut rand::rngs::OsRng)`.
6. Compute SHA-256 of the encrypted payload COSE bytes →
   `Sha256`.
7. Build `ConnectorTokenClaims`:
   - `iss`: `self.issuer`
   - `exp`: `UnixMillis(now + self.token_lifetime_ms)`
   - `iat`: `UnixMillis(now)`
   - `kid`: `self.signing_key.kid()`
   - `realm`, `tenant` (from `subject.tenant_id`),
     `inst` (from `instance_id`), `step` (from `step_seq`),
     `config_uuid`, `payload_hash`
8. `self.signing_key.mint_token(&claims)` → COSE_Sign1 bytes.
9. Return JSON:
   ```json
   {
     "token": "<hex of COSE_Sign1 bytes>",
     "encrypted_payload": "<hex of COSE_Encrypt0 bytes>"
   }
   ```

Token lifetime: **600_000 ms (10 minutes)**.

### `HttpStepExecutor` design

The `execute()` method:
1. Parse `lowered_config`: expect `token` and
   `encrypted_payload` hex strings.
2. POST to `self.connector_url`:
   - Header `Authorization: Bearer <token-hex>`
   - Header `X-Encrypted-Payload: <encrypted-payload-hex>`
   - Header `Content-Type: application/json`
   - Body: step request JSON (extract from `lowered_config`
     if present, or use empty `{}`)
3. Check response status: 2xx → parse JSON body → return.
   Non-2xx → `StepExecutionError`.

Use `reqwest::Client` (shared, constructed once at startup).

### Config wiring in `main.rs`

```rust
// If lowerer config is present, build real lowerer
let lowerer: Arc<dyn ConfigLowerer> = if has_lowerer_config {
    Arc::new(ConnectorConfigLowerer::new(...))
} else {
    eprintln!("...: lowerer not configured, using stub");
    Arc::new(StubLowerer)
};

// Same pattern for executor
let executor: Arc<dyn StepExecutor> = if has_executor_config {
    Arc::new(HttpStepExecutor::new(...))
} else {
    eprintln!("...: executor not configured, using stub");
    Arc::new(StubExecutor)
};
```

## Rules

- **Do NOT commit, push, or publish.**
- Use `CARGO_TARGET_DIR=target-main` for raw cargo commands.
- Run from the workspace root, not from inside the submodule.
- You MAY modify `philharmonic/Cargo.toml` to add deps.
- Do NOT modify files outside `philharmonic/` except `Cargo.lock`.
- Do NOT add `unsafe` code.
- Do NOT modify any library crate.
</task>

<structured_output_contract>
When you are done, produce a summary listing:
1. Files created or modified.
2. All verification commands run and their pass/fail status.
3. Any design decisions or deviations from the proposal.
4. Confirmation that you did NOT commit or push.
</structured_output_contract>

<completeness_contract>
Both `ConnectorConfigLowerer` and `HttpStepExecutor` must be
fully implemented — no TODOs or placeholder returns. The bin
must compile with and without the lowerer/executor config
fields present. If an API doesn't match the prompt, trust the
library code.
</completeness_contract>

<verification_loop>
Before finishing:
1. `CARGO_TARGET_DIR=target-main cargo build -p philharmonic
   --bin philharmonic-api` — must compile.
2. `CARGO_TARGET_DIR=target-main cargo clippy -p philharmonic
   --bin philharmonic-api -- -D warnings` — clean.
3. `./target-main/debug/philharmonic-api version` — runs.
</verification_loop>

<missing_context_gating>
If a library API doesn't match the design proposal, describe
the gap. Trust the library code over this prompt.
</missing_context_gating>

<action_safety>
- Do NOT commit. Do NOT push. Do NOT publish.
- Do NOT modify files outside philharmonic/ except Cargo.lock.
- Do NOT add unsafe code.
- Do NOT modify any library crate.
</action_safety>
