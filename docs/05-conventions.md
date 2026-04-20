# Philharmonic — Conventions

This document covers workspace-wide practices: how crates are
named, versioned, licensed, and structured. These aren't design
decisions about what the system does, but they're decisions every
crate in the workspace honors. Documenting them once means new
crates have a template to follow and existing crates have a
reference for "why does it work this way."

## Licensing

All crates in the workspace are licensed under
`Apache-2.0 OR MPL-2.0`. Both license files are present at the
crate root (`LICENSE-APACHE` and `LICENSE-MPL`).

The dual license gives consumers a choice. Apache-2.0 is the
standard permissive license in the Rust ecosystem; most consumers
will pick it. MPL-2.0 is for consumers who specifically want
file-level copyleft (modifications to MPL-licensed files must
remain open) without the project-level copyleft of GPL.

The pairing is deliberately not the more common
`Apache-2.0 OR MIT`. The reasoning:

**MPL-2.0 is FSF-compatible in a way MIT isn't ambiguous about.**
The MPL has explicit GPL-2.0-or-later compatibility via its
secondary-license clause; consumers who need GPL-compatible code
have a clean path. MIT is generally GPL-compatible but the
relationship is informal.

**MPL-2.0 preserves modification visibility.** Consumers who fork
a file and modify it must keep their modifications under MPL.
This isn't a strong copyleft (it's per-file, not per-project), but
it does ensure that improvements to the philharmonic crates remain
visible to the community when consumers redistribute their
modifications.

**Apache-2.0 covers patent grants.** The Apache license has
explicit patent grants that protect both contributors and
consumers. MIT doesn't address patents at all, which can be a
concern for commercial deployments. MPL-2.0 also has patent
grants, so the dual license is patent-safe regardless of which
arm a consumer chooses.

The combination — Apache-2.0 for permissive use, MPL-2.0 for
copyleft preference, both with patent protection — covers more
deployment scenarios than `Apache-2.0 OR MIT` while staying
clearly open-source.

In `Cargo.toml`:
```
license = "Apache-2.0 OR MPL-2.0"
```

This is an SPDX expression. The `OR` operator means consumers
choose; they don't need to comply with both.

## Naming

Crate names follow a small set of patterns.

**Subsystem prefix.** Crates that belong to philharmonic are
prefixed `philharmonic-` (the storage substrate, the workflow
layer, the policy layer). Crates that belong to mechanics are
prefixed `mechanics-` (the executor library, the HTTP service).
The prefix indicates which subsystem the crate is part of.

**Concern after prefix.** The part after the prefix names the
concern: `philharmonic-types` is the cornerstone vocabulary,
`philharmonic-store` is the storage trait surface,
`philharmonic-workflow` is the orchestration layer. The name is a
short noun phrase, not an abbreviation.

**Implementation suffix when needed.** When a single concern has
multiple implementations, the implementation gets a suffix:
`philharmonic-store-sqlx-mysql` is "the storage trait surface,
implemented via sqlx for MySQL." Future implementations would
follow the same pattern: `philharmonic-store-sqlx-pgsql` for
Postgres, `philharmonic-store-mem` for in-memory.

**Meta-crate is unsuffixed.** `philharmonic` (no suffix) is the
top-level meta-crate, currently a placeholder. If it ever becomes
a convenience re-export crate, the bare name signals "the whole
system" while the suffixed crates are individual concerns.

The naming pattern lets readers infer relationships from names
alone. `philharmonic-store` is the trait;
`philharmonic-store-sqlx-mysql` is one implementation;
`philharmonic-workflow` consumes the trait. The hierarchy is
visible without reading the docs.

## Crate ownership and the meta-crate

The `philharmonic` meta-crate exists as a name reservation on
crates.io, currently published as version 0.0.0 with empty
contents. Same for `mechanics` (the HTTP service crate, also a
placeholder until the implementation lands).

The 0.0.0 pattern serves a few purposes:

**Name protection.** Once a crate name is published, no one else
can take it. Reserving names early prevents squatting and
prevents accidental conflicts with future organic publishes.

**Dependency-graph honesty.** A 0.0.0 crate signals "this name is
claimed but the contents aren't ready." Consumers can't depend on
it usefully, which matches reality.

**Future-proofing.** When the meta-crate gets real contents, it
publishes 0.1.0 (or whatever the first real version is) and
consumers can depend on it. The name doesn't need to be acquired
or transferred; it's already owned.

The full set of currently-claimed names is:

- `philharmonic` (meta-crate placeholder)
- `philharmonic-types` (cornerstone, real)
- `philharmonic-store` (trait surface, real)
- `philharmonic-store-sqlx-mysql` (SQL backend, real)
- `mechanics` (HTTP service, placeholder)
- `mechanics-core` (JS executor library, real)

Defensive name claims for crates not yet started:

- `philharmonic-workflow` (planned)
- `philharmonic-policy` (planned, scope undefined)
- `philharmonic-api` (planned, scope undefined)
- `philharmonic-connector` (planned, scope undefined)
- `philharmonic-realm` (planned, scope undefined)

The defensive claims happen as 0.0.0 stubs once the names are
chosen. They have no contents and no consumers; they exist to
prevent the names from being taken before the implementations are
ready.

## Versioning

The workspace uses Semantic Versioning, with the standard caveats
for pre-1.0 crates.

**Patch releases (0.x.y → 0.x.(y+1))** are for additive changes
and bug fixes: new types, new methods, new derive impls, new
re-exports, documentation improvements, internal refactors that
don't affect the public API. Patch releases don't break consumers.

**Minor releases (0.x.y → 0.(x+1).0)** are for changes to existing
APIs: signature changes, removed methods, renamed types,
behavioral changes that consumers might rely on. In pre-1.0
versioning, minor bumps signal "this version may break consumers";
in 1.0+ versioning, that role moves to major bumps.

**Major releases (0.x.y → 1.0.0 → 2.0.0)** mark stability
boundaries. The 1.0 release of any crate signals "the API is
stable enough that breaking changes are rare and deliberate."
2.0+ releases are for significant breaking changes that justify a
coordinated update across the ecosystem.

The cornerstone (`philharmonic-types`) is on the strict end of
this discipline. Because so many other crates depend on it,
breaking changes have outsized consequences. Patch releases for
additions; minor releases bundled and announced; major releases
treated as ecosystem events.

The leaf crates (workflow, future API layer) can be more relaxed
about minor releases, since they have fewer dependents. The
discipline scales with dependency count.

**Cornerstone version pinning in dependents.** Crates that depend
on the cornerstone pin to a minor version (`philharmonic-types =
"0.3"`). This picks up patch-level additions automatically while
protecting against minor-version breakage. Across the workspace,
all crates pin to the same cornerstone minor version so that the
re-exported types resolve to the same definitions.

**Peer crate version pinning.** Crates that depend on each other
within the workspace pin loosely to peers (`philharmonic-store =
"0.1"` rather than `"=0.1.0"`). The workspace as a whole evolves
together; tight peer pinning would force coordinated releases for
unrelated changes.

A future 1.0 release for the cornerstone would happen when its API
has stabilized — probably after the workflow layer and at least
one upper layer (policy or API) have validated the vocabulary's
fitness. Other crates' 1.0 releases follow as their own APIs
stabilize.

## Edition and MSRV

All workspace crates currently use Rust edition 2024 with a
minimum supported Rust version of 1.85.

**Edition 2024** is the current Rust edition at the time of
writing. Editions don't introduce new language features per se —
they enable changes that would be backward-incompatible if
applied to older code — but they signal a baseline of language
features and idioms the workspace assumes.

**MSRV 1.85** is set conservatively: high enough to use needed
features (async functions in traits without `async-trait` would
be MSRV 1.75+, but the workspace currently uses `async-trait` for
trait object compatibility, so the practical MSRV is determined by
other features), low enough to be available in current stable
toolchains.

The MSRV is documented in each crate's `Cargo.toml`:

```
rust-version = "1.85"
```

When the workspace bumps MSRV (e.g., to use a new stable
language feature), it does so in a coordinated minor release
across all affected crates. MSRV bumps are announced; consumers
on older toolchains can pin to prior versions.

## Build targets

Production builds target `x86_64-unknown-linux-musl` for static
linking. The musl target produces statically-linked binaries
without glibc dependencies, simplifying deployment to minimal
container images and across Linux distributions.

The workspace's binary crates (`mechanics` the HTTP service,
future API services) build for musl in CI. Library crates build
for any target a consumer chooses; their portability is a
consumer concern.

The musl target imposes a few constraints:

- C library dependencies must be statically linkable. Most pure
  Rust crates are fine; crates that wrap C libraries may need
  vendored or musl-compatible builds.
- Some operations (DNS resolution, locale handling) work
  differently under musl than glibc. Crates that depend on
  glibc-specific behavior may need workarounds.

The benefit — single self-contained binaries that run anywhere
Linux runs — outweighs these constraints for the workspace's
deployment model.

## Repository structure

Each crate lives in its own GitHub repository under the
`metastable-void` organization (or whichever organization owns
the project). The repository names match the crate names exactly:

- `github.com/metastable-void/philharmonic`
- `github.com/metastable-void/philharmonic-types`
- `github.com/metastable-void/philharmonic-store`
- etc.

This is the one-crate-per-repo pattern, as opposed to a monorepo
holding all crates. The trade-off:

**One-crate-per-repo benefits:**
- Each crate has its own issue tracker, scoped to its concerns.
- Each crate's release cycle is independent.
- New contributors can clone just the crate they're working on.
- CI is simpler (one crate, one build matrix).

**Monorepo benefits (not chosen):**
- Cross-crate refactors are atomic.
- Shared CI configuration lives once.
- Discovery is easier (one repo to find everything).

The one-crate-per-repo choice reflects the workspace's intended
model: each crate is independently useful, with its own consumers
who don't necessarily use the others. Someone using
`philharmonic-store-sqlx-mysql` for non-philharmonic purposes
shouldn't have to navigate the entire ecosystem to find what they
need.

The trade-off cost — cross-crate refactors require coordinated PRs
across multiple repos — is real but manageable. The cornerstone's
versioning discipline absorbs most of it: cross-crate vocabulary
changes happen via patch-level cornerstone additions, picked up
transitively without coordinated work.

## Documentation

Each crate's `README.md` covers basic usage, installation, and a
pointer to docs.rs for the API reference. The README is the
landing page for someone discovering the crate; it should answer
"what is this crate, and how do I get started" within the first
screen.

API documentation lives in rustdoc, generated by docs.rs. Every
public type and method should have a doc comment; the workspace
targets the highest-feasible documentation coverage. The current
cornerstone and substrate crates are at >99% documented.

System-level documentation (the documents in this directory) lives
in the meta-crate's repository (`philharmonic`), since it covers
cross-crate concerns. Per-crate design docs (if any) live in the
respective crate's repo.

The convention: code documentation answers "how do I use this
type?", system documentation answers "how does the system work?".
The two complement each other; neither replaces the other.

## CI and quality gates

Each crate's CI runs at minimum:

- `cargo build` against the documented MSRV.
- `cargo test` against current stable Rust.
- `cargo clippy --all-targets` with warnings treated as errors.
- `cargo fmt --check` to enforce consistent formatting.
- `cargo doc --no-deps` to verify documentation builds.

Crates with integration tests (the SQL backend, the workflow
crate when it exists) run those in CI too. Integration tests that
require external infrastructure (testcontainers for MySQL) are
gated behind a feature flag so they can be skipped in
constrained environments.

Release quality gates add:

- Successful publish to crates.io.
- Successful docs.rs build.
- Tagged release in the GitHub repo.

These are manual steps for now; automating them is a future
quality-of-life improvement.

## Trait crate vs. implementation crate split

When a concern has both a trait surface and one or more
implementations, the workspace pattern is to split them across
crates rather than feature-gating implementations within a single
crate.

The substrate exemplifies this: `philharmonic-store` is the trait
surface; `philharmonic-store-sqlx-mysql` is one implementation. A
future in-memory implementation would be `philharmonic-store-mem`,
not a `mem` feature on `philharmonic-store`.

The reasoning:

**Dependency hygiene.** The trait crate depends only on what the
trait surface needs (the cornerstone, `async-trait`, `thiserror`).
Implementation crates depend on what they need (sqlx for the SQL
backend; nothing for the in-memory backend). Consumers who only
need the trait surface (or who provide their own implementation)
don't pull in implementation dependencies.

**Independent versioning.** A bug fix in the SQL backend doesn't
require a release of the trait crate. A new method on the trait
surface (a non-breaking addition) doesn't force every implementation
to release.

**Implementation discoverability.** Listing implementations as
sibling crates makes them discoverable on crates.io. A
`philharmonic-store-sqlx-pgsql` crate would appear in search
results next to its sibling, with its own page and description.
Feature flags on a single crate don't surface this way.

**Avoid feature-flag combinatorics.** A single crate with multiple
implementations behind feature flags has 2^n possible feature
combinations to test. Separate crates don't have this problem;
each is tested in isolation.

The cost: more crate names to track, slightly more publishing
overhead. The benefit: cleaner dependency graphs and clearer
crate purposes. The workspace prefers the trade.

## Re-export discipline at crate boundaries

When crates depend on each other, the dependent crate re-exports
the upstream crate's types that appear in its own public API.

For example, `philharmonic-store` depends on `philharmonic-types`
and uses `Uuid`, `Sha256`, `EntityId<T>`, etc., in its trait
signatures. These types are re-exported from `philharmonic-store`
so that consumers depending only on `philharmonic-store` can use
them directly:

```
// In application code:
use philharmonic_store::{ContentStore, Uuid, Sha256};
```

The consumer doesn't need to also depend on `philharmonic-types`
explicitly. The transitive dependency is satisfied by the
re-export.

This is the same pattern the cornerstone uses for `Uuid` and
`JsonValue` (re-exported from upstream `uuid` and `serde_json`),
applied recursively up the dependency graph. Each crate re-exports
the types from below that it uses in its own surface; consumers
get a flat namespace of types from whichever crate they depend on
directly.

The discipline:

- A crate re-exports types it uses in its own public API, when
  those types come from a direct dependency.
- A crate doesn't re-export types from transitive dependencies; if
  consumers need them, they should depend on the right intermediate
  crate.
- A crate doesn't re-export types it doesn't itself use; that
  would be misleading.

Consistent re-exports prevent consumers from needing to depend on
the entire crate hierarchy when they only want one layer's
functionality.

## Workspace inspirations

The conventions above draw from patterns in the broader Rust
ecosystem:

**Tokio's crate split.** `tokio`, `tokio-util`, `tokio-stream`,
etc. — separate crates for separate concerns, with consistent
naming. The philharmonic naming pattern follows the same model.

**sqlx's backend pattern.** `sqlx` defines the trait surface;
backend support lives in features (`sqlx` with `mysql`/`postgres`/
`sqlite` features). Philharmonic chose separate crates instead of
features for the reasons in the trait/implementation split section,
but the trait/backend separation is the same idea.

**The Rust API guidelines.** The workspace tries to follow the
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
where applicable: naming conventions, error type design, doc
comment formatting, etc. The guidelines are aspirational targets;
exact compliance isn't a release blocker, but they shape decisions
when conventions are otherwise undecided.

## When conventions should change

Like boundaries, conventions aren't immutable. The signs that a
convention should be revisited:

**Repeated workarounds.** If multiple crates work around a
convention, the convention may be wrong for the workspace's
actual needs.

**Friction for new crates.** If adding a new crate involves
significant ceremony to comply with conventions that don't
benefit it, the conventions are over-engineered.

**Ecosystem drift.** If broader Rust conventions change (a new
edition, new tooling, new community norms), the workspace
should follow rather than diverge.

Convention changes are workspace-wide events. They get announced,
applied to all affected crates in coordinated releases, and
documented as updates to this file. The conventions document is
the source of truth; if a crate diverges from it, either the
crate is wrong or the document is wrong, and the discrepancy
should be resolved.

## What this document is

A reference, not a tutorial. Someone setting up a new crate in
the workspace consults this document to know what name pattern
to use, what license header to apply, what `Cargo.toml` fields to
fill in. Someone evaluating the workspace consults this document
to understand the consistency they should expect across crates.

It's not a contributor's guide (that's a separate concern, for a
separate document if needed). It's not a roadmap (deferred
features have their own document). It's not a design rationale
for the system itself (that's covered in the per-layer docs).

It's the workspace's housekeeping rules, written down so they
don't have to be rediscovered.
