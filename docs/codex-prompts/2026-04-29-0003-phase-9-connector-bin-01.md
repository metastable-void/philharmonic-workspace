# Phase 9 task 4b — `philharmonic-connector` bin target

**Date:** 2026-04-29
**Slug:** `phase-9-connector-bin`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

The `mechanics-worker` bin proved the shared server infra
pattern (Clap + TOML config + SIGHUP reload). This dispatch
creates the second bin — `philharmonic-connector` — which
wraps the connector service framework
(`philharmonic-connector-service`) and the connector router
(`philharmonic-connector-router`) together with all shipped
connector implementation crates into a single per-realm
binary.

Non-crypto task (the crypto primitives live in the library
crates; this bin only wires them). No crypto-review gate.

## References

- [`ROADMAP.md`](../../ROADMAP.md) §Phase 9 — task 4,
  `philharmonic-connector` bullet.
- [`HUMANS.md`](../../HUMANS.md) §Integration — "the
  Connector Service. Ships everything supported at the moment
  by default."
- `philharmonic-connector-service/src/lib.rs` — re-exports:
  `MintingKeyRegistry`, `MintingKeyEntry`,
  `RealmPrivateKeyRegistry`, `RealmPrivateKeyEntry`,
  `verify_token`, `verify_and_decrypt`,
  `VerifiedDecryptedPayload`, `ConnectorCallContext`,
  `TokenVerifyError`.
- `philharmonic-connector-router/src/lib.rs` — re-exports:
  `DispatchConfig`, `router`, `RouterState`, `HyperForwarder`,
  `Forwarder`.
- `philharmonic-connector-impl-api/src/lib.rs` —
  `Implementation` trait (the contract each impl crate
  satisfies).
- `philharmonic/src/bin/mechanics_worker/` — the pattern to
  follow (Clap + config + SIGHUP).

## Context files pointed at

- `philharmonic/Cargo.toml` — meta-crate manifest.
- `philharmonic/src/server/*.rs` — shared infra.
- `philharmonic/src/bin/mechanics_worker/` — pattern reference.
- `philharmonic-connector-service/src/lib.rs`
- `philharmonic-connector-router/src/lib.rs`
- `philharmonic-connector-router/src/dispatch.rs`
- `philharmonic-connector-impl-api/src/lib.rs`
- Connector impl crates (one per capability):
  `philharmonic-connector-impl-http-forward`,
  `philharmonic-connector-impl-llm-openai-compat`,
  `philharmonic-connector-impl-sql-postgres`,
  `philharmonic-connector-impl-sql-mysql`,
  `philharmonic-connector-impl-embed`,
  `philharmonic-connector-impl-vector-search`.

## Scope

### In scope

#### 1. Config struct (`philharmonic/src/bin/philharmonic_connector/config.rs`)

```rust
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct ConnectorConfig {
    pub bind: SocketAddr,

    /// Realm identifier for this connector service instance.
    pub realm_id: Option<philharmonic_types::Uuid>,

    /// Upstream connector-service URLs keyed by realm ID
    /// (for the router's dispatch table). Only needed if
    /// this binary is acting as a router; if it's a
    /// single-realm service, the dispatch table can be
    /// omitted and the service serves directly.
    #[serde(default)]
    pub dispatch: std::collections::HashMap<String, String>,

    /// Minting (signing) key entries for token verification.
    #[serde(default)]
    pub minting_keys: Vec<MintingKeyConfig>,

    /// Realm private key entries for payload decryption.
    #[serde(default)]
    pub realm_keys: Vec<RealmKeyConfig>,

    #[cfg(feature = "https")]
    pub tls: Option<super::mechanics_worker::config::TlsFileConfig>,
    // Reuse TlsFileConfig if possible, or define a local one.
}

#[derive(Debug, serde::Deserialize)]
pub struct MintingKeyConfig {
    pub kid: String,
    pub public_key_path: PathBuf,
}

#[derive(Debug, serde::Deserialize)]
pub struct RealmKeyConfig {
    pub kid: String,
    pub realm_id: String,
    pub private_key_path: PathBuf,
}
```

Default `bind`: `127.0.0.1:3002`.

**Note**: The exact config shape for minting keys and realm
keys depends on what `MintingKeyRegistry` and
`RealmPrivateKeyRegistry` accept. Read those types' constructors
and `add_*` methods to determine the right config fields. The
shapes above are guidelines — adjust to match the actual API.

#### 2. Main binary (`philharmonic/src/bin/philharmonic_connector/main.rs`)

Follow the `mechanics-worker` pattern:

1. Clap CLI with `BaseCommand` (serve/version).
2. TOML config loading with graceful fallback.
3. Build registries from config:
   - `MintingKeyRegistry` from `minting_keys` config entries
     (read public key files, register each).
   - `RealmPrivateKeyRegistry` from `realm_keys` entries
     (read private key files, register each).
4. Set up the connector router or direct service:
   - If `dispatch` table is populated, set up
     `philharmonic-connector-router`'s `RouterState` with
     `DispatchConfig` + `HyperForwarder`.
   - Build an axum/hyper router via
     `philharmonic_connector_router::router(state)`.
5. Start the HTTP(S) server.
6. SIGHUP reload loop: re-read config, rebuild registries,
   log changes.

**Important**: Read the actual API surface of
`connector-service`, `connector-router`, and `connector-impl-api`
carefully. The config and wiring may need to differ from what's
described here. Trust the library crate APIs over this prompt.

#### 3. Cargo.toml

Add `[[bin]]` section:

```toml
[[bin]]
name = "philharmonic-connector"
path = "src/bin/philharmonic_connector/main.rs"
```

Add any missing direct dependencies needed by the bin
(e.g. `axum` if the router uses it, `ed25519-dalek` if
key loading needs it). Check what the connector crates
re-export vs. what you need to import directly.

### Out of scope

- **`install` subcommand** — deferred.
- **Individual connector impl registration** — the bin
  compiles with all shipped impls via the meta-crate's
  default features. Actually wiring each `Implementation`
  into a dispatch registry is a follow-up if the framework
  needs it; for now the router dispatches to upstream URLs.
- **Tests** — e2e tests come in the testcontainers task.

## Outcome

Pending — will be updated after Codex run.

---

## Prompt (verbatim)

<task>
You are working inside the `philharmonic-workspace` Cargo workspace.
Your target crate is `philharmonic` (the meta-crate) at
`philharmonic/` — it's a git submodule with its own repo.

## What to build

Create the `philharmonic-connector` bin target inside the
`philharmonic` meta-crate. This is the second bin target — it
wraps the connector service + router framework into a runnable
per-realm binary.

**Read these files first** (they are the authoritative API surface):
- `philharmonic-connector-service/src/lib.rs` and its submodules
- `philharmonic-connector-router/src/lib.rs` and its submodules
- `philharmonic-connector-impl-api/src/lib.rs`
- `philharmonic/src/bin/mechanics_worker/` (pattern to follow)
- `philharmonic/src/server/*.rs` (shared infra)
- `CONTRIBUTING.md`

### File layout

```
philharmonic/src/bin/philharmonic_connector/
├── config.rs    — config struct + defaults
└── main.rs      — Clap CLI + server startup + reload loop
```

### 1. `config.rs`

Define a `ConnectorConfig` struct that covers:

- `bind: SocketAddr` (default `127.0.0.1:3002`)
- Minting key entries for `MintingKeyRegistry` (kid + public key
  file path — read the registry's API to determine exact fields).
- Realm private key entries for `RealmPrivateKeyRegistry` (kid +
  realm_id + private key file path).
- Router dispatch table: `HashMap<String, String>` mapping realm
  IDs to upstream URLs, for `DispatchConfig`.
- Optional TLS config (cert_path + key_path), gated behind
  `#[cfg(feature = "https")]`. You can define a local
  `TlsFileConfig` struct or share one.

Use `#[serde(default)]` on the main struct so a minimal config
file works. Implement `Default`.

**Trust the library crate APIs.** If `MintingKeyRegistry` or
`RealmPrivateKeyRegistry` need different fields than described
above, match what they actually accept.

### 2. `main.rs`

Follow the `mechanics-worker` pattern exactly:

1. `#[derive(clap::Parser)]` struct with
   `Option<BaseCommand>`, defaulting to `Serve`.
2. `#[tokio::main] async fn main()` → `run()` → `serve()`.
3. In `serve()`:
   a. `resolve_config_paths("connector", &args)`.
   b. `load_config::<ConnectorConfig>(...)` with graceful
      fallback to defaults when no config file and no `-c`.
   c. Build `MintingKeyRegistry` from config entries (read
      public key files, register each key).
   d. Build `RealmPrivateKeyRegistry` from config entries
      (read private key files, register each key).
   e. Build `DispatchConfig` from the dispatch table in config.
   f. Create a `RouterState` with `HyperForwarder` and set up
      the axum router via
      `philharmonic_connector_router::router(state)`.
   g. Start the HTTP(S) server using axum's `serve()` (or
      hyper directly — match what the router crate expects).
   h. Print startup info to stderr.
   i. `ReloadHandle::new()`, loop on `notified()` — re-read
      config, rebuild registries + dispatch config, log changes.
      For TLS: note that cert changes require restart.

### 3. Cargo.toml

Add `[[bin]]` section:

```toml
[[bin]]
name = "philharmonic-connector"
path = "src/bin/philharmonic_connector/main.rs"
```

Add any direct dependencies the bin needs that aren't already
in `philharmonic/Cargo.toml` (e.g. `axum` if the router
returns an axum `Router`, `hyper`/`hyper-util` for server
binding, `ed25519-dalek` for public key loading). Check what
the connector crates re-export.

### Error handling

Same as `mechanics-worker`: no `.unwrap()`, print errors to
stderr, exit(1) on fatal, continue on non-fatal reload errors.

## Rules

- Commit via `./scripts/commit-all.sh "message"` only. Never
  raw `git commit` or `git push`. Do NOT run `push-all.sh`.
- Run `./scripts/pre-landing.sh` before the final commit.
  Fix any failures before committing.
- Use `CARGO_TARGET_DIR=target-main` for any raw cargo commands.
- You MAY modify `philharmonic/Cargo.toml` to add deps.
- Do NOT modify files outside `philharmonic/` except the
  workspace-level `Cargo.lock`.

## Authoritative references

- `CONTRIBUTING.md` — if anything here contradicts it, the doc wins.
- `ROADMAP.md` §Phase 9.
</task>

<structured_output_contract>
When you are done, produce a summary listing:
1. Files created or modified (paths relative to workspace root).
2. All verification commands run and their pass/fail status.
3. Any residual concerns or open questions.
4. The git commit SHA(s) produced by commit-all.sh.
5. Confirmation that you did NOT run push-all.sh.
</structured_output_contract>

<completeness_contract>
Do not leave TODOs, placeholder implementations, or "will be
added later" stubs. The bin must compile and run (at least to
the point of printing help / version / starting with default
config). If a design decision arises that isn't covered by the
prompt, make a reasonable choice, document it in a code comment,
and note it in your summary.
</completeness_contract>

<verification_loop>
Before your final commit:
1. `./scripts/pre-landing.sh` — must pass.
2. `CARGO_TARGET_DIR=target-main cargo build -p philharmonic
   --bin philharmonic-connector` — verify the bin compiles.
3. `CARGO_TARGET_DIR=target-main cargo build -p philharmonic
   --bin philharmonic-connector --features https` — with TLS.
4. `./target-main/debug/philharmonic-connector version`
5. `./target-main/debug/philharmonic-connector --help`

If any step fails, fix and re-run before committing.
</verification_loop>

<missing_context_gating>
If you discover that a dependency, trait, or type you need doesn't
exist or doesn't match this prompt's description, stop and describe
what's missing in your summary rather than inventing a workaround.
Trust the library crate APIs over this prompt.
</missing_context_gating>

<action_safety>
- Do NOT run `./scripts/push-all.sh`.
- Do NOT run `cargo publish`.
- Do NOT modify files outside `philharmonic/` except `Cargo.lock`.
- Do NOT add `unsafe` code.
</action_safety>
