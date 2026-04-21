# Release-blocker audit: mechanics-config 0.1.0 + mechanics-core 0.2.3

**Date:** 2026-04-21
**Context:** HUMANS.md short-term TODO "release mechanics-config /
mechanics-core". This note records the pre-release audit so the
actual release session can start from a known state rather than
re-deriving the punch list.
**Audit inputs:**
- `./scripts/status.sh` / `./scripts/heads.sh` (tree + signatures).
- `./scripts/publish-crate.sh --dry-run <crate>` (both crates).
- `./scripts/cargo-audit.sh` (RustSec advisories).
- `./scripts/crates-io-versions.sh` (what's already on crates.io).
- `cargo semver-checks check-release` (attempted — see §5).
- Crate-root file inventory, Cargo.toml metadata, CHANGELOG state.

## TL;DR

Nothing strictly prevents publishing. There is **one hard
ordering constraint** (`mechanics-config` before `mechanics-core`)
and **one soft pre-release fix** (add missing LICENSE files to
`mechanics-config`). Everything else is informational.

## 1. Hard ordering constraint (sequencing, not fixable)

`mechanics-core/Cargo.toml:25` pins `mechanics-config = "0.1.0"`.
`mechanics-config` is a **new crate** — not yet on crates.io
(confirmed HTTP 404 via `curl`; `crates-io-versions.sh
mechanics-config` also 404s).

Consequence: `cargo publish -p mechanics-core` will fail with
`no matching package named mechanics-config found` until
`mechanics-config 0.1.0` is live. The dry-run already demonstrates
this failure mode.

Not fixable — this is the dependency order baked into Phase 1.
Just honor it: publish `mechanics-config` first, then
`mechanics-core`.

## 2. Soft blocker: `mechanics-config` is missing LICENSE files

Repo root has only `CHANGELOG.md`, `Cargo.toml`, `README.md`,
`src/`. No `LICENSE-APACHE`, no `LICENSE-MPL`. `mechanics-core`
(the sibling it was extracted from) ships both.

Why it's not a **hard** blocker: `Cargo.toml` declares
`license = "Apache-2.0 OR MPL-2.0"` as a valid SPDX expression, so
crates.io accepts the metadata without LICENSE files in the
tarball. `cargo publish --dry-run` passes cleanly.

Why it's worth fixing anyway:
- Inconsistent with `mechanics-core` and the workspace convention.
- End users unpacking the `.crate` get no license text.
- Easy fix: copy `LICENSE-APACHE` and `LICENSE-MPL` from
  `mechanics-core` into `mechanics-config/`, commit inside the
  submodule via `commit-all.sh`, push. Five minutes.

## 3. `mechanics-config/README.md` is 3 lines

Functionally fine — cargo doesn't enforce README length — but
sparse for a crates.io landing page. Optional; not tracking as a
blocker. A short paragraph ("schema types shared between
mechanics-core and the Philharmonic connector lowerer; Boa-free
and runtime-free") would suffice if we choose to expand.

## 4. Semver bump judgment: `mechanics-core 0.2.2 → 0.2.3`

`mechanics-core/CHANGELOG.md` flags a **behavior change**: schema
validation now fails at config-construction time instead of at
job-call time. Callers that deliberately constructed invalid
`MechanicsConfig`/`HttpEndpoint` values and relied on lazy error
surfacing will now see errors at the construction site.

Strict SemVer at `0.x.y` permits any change as a patch bump (the
`0.x` range is explicitly unstable). Some maintainers would still
bump `0.2.x → 0.3.0` to signal observable behavior change.

Not a blocker; judgment call. The current `0.2.3` bump is
defensible. Noting here so the decision is traceable after publish.

## 5. `cargo-semver-checks` is gated behind §1

Attempted `cargo semver-checks check-release --package
mechanics-core --baseline-version 0.2.2` during the audit — it
fails with the same
`no matching package named mechanics-config` error as the
mechanics-core dry-run, because the semver-check builds a
synthetic package that transitively depends on the unpublished
`mechanics-config`.

Order of operations this implies for the release session:
1. Publish `mechanics-config`.
2. **Then** run `cargo semver-checks check-release --package
   mechanics-core --baseline-version 0.2.2`.
3. If green, publish `mechanics-core`.

Don't skip step 2 — the behavior change described in §4 should
not also introduce a surface-level API break without a minor bump.
`cargo-semver-checks` is the mechanical check for the
surface-level part.

## 6. Transitive security advisory reaches `mechanics-core`

`cargo-audit` findings as of 2026-04-21:

| ID                 | Crate            | Severity    | Path                                                      |
|--------------------|------------------|-------------|-----------------------------------------------------------|
| RUSTSEC-2026-0009  | `time 0.3.45`    | 6.8 medium  | `boa_engine → mechanics-core`                             |
| RUSTSEC-2023-0071  | `rsa 0.9.10`     | 5.9 medium  | `sqlx-mysql → philharmonic-store-sqlx-mysql` (**not** mc) |
| RUSTSEC-2024-0436  | `paste 1.0.15`   | unmaintained (warning) | `boa_engine → mechanics-core`               |

Only RUSTSEC-2026-0009 meaningfully touches the release surface:

- **Not a hard blocker.** Published lib crates don't ship
  `Cargo.lock`; downstream resolvers pick whatever `time` they
  want. The `.crate` tarball we upload is unaffected by the
  workspace lockfile.
- **Should still be nudged.** `cargo update -p time` locally to
  pull a `>= 0.3.47` version; commit the Cargo.lock bump. This
  keeps our own CI and local builds on a patched `time` without
  touching any Cargo.toml.

`rsa` and `paste` are out of scope for this release (one doesn't
reach either crate; the other is an unmaintained-warning, not a
vulnerability).

## 7. `[profile.release]` warnings during dry-run

`cargo publish --dry-run` emits ~22 lines of
`warnings: profiles for the non root package will be ignored, specify profiles at the workspace root`.
Several workspace crates — including `mechanics-core` — declare
their own `[profile.release]` block. Cargo ignores these in
workspace members; only the workspace-root profile is honored.

Cleanup opportunity (delete those blocks from non-root
Cargo.tomls). **Not a release blocker** — just noise. Can be a
follow-up commit after the release ships.

## 8. Sibling impact: `mechanics` pins `mechanics-core = "0.2.2"`

`mechanics/Cargo.toml:17` has `mechanics-core = "0.2.2"`. Cargo's
caret requirement means this resolves to `>=0.2.2, <0.3.0`, so
after we publish `0.2.3`, downstream consumers of `mechanics`
auto-pick 0.2.3 on next `cargo update`. **No action needed for
this release.**

Flagging only as a reminder for whoever releases `mechanics` next:
consider tightening the dep to `"0.2.3"` to pin against the
post-behavior-change version explicitly.

## 9. No `v0.2.2` tag inside `mechanics-core` submodule

The release-tagging convention (`publish-crate.sh` creates
`v<version>` annotated signed tags) post-dates the `0.2.2`
release. `git tag -l 'v*'` inside `mechanics-core` returns
nothing.

Consequence: `cargo-semver-checks` for 0.2.3 has to use
`--baseline-version 0.2.2` (fetches from crates.io) rather than
`--baseline-rev v0.2.2` (local git tag). Functionally equivalent;
just noting why the `--baseline-version` form is used.

Starting with this release, the tags will exist going forward —
`publish-crate.sh` creates `v0.1.0` and `v0.2.3` on success.

## 10. Cleared / green

- Parent + all submodules: clean tree, on branch `main`, HEAD
  commits all `G`-signed (`heads.sh`).
- CI green on current main (`b762834` → `7676ff9`).
- `cargo publish --dry-run mechanics-config` succeeds (26 files,
  78.5 KiB; 17.4 KiB compressed).
- `CHANGELOG.md`, `README.md`, Cargo.toml metadata (license,
  description, keywords, categories, repository) complete in both
  crates.
- Neither crate touches the crypto-sensitive paths covered by
  `crypto-review-protocol` (no SCK / COSE / ML-KEM / X25519 /
  HKDF / AES-256-GCM / `pht_` surfaces). No two-gate review
  triggers for this release.

## Suggested order of operations for the release session

1. Copy `LICENSE-APACHE` + `LICENSE-MPL` from `mechanics-core`
   into `mechanics-config/`. Commit inside the submodule via
   `./scripts/commit-all.sh`. `./scripts/push-all.sh`.
2. (Optional, not blocking) `cargo update -p time` at the
   workspace root; commit the Cargo.lock bump.
3. `./scripts/publish-crate.sh --dry-run mechanics-config` → if
   green → `./scripts/publish-crate.sh mechanics-config` →
   `./scripts/push-all.sh` (pushes the signed `v0.1.0` tag).
4. `cargo semver-checks check-release --package mechanics-core
   --baseline-version 0.2.2` — now unblocked.
5. `./scripts/publish-crate.sh --dry-run mechanics-core` → if
   green → `./scripts/publish-crate.sh mechanics-core` →
   `./scripts/push-all.sh` (pushes the signed `v0.2.3` tag).
6. Update HUMANS.md to remove the "release mechanics-config /
   mechanics-core" TODO. Update `ROADMAP.md` §Phase 1 to reflect
   completion. Commit in the same session as the publish, not as
   a follow-up (per CLAUDE.md "ROADMAP is living").

Only step 1 is load-bearing; the rest is the mechanical sequence.
