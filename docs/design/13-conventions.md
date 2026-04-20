# Workspace Conventions

Workspace-wide practices. Not design decisions about what the system
does, but decisions every crate honors for consistency.

## Licensing

All crates: `Apache-2.0 OR MPL-2.0`.

Both license files at the crate root (`LICENSE-APACHE`,
`LICENSE-MPL`). `Cargo.toml`:

```toml
license = "Apache-2.0 OR MPL-2.0"
```

Rationale: Apache-2.0 for permissive use with patent grants, MPL-2.0
for file-level copyleft with GPL compatibility via secondary license
clause. Covers more deployment scenarios than common
`Apache-2.0 OR MIT` without compromising openness.

## Naming

Pattern: `<subsystem>-<concern>[-<implementation>]`

- Subsystem prefix identifies the project: `philharmonic-`,
  `mechanics-`.
- Concern after prefix names what the crate is for: `-types`,
  `-store`, `-workflow`, `-connector`.
- Implementation suffix for multiple implementations of one concern:
  `philharmonic-store-sqlx-mysql` is "the storage trait, implemented
  via sqlx for MySQL."
- Meta-crate unsuffixed: `philharmonic`, `mechanics`.

Readers can infer relationships from names. `philharmonic-store` is
the trait; `philharmonic-store-sqlx-mysql` is an implementation.

## Crate name claims

Defensive early publishing: reserve crate names on crates.io before
implementations are complete. Use `0.0.0` placeholders with empty or
minimal content.

This prevents name squatting and ensures the namespace is
predictable. When implementation begins, the crate transitions from
placeholder to real content via normal version bumps.

## Versioning

Semantic Versioning with pre-1.0 caveats:

- **Patch (0.x.y → 0.x.(y+1))** — additive changes, bug fixes,
  docs.
- **Minor (0.x.y → 0.(x+1).0)** — changes to existing APIs.
  Pre-1.0 breakage signal.
- **Major (0.x.y → 1.0.0+)** — stability boundary.

The cornerstone (`philharmonic-types`) is on the strict end: many
dependents, so breaking changes are painful. Bundled, announced.

Downstream crates pin cornerstone to minor version
(`philharmonic-types = "0.3"`) to pick up patches automatically
while protecting against minor-version breakage.

Peer crates within the workspace pin loosely to each other
(`philharmonic-store = "0.1"`) — the workspace evolves together.

**Pinning to a patch version.** When a crate relies on a feature
or fix introduced in a specific patch, pin to that exact patch
(`philharmonic-types = "0.3.3"`) so `cargo` refuses to resolve
against an older patch that lacks the required feature. This is
the *only* reason to tighten beyond a minor pin; don't pin to a
patch for hygiene or habit. When the dependency publishes a
further patch whose feature set you start using, bump the pin.

## Git workflow

All Git operations on this workspace go through the helper
scripts in `scripts/`:

- `setup.sh` — one-time (or post-fresh-clone) initialization.
  Initializes every submodule recursively and warns if the Rust
  toolchain isn't on PATH. Idempotent; safe to rerun.
- `status.sh` — parent + every submodule's working tree.
- `pull-all.sh` — update submodules to their tracked branches.
- `commit-all.sh [--parent-only] [msg]` — commit pending changes
  in each submodule first, then the parent (bumping submodule
  pointers). With `--parent-only`, skip the submodule walk and
  commit only the parent — useful when the parent has its own
  pending work (docs, scripts) that should land independently
  of whatever state the submodules are in (e.g. while Codex has
  in-progress uncommitted work).
- `push-all.sh` — push each submodule's current branch, then the
  parent.

**Don't invoke `git commit` or `git push` ad-hoc.** The scripts
encode submodule ordering, default arguments, and the signoff
rule below. Ad-hoc invocations drift from those defaults. If the
script doesn't support what you need, extend the script (and
document the change here) before proceeding.

**Every commit is signed off.** The scripts pass `-s` to
`git commit`; a `Signed-off-by:` trailer is mandatory on every
commit in every repo in this workspace (parent and submodules).
This is a Developer Certificate of Origin-style assertion and is
a hard requirement, not a preference.

## Codex prompt archive

Claude hands substantive coding to Codex (see CLAUDE.md §Claude
vs. Codex division of labor). Every prompt Claude writes for
Codex is archived and committed — there are no ephemeral Codex
invocations.

**Location.** `docs/codex-prompts/YYYY-MM-DD-<slug>.md`, where
`<slug>` names the task (`auth-middleware-rewrite`,
`sqlx-mysql-store-skeleton`). One file per prompt. If a task
needs multiple rounds of Codex work, use a numeric suffix
(`-01`, `-02`) rather than overwriting.

**Contents.** The full prompt text Claude sent to Codex,
verbatim, plus a short preamble with:

- The task's motivation (one or two sentences).
- Links to the relevant design doc sections or ROADMAP entries.
- Any context files Claude pointed Codex at.

**Commit cadence.** Commit the prompt file *before* spawning
Codex, via `scripts/commit-all.sh`. The resulting code changes
land in a subsequent commit (or commits). Ordering the prompt
first means the archive is complete even if the Codex run is
abandoned partway through.

**Why.** The prompts are where design intent gets translated
into implementation instructions; they're the most useful
artifact for reconstructing why a chunk of code looks the way
it does, and they're where Claude/Codex collaboration mistakes
become visible in review. Losing them to chat history defeats
the reviewability of the workflow.

## Edition and MSRV

- **Edition 2024.**
- **MSRV 1.85.**

Documented in each `Cargo.toml`:

```toml
edition = "2024"
rust-version = "1.85"
```

MSRV bumps happen in coordinated minor releases across the
workspace.

## Build targets

Production: `x86_64-unknown-linux-musl` (static linking).

Library crates build for consumer-chosen targets. Binary crates
(`mechanics`, future API and connector binaries) ship as
statically-linked musl binaries for minimal containers.

Constraint: C library dependencies must be statically linkable or
vendored. Most pure-Rust crates fine; special attention for crates
wrapping native libraries.

## Repository structure

One-crate-per-repo under `github.com/metastable-void/*`. Each crate
has its own:

- Issue tracker.
- CI configuration.
- Release cycle.
- README, changelog.

Cross-crate refactors require coordinated PRs across repos.
Cornerstone versioning discipline absorbs most of this (cross-crate
vocabulary changes happen via cornerstone patch releases, picked up
transitively).

## Documentation

### Per-crate

Each crate has `README.md` covering usage and installation, plus
rustdoc for API reference. Current cornerstone and substrate crates
are at 99%+ documented; other crates target similar coverage.

### System-level

Cross-crate design docs live in the meta-crate's repository
(`philharmonic`). These cover:

- System overview.
- Design principles.
- Per-component deep dives.
- Layer boundaries.
- Deferred decisions.
- Conventions (this file).

Currently published at `metastable-void.github.io/philharmonic/`.

## CI

Each crate's CI runs at minimum:

- `cargo build` against MSRV.
- `cargo test` against current stable.
- `cargo clippy --all-targets` with warnings as errors.
- `cargo fmt --check`.
- `cargo doc --no-deps`.

Integration tests requiring external infrastructure (testcontainers
for MySQL, real connector services for some tests) are gated
behind features for CI flexibility.

## Trait crate vs. implementation crate split

When a concern has a trait surface and one or more implementations,
they live in separate crates rather than being feature-gated within
one.

Reasoning:

- **Dependency hygiene**: trait crate minimal; implementations carry
  their own dependencies.
- **Independent versioning**: bug fix in one implementation doesn't
  require trait crate release.
- **Discoverability**: implementations are separate crates on
  crates.io with their own pages.
- **No feature flag combinatorics**: each crate tested in isolation.

Example: `philharmonic-store` (traits) + `philharmonic-store-sqlx-
mysql` (SQL impl); not `philharmonic-store` with `mysql` feature.

## Re-export discipline

Crates re-export types from their direct dependencies that appear in
their own public APIs. Consumers get a flat namespace.

Example:

```rust
// philharmonic-store re-exports Uuid, Sha256, EntityId, etc.
// from philharmonic-types.
use philharmonic_store::{ContentStore, Uuid, EntityId};  // works
```

Rules:

- Re-export what appears in the crate's own public API.
- Don't re-export transitive dependencies.
- Don't re-export types the crate doesn't itself use.

## Error types

Errors use `thiserror` for display and source-chain. Partition by
what the caller does with them (semantic violations, concurrency
outcomes, backend failures). Methods like `is_retryable()` give
uniform checks.

Don't use `anyhow` in library crates — callers can't match on
specific failure modes. Use `anyhow` in application binaries where
appropriate.

## Async runtime

`tokio` is the workspace default. Avoid `async-std` or other
runtimes for consistency. Use `tokio::sync` primitives where
appropriate.

`async-trait` is used where trait objects need to be dyn-compatible
(current Rust stable async-in-traits support is insufficient for
trait-object use).

## Testing

Unit tests colocated with source (`#[cfg(test)] mod tests` or
`mod.rs` tests). Integration tests in `tests/` directory. Real-
infrastructure tests (testcontainers, network) gated behind
features when appropriate.

Test helpers factored into a shared module within the tests
directory; no separate testing crates unless the helpers are
genuinely shared across crates.

## Workspace inspirations

Conventions draw from:

- **Tokio's crate split**: `tokio`, `tokio-util`, `tokio-stream`.
  Separate crates for separate concerns with consistent naming.
- **sqlx's trait/backend pattern**: one trait surface, multiple
  backends. Philharmonic chose separate crates over features for
  the reasons above.
- **The Rust API Guidelines**: followed where applicable.

## When conventions should change

Conventions aren't immutable. Signs one should be revisited:

- **Repeated workarounds**: multiple crates work around the same
  convention.
- **Friction for new crates**: adding a new crate requires
  significant ceremony.
- **Ecosystem drift**: broader Rust conventions change.

Convention changes are workspace events: announced, applied in
coordinated releases, documented here.
