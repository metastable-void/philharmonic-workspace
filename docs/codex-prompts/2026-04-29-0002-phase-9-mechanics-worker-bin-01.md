# Phase 9 task 4a — `mechanics-worker` bin target

**Date:** 2026-04-29
**Slug:** `phase-9-mechanics-worker-bin`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

The shared server infrastructure (Clap, TOML config, SIGHUP
reload) landed in task 3. This dispatch creates the first bin
target — `mechanics-worker` — inside the `philharmonic`
meta-crate. It wraps the `mechanics` crate's
`MechanicsServer` with the shared server infra, proving the
pattern before the more complex bins are added.

Non-crypto task. No crypto-review gate.

## References

- [`ROADMAP.md`](../../ROADMAP.md) §Phase 9 — task 4,
  `mechanics-worker` bullet.
- [`HUMANS.md`](../../HUMANS.md) §Integration — describes
  `mechanics-worker` as a "bit better HTTP server wrapper
  supporting Clap CLI and config files for mechanics (JS
  executor)."
- `philharmonic/src/server/cli.rs` — `BaseArgs`,
  `BaseCommand`, `resolve_config_paths`.
- `philharmonic/src/server/config.rs` — `load_config<T>`.
- `philharmonic/src/server/reload.rs` — `ReloadHandle`.
- `mechanics/src/lib.rs` — `MechanicsServer`, `run()`,
  `run_tls()` (`https` feature), `MechanicsPoolConfig`,
  `TlsConfig::from_pem()`.
- `mechanics/src/bin/mechanics-rs.rs` — the existing bare-bones
  bin for reference (env-var-based, no config file, no TLS).

## Context files pointed at

- `philharmonic/Cargo.toml` — meta-crate manifest.
- `philharmonic/src/lib.rs` — re-exports + `pub mod server`.
- `philharmonic/src/server/*.rs` — the shared infra modules.
- `mechanics/src/lib.rs` — `MechanicsServer` API.
- `mechanics/src/tls.rs` — `TlsConfig::from_pem()`.
- `mechanics/src/bin/mechanics-rs.rs` — old bin for reference.

## Scope

### In scope

#### 1. Config struct (`philharmonic/src/bin/mechanics_worker/config.rs`)

A serde-deserializable config struct for the mechanics worker:

```rust
#[derive(Debug, serde::Deserialize)]
pub struct MechanicsWorkerConfig {
    /// Socket address to bind. Default: 127.0.0.1:3001.
    #[serde(default = "default_bind")]
    pub bind: std::net::SocketAddr,

    /// Comma-separated Bearer tokens (for auth).
    #[serde(default)]
    pub tokens: Vec<String>,

    /// Pool configuration.
    #[serde(default)]
    pub pool: PoolConfig,

    /// TLS configuration (optional, only with `https` feature).
    #[cfg(feature = "https")]
    pub tls: Option<TlsFileConfig>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PoolConfig {
    #[serde(default = "default_execution_timeout_secs")]
    pub execution_timeout_secs: u64,
    #[serde(default = "default_run_timeout_secs")]
    pub run_timeout_secs: u64,
    #[serde(default = "default_http_timeout_ms")]
    pub default_http_timeout_ms: u64,
    #[serde(default = "default_max_memory")]
    pub max_memory: usize,
    #[serde(default = "default_max_stack")]
    pub max_stack: usize,
    #[serde(default = "default_max_output")]
    pub max_output: usize,
}

/// TLS cert/key file paths (only with `https` feature).
#[cfg(feature = "https")]
#[derive(Debug, serde::Deserialize)]
pub struct TlsFileConfig {
    pub cert_path: std::path::PathBuf,
    pub key_path: std::path::PathBuf,
}
```

Provide reasonable defaults matching the current
`mechanics-rs.rs` bin:
- `bind`: `127.0.0.1:3001`
- `execution_timeout_secs`: `3600`
- `run_timeout_secs`: `3600`
- `default_http_timeout_ms`: `300_000`
- `max_memory`: `65536`
- `max_stack`: `65536`
- `max_output`: `131072`

#### 2. Main binary (`philharmonic/src/bin/mechanics_worker/main.rs`)

```
philharmonic/src/bin/mechanics_worker/
├── config.rs
└── main.rs
```

The main binary:

1. Parses CLI via a wrapper struct that embeds `BaseCommand`:
   ```rust
   #[derive(clap::Parser)]
   #[command(name = "mechanics-worker", version, about = "Philharmonic mechanics JS executor")]
   struct Cli {
       #[command(subcommand)]
       command: Option<BaseCommand>,
   }
   ```
   If no subcommand given, default to `Serve` with default args.

2. On `Version` subcommand: print version info and exit.

3. On `Serve`:
   - Resolve config paths via `resolve_config_paths("mechanics", &args)`.
   - Load config via `load_config::<MechanicsWorkerConfig>(...)`.
     If config file doesn't exist and user didn't pass `-c`,
     use `MechanicsWorkerConfig::default()` (log a note).
   - Override `bind` if `--bind` CLI flag was given.
   - Build `MechanicsPoolConfig` from config values.
   - Create `MechanicsServer::new(pool_config)`.
   - Add tokens from config.
   - If TLS is configured (feature `https` + `tls` section in
     config), read cert/key files and call `run_tls()`.
     Otherwise call `run()`.
   - Create a `ReloadHandle`.
   - In a loop: `reload_handle.notified().await` → reload
     config (re-read TOML, update tokens — log what changed).
     If TLS, re-read cert/key files (the mechanics server
     would need to be restarted for new certs; for now, just
     log that cert reload requires restart — full hot-reload
     is a future enhancement).
   - The main function uses `#[tokio::main]`.

4. Error handling: use `std::process::exit(1)` on fatal errors
   after printing to stderr. No `.unwrap()` on
   `Result`/`Option` — the bin is user-facing.

5. Logging: use `eprintln!` for startup/reload messages. No
   `tracing` dependency yet (that's a future enhancement).

#### 3. Cargo.toml additions

Add `[[bin]]` section to `philharmonic/Cargo.toml`:

```toml
[[bin]]
name = "mechanics-worker"
path = "src/bin/mechanics_worker/main.rs"
```

The `tokio` dependency needs `full` features for the bin's
`#[tokio::main]`. Update the existing tokio dep:

```toml
tokio = { version = "1", features = ["full"] }
```

(This replaces the current `features = ["signal", "sync", "rt"]`
— `full` is a superset.)

### Out of scope

- **`install` subcommand** — deferred.
- **`tracing` / structured logging** — future enhancement.
- **Hot TLS cert reload** — log that restart is needed for now.
- **Other bin targets** — separate prompts.
- **Tests for the bin itself** — integration tests for bins
  come in the e2e testcontainers task. The shared server
  module already has unit tests.

## Outcome

Completed after one gap-and-resume cycle:

1. First dispatch hit `<missing_context_gating>`:
   `MechanicsServer` had no `replace_tokens()` API for SIGHUP
   reload. Claude added `replace_tokens(impl IntoIterator<Item
   = String>)` to `mechanics/src/lib.rs` (mechanics `3536985`).
2. Second dispatch completed cleanly. Files created:
   - `philharmonic/src/bin/mechanics_worker/config.rs` —
     `MechanicsWorkerConfig`, `PoolConfig`, `TlsFileConfig`
     with serde defaults.
   - `philharmonic/src/bin/mechanics_worker/main.rs` — full
     Clap CLI, TOML config loading with graceful fallback,
     `MechanicsServer` wiring, SIGHUP reload loop with
     `replace_tokens`, optional TLS via `#[cfg(feature =
     "https")]`.
   - `philharmonic/Cargo.toml` — `[[bin]]` section + tokio
     upgraded to `features = ["full"]`.

Verification (Codex ran): build, build --features https,
version, --help, rust-lint, rust-test, rust-test --ignored
all passed. `pre-landing.sh` failed only on fmt drift in
`mechanics/` (outside Codex's write scope); Claude ran
`cargo fmt -p mechanics` and re-ran pre-landing (passed).

Committed as mechanics `aaf342c`, philharmonic `46d20cf`,
parent `e738bac`. Did NOT run push-all.sh (Claude pushed
after review).

---

## Prompt (verbatim)

<task>
You are working inside the `philharmonic-workspace` Cargo workspace.
Your target crate is `philharmonic` (the meta-crate) at
`philharmonic/` — it's a git submodule with its own repo.

## What to build

Create the `mechanics-worker` bin target inside the `philharmonic`
meta-crate. This is the first binary target — it wraps the
`mechanics` crate's `MechanicsServer` with Clap CLI, TOML config
file loading, and SIGHUP-based config reload.

### File layout

```
philharmonic/src/bin/mechanics_worker/
├── config.rs    — config struct + defaults
└── main.rs      — Clap CLI + server startup + reload loop
```

### 1. `config.rs`

```rust
use std::net::SocketAddr;

#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct MechanicsWorkerConfig {
    pub bind: SocketAddr,
    pub tokens: Vec<String>,
    pub pool: PoolConfig,
    #[cfg(feature = "https")]
    pub tls: Option<TlsFileConfig>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct PoolConfig {
    pub execution_timeout_secs: u64,
    pub run_timeout_secs: u64,
    pub default_http_timeout_ms: u64,
    pub max_memory: usize,
    pub max_stack: usize,
    pub max_output: usize,
}

#[cfg(feature = "https")]
#[derive(Debug, serde::Deserialize)]
pub struct TlsFileConfig {
    pub cert_path: std::path::PathBuf,
    pub key_path: std::path::PathBuf,
}
```

Default values (implement via `Default` trait):
- `bind`: `127.0.0.1:3001`
- `tokens`: empty vec
- `pool.execution_timeout_secs`: `3600`
- `pool.run_timeout_secs`: `3600`
- `pool.default_http_timeout_ms`: `300_000`
- `pool.max_memory`: `65536`
- `pool.max_stack`: `65536`
- `pool.max_output`: `131072`

### 2. `main.rs`

```rust
use clap::Parser;
use philharmonic::server::cli::{BaseArgs, BaseCommand, resolve_config_paths};
use philharmonic::server::config::load_config;
use philharmonic::server::reload::ReloadHandle;

mod config;
use config::MechanicsWorkerConfig;
```

The Clap CLI:

```rust
#[derive(Parser)]
#[command(name = "mechanics-worker", version, about = "Philharmonic mechanics JS executor")]
struct Cli {
    #[command(subcommand)]
    command: Option<BaseCommand>,
}
```

Main flow (`#[tokio::main]`):

1. Parse CLI. If `command` is `None`, treat as `Serve` with
   default `BaseArgs`.
2. If `Version`: print `env!("CARGO_PKG_VERSION")` and exit.
3. If `Serve(args)`:
   a. `resolve_config_paths("mechanics", &args)` → `(primary, drop_in)`.
   b. Try `load_config::<MechanicsWorkerConfig>(&primary, &drop_in)`.
      - If the primary file doesn't exist AND user didn't pass
        `-c`, fall back to `MechanicsWorkerConfig::default()`
        and print a note to stderr.
      - Other errors → print and exit(1).
   c. If `args.bind` is `Some`, override `config.bind`.
   d. Build `MechanicsPoolConfig`:
      ```rust
      let limits = MechanicsExecutionLimits::new(
          Duration::from_secs(config.pool.execution_timeout_secs),
          config.pool.max_memory,
          config.pool.max_stack,
          config.pool.max_output,
      ).map_err(/* ... */)?;
      let pool_config = MechanicsPoolConfig::default()
          .with_execution_limits(limits)
          .with_run_timeout(Duration::from_secs(config.pool.run_timeout_secs))
          .with_default_http_timeout_ms(Some(config.pool.default_http_timeout_ms));
      ```
      Use `mechanics_core::job::MechanicsExecutionLimits` via
      `philharmonic::mechanics_core::job::MechanicsExecutionLimits`.
   e. Create `MechanicsServer::new(pool_config)` — exit(1) on error.
   f. Add each token from `config.tokens` via `server.add_token()`.
   g. Start the server:
      - With `https` feature AND `config.tls` is `Some(tls)`:
        read `tls.cert_path` and `tls.key_path` as bytes,
        create `TlsConfig::from_pem(&cert_bytes, &key_bytes)`,
        call `server.run_tls(config.bind, tls_config)`.
      - Otherwise: `server.run(config.bind)`.
      Exit(1) on error.
   h. Print startup message to stderr:
      `"mechanics-worker listening on {bind} ({protocol})"`
      where protocol is "https" or "http".
      Print token count.
   i. Create `ReloadHandle::new()`.
   j. Loop on `reload_handle.notified().await`:
      - Re-read config (same paths).
      - If successful, update tokens (clear old, add new).
        Print what changed to stderr.
      - If config read fails, print error to stderr and
        continue (don't crash).
      - For TLS: print a note that cert changes require
        restart.

### 3. Cargo.toml changes

Add to `philharmonic/Cargo.toml`:

```toml
[[bin]]
name = "mechanics-worker"
path = "src/bin/mechanics_worker/main.rs"
```

Change the `tokio` dependency from
`features = ["signal", "sync", "rt"]` to `features = ["full"]`
so `#[tokio::main]` works in the bin target.

### Error handling

- No `.unwrap()` or `.expect()` anywhere in the bin code.
  Use `match` / `if let` / early-return with `process::exit(1)`.
- Print errors to stderr, not stdout.
- Fatal errors (can't bind, can't create pool) → exit(1).
- Non-fatal errors (reload parse failure) → log and continue.

## Rules

- Commit via `./scripts/commit-all.sh "message"` only. Never
  raw `git commit` or `git push`. Do NOT run `push-all.sh`.
- Run `./scripts/pre-landing.sh` before the final commit.
  It auto-detects dirty crates and runs fmt + check + clippy
  (-D warnings) + test. Fix any failures before committing.
- Use `CARGO_TARGET_DIR=target-main` if you run any cargo
  commands outside the scripts.
- Bin code may use `.unwrap()` / `eprintln!` in the main
  function (it's a bin, not library code). But prefer clean
  error handling as described above.

## Authoritative references

- `CONTRIBUTING.md` — workspace conventions. If anything in
  this prompt contradicts CONTRIBUTING.md, the doc wins.
- `ROADMAP.md` §Phase 9.

Read them before starting.
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
added later" stubs. The bin must compile and run (at least
to the point of printing help / version / starting with
default config). If a design decision arises that isn't covered
by the prompt, make a reasonable choice, document it in a code
comment, and note it in your summary.
</completeness_contract>

<verification_loop>
Before your final commit:
1. `./scripts/pre-landing.sh` — must pass.
2. `CARGO_TARGET_DIR=target-main cargo build -p philharmonic
   --bin mechanics-worker` — verify the bin compiles.
3. `CARGO_TARGET_DIR=target-main cargo build -p philharmonic
   --bin mechanics-worker --features https` — verify with TLS.
4. `CARGO_TARGET_DIR=target-main ./target-main/debug/mechanics-worker
   version` — verify it prints version.
5. `CARGO_TARGET_DIR=target-main ./target-main/debug/mechanics-worker
   --help` — verify help output.

If any step fails, fix and re-run before committing.
</verification_loop>

<missing_context_gating>
If you discover that a dependency, trait, or type you need doesn't
exist or doesn't match the prompt's description, stop and describe
what's missing in your summary rather than inventing a workaround
that might conflict with the rest of the workspace.
</missing_context_gating>

<action_safety>
- Do NOT run `./scripts/push-all.sh`.
- Do NOT run `cargo publish`.
- Do NOT modify files outside `philharmonic/` except the
  workspace-level `Cargo.lock` (which updates automatically).
- Do NOT add `unsafe` code.
</action_safety>
