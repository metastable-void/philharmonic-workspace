# Phase 9 task 9 — `install` subcommand for all three bins

**Date:** 2026-04-30
**Slug:** `phase-9-install-subcommand`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Each bin target needs an `install` subcommand that copies the
binary to `/usr/local/bin/`, installs a systemd service unit,
creates config directories, and runs `systemctl enable`. This
enables single-command deployment on GNU/Linux hosts.

## References

- `ROADMAP.md` §Phase 9 — task 9.
- `HUMANS.md` §Integration — "one `install` subcommand
  (requires a root) installs the binary to `/usr/local/bin/`,
  and installs a systemd service unit file at
  `/usr/local/lib/systemd/system/*.service`."
- `philharmonic/src/bin/mechanics_worker/main.rs` — existing
  `BaseCommand` enum with `Serve`/`Version`.
- `philharmonic/src/server/cli.rs` — `BaseCommand` definition.

## Scope

### In scope

#### 1. Add `Install` variant to `BaseCommand`

In `philharmonic/src/server/cli.rs`:

```rust
#[derive(clap::Subcommand)]
pub enum BaseCommand {
    Serve(BaseArgs),
    Version,
    /// Install the binary, systemd unit, and config directory.
    /// Requires root privileges.
    Install(InstallArgs),
}

#[derive(Clone, Debug, clap::Args)]
pub struct InstallArgs {
    /// Override the binary install path (default: /usr/local/bin/).
    #[arg(long, default_value = "/usr/local/bin")]
    pub bin_dir: PathBuf,

    /// Override the systemd unit directory
    /// (default: /usr/local/lib/systemd/system/).
    #[arg(long, default_value = "/usr/local/lib/systemd/system")]
    pub unit_dir: PathBuf,

    /// Override the config directory
    /// (default: /etc/philharmonic/).
    #[arg(long, default_value = "/etc/philharmonic")]
    pub config_dir: PathBuf,

    /// Don't run systemctl enable.
    #[arg(long)]
    pub no_enable: bool,
}
```

#### 2. Shared install logic

In `philharmonic/src/server/install.rs` (new module):

```rust
pub struct InstallPlan {
    pub service_name: String,      // e.g. "mechanics-worker"
    pub binary_name: String,       // e.g. "mechanics-worker"
    pub description: String,
    pub config_file_name: String,  // e.g. "mechanics.toml"
    pub default_bind: String,      // for the example config
    pub args: InstallArgs,
}

pub fn execute_install(plan: &InstallPlan) -> Result<(), String>;
```

The `execute_install` function:

1. Check `geteuid() == 0` (root). If not, print error and
   return Err.
2. Copy the current binary (`std::env::current_exe()`) to
   `{bin_dir}/{binary_name}`. Create `bin_dir` if needed.
3. Write a systemd unit file to
   `{unit_dir}/{service_name}.service`. Create `unit_dir`
   and intermediate directories if needed.
4. Create `{config_dir}/` and `{config_dir}/{config_file_name}.d/`
   if they don't exist.
5. Write a default config file at
   `{config_dir}/{config_file_name}` if it doesn't exist
   (don't overwrite existing config).
6. Run `systemctl enable {service_name}.service` (unless
   `--no-enable`).
7. Print setup instructions at the end.

Idempotent: re-running is safe (overwrites binary + unit,
skips existing config).

#### 3. Systemd unit template

```ini
[Unit]
Description={description}
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={bin_path} serve -c {config_path}
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

#### 4. Wire into each bin's `main.rs`

Each bin handles `BaseCommand::Install(args)` by calling
`execute_install` with the appropriate `InstallPlan`.

### Out of scope

- Uninstall subcommand.
- Windows service support.
- Non-systemd init systems.

## Outcome

Completed. Codex created `server/install.rs` (shared
`InstallPlan` + `execute_install`) and wired `Install` into
all three bins. Also added LFS auto-tracking in the
pre-commit hook for files > 20 MiB. Build passes, `--help`
shows `install` on all three bins. Not tested with root
(requires sudo). Committed as philharmonic `6cfb0de`,
parent `77c10e6`.

---

## Prompt (verbatim)

<task>
You are working inside the `philharmonic-workspace` Cargo workspace.
Your target is the `philharmonic` meta-crate at `philharmonic/`.

## IMPORTANT: Do NOT commit or push

**Do NOT run `./scripts/commit-all.sh`.** Do NOT run any git
commit command. Do NOT run `./scripts/push-all.sh`. Leave the
working tree dirty.

## What to build

Add an `install` subcommand to all three bin targets. The shared
install logic lives in a new `philharmonic/src/server/install.rs`
module. The `Install` variant is added to `BaseCommand` in
`philharmonic/src/server/cli.rs`.

**Read these files first:**
- `philharmonic/src/server/cli.rs` — `BaseCommand` enum
- `philharmonic/src/server/mod.rs` — server module
- `philharmonic/src/bin/mechanics_worker/main.rs`
- `philharmonic/src/bin/philharmonic_connector/main.rs`
- `philharmonic/src/bin/philharmonic_api/main.rs`
- `CONTRIBUTING.md`

### Files to create/modify

1. **`philharmonic/src/server/cli.rs`** — add `Install(InstallArgs)`
   to `BaseCommand`, add `InstallArgs` struct.
2. **`philharmonic/src/server/install.rs`** (new) — shared
   install logic.
3. **`philharmonic/src/server/mod.rs`** — add `pub mod install;`.
4. **Each bin's `main.rs`** — handle `BaseCommand::Install`.

### `InstallArgs`

```rust
#[derive(Clone, Debug, clap::Args)]
pub struct InstallArgs {
    #[arg(long, default_value = "/usr/local/bin")]
    pub bin_dir: std::path::PathBuf,

    #[arg(long, default_value = "/usr/local/lib/systemd/system")]
    pub unit_dir: std::path::PathBuf,

    #[arg(long, default_value = "/etc/philharmonic")]
    pub config_dir: std::path::PathBuf,

    #[arg(long)]
    pub no_enable: bool,
}
```

### `install.rs` — `InstallPlan` + `execute_install`

```rust
pub struct InstallPlan {
    pub service_name: String,
    pub binary_name: String,
    pub description: String,
    pub config_file_name: String,
    pub default_config_content: String,
    pub args: InstallArgs,
}

pub fn execute_install(plan: &InstallPlan) -> Result<(), String> {
    // 1. Check root: unsafe { libc::geteuid() } == 0
    //    Actually, avoid unsafe. Use std::process::Command
    //    to run `id -u` and check if output is "0".
    //    Or read /proc/self/status for Uid line.
    //    Simplest: check if we can write to bin_dir.
    //
    // 2. Copy current exe to bin_dir/binary_name.
    //    std::fs::copy(std::env::current_exe()?, dest)?;
    //    Set permissions: chmod 755.
    //
    // 3. Write systemd unit to unit_dir/service_name.service.
    //    Create unit_dir with create_dir_all.
    //
    // 4. Create config_dir and config_dir/config_file_name.d/.
    //
    // 5. Write default config file if it doesn't exist.
    //
    // 6. If !no_enable: run `systemctl enable service_name`.
    //
    // 7. Print instructions.
}
```

For the root check, use `std::fs::metadata` on `/usr/local/bin`
to check write permission, or simply attempt the copy and report
the permission error if it fails. Do NOT use `unsafe` or `libc`.

### Systemd unit template

```
[Unit]
Description={description}
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={bin_path} serve -c {config_path}
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

Where:
- `{bin_path}` = `{bin_dir}/{binary_name}`
- `{config_path}` = `{config_dir}/{config_file_name}`

### Default config content per bin

**mechanics-worker** (`mechanics.toml`):
```toml
bind = "127.0.0.1:3001"
tokens = []

[pool]
execution_timeout_secs = 3600
run_timeout_secs = 3600
```

**philharmonic-connector** (`connector.toml`):
```toml
bind = "127.0.0.1:3002"
realm_id = "default"
```

**philharmonic-api** (`api.toml`):
```toml
bind = "127.0.0.1:3000"
database_url = "mysql://philharmonic@localhost/philharmonic"
issuer = "philharmonic"
```

### Wiring in each bin

```rust
BaseCommand::Install(args) => {
    server::install::execute_install(&InstallPlan {
        service_name: "mechanics-worker".to_string(),
        binary_name: "mechanics-worker".to_string(),
        description: "Philharmonic mechanics JS executor".to_string(),
        config_file_name: "mechanics.toml".to_string(),
        default_config_content: DEFAULT_CONFIG.to_string(),
        args,
    })
}
```

### Error handling

Print clear error messages for common failures: not root,
can't write to destination, systemctl not found. No
`.unwrap()` — map errors to user-friendly strings.

## Rules

- **Do NOT commit, push, or publish.**
- Use `CARGO_TARGET_DIR=target-main` for raw cargo commands.
- Run from the workspace root.
- You MAY modify `philharmonic/Cargo.toml` if needed.
- Do NOT modify files outside `philharmonic/` except `Cargo.lock`.
- Do NOT use `unsafe` code. No `libc::geteuid()`.
</task>

<structured_output_contract>
When you are done, produce a summary listing:
1. Files created or modified.
2. All verification commands run and their pass/fail status.
3. Confirmation that you did NOT commit or push.
</structured_output_contract>

<completeness_contract>
All three bins must handle the `Install` subcommand. The
install logic must be fully implemented — no TODOs. The binary
must compile and `--help` must show the `install` subcommand.
</completeness_contract>

<verification_loop>
1. `CARGO_TARGET_DIR=target-main cargo build -p philharmonic`
2. `./target-main/debug/mechanics-worker --help` — shows install
3. `./target-main/debug/philharmonic-connector --help` — shows install
4. `./target-main/debug/philharmonic-api --help` — shows install
5. `CARGO_TARGET_DIR=target-main cargo clippy -p philharmonic
   --all-targets -- -D warnings`
</verification_loop>

<action_safety>
- Do NOT commit. Do NOT push. Do NOT publish.
- Do NOT use unsafe code.
- Do NOT modify files outside philharmonic/ except Cargo.lock.
</action_safety>
