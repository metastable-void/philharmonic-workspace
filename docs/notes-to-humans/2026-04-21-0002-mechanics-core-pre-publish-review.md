# mechanics-core pre-publish review

**Date:** 2026-04-21
**Scope:** Review of mechanics-core post-Phase-1 extraction before
publishing `mechanics-core 0.2.3`. Triggered by Yuka's question:
"is Boa GC-flagged types only internal to mechanics-core and not
exposed?"

## Direct answer: GC-flagged types are all internal

Every `#[derive(...Trace, Finalize, JsData, ...)]` / field-level
`#[unsafe_ignore_trace]` in mechanics-core lives behind a private
boundary. Nothing Boa-GC-derived is reachable from
`mechanics_core::*` in the public API.

**Exhaustive list of GC-flagged types in mechanics-core:**

1. `BoaMechanicsConfig` at
   `mechanics-core/src/internal/http/wrappers.rs:6` — declared
   `pub(crate)`. Re-exported only as
   `crate::internal::http::BoaMechanicsConfig`
   (`internal/http/mod.rs:22`), which is itself inside
   `pub(crate) mod internal` (lib.rs:15). Unreachable externally.
2. `MechanicsState` at
   `mechanics-core/src/internal/runtime.rs:58` — declared
   `pub(crate)`. All five `#[unsafe_ignore_trace]` field
   attributes live inside this struct. Never exposed.
3. `MechanicsConfig` at
   `mechanics-core/src/internal/http/config.rs:12` — declared
   `pub struct`, but **the file is an orphan** (no `mod config;`
   in `internal/http/mod.rs`), so it isn't compiled. See next
   section.

**What the public API actually exposes for `MechanicsConfig`:**
`mechanics_core::job::MechanicsConfig` re-exports
`mechanics_config::MechanicsConfig`, the *pure* schema type from
the newly-extracted `mechanics-config` crate (no Boa, no GC
traits). Downstream consumers never see GC derives.

**Public surface audit (from `mechanics-core/src/lib.rs`):**
`MechanicsJob`, `MechanicsExecutionLimits`, `MechanicsError`,
`MechanicsErrorKind`, `MechanicsPool`, `MechanicsPoolConfig`,
`MechanicsPoolStats` — each checked; none derive Boa-GC traits
or hold fields of GC-derived types at a public boundary.

Conclusion for this question: **safe to publish on the GC-
exposure axis.**

## Pre-publish concern: orphan source files in mechanics-core

The Phase 1 extraction moved schema logic to `mechanics-config`
but left the original files behind under `mechanics-core/src/
internal/http/`. `internal/http/mod.rs` no longer declares them
as modules, so they are not compiled — but `cargo package
--list` still includes them in the tarball that `cargo publish`
would upload to crates.io.

**Five orphan `.rs` files** (confirmed via `cargo package --list
--allow-dirty` against the current tree):

- `mechanics-core/src/internal/http/config.rs` (dead
  `MechanicsConfig` with Boa GC derives — superseded by
  `mechanics_config::MechanicsConfig`)
- `mechanics-core/src/internal/http/query.rs` (superseded by
  `mechanics-config/src/query.rs`)
- `mechanics-core/src/internal/http/retry.rs` (superseded by
  `mechanics-config/src/retry.rs`)
- `mechanics-core/src/internal/http/template.rs` (superseded by
  `mechanics-config/src/template.rs`)
- `mechanics-core/src/internal/http/endpoint/validate.rs`
  (superseded by `mechanics-config/src/endpoint/validate.rs`)

Because they're not in the module tree, they don't affect
compilation today. Risks of publishing them anyway:

- **Reader confusion on crates.io / docs.rs source view.** Anyone
  browsing mechanics-core's source will see a second copy of
  `pub struct MechanicsConfig` with Boa-GC derives inside
  `internal/http/config.rs`, contradicting the real public type
  at `job::MechanicsConfig`.
- **Accidental resurrection.** If someone adds `mod config;` to
  `internal/http/mod.rs` later, compilation will either fail
  (name collision with the re-exported `MechanicsConfig`) or
  silently pick the orphan definition depending on ordering —
  both bad.
- **Unnecessary tarball bloat** (minor).

## Recommendation

**Delete the five orphan files before running
`./scripts/publish-crate.sh mechanics-core`.** It's a one-submodule
cleanup commit inside mechanics-core:

```
cd mechanics-core
rm src/internal/http/config.rs \
   src/internal/http/query.rs \
   src/internal/http/retry.rs \
   src/internal/http/template.rs \
   src/internal/http/endpoint/validate.rs
```

Then `cargo check --workspace && cargo clippy --workspace
--all-targets -- -D warnings && cargo test --workspace` to
re-verify (nothing should break; the files aren't referenced).
Follow with the usual `./scripts/commit-all.sh "mechanics-core:
remove dead files from Phase 1 extraction"` → `push-all.sh`.

This also makes `cargo-semver-checks` output cleaner when
comparing against the `v0.2.2` baseline.

I can do this cleanup on request.
