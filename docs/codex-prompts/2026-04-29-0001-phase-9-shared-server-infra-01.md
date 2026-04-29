# Phase 9 task 3 — shared server infrastructure module

**Date:** 2026-04-29
**Slug:** `phase-9-shared-server-infra`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Phase 9 turns the published Philharmonic library crates into
runnable binaries. Before writing any bin target, the shared
server infrastructure they all consume must exist: Clap CLI
skeleton, TOML config loading with drop-in overlays, and a
SIGHUP handler for live config + TLS cert reload. This module
lives inside the `philharmonic` meta-crate at
`philharmonic/src/server/`.

Non-crypto task. No crypto-review gate.

## References

- [`ROADMAP.md`](../../ROADMAP.md) §Phase 9 — "Shared server
  module" paragraph.
- [`HUMANS.md`](../../HUMANS.md) §Integration — describes the
  three bin targets and their shared shape (TOML config, SIGHUP,
  Clap, `install` subcommand, HTTPS feature).
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md) §10.3 (no panics
  in library src), §10.4 (library crates take bytes, not file
  paths — but this is bin-adjacent infra so file I/O is
  acceptable here), §10.9 (TLS stack: rustls only, vendored
  crypto, no system OpenSSL).
- [`docs/notes-to-humans/2026-04-29-0001-phase-9-integration-sketch.md`](../notes-to-humans/2026-04-29-0001-phase-9-integration-sketch.md)
  — full integration sketch with confirmed decisions.

## Context files pointed at

- `philharmonic/Cargo.toml` — meta-crate manifest (just wired
  with all library deps + feature flags; no bin targets yet).
- `philharmonic/src/lib.rs` — re-exports only, no logic.
- `mechanics/src/lib.rs` — `MechanicsServer` with `run()` and
  `run_tls()` (the `https` feature landed today).
- `mechanics/src/tls.rs` — `TlsConfig::from_pem()` pattern to
  reuse or align with.

## Scope

### In scope

Create `philharmonic/src/server/` module with the following
public types and functions. The module is `pub` from `lib.rs`
so bin targets (added in a future round) can import it.

#### 1. TOML config loader (`philharmonic/src/server/config.rs`)

A generic, serde-based TOML config loader:

```rust
/// Load a TOML config file, then merge any `.toml` files found in
/// the drop-in directory (lexicographic order). Later files override
/// earlier ones at the top-level key level.
pub fn load_config<T: serde::de::DeserializeOwned>(
    primary: &Path,
    drop_in_dir: &Path,
) -> Result<T, ConfigError>;
```

- `ConfigError` is a public enum with variants for I/O errors,
  TOML parse errors, and merge errors. Implements
  `std::error::Error` + `Display`.
- The merge strategy: parse the primary file as
  `toml::Value::Table`, then for each `.toml` file in the
  drop-in directory (sorted lexicographically by filename),
  parse it as a `Table` and merge top-level keys (drop-in
  wins on conflict). Finally deserialize the merged table
  into `T`.
- If the primary file doesn't exist, return `ConfigError`.
  If the drop-in directory doesn't exist, skip the overlay
  step (no error).
- Add `toml` and `serde` as dependencies in
  `philharmonic/Cargo.toml`. `serde` with `derive` feature.

#### 2. SIGHUP reload handle (`philharmonic/src/server/reload.rs`)

```rust
/// A handle that watches for SIGHUP and notifies waiters.
pub struct ReloadHandle { /* ... */ }

impl ReloadHandle {
    /// Create a new handle and spawn the signal listener task.
    /// Must be called from within a tokio runtime.
    pub fn new() -> std::io::Result<Self>;

    /// Wait for the next SIGHUP. Returns immediately if one has
    /// been received since the last call.
    pub async fn notified(&self);
}
```

- Uses `tokio::signal::unix::signal(SignalKind::hangup())`.
- Internally uses `tokio::sync::Notify` to fan out to
  multiple waiters.
- The signal listener task runs in the background; dropping
  all `ReloadHandle` clones stops it (use a
  `CancellationToken` or `Arc` + `Weak` pattern).
- `ReloadHandle` must be `Clone + Send + Sync`.

#### 3. Clap CLI skeleton (`philharmonic/src/server/cli.rs`)

A reusable Clap `Command` builder that each bin target extends:

```rust
/// Base CLI arguments shared across all Philharmonic bin targets.
#[derive(clap::Parser)]
pub struct BaseArgs {
    /// Path to the primary TOML config file.
    #[arg(long, short = 'c')]
    pub config: Option<PathBuf>,

    /// Path to the drop-in config directory.
    #[arg(long)]
    pub config_dir: Option<PathBuf>,

    /// Socket address to bind to (overrides config file).
    #[arg(long, short = 'b')]
    pub bind: Option<SocketAddr>,
}

/// Top-level subcommands shared across bins.
#[derive(clap::Subcommand)]
pub enum BaseCommand {
    /// Start the server (default if no subcommand given).
    Serve(BaseArgs),
    /// Print version information and exit.
    Version,
}
```

- Use `clap` with derive macros. Add `clap` with
  `features = ["derive"]` to `philharmonic/Cargo.toml`.
- The `BaseArgs` and `BaseCommand` are designed to be
  embedded/extended by each bin's own Args struct via
  `#[command(flatten)]` or similar. Keep them minimal —
  bin-specific args (like SCK path, signing key path) are
  added by the bin, not here.
- Include a `resolve_config_paths(name: &str, args: &BaseArgs)`
  helper that returns `(primary_path, drop_in_dir)` — if
  `args.config` is `Some`, use that; otherwise default to
  `/etc/philharmonic/<name>.toml` and
  `/etc/philharmonic/<name>.toml.d/`.

#### 4. Module root (`philharmonic/src/server/mod.rs`)

Re-export the three sub-modules:

```rust
pub mod cli;
pub mod config;
pub mod reload;
```

#### 5. Wire into `philharmonic/src/lib.rs`

Add `pub mod server;` to the existing lib.rs (after the
re-exports). No feature gate — the server module is always
available.

#### 6. Tests

- `config.rs` tests:
  - Load a single TOML file into a struct.
  - Load with a drop-in overlay that overrides one key.
  - Drop-in directory doesn't exist → graceful skip.
  - Primary file doesn't exist → error.
  - Invalid TOML → error.
- `reload.rs` test:
  - Create a `ReloadHandle`, send SIGHUP to the current
    process, assert `notified()` completes.
- `cli.rs` test:
  - Parse `serve -c /tmp/foo.toml --bind 127.0.0.1:8080`
    and verify fields.
  - Parse `version` subcommand.
  - `resolve_config_paths` with and without overrides.

### Out of scope

- **Bin targets** — added in the next round.
- **`install` subcommand** — deferred (ROADMAP says can slip
  past 5/2).
- **TLS cert loading / refresh logic** — the `mechanics`
  crate already has `TlsConfig::from_pem`. The reload loop
  that re-reads cert files on SIGHUP belongs in the bin
  target, not in this shared module. This module only
  provides the `ReloadHandle` notification primitive.
- **Actual config struct definitions** — each bin defines its
  own config struct. This module provides the generic loader.

### Dependencies to add to `philharmonic/Cargo.toml`

```toml
clap = { version = "4", features = ["derive"] }
toml = "0.8"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["signal", "sync", "rt"] }
```

`serde` and `tokio` are likely already transitive deps but
must be declared direct since we use them in our own code.

## Outcome

Completed cleanly. Codex produced all four files matching the
spec:

- `philharmonic/src/server/mod.rs` — re-exports.
- `philharmonic/src/server/cli.rs` — `BaseArgs` (clap::Args),
  `BaseCommand` (Subcommand), `resolve_config_paths`. 4 tests.
- `philharmonic/src/server/config.rs` — `load_config<T>`,
  `ConfigError` with Io/Parse/Merge, `read_table`, drop-in
  overlay merge. 5 tests.
- `philharmonic/src/server/reload.rs` — `ReloadHandle` with
  `Arc<Weak>` shutdown, `AtomicU64` generation counter,
  `Notify` fan-out. 1 test (`#[ignore]`, sends real SIGHUP).

Also added `clap 4 (derive)`, `toml 0.8`, `serde 1 (derive)`,
`tokio 1 (signal, sync, rt)` to `philharmonic/Cargo.toml` and
`pub mod server;` to `lib.rs`.

Claude review: no issues. All 9 non-ignored tests pass; clippy
clean; compiles with `--features https` and
`--no-default-features`.

Committed as philharmonic `8f5262d`, parent `c39da59`.
Did NOT run push-all.sh (Claude pushed after review).

---

## Prompt (verbatim)

<task>
You are working inside the `philharmonic-workspace` Cargo workspace.
Your target crate is `philharmonic` (the meta-crate) at
`philharmonic/` — it's a git submodule with its own repo.

## What to build

Create a `server` module at `philharmonic/src/server/` with three
sub-modules providing shared infrastructure for the bin targets
that will be added later. The module is library code inside the
meta-crate, not a bin target itself.

### 1. `server::config` — generic TOML config loader

File: `philharmonic/src/server/config.rs`

```rust
use std::path::Path;

/// Load a TOML config from `primary`, then merge any `.toml` files
/// in `drop_in_dir` (lexicographic order, top-level keys override).
/// Deserialize the merged result into `T`.
pub fn load_config<T: serde::de::DeserializeOwned>(
    primary: &Path,
    drop_in_dir: &Path,
) -> Result<T, ConfigError>;
```

`ConfigError` is a public enum:
- `Io(std::io::Error)` — file read failed.
- `Parse { path: std::path::PathBuf, source: toml::de::Error }` —
  TOML parse failed (include which file).
- `Merge(String)` — merge-level error (if needed; can be folded
  into Parse).

Merge strategy:
1. Read and parse `primary` as `toml::Value::Table`.
2. Read `drop_in_dir`, list `*.toml` files, sort by filename.
3. For each drop-in file, parse as `Table`, merge top-level keys
   into the primary table (drop-in wins on conflict).
4. Deserialize the merged `Table` into `T` via
   `T::deserialize(toml::Value::Table(merged))`.

Edge cases:
- Primary file missing → `ConfigError::Io`.
- Drop-in directory missing → skip overlay, no error.
- Drop-in file parse error → `ConfigError::Parse` with the
  offending path.
- Empty primary (valid TOML, zero keys) → valid, drop-ins still
  apply.

### 2. `server::reload` — SIGHUP notification

File: `philharmonic/src/server/reload.rs`

```rust
/// Watches for SIGHUP signals and notifies async waiters.
#[derive(Clone)]
pub struct ReloadHandle { /* ... */ }

impl ReloadHandle {
    /// Spawn a background task that listens for SIGHUP.
    /// Must be called within a tokio runtime.
    pub fn new() -> std::io::Result<Self>;

    /// Wait until the next SIGHUP arrives after this call.
    pub async fn notified(&self);
}
```

Implementation:
- `tokio::signal::unix::signal(SignalKind::hangup())` in a
  spawned task.
- `tokio::sync::Notify` for fan-out.
- Dropping all clones should stop the background task. Use
  `Arc<Notify>` + `tokio_util::sync::CancellationToken` or
  `Arc`/`Weak` so the spawned task exits when no handles remain.

### 3. `server::cli` — Clap CLI skeleton

File: `philharmonic/src/server/cli.rs`

```rust
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct BaseArgs {
    /// Path to the primary TOML config file.
    #[arg(long, short = 'c')]
    pub config: Option<PathBuf>,

    /// Path to the drop-in config directory.
    #[arg(long)]
    pub config_dir: Option<PathBuf>,

    /// Socket address to bind (overrides config file value).
    #[arg(long, short = 'b')]
    pub bind: Option<SocketAddr>,
}

#[derive(clap::Subcommand)]
pub enum BaseCommand {
    /// Start the server (default when no subcommand is given).
    Serve(BaseArgs),
    /// Print version information.
    Version,
}

/// Resolve config file paths from CLI args or defaults.
/// `name` is the service name (e.g. "mechanics", "api").
/// Returns (primary_config_path, drop_in_dir_path).
pub fn resolve_config_paths(
    name: &str,
    args: &BaseArgs,
) -> (PathBuf, PathBuf) {
    let primary = args.config.clone().unwrap_or_else(||
        PathBuf::from(format!("/etc/philharmonic/{name}.toml"))
    );
    let drop_in = args.config_dir.clone().unwrap_or_else(||
        PathBuf::from(format!("/etc/philharmonic/{name}.toml.d"))
    );
    (primary, drop_in)
}
```

### 4. Module root

File: `philharmonic/src/server/mod.rs`

```rust
pub mod cli;
pub mod config;
pub mod reload;
```

### 5. Wire into lib.rs

Add `pub mod server;` to `philharmonic/src/lib.rs` — after the
existing `pub use` re-exports. No feature gate.

### 6. Dependencies

Add to `philharmonic/Cargo.toml` `[dependencies]`:

```toml
clap = { version = "4", features = ["derive"] }
toml = "0.8"
serde = { version = "1", features = ["derive"] }
```

`tokio` is already a transitive dep but we use it directly in
`reload.rs`, so add it as a direct dep too:

```toml
tokio = { version = "1", features = ["signal", "sync", "rt"] }
```

### 7. Tests

Write tests as `#[cfg(test)] mod tests` inside each file:

**config.rs tests:**
- `load_single_file` — write a temp TOML, load into a test
  struct, verify values.
- `load_with_drop_in_override` — primary has `port = 3000`,
  drop-in has `port = 4000`, verify drop-in wins.
- `drop_in_dir_missing` — load with a nonexistent drop-in dir,
  verify success (primary values used).
- `primary_missing` — verify `ConfigError::Io`.
- `invalid_toml` — verify `ConfigError::Parse`.

**reload.rs tests:**
- `sighup_notifies` — create handle, send `SIGHUP` to self
  (`nix::sys::signal::kill` or `libc::kill`), assert
  `notified()` completes within a timeout. Use
  `tokio::time::timeout`. Mark `#[ignore]` since it sends a
  real signal.

**cli.rs tests:**
- `parse_serve_args` — parse
  `["test", "serve", "-c", "/tmp/foo.toml", "-b", "127.0.0.1:8080"]`,
  verify fields.
- `parse_version` — parse `["test", "version"]`, verify variant.
- `resolve_defaults` — call with empty `BaseArgs`, verify
  `/etc/philharmonic/<name>.toml` paths.
- `resolve_overrides` — call with populated args, verify override
  paths.

## Rules

- Commit via `./scripts/commit-all.sh "message"` only. Never
  raw `git commit` or `git push`. Do NOT run `push-all.sh`.
- Run `./scripts/pre-landing.sh` before the final commit.
  It auto-detects dirty crates and runs fmt + check + clippy
  (-D warnings) + test. Fix any failures before committing.
- No `.unwrap()` / `.expect()` on `Result`/`Option` in library
  src (tests are exempt). Use `?` or proper error types.
- No `println!` / `eprintln!` / `tracing` in library code.
- `ConfigError` must implement `std::fmt::Display` and
  `std::error::Error`.
- Keep the module focused: no bin-target logic, no
  config-struct definitions (those belong in the bin targets),
  no TLS cert loading (the bins do that with `ReloadHandle` +
  `TlsConfig::from_pem`).
- Use `CARGO_TARGET_DIR=target-main` if you run any cargo
  commands outside the scripts.

## Authoritative references

- `CONTRIBUTING.md` — workspace conventions. If anything in this
  prompt contradicts CONTRIBUTING.md, the doc wins.
- `ROADMAP.md` §Phase 9 — "Shared server module" paragraph.

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
added later" stubs. Every type and function described above must
be fully implemented with real logic and real tests. If a design
decision arises that isn't covered by the prompt, make a
reasonable choice, document it in a code comment, and note it in
your summary.
</completeness_contract>

<verification_loop>
Before your final commit:
1. `./scripts/pre-landing.sh` — must pass (fmt + check + clippy
   -D warnings + test).
2. `CARGO_TARGET_DIR=target-main cargo check -p philharmonic
   --features https` — verify the https feature still compiles
   with the new deps.
3. `CARGO_TARGET_DIR=target-main cargo check -p philharmonic
   --no-default-features` — verify minimal feature set compiles.
4. All tests pass.

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
