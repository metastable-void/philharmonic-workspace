# Split bin targets from meta-crate into separate in-tree crates

**Date:** 2026-04-30
**Slug:** `split-bins-from-meta-crate`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

All three bin targets (`mechanics-worker`,
`philharmonic-connector`, `philharmonic-api`) are currently
`[[bin]]` entries in the `philharmonic` meta-crate. This means
ALL bins link against ALL deps — including the 2.28 GB bge-m3
ONNX model weights from `philharmonic-connector-impl-embed`.
Only the connector bin needs those weights. The API server and
mechanics worker should be ~20-50 MB, not 2.2 GB.

## Scope

Move the three bin targets into separate in-tree (non-published,
non-submodule) crates under `bins/` at the workspace root.
Each bin crate depends only on what it needs.

## Outcome

Pending — will be updated after Codex run.

---

## Prompt (verbatim)

<task>
You are working inside the `philharmonic-workspace` root.

## IMPORTANT: Do NOT commit or push

**Do NOT run `./scripts/commit-all.sh`.** Do NOT run any git
commit command. Do NOT run `./scripts/push-all.sh`. Leave the
working tree dirty.

## What to build

Split the three bin targets from `philharmonic/` into separate
in-tree crates under `bins/` at the workspace root.

**Read these files first:**
- `philharmonic/Cargo.toml` — current `[[bin]]` entries + deps
- `philharmonic/src/bin/mechanics_worker/main.rs` + `config.rs`
- `philharmonic/src/bin/philharmonic_connector/main.rs` + `config.rs`
- `philharmonic/src/bin/philharmonic_api/main.rs` + all sibling files
- `philharmonic/src/server/` — shared server module (cli, config, reload, install)
- `philharmonic/src/lib.rs` — re-exports
- `xtask/Cargo.toml` — pattern for in-tree non-published crate

### Step 1: Create three crate directories

```
bins/mechanics-worker/
bins/philharmonic-connector/
bins/philharmonic-api-server/
```

### Step 2: Create Cargo.toml for each

Each crate: `publish = false`, `edition = "2024"`,
`rust-version = "1.88"`.

**`bins/mechanics-worker/Cargo.toml`:**
```toml
[package]
name = "mechanics-worker"
version = "0.0.0"
edition = "2024"
rust-version = "1.88"
publish = false

[dependencies]
philharmonic = { path = "../../philharmonic", default-features = false }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
toml = "0.8"
```

The key: `default-features = false` on `philharmonic` so no
connector impls are pulled in. `mechanics-worker` only needs
the `mechanics` re-export and the `server` module from
`philharmonic`.

**`bins/philharmonic-connector/Cargo.toml`:**
```toml
[package]
name = "philharmonic-connector-bin"
version = "0.0.0"
edition = "2024"
rust-version = "1.88"
publish = false

[[bin]]
name = "philharmonic-connector"
path = "src/main.rs"

[dependencies]
philharmonic = { path = "../../philharmonic" }
# ^^^ default features = ALL connector impls including embed
async-trait = "0.1"
axum = "0.8"
clap = { version = "4", features = ["derive"] }
hex = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tokio-rustls = { version = "0.26", optional = true }
toml = "0.8"
x25519-dalek = { version = "2", features = ["static_secrets"] }
zeroize = { version = "1", features = ["derive"] }

[features]
default = []
https = ["philharmonic/https", "dep:tokio-rustls"]
```

This is the ONLY bin that enables default features (which
include `connector-embed` with the 2.28 GB model weights).

**`bins/philharmonic-api-server/Cargo.toml`:**
```toml
[package]
name = "philharmonic-api-server"
version = "0.0.0"
edition = "2024"
rust-version = "1.88"
publish = false

[[bin]]
name = "philharmonic-api"
path = "src/main.rs"

[dependencies]
philharmonic = { path = "../../philharmonic", default-features = false }
async-trait = "0.1"
axum = "0.8"
clap = { version = "4", features = ["derive"] }
coset = "0.4"
hex = "0.4"
http = "1"
hyper-util = { version = "0.1", features = ["full"] }
rand_core = { version = "0.6", features = ["getrandom"] }
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls"] }
rust-embed = "8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tokio-rustls = { version = "0.26", optional = true }
toml = "0.8"
tower = "0.5"
zeroize = { version = "1", features = ["derive"] }

[features]
default = []
https = ["philharmonic/https", "dep:tokio-rustls"]
```

No connector impls → no embed weights → ~30 MB binary.

### Step 3: Copy source files

Copy (not move — the originals stay until we verify) each
bin's source files:

- `philharmonic/src/bin/mechanics_worker/*.rs`
  → `bins/mechanics-worker/src/*.rs`
- `philharmonic/src/bin/philharmonic_connector/*.rs`
  → `bins/philharmonic-connector/src/*.rs`
- `philharmonic/src/bin/philharmonic_api/*.rs`
  → `bins/philharmonic-api-server/src/*.rs`

### Step 4: Fix imports

The source files currently use `use philharmonic::...` which
still works since they depend on the `philharmonic` crate.
No import changes should be needed.

The `webui.rs` file uses `#[folder = "webui/dist/"]` for
`rust-embed`. This path is relative to the crate root. Since
the API server is now at `bins/philharmonic-api-server/`, the
path needs to be `../../philharmonic/webui/dist/`.

### Step 5: Add to workspace members

In root `Cargo.toml`, add under `[workspace] members`:
```toml
"bins/mechanics-worker",
"bins/philharmonic-connector",
"bins/philharmonic-api-server",
```

### Step 6: Remove `[[bin]]` from meta-crate

Remove all three `[[bin]]` entries from
`philharmonic/Cargo.toml`. Also remove dependencies that
were only needed by the bins (keep deps used by `lib.rs` and
`src/server/`):

The meta-crate `philharmonic/Cargo.toml` should keep:
- Re-exported library crate deps (types, store, policy, etc.)
- `server` module deps: `clap`, `serde`, `toml`, `tokio`

Remove from the meta-crate (bin-only deps):
- `async-trait` (used by lowerer/executor)
- `axum` (used by connector/API bins)
- `coset` (used by lowerer)
- `hex` (used by key loading)
- `http` (used by scope resolver)
- `hyper-util` (used by TLS)
- `rand_core` (used by lowerer)
- `reqwest` (used by executor)
- `rust-embed` (used by webui)
- `serde_json` (check if server module uses it)
- `tower` (used by API bin)
- `x25519-dalek` (used by connector bin)
- `zeroize` (used by key loading)
- `tokio-rustls` (used by TLS bins)

**Be careful**: check if `src/server/*.rs` or `src/lib.rs` uses
any of these before removing. Read the files first.

### Step 7: Verify

1. `CARGO_TARGET_DIR=target-main cargo build -p mechanics-worker`
2. `CARGO_TARGET_DIR=target-main cargo build -p philharmonic-connector-bin`
3. `CARGO_TARGET_DIR=target-main cargo build -p philharmonic-api-server`
4. `CARGO_TARGET_DIR=target-main cargo clippy -p mechanics-worker
   -p philharmonic-connector-bin -p philharmonic-api-server
   -- -D warnings`
5. Each binary: `version` and `--help`

## Rules

- **Do NOT commit, push, or publish.**
- You MAY modify `philharmonic/Cargo.toml` to remove bin-only deps.
- You MAY modify the root `Cargo.toml` to add workspace members.
- Do NOT modify library crate source (src/lib.rs, src/server/*).
- Do NOT delete the old bin sources from philharmonic/src/bin/ yet
  — just leave them; Claude will clean up after verification.
</task>

<structured_output_contract>
1. Files created or modified.
2. All verification commands and pass/fail.
3. Binary sizes if available.
4. Confirmation: no commit, no push.
</structured_output_contract>

<completeness_contract>
All three bins must compile and run (version, --help). No TODOs.
</completeness_contract>

<verification_loop>
1. All three bins compile.
2. Clippy clean.
3. `version` and `--help` work for each.
4. The meta-crate `philharmonic` still compiles as a library.
</verification_loop>

<action_safety>
- Do NOT commit. Do NOT push. Do NOT publish.
- Do NOT delete old bin sources from philharmonic/src/bin/.
</action_safety>
