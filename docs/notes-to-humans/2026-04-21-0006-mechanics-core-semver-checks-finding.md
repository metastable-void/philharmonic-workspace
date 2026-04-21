# mechanics-core 0.2.3 fails cargo-semver-checks vs. 0.2.2 — 0.2.3 vs 0.3.0 decision

**Date:** 2026-04-21
**Surfaced by:** the first working run of
`./scripts/check-api-breakage.sh mechanics-core` after the script
was rewritten (same session — see §"Script fix" below).
**Status:** **Resolved (2026-04-21, same session): Path A chosen.**
- `mechanics-core/Cargo.toml` bumped to `0.3.0`; CHANGELOG's
  `[0.2.3]` heading re-cut as `[0.3.0]` with the reasoning
  recorded in the section preamble.
- `mechanics/Cargo.toml`: dep pin moved from
  `mechanics-core = "0.2.2"` to `"0.3.0"`, and `mechanics`'s own
  version bumped `0.2.1 → 0.3.0` in the same session with a
  CHANGELOG `[0.3.0]` entry. Bumping `mechanics`'s own version
  (not just the dep pin) is what forces downstream consumers of
  `mechanics` to opt into the new `mechanics-core` type identity
  explicitly — otherwise a caret-range upgrade of `mechanics`
  alone would cross the boundary silently via the re-exports.
- Publish still pending — next steps in `ROADMAP.md` §Phase 1
  Remaining work.

## The finding

`cargo semver-checks check-release -p mechanics-core` (baseline =
latest on crates.io = `0.2.2`) reports **2 major failures**:

```
--- failure enum_missing: pub enum removed or renamed ---
  enum mechanics_core::endpoint::EndpointBodyType
  enum mechanics_core::endpoint::QuerySpec
  enum mechanics_core::endpoint::HttpMethod
  enum mechanics_core::endpoint::SlottedQueryMode

--- failure struct_missing: pub struct removed or renamed ---
  struct mechanics_core::job::MechanicsConfig
  struct mechanics_core::endpoint::HttpEndpoint
  struct mechanics_core::endpoint::EndpointRetryPolicy
  struct mechanics_core::endpoint::UrlParamSpec

Summary semver requires new major version:
  2 major and 0 minor checks failed
```

## What's actually happening

`cargo-semver-checks` determines "is this type the same?" by the
tuple `(defining crate, defining path)`. It does **not** follow
`pub use` re-exports to prove equivalence.

In `0.2.2` these 8 types were defined in
`mechanics_core::internal::*` and re-exported at
`mechanics_core::endpoint::*` / `mechanics_core::job::*`. In
`0.2.3` they're defined in `mechanics_config::*` (the new sibling
crate) and re-exported at the same `mechanics_core::endpoint::*` /
`mechanics_core::job::*` paths.

From the tool's perspective:
- "The `mechanics_core::endpoint::HttpEndpoint` that was in 0.2.2
  (a `mechanics_core`-defined struct) has disappeared."
- "There's a `mechanics_core::endpoint::HttpEndpoint` re-export in
  0.2.3, but it's a `mechanics_config`-defined struct — different
  identity."
- Conclusion: the old type was removed.

This is **not** an overreach; cargo-semver-checks is correctly
implementing the conservative interpretation. The question is
whether that interpretation matches the observable behavior for
downstream users.

## Does it actually break users?

For plain **call-site usage** — importing the re-exported paths,
constructing values, calling methods — the answer is almost
certainly **no**. Rust sees `mechanics_core::endpoint::HttpEndpoint`
as a re-export of `mechanics_config::HttpEndpoint`; the type
has the same fields, same serde derives, same impls, so user code
that does `use mechanics_core::endpoint::HttpEndpoint; let e = ...;`
keeps compiling.

For **trait-impl edge cases** — third-party crates that did
`impl ThirdPartyTrait for mechanics_core::endpoint::HttpEndpoint`
— the impl survives because rust resolves the re-export to the
same type identity, and coherence permits the impl on a type
foreign to the defining crate. (The trait has to be local or
there'd be an orphan-rule problem, which is independent of this
change.)

The one theoretically-breaking case is serialization-format
identity or reflection that depends on `std::any::TypeId`. Nothing
in `mechanics-core`'s advertised surface exposes `TypeId`.

**Empirically safe.** Cargo's default caret resolver will happily
upgrade a downstream from `0.2.2` to `0.2.3` under a `"0.2.2"`
dependency declaration, and compilation is expected to succeed.

## But cargo's semver model disagrees

Under cargo's pre-1.0 semver rules, the "major" digit for a `0.x`
crate is **the minor digit** (`0.2.x`). A breaking change at that
level requires `0.(x+1).0`, not `0.x.(y+1)`.
`cargo-semver-checks` enforces this rule: 2 major checks failed ⇒
"requires new major version" ⇒ under `0.x`, that's
`0.2.x → 0.3.0`.

So we have a conflict:

- **Observable user impact:** no break, 0.2.3 is fine.
- **Tool verdict:** major break, needs 0.3.0.
- **Convention among cargo maintainers:** follow the tool; every
  `cargo update` that silently upgrades someone across a
  type-identity change is a future footgun even if today's code
  compiles.

## The decision

Two paths, both defensible. **Yuka picks.**

### Path A: Republish as 0.3.0 (follow the tool)

- Bump `mechanics-core/Cargo.toml` to `0.3.0`.
- Update `mechanics-core/CHANGELOG.md` — move the current `[0.2.3]`
  heading to `[0.3.0]`, keep the content.
- The `mechanics` submodule pins `mechanics-core = "0.2.2"` at
  [mechanics/Cargo.toml:17] — the caret range is `>=0.2.2, <0.3.0`,
  so it will **no longer auto-upgrade** to 0.3.0. Bump that pin
  explicitly to `mechanics-core = "0.3.0"` as part of the same
  change, inside the `mechanics` submodule. This is the whole
  reason cargo-semver-checks is flagging it: the wire-signal that
  downstreams need to deliberately opt in.
- Commit through both submodules via `./scripts/commit-all.sh`.
  `./scripts/publish-crate.sh mechanics-core` produces `v0.3.0`.
- Pro: honors the tool and the convention; no surprising silent
  upgrades for downstream consumers.
- Con: one more round-trip; the `mechanics` bump is extra work.

### Path B: Ship as 0.2.3 (override the tool)

- Keep `mechanics-core/Cargo.toml` at `0.2.3`.
- `mechanics` stays on `mechanics-core = "0.2.2"`, which will
  auto-upgrade to `0.2.3` — in practice transparent because
  call-site usage is preserved.
- Publish via `./scripts/publish-crate.sh mechanics-core 0.2.3`.
  `cargo-semver-checks` will fail if run; the failure is known
  and documented.
- Pro: ships now, one fewer decision, matches the changelog's
  framing of this as "extraction preserving compatibility".
- Con: every future release of `mechanics-core` that runs
  `check-api-breakage.sh` will show this pre-existing break until
  we eventually cross over into a genuine `0.3.0` release. Noise
  in the tooling. Also: if any downstream relied on
  `std::any::TypeId` of these types (very unlikely, not a pattern
  anyone should use), they'll silently see the wrong thing.

### My recommendation (Claude)

**Path A (0.3.0).** Two reasons:

1. The tool is right about the semantics even if users don't
   observe a break today. Type identity IS part of the API under
   Rust's model; changing the defining crate is a real wire-level
   change. cargo's resolver treats `0.2 → 0.3` as an opt-in break;
   that matches the underlying reality here.
2. The `mechanics` bump is one extra commit in an already
   in-flight release session. Low marginal cost compared to
   carrying a known semver-checks failure forward.

Pure judgment call — Path B is defensible and Yuka has veto.

## Script fix (context for how we got here)

`./scripts/check-api-breakage.sh` used to invoke

```
cargo semver-checks --workspace --all-features \
                    --baseline-rev origin/main
```

which fails in this repo because cargo-semver-checks resolves
`--baseline-rev` by `git clone`ing the parent repo at that
revision, but that clone does not recurse into submodules. Every
workspace member lives in a submodule, so at the baseline root
there's no `mechanics-core/Cargo.toml`, no `mechanics-config/`,
etc. The tool aborts with:

```
error: failed to retrieve local crate data from git revision
  1: failed to parse .../Cargo.toml: no `package` table
  2: package `mechanics` not found in ...
```

The fix is to switch from repo-level (`--workspace --baseline-rev`)
to per-crate (`--baseline-version`), where the baseline comes
from crates.io rather than a git clone. The script now takes

```
./scripts/check-api-breakage.sh <crate> [<baseline-version>]
```

If `<baseline-version>` is omitted, cargo-semver-checks queries
crates.io for the newest published version of `<crate>` and uses
that. An explicit version (e.g. `0.2.2`) overrides.

Quick verification after the rewrite:
- `./scripts/check-api-breakage.sh mechanics-config` → 196 checks,
  no semver update required. (Self-vs-self baseline since 0.1.0
  was just published.)
- `./scripts/check-api-breakage.sh mechanics-core` → 2 major
  failures, surfaced as above.

`ROADMAP.md` §Phase 1 "Remaining work" has been updated to the new
syntax (`check-api-breakage.sh mechanics-core 0.2.2`).
