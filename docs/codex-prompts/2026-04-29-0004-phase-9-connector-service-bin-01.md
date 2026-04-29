# Phase 9 task 4b rework — `philharmonic-connector` as a connector SERVICE (not router)

**Date:** 2026-04-29
**Slug:** `phase-9-connector-service-bin`
**Round:** 01 (rework — previous attempt incorrectly wired the bin as a connector router)
**Subagent:** `codex:codex-rescue`

## Motivation

The previous implementation of `philharmonic-connector`
incorrectly wired the bin as a connector **router** (dispatching
to upstream URLs). The correct architecture:

- **`philharmonic-connector`** = connector **service** entry
  point. Receives HTTP requests, verifies COSE_Sign1 tokens,
  decrypts hybrid-KEM payloads, looks up the
  `Implementation` by name, calls `Implementation::execute`,
  and returns the response. One per realm.
- The connector **router** (`philharmonic-connector-router`)
  belongs in the `philharmonic-api` binary.

The previous bin has been deleted. This dispatch creates the
correct service-oriented connector bin.

Non-crypto task in the bin itself (the crypto lives in the
library crates; the bin just calls `verify_and_decrypt`).

## References

- `ROADMAP.md` §Phase 9 — `philharmonic-connector` bullet
  (updated to say "connector service" explicitly).
- `docs/design/08-connector-architecture.md` — the
  authoritative architecture.
- `philharmonic-connector-service/src/lib.rs` — re-exports:
  `verify_and_decrypt`, `VerifiedDecryptedPayload`,
  `MintingKeyRegistry`, `MintingKeyEntry`,
  `RealmPrivateKeyRegistry`, `RealmPrivateKeyEntry`,
  `ConnectorCallContext`, `TokenVerifyError`.
- `philharmonic-connector-service/src/verify.rs` — the
  `verify_and_decrypt` function signature.
- `philharmonic-connector-impl-api/src/lib.rs` —
  `Implementation` trait + `async_trait`.
- Shipped impl crates and their constructors:
  - `HttpForward::new()` (http-forward)
  - Each impl has `fn name(&self) -> &str` and
    `async fn execute(&self, config, request, ctx)`.
- `philharmonic/src/bin/mechanics_worker/` — pattern reference.

## Context files pointed at

- `philharmonic/Cargo.toml`
- `philharmonic/src/server/*.rs`
- `philharmonic/src/bin/mechanics_worker/`
- `philharmonic-connector-service/src/lib.rs`
- `philharmonic-connector-service/src/verify.rs`
- `philharmonic-connector-impl-api/src/lib.rs`
- `philharmonic-connector-impl-http-forward/src/lib.rs`
  (example impl pattern)

## Scope

### In scope

#### 1. Config struct (`philharmonic/src/bin/philharmonic_connector/config.rs`)

```rust
#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct ConnectorConfig {
    pub bind: SocketAddr,         // default 127.0.0.1:3002

    pub minting_keys: Vec<MintingKeyConfig>,
    pub realm_keys: Vec<RealmKeyConfig>,

    #[cfg(feature = "https")]
    pub tls: Option<TlsFileConfig>,
}
```

`MintingKeyConfig` and `RealmKeyConfig` shapes must match what
`MintingKeyRegistry` and `RealmPrivateKeyRegistry` accept.
Read those types' APIs from `philharmonic-connector-service`.

#### 2. Implementation registry

At startup, build a `HashMap<String, Box<dyn Implementation>>`
from the shipped impl crates:

```rust
fn build_implementation_registry()
    -> HashMap<String, Box<dyn Implementation>>
{
    let mut registry = HashMap::new();

    // Each shipped impl crate's constructor + registration.
    // Feature-gate each to match the meta-crate's feature flags.
    #[cfg(feature = "connector-http-forward")]
    register(&mut registry, || HttpForward::new());

    #[cfg(feature = "connector-llm-openai-compat")]
    register(&mut registry, || LlmOpenaiCompat::new());

    // ... etc for each shipped impl

    registry
}
```

Read each impl crate's `lib.rs` to find the struct name and
constructor. Some may return `Result`, some may be infallible.

#### 3. Request handling

The HTTP handler receives a POST request. The flow:

1. Extract the raw request body bytes.
2. Call `verify_and_decrypt(body, minting_registry, realm_registry, now)`
   from `philharmonic-connector-service`. This returns a
   `VerifiedDecryptedPayload` containing the decrypted
   `config`, `request`, `impl` name, and `ConnectorCallContext`.
3. Look up the `Implementation` in the registry by the `impl` name.
4. Call `implementation.execute(&config, &request, &ctx)`.
5. Return the response as JSON.

Error mapping:
- Token verification failure → 401.
- Unknown implementation → 404.
- `ImplementationError` → map to appropriate HTTP status.

#### 4. Main binary (`philharmonic/src/bin/philharmonic_connector/main.rs`)

Follow the `mechanics-worker` pattern:
- Clap CLI with `BaseCommand`.
- TOML config loading with graceful fallback.
- Build minting/realm key registries from config.
- Build implementation registry.
- Start axum HTTP server with the handler above.
- SIGHUP reload loop: re-read config, rebuild key registries
  (not the implementation registry — impls are static).

#### 5. Cargo.toml

Add `[[bin]]` section back:

```toml
[[bin]]
name = "philharmonic-connector"
path = "src/bin/philharmonic_connector/main.rs"
```

The deps Codex added in the previous attempt (`axum`, `hex`,
`hyper-util`, `x25519-dalek`, `zeroize`, `rustls-pemfile`,
`tokio-rustls`) are already in `Cargo.toml` — reuse them.
Add `async-trait` if it's not already present (needed for
`Implementation` trait dispatch).

### Out of scope

- **Connector router** — that goes in `philharmonic-api`.
- **`install` subcommand**.
- **Tests** — e2e testcontainers task.

## Outcome

Pending — will be updated after Codex run.

---

## Prompt (verbatim)

<task>
You are working inside the `philharmonic-workspace` Cargo workspace.
Your target crate is `philharmonic` (the meta-crate) at
`philharmonic/` — it's a git submodule with its own repo.

## IMPORTANT: Do NOT commit or push

**Do NOT run `./scripts/commit-all.sh`.** Do NOT run any git
commit command. Do NOT run `./scripts/push-all.sh`. Do NOT run
`cargo publish`. Leave the working tree dirty. Claude will
review and commit after you finish.

You MAY run `./scripts/pre-landing.sh`, `./scripts/rust-lint.sh`,
`./scripts/rust-test.sh` for verification — those are read-only
checks, not state-changing operations.

## Background

The previous `philharmonic-connector` bin was deleted because
it incorrectly acted as a connector **router** (dispatching to
upstream URLs). The correct architecture:

- `philharmonic-connector` = connector **service**. Receives
  requests, verifies COSE_Sign1 tokens via
  `verify_and_decrypt()`, dispatches to `Implementation`
  trait objects, returns responses. One per realm.
- The connector **router** belongs in `philharmonic-api` (the
  API binary embeds it).

## What to build

Create the `philharmonic-connector` bin as a connector
**service** entry point.

**Read these files first** (authoritative API surface):
- `philharmonic-connector-service/src/lib.rs` and `src/verify.rs`
- `philharmonic-connector-impl-api/src/lib.rs`
- `philharmonic-connector-impl-http-forward/src/lib.rs` (example)
- `philharmonic-connector-impl-llm-openai-compat/src/lib.rs`
- `philharmonic-connector-impl-sql-postgres/src/lib.rs`
- `philharmonic-connector-impl-sql-mysql/src/lib.rs`
- `philharmonic-connector-impl-embed/src/lib.rs`
- `philharmonic-connector-impl-vector-search/src/lib.rs`
- `philharmonic/src/bin/mechanics_worker/` (pattern reference)
- `philharmonic/src/server/*.rs` (shared infra)
- `CONTRIBUTING.md`

### File layout

```
philharmonic/src/bin/philharmonic_connector/
├── config.rs    — config struct + defaults
└── main.rs      — CLI + service startup + handler + reload
```

### 1. `config.rs`

- `ConnectorConfig` with `bind` (default `127.0.0.1:3002`),
  `minting_keys`, `realm_keys`, optional TLS.
- Match the registry APIs exactly — read `MintingKeyRegistry`,
  `MintingKeyEntry`, `RealmPrivateKeyRegistry`,
  `RealmPrivateKeyEntry` to determine what fields the config
  entries need.

### 2. `main.rs`

Follow mechanics-worker pattern (Clap + config + SIGHUP), plus:

**Implementation registry**: At startup, build a
`HashMap<String, Box<dyn Implementation>>` by constructing
each shipped impl. Feature-gate each behind the meta-crate's
connector features:

```rust
#[cfg(feature = "connector-http-forward")]
{
    // Read the impl crate to find the constructor
    let imp = HttpForward::new()?;  // or whatever the constructor is
    registry.insert(imp.name().to_string(), Box::new(imp));
}
// ... repeat for each shipped impl
```

**Request handler** (axum):

1. `POST /` — the single endpoint. Takes raw body bytes.
2. Call `verify_and_decrypt(...)` from connector-service.
   Read the function signature to see exactly what args it
   needs (body bytes, minting registry, realm registry,
   current time).
3. From the result, extract the `impl` name, `config`,
   `request`, and `ConnectorCallContext`.
4. Look up the impl in the registry by name.
5. Call `impl.execute(&config, &request, &ctx).await`.
6. Return the result as JSON (200 on success).
7. Error mapping: verification failure → 401, unknown impl
   → 404, `ImplementationError` → appropriate status.

**SIGHUP reload**: Re-read config, rebuild key registries.
The implementation registry is static (impls don't change
at runtime).

### 3. Cargo.toml

Add back:
```toml
[[bin]]
name = "philharmonic-connector"
path = "src/bin/philharmonic_connector/main.rs"
```

Add `async-trait` if not already present. The other deps
(`axum`, `hex`, `x25519-dalek`, `zeroize`, etc.) are already
in the manifest from the previous attempt.

### Error handling

Same rules as mechanics-worker: no `.unwrap()`, stderr errors,
exit(1) on fatal, continue on non-fatal reload.

## Rules

- **Do NOT run `./scripts/commit-all.sh` or any git commit.**
- **Do NOT run `./scripts/push-all.sh`.**
- **Do NOT run `cargo publish`.**
- You MAY run lint/test scripts for verification.
- Use `CARGO_TARGET_DIR=target-main` for raw cargo commands.
- You MAY modify `philharmonic/Cargo.toml` to add deps.
- Do NOT modify files outside `philharmonic/` except `Cargo.lock`.

## Authoritative references

- `CONTRIBUTING.md` — if anything contradicts it, the doc wins.
- `ROADMAP.md` §Phase 9.
- `docs/design/08-connector-architecture.md`.
</task>

<structured_output_contract>
When you are done, produce a summary listing:
1. Files created or modified (paths relative to workspace root).
2. All verification commands run and their pass/fail status.
3. Any residual concerns or open questions.
4. Confirmation that you did NOT commit or push.
</structured_output_contract>

<completeness_contract>
Do not leave TODOs or placeholder implementations. The bin must
compile and run (`version`, `--help`, start with default config).
If you discover the verify_and_decrypt API doesn't match this
prompt's description, trust the library code over this prompt.
</completeness_contract>

<verification_loop>
Before finishing:
1. `CARGO_TARGET_DIR=target-main cargo build -p philharmonic
   --bin philharmonic-connector` — must compile.
2. `CARGO_TARGET_DIR=target-main cargo build -p philharmonic
   --bin philharmonic-connector --features https` — with TLS.
3. `./target-main/debug/philharmonic-connector version`
4. `./target-main/debug/philharmonic-connector --help`
5. `CARGO_TARGET_DIR=target-main cargo clippy -p philharmonic
   --bin philharmonic-connector --features https -- -D warnings`

If any step fails, fix and re-run.
</verification_loop>

<missing_context_gating>
If a dependency, trait, or type doesn't exist or doesn't match,
stop and describe what's missing rather than inventing a
workaround. Trust the library crate APIs over this prompt.
</missing_context_gating>

<action_safety>
- Do NOT commit. Do NOT push. Do NOT publish.
- Do NOT modify files outside `philharmonic/` except `Cargo.lock`.
- Do NOT add `unsafe` code.
</action_safety>
