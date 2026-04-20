# Philharmonic Workspace

Development harness for the Philharmonic crate family — a workflow
orchestration system built as a set of independent Rust crates.

This repository contains the Cargo workspace manifest, shared
development scripts, and Git submodules pointing at each crate's own
repository. Each crate is published independently to crates.io and
has its own issue tracker, CI, and release cycle. The workspace lets
you develop across all of them at once without giving up that
independence.

## About Philharmonic

Philharmonic is a workflow orchestration system with JavaScript-based
workflows, sandboxed execution in stateless Boa runtimes, and per-step
encrypted authorization between scripts and external services. The
storage substrate is append-only and content-addressed; the execution
substrate runs JavaScript jobs as a horizontally-scalable HTTP service;
the connector layer mediates all external I/O under per-realm
isolation with hybrid post-quantum cryptography.

See the design documentation for the full architectural picture.
Start with `docs/01-project-overview.md` (to be published alongside the
crates; currently in the design repo).

## Repository structure

```
philharmonic-workspace/
├── Cargo.toml                                 # workspace manifest
├── scripts/                                   # helper scripts
├── philharmonic-types/                        # submodule
├── philharmonic-store/                        # submodule
├── philharmonic-store-sqlx-mysql/             # submodule
├── mechanics-config/                          # submodule
├── mechanics-core/                            # submodule
├── mechanics/                                 # submodule
├── philharmonic-policy/                       # submodule
├── philharmonic-workflow/                     # submodule
├── philharmonic-connector-common/             # submodule
├── philharmonic-connector-client/             # submodule
├── philharmonic-connector-router/             # submodule
├── philharmonic-connector-service/            # submodule
├── philharmonic-connector-impl-*/             # submodules (one per impl)
└── philharmonic-api/                          # submodule
```

Each submodule is a standalone Git repository at
`github.com/metastable-void/<crate-name>`.

### Crates at a glance

**Core vocabulary:**
- `philharmonic-types` — cornerstone types (`Uuid`, `JsonValue`, `Sha256`, `EntityId<T>`, etc.)

**Storage substrate:**
- `philharmonic-store` — backend-agnostic storage traits
- `philharmonic-store-sqlx-mysql` — MySQL-family backend

**Execution substrate:**
- `mechanics-config` — Boa-free schema types
- `mechanics-core` — JavaScript executor library (Boa-backed)
- `mechanics` — HTTP service wrapping `mechanics-core`

**Policy and workflow:**
- `philharmonic-policy` — tenants, principals, per-tenant endpoint configs, roles, minting authorities
- `philharmonic-workflow` — orchestration engine

**Connector layer:**
- `philharmonic-connector-common` — COSE token formats, realm model, shared types
- `philharmonic-connector-client` — the lowerer (produces per-step encrypted payloads)
- `philharmonic-connector-router` — per-realm HTTP dispatcher
- `philharmonic-connector-service` — service framework for per-realm connector binaries

**Connector implementations** (one crate each):
- `philharmonic-connector-impl-http-forward`
- `philharmonic-connector-impl-llm-openai-compat`
- `philharmonic-connector-impl-llm-anthropic`
- `philharmonic-connector-impl-llm-gemini`
- `philharmonic-connector-impl-sql-postgres`
- `philharmonic-connector-impl-sql-mysql`
- `philharmonic-connector-impl-email-smtp`
- `philharmonic-connector-impl-embed`
- `philharmonic-connector-impl-vector-search`

**API:**
- `philharmonic-api` — public HTTP API

## Status

Design is substantially settled; implementation is in active progress.
Most crates are currently published as `0.0.0` placeholders on
crates.io; substantial implementation is rolling out crate-by-crate
through 2026.

Already published with substantive content:
`philharmonic-types`, `philharmonic-store`,
`philharmonic-store-sqlx-mysql`, `mechanics-core`, `mechanics`.

## Prerequisites

- Rust 1.85 or newer (edition 2024).
- Git 2.30 or newer (for modern submodule semantics).
- A MySQL-family database (MySQL 8, MariaDB 10.5+, or TiDB) for
  running storage backend tests. Containerized setups via Docker
  or Podman work well.

## Cloning

Fresh clone, including submodules:

```bash
git clone --recurse-submodules https://github.com/metastable-void/philharmonic-workspace.git
cd philharmonic-workspace
```

If you already cloned without `--recurse-submodules`:

```bash
cd philharmonic-workspace
git submodule update --init --recursive
```

## Development workflow

Open the repository root in your editor. `rust-analyzer` and other
IDE tooling see the workspace through `Cargo.toml` and provide
cross-crate navigation, refactoring, and type hints exactly as they
would for a single-repo workspace.

Each crate directory is an independent Git repository (a submodule).
When you make changes inside a crate:

1. Work on the crate's files as normal.
2. Commit and push **inside the submodule** on a feature branch.
3. In the parent workspace, `git add` the submodule directory to
   bump the pinned commit.
4. Commit and push the workspace.

For cross-crate changes (a type added in one crate, consumed in
another), repeat the inside-submodule commit for each, then bump all
the submodule pointers in a single workspace commit.

### Helper scripts

The `scripts/` directory contains conveniences:

- `scripts/status.sh` — status across all submodules plus the parent.
- `scripts/pull-all.sh` — update all submodules to their tracked
  branches' latest commits.
- `scripts/push-all.sh` — push uncommitted work across submodules
  before pushing the parent.

Run `scripts/status.sh` at the start of a session to see where
everything stands.

### Recommended Git configuration

Set these once to avoid the most common submodule footguns:

```bash
git config status.submoduleSummary true
git config diff.submodule log
git config fetch.recurseSubmodules on-demand
git config push.recurseSubmodules check
```

The last one — `push.recurseSubmodules=check` — causes
`git push` on the parent to fail if any submodule has commits
that haven't been pushed to their own remotes. This prevents the
single most common submodule error: pushing a workspace pointer
that references a submodule commit nobody else can fetch.

## Building and testing

From the workspace root:

```bash
cargo check --workspace          # fast type-check everything
cargo build --workspace          # build everything
cargo test --workspace           # run all tests
cargo clippy --workspace         # lint everything
```

Individual crates can also be built standalone from their own
directories; each crate's `Cargo.toml` inherits workspace settings
but resolves dependencies with `path` pointing at its siblings, so
local changes in one crate are instantly visible to dependents.

### The dual-dependency-spec pattern

Workspace `[workspace.dependencies]` entries carry both `version` and
`path`:

```toml
philharmonic-types = { version = "0.3.3", path = "philharmonic-types" }
```

Cargo uses `path` for local builds (seeing source changes instantly)
and `version` for publishing (Cargo strips `path` automatically on
`cargo publish`). This is the standard pattern for path-linked
workspaces that publish crates independently.

## Publishing

Each crate is published to crates.io independently from its own
submodule directory:

1. Inside the submodule, bump the version in `Cargo.toml`, commit,
   push, tag the release.
2. `cargo publish --dry-run` to verify.
3. `cargo publish`.
4. After the crates.io index updates (a minute or two), bump the
   workspace dependency's `version` in the parent's `Cargo.toml` to
   match the newly-published version.
5. Commit and push the parent workspace, which also bumps the
   submodule pointer.

Crates must be published in dependency order: cornerstone first
(`philharmonic-types`), dependents after. For coordinated
multi-crate releases, tooling like `cargo-workspaces` or `release-plz`
can help — but for pre-1.0 infrequent releases, the manual process is
fine.

## Design documentation

The full design corpus (architecture, crypto design, entity models,
API surface, v1 scope) lives in a separate design documentation
repository. Implementation should match what's specified there; when
implementation discovers that the docs got something wrong, update
the docs first, then the code. Docs that describe reality are useful;
docs describing an aspirational past are worse than nothing.

## Editions and MSRV

- Edition 2024.
- MSRV 1.85.

Set in workspace-level `[workspace.package]` and inherited by each
crate.

## License

All crates are dual-licensed under `Apache-2.0 OR MPL-2.0` at the
consumer's choice:

- **Apache-2.0**: standard permissive open-source license with patent
  grants.
- **MPL-2.0**: file-level copyleft, FSF-compatible, GPL-2.0+
  compatible via the secondary license clause.

This dual-license combination covers more deployment scenarios than
the common `Apache-2.0 OR MIT` while staying clearly open-source.

Individual crates carry their own `LICENSE` files under each
submodule. The workspace repository itself carries copies for
reference.
