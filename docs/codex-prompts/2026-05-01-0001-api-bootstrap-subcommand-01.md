# API bootstrap subcommand (initial dispatch)

**Date:** 2026-05-01
**Slug:** `api-bootstrap-subcommand`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

The API server requires a `pht_` long-lived token to authenticate
any request (including operator-level tenant management). But
creating a Principal entity requires an already-authenticated
request — chicken-and-egg. A `bootstrap` subcommand seeds the
database with the first operator tenant + principal and prints the
one-time `pht_` token to stdout, unblocking first deployment.

## References

- `bins/philharmonic-api-server/src/main.rs` — CLI dispatch,
  `BaseCommand` enum, `serve()`, entity creation patterns
- `philharmonic/src/server/cli.rs` — `BaseCommand` enum definition
- `philharmonic-api/src/routes/principals.rs` lines 72-115 —
  `create_principal` handler (entity + token creation pattern)
- `philharmonic-api/src/routes/operator.rs` lines 65-106 —
  `create_tenant` handler (tenant entity creation pattern)
- `philharmonic-api/src/middleware/auth.rs` — `pht_` token auth
  flow, credential_hash lookup
- `philharmonic-policy/src/token.rs` — `generate_api_token()`,
  `TOKEN_PREFIX`
- `philharmonic-store/src/ext.rs` — `create_entity_minting<T>()`

## Context files pointed at

- `bins/philharmonic-api-server/src/main.rs`
- `bins/philharmonic-api-server/src/config.rs`
- `philharmonic/src/server/cli.rs`
- `philharmonic-api/src/routes/principals.rs`
- `philharmonic-api/src/routes/operator.rs`
- `philharmonic-policy/src/token.rs`
- `philharmonic-store-sqlx-mysql/src/schema.rs`

## Outcome

Pending — will be updated after Codex run.

---

## Prompt (verbatim)

<task>
Add a `bootstrap` subcommand to the `philharmonic-api` binary that
seeds the database with the initial operator tenant and principal,
printing the one-time `pht_` API token to stdout.

## Why

The API server authenticates every non-meta request with a bearer
token. Long-lived tokens (`pht_*`) are looked up by credential hash
in the substrate. Creating a Principal requires an authenticated
request — chicken-and-egg problem. The bootstrap command breaks the
cycle for first-time deployment.

## What to implement

### 1. Add `Bootstrap` variant to `BaseCommand`

In `philharmonic/src/server/cli.rs`, add:

```rust
/// Bootstrap the database with an initial operator tenant and
/// principal. Prints the one-time API token to stdout.
Bootstrap(BootstrapArgs),
```

Add a new struct:

```rust
#[derive(Clone, Debug, clap::Args)]
pub struct BootstrapArgs {
    /// Path to the primary TOML config file.
    #[arg(long, short = 'c')]
    pub config: Option<PathBuf>,

    /// Path to the drop-in config directory.
    #[arg(long)]
    pub config_dir: Option<PathBuf>,

    /// Display name for the bootstrap tenant.
    #[arg(long, default_value = "Operator")]
    pub tenant_name: String,

    /// Subdomain name for the bootstrap tenant.
    #[arg(long, default_value = "operator")]
    pub subdomain_name: String,
}
```

### 2. Wire up in `bins/philharmonic-api-server/src/main.rs`

In the `run()` function's match, add:

```rust
BaseCommand::Bootstrap(args) => bootstrap(args).await,
```

### 3. Implement `bootstrap()` function

Add an `async fn bootstrap(args: BootstrapArgs) -> Result<(), String>`
in `main.rs`. It should:

a) Load config the same way `serve()` does (via
   `resolve_config_paths("api", ...)` — but BootstrapArgs has its
   own `config` and `config_dir` fields, NOT BaseArgs. Build the
   paths manually using the same logic: default to
   `/etc/philharmonic/api.toml` if `args.config` is None, and
   `/etc/philharmonic/api.toml.d/` if `args.config_dir` is None.
   Use `load_config::<ApiConfig>()` the same way `load_api_config`
   does, falling back to `ApiConfig::default()` if the file is not
   found and no --config was explicitly passed.

b) Connect to the database via `SinglePool::connect(&config.database_url)`.

c) Run schema migration via `migrate(pool.pool())`.

d) Check that no Principal entities exist yet. Use
   `pool.pool()` to run a raw sqlx query:
   ```rust
   let count: (i64,) = sqlx::query_as(
       "SELECT COUNT(*) FROM entity WHERE kind = ?"
   )
   .bind(Principal::KIND.as_bytes().as_slice())
   .execute(pool.pool())
   .await
   ```
   If count > 0, print an error message
   "bootstrap: database already has principal entities; refusing to
   re-bootstrap" and return `Err(...)`.

e) Create a Tenant entity using the SqlStore:
   ```rust
   let store = SqlStore::from_pool(pool.pool().clone());
   ```
   Then follow the exact same pattern as `create_tenant` in
   `philharmonic-api/src/routes/operator.rs` lines 74-95:
   - `store.create_entity_minting::<Tenant>()`
   - Put display_name as JSON string content
   - Put settings as `{"subdomain_name": "..."}` content
   - Append revision with content attrs `display_name`, `settings`
     and scalar `status` = `TenantStatus::Active.as_i64()`

f) Create a Principal entity following the exact same pattern as
   `create_principal` in
   `philharmonic-api/src/routes/principals.rs` lines 79-104:
   - `store.create_entity_minting::<Principal>()`
   - `generate_api_token()` to get `(token, token_hash)`
   - Store `credential_hash` as content (the token_hash bytes
     wrapped in `ContentValue::new(token_hash.0.to_vec()).digest()`,
     then stored via `store.put_content(content_value)`)
   - Store `display_name` as content (JSON string `"Bootstrap Operator"`)
   - Append revision with:
     - content: `credential_hash`, `display_name`
     - entity ref: `tenant` pointing to the tenant entity
       (`EntityRefValue::pinned(tenant_id.internal().as_uuid(), 0)`)
     - scalars: `kind` = `PrincipalKind::Operator.as_i64()`,
       `epoch` = 0i64, `is_retired` = false

g) Print to stdout (NOT stderr):
   ```
   Bootstrap complete.

   Tenant ID: <public UUID>
   Principal ID: <public UUID>

   API token (save this — it will not be shown again):
     <pht_...token...>
   ```

   The token is `token.as_str()` from the `Zeroizing<String>`
   returned by `generate_api_token()`.

### 4. Imports

You will need these imports (some already present in main.rs):

From `philharmonic::policy`:
- `generate_api_token`, `Principal`, `PrincipalKind`, `Tenant`,
  `TenantStatus`

From `philharmonic::store`:
- `EntityStoreExt`, `RevisionInput`, `StoreExt`

From `philharmonic::types`:
- `ContentValue`, `Entity`, `EntityRefValue`, `JsonValue`,
  `ScalarValue`

From `philharmonic::server::cli`:
- `BootstrapArgs`

### 5. Important constraints

- **No panics in library code.** But this is a bin, so `.unwrap()`
  on programmer errors is acceptable. Still prefer `map_err` with
  descriptive messages where failure is plausible (DB errors, IO).
- The `PrincipalKind` enum: check
  `philharmonic-policy/src/principal.rs` for the exact type name
  and `as_i64()` method. If it's not `PrincipalKind::Operator`,
  find the right variant — there should be an operator/admin kind.
  If there's no distinct operator kind, use the standard
  principal kind (likely value 0 or whatever the default is).
- Use `serde_json::json!()` for building the JSON values, same as
  the operator route does.
- The `put_content` method on `EntityStore` or `StoreExt` takes a
  `ContentValue` and returns a content hash. Look at how
  `put_json` works in `philharmonic-api/src/routes/identity.rs`
  and replicate that pattern. You may need to use the store
  directly: `store.put_content(ContentValue::new(bytes))` where
  bytes is the JSON-serialized value.
- For the `credential_hash` content: look at `put_token_hash` in
  `principals.rs` for the exact pattern. The token hash bytes
  (32 bytes) are wrapped in `ContentValue::new()`, digested to
  get the content hash, then stored. Replicate this exactly.

## Verification

After implementing:

1. Run `./scripts/pre-landing.sh` — it auto-detects touched crates
   and runs fmt + check + clippy (-D warnings) + test.
2. Do NOT run raw `cargo fmt/check/clippy/test` — use the scripts.
3. If `pre-landing.sh` finds issues, fix them and re-run.

## Git rules

- Commit via `./scripts/commit-all.sh "<message>"` ONLY.
- Do NOT run `./scripts/push-all.sh` or `cargo publish`.
- Do NOT run raw `git commit` / `git add` / `git push`.
- All commits are automatically signed-off and signed by the
  commit hooks.

## What's out of scope

- Do NOT modify any existing routes or middleware.
- Do NOT add a new HTTP endpoint — this is CLI-only.
- Do NOT modify any library crate's `src/lib.rs` except
  `philharmonic/src/server/cli.rs` (for the new enum variant).
- Do NOT add tests — this is a deployment bootstrap tool.
</task>

<default_follow_through_policy>
If a step produces warnings, errors, or unexpected output, address
them immediately before proceeding to the next step. Do not defer
fixes to "later".
</default_follow_through_policy>

<completeness_contract>
The task is complete when:
1. `BaseCommand::Bootstrap(BootstrapArgs)` exists in cli.rs
2. `bins/philharmonic-api-server/src/main.rs` handles the
   Bootstrap variant
3. The bootstrap function creates tenant + principal + prints token
4. `./scripts/pre-landing.sh` passes cleanly
5. Changes are committed via `./scripts/commit-all.sh`
</completeness_contract>

<verification_loop>
After each significant code change:
1. Run `./scripts/pre-landing.sh`
2. If it fails, fix and re-run
3. Only commit after a clean pass
Use `./scripts/build-status.sh` if cargo appears stuck (no output
for >60 seconds) to check what's actually building.
</verification_loop>

<missing_context_gating>
If you cannot find a type, method, or pattern referenced in this
prompt, grep for it before inventing alternatives. The codebase is
the authority. If something truly doesn't exist, flag it in your
output rather than guessing.
</missing_context_gating>

<action_safety>
- Never run `./scripts/push-all.sh`
- Never run `cargo publish`
- Never run raw git commands
- Never modify files outside the scope listed above
</action_safety>

<structured_output_contract>
When done, report:
- Summary: what was implemented
- Files touched: list with brief description of changes
- Verification: pre-landing.sh output (pass/fail)
- Residual risks: anything uncertain or worth reviewing
- Git state: commit SHA, branch, pushed=no
</structured_output_contract>
