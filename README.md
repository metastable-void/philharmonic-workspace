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

## Notes for humans
Notes for humans live at HUMANS.md. Claude Code can and must
commit human-made changes to it on every commit the agent makes.
Normal `./scripts/commit-all.sh` invocations will do that.

Coding agents (Claude Code and Codex) can freely read the contents
of HUMANS.md, but they MUST NOT change the contents there.

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
├── philharmonic-api/                          # submodule
└── philharmonic/                              # submodule
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

**Meta-crate:**
- `philharmonic` — meta crate

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
./scripts/setup.sh
```

`setup.sh` initializes all submodules recursively and configures
`push.recurseSubmodules=check` on the local Git config (required
for `push-all.sh`'s safety rail). It's idempotent — re-run any
time.

If you already cloned without `--recurse-submodules`, just run
`./scripts/setup.sh` from the workspace root; it handles the
`git submodule update --init --recursive` step for you.

## Development workflow

Open the repository root in your editor. `rust-analyzer` and other
IDE tooling see the workspace through `Cargo.toml` and provide
cross-crate navigation, refactoring, and type hints exactly as they
would for a single-repo workspace.

**Git state changes go through the workspace scripts, never through
raw `git`.** The repository has ~23 submodules; the scripts encode
submodule-first commit/push ordering, mandatory `-s` signoff, and
detached-HEAD guards that raw `git commit`/`git push` skip. They're
the only supported path — not a convenience layer. This applies to
every contributor (humans and AI agents alike).

Read-only git is fine for history browsing (`git log`,
`git diff`, `git show`, `git status`, `git blame`,
`git rev-parse`). The prohibition is on state-changing
operations (`commit`, `push`, `add` outside what the scripts do,
`reset`, `rebase`, branch create/delete, etc.) — and there's one
read-only exception: use `./scripts/heads.sh`, not raw
`git log -n 1`, to show the current HEAD commit across the
workspace.
If a script doesn't cover a case you need, extend the script
rather than reaching for raw git — see
`docs/design/13-conventions.md §Git workflow`.

Typical flow for a cross-crate change:

1. Edit whatever files the change touches, across the submodule
   directories it spans. The working tree can stay dirty; don't
   manually `git add` or `git commit`.
2. `./scripts/commit-all.sh "message"` from the workspace root —
   walks each dirty submodule, commits there with `-s` signoff,
   then commits the parent (bumping every touched submodule's
   pointer) with the same message.
3. `./scripts/push-all.sh` — pushes submodules first, then the
   parent. A submodule push failure aborts before the parent is
   pushed, so origin never sees a parent pointer referencing an
   unpushed submodule commit.

For parent-only changes (docs, ROADMAP, the scripts themselves),
use `./scripts/commit-all.sh --parent-only "message"`.

### The workspace scripts

All live under `scripts/` at the repo root. POSIX sh (`#!/bin/sh`);
work on Linux (including Alpine/busybox), FreeBSD, and macOS.
Invoke by path (`./scripts/foo.sh`), not via `bash`.

- `./scripts/setup.sh` — one-time (or post-fresh-clone).
  Initializes every submodule recursively; sets
  `push.recurseSubmodules=check`; warns if Rust isn't on PATH.
  Idempotent.
- `./scripts/status.sh` — working-tree status of the parent and
  every submodule (clean submodules are hidden). Run at the
  start of a session.
- `./scripts/pull-all.sh` — rebase-pull the parent and update
  each submodule to the tip of its tracked remote branch. Does
  *not* commit the bumped submodule pointers.
- `./scripts/commit-all.sh [--parent-only] [message]` — commit
  pending changes. Walks each dirty submodule first (committing
  with `-s`), then the parent.
- `./scripts/push-all.sh` — push each submodule's current
  branch, then the parent. Aborts before pushing the parent if
  any submodule push fails.
- `./scripts/codex-status.sh` — list Codex processes spawned by
  Claude Code and their descendants.
- `./scripts/heads.sh` — show the current HEAD commit for the
  parent and every submodule, with short SHA, signature
  indicator, and subject. Use after `commit-all.sh` /
  `push-all.sh` to verify every commit carries a cryptographic
  signature. Canonical replacement for raw `git log -n 1` across
  repos.
- `./scripts/rust-lint.sh [<crate>]` — workspace-wide (or
  per-crate) `cargo fmt --check` + `cargo check` + `cargo
  clippy --all-targets -- -D warnings`. Canonical pre-landing
  lint pass.
- `./scripts/rust-test.sh [--include-ignored|--ignored] [<crate>]` —
  `cargo test`, workspace or per-crate, with optional
  `#[ignore]` control. Default skips `#[ignore]`'d tests;
  `--ignored` runs only them (use per modified crate to exercise
  its integration tests); `--include-ignored` runs everything.
- `./scripts/test-scripts.sh` — POSIX-parse-checks every
  `scripts/*.sh` with `dash -n` (fallback `sh -n`). Mandatory
  after any script change; GitHub CI runs the same check.
- `./scripts/check-api-breakage.sh [baseline-rev]` — run
  `cargo-semver-checks --workspace --all-features` against a git
  baseline (defaults to `origin/main`). Installs
  `cargo-semver-checks` on first run.
- `./scripts/publish-crate.sh [--dry-run] <crate>` — publish one
  crate to crates.io and tag the release inside the submodule.
  Tags are created only after `cargo publish` succeeds.

See `docs/design/13-conventions.md §Shell scripts` for the POSIX-
sh conventions the scripts follow and explicit deviations.

### Recommended Git configuration

`./scripts/setup.sh` sets `push.recurseSubmodules=check` — the
guardrail that makes `push-all.sh` safe. The settings below are
ergonomic (not safety-critical) and worth setting manually once:

```bash
git config status.submoduleSummary true
git config diff.submodule log
git config fetch.recurseSubmodules on-demand
```

They make `git status` / `git diff` show submodule context and
let `git fetch` pick up submodule updates on demand — useful for
the read-only git the scripts invoke under the hood.

## AI-assisted development

This project is developed primarily with AI pair-programming, and
with a deliberate split between two agents:

- **Claude Code** — designer, reviewer, and workspace caretaker.
  Owns architecture, API shape, design docs, ROADMAP updates,
  `Cargo.toml` plumbing, submodule wrangling, and review of
  Codex's output. Git operations (commit, push) go through the
  workspace helper scripts, driven by Claude.
- **Codex CLI** — implementation partner. Claude spawns Codex
  (via the Claude-Code `codex:` plugin) for substantive coding:
  a crate's implementation, an algorithm, a connector, a
  real-sized test suite. Claude writes the prompt; Codex writes
  the code; Claude reviews.

The rule of thumb: if the question is "what should this look
like?", Claude answers. If the question is "now write the
thing", Claude hands off to Codex, unless the work is plumbing or
housekeeping that doesn't warrant the round-trip.

The two agents read different instruction files:

- `CLAUDE.md` at the repo root → Claude Code's conventions and
  session bootstrap.
- `AGENTS.md` at the repo root → Codex's conventions (auto-
  loaded by Codex when it runs here).
- `.codex/config.toml` → project-local Codex CLI settings,
  activated by pointing `CODEX_HOME` at `.codex/`.

Every Codex prompt Claude writes is archived under
`docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>[-NN].md` and committed
*before* Codex is spawned. That makes the design-to-
implementation path reviewable after the fact, which is hard to
reconstruct from chat history alone.

Significant observations Claude makes during a session — audit
results, platform caveats, mid-implementation design calls — go
to `docs/notes-to-humans/YYYY-MM-DD-NNNN-<slug>[-NN].md`, committed
in the same session. Session scrollback is disposable; the notes
directory is a durable journal of what was surprising or
important to flag.

Contributions from purely-human workflows are welcome — nothing
in the code requires the AI workflow. But the project's docs,
scripts, and prompt archive assume it, so mirroring the design-
then-implement split (even manually) tends to produce cleaner
history and easier review.

## Building and testing

From the workspace root:

```bash
cargo check --workspace          # fast type-check everything
cargo build --workspace          # build everything
```

### Pre-landing checks (mandatory)

Every commit that touches Rust code must pass all of the
following at the workspace root. This applies equally to humans
and AI agents.

```bash
./scripts/rust-lint.sh                      # fmt-check + check + clippy -D warnings
./scripts/rust-test.sh                      # workspace tests, skips #[ignore]
# Plus, for every crate modified in the commit:
./scripts/rust-test.sh --ignored <crate>    # its #[ignore]-gated integration tests
```

- `./scripts/rust-lint.sh` runs `cargo fmt --all --check`, then
  `cargo check --workspace`, then `cargo clippy --workspace
  --all-targets -- -D warnings`. Pass a crate name as the only
  argument to scope to one crate. No crate-scope
  `#![allow(...)]`; only narrow-scope `#[allow(clippy::<lint>)]`
  with a one-line justification. If fmt-check fails, run
  `cargo fmt --all` to apply and re-run.
- `./scripts/rust-test.sh` runs `cargo test`. Default skips
  `#[ignore]`-gated tests (the fast path). Flags:
  `--ignored` (only ignored), `--include-ignored` (all). Pass a
  crate name as a trailing argument to scope.
- `#[ignore]` is reserved for integration tests that need real
  infrastructure (testcontainers, live services, network). The
  workspace-level run skips them for speed; the per-touched-crate
  `--ignored` run exercises them for crates you actually
  changed.

**Do not run raw `cargo fmt/check/clippy/test`** when the
scripts cover the case. Bespoke cargo invocations (e.g.
`cargo test some_test` for focused debugging) remain fine for
exceptional cases.

Doc-only / script-only / config-only commits may skip these.
Anything that could affect a `.rs` file's compilation or test
outcome must run all phases. See
`docs/design/13-conventions.md §Pre-landing checks`.

Individual crates can also be built standalone from their own
directories, using only that crate's own manifest and dependency
graph. This is important because each submodule is its own published
crate and CI pipeline.

### Standalone crates + workspace integration

Crate manifests use normal versioned dependencies so they remain
independently buildable and publishable from within the submodule
itself.

At the workspace root, this repository uses `[patch.crates-io]` to
redirect Philharmonic crate dependencies to local submodule paths:

```toml
[patch.crates-io]
philharmonic-types = { path = "philharmonic-types" }
philharmonic-store = { path = "philharmonic-store" }
mechanics-core = { path = "mechanics-core" }
```

This gives the best of both modes:
- standalone crate builds from each submodule still work as published
  crates would.
- workspace builds resolve to local sources, so cross-crate changes are
  visible immediately without publishing intermediate versions.

When bumping crate versions, keep semver requirements and local crate
versions in sync so patched local crates satisfy all dependency
constraints.

## Publishing

Each crate is published to crates.io independently. The release
flow is scripted end-to-end; the per-crate tag is created only
after a successful `cargo publish`.

1. Edit `Cargo.toml` inside the crate's submodule directory to
   bump the version. Update the crate's `CHANGELOG.md`. Leave
   the tree dirty — don't commit manually.
2. From the workspace root, `./scripts/commit-all.sh "release
   <crate> vX.Y.Z"`. This commits the submodule (signed off,
   signed) and the parent (bumping the pointer) in one pass.
3. `./scripts/push-all.sh` — pushes the submodule first, then
   the parent. (Tag push happens later, via the same script.)
4. Run the pre-release API-breakage check:

   ```bash
   ./scripts/check-api-breakage.sh v<previous-version>
   ```

   This runs `cargo-semver-checks --workspace --all-features`
   against the previous release tag; installs the tool on first
   use. Investigate any breakage before continuing.
5. Publish (dry-run first):

   ```bash
   ./scripts/publish-crate.sh --dry-run <crate>
   ./scripts/publish-crate.sh           <crate>
   ```

   `publish-crate.sh` runs `cargo publish --dry-run` internally,
   then `cargo publish` (unless `--dry-run` was passed), then
   creates a signed annotated tag `vX.Y.Z` inside the submodule
   repo. A failed publish does not leave a dangling tag.
6. `./scripts/push-all.sh` — pushes the new tag (it uses
   `git push --follow-tags`; the tag travels alongside the
   branch).
7. After the crates.io index updates (a minute or two), bump the
   workspace dependency's `version` in the parent's `Cargo.toml`
   to match. Commit and push with
   `./scripts/commit-all.sh --parent-only "bump <crate> dep to vX.Y.Z"`
   and `./scripts/push-all.sh`.

Crates must be published in dependency order: cornerstone first
(`philharmonic-types`), dependents after. `cargo publish` will
refuse if a dependency version isn't yet on crates.io, so the
ordering error surfaces early. For coordinated multi-crate
releases, tooling like `cargo-workspaces` or `release-plz` can
help — but for pre-1.0 infrequent releases, `publish-crate.sh`
run per crate is fine.

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

Declared in workspace policy and mirrored in each crate manifest.

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
