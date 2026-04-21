# Workspace `cargo fmt` drift — pre-existing, surfaced during pre-publish review

**Date:** 2026-04-21
**Scope:** `cargo fmt --all --check` at the workspace root.

## The finding

While running the pre-landing trio to verify the orphan-file
cleanup (see `2026-04-21-0002-mechanics-core-pre-publish-review.md`),
`cargo fmt --all --check` failed. The failures are in files I
did **not** touch — this is pre-existing drift, surfaced but
not caused by today's work.

**21 formatting diffs across 9 files:**

- `mechanics-config/src/config.rs` (4 sites)
- `mechanics-config/src/endpoint/mod.rs` (2 sites)
- `mechanics-config/src/endpoint/validate.rs` (1 site)
- `mechanics-config/src/headers.rs` (1 site)
- `mechanics-config/src/lib.rs` (1 site)
- `mechanics-core/src/internal/http/endpoint/execute.rs` (4 sites)
- `mechanics-core/src/internal/http/mod.rs` (1 site)
- `mechanics-core/src/internal/http/endpoint/../tests/*.rs` (5 files, use-ordering)
- `mechanics/src/bin/mechanics-rs.rs` (1 site)

All diffs look like cosmetic rustfmt idiom updates (breaking long
lines, re-ordering `use` statements alphabetically, wrapping
chained generics). No real logic touched.

## Why this matters now

The workspace's mandated pre-landing trio is `cargo fmt --all
--check && cargo clippy --workspace --all-targets -- -D warnings
&& cargo test --workspace`. Today:

- `cargo check` ✓
- `cargo clippy ... -- -D warnings` ✓
- `cargo test` ✓ (all pass, ~400s end-to-end due to pool tests)
- `cargo fmt --all --check` ✗ — **fails**.

Meaning: *any* future commit to Rust code in this workspace
that follows the rule strictly would fail the local pre-landing
check on the fmt step, forcing either a fix-the-rest fix before
unrelated work or a waiver. It's a broken window.

## Likely cause (guess, not confirmed)

The Phase 1 extract commit message claims `cargo fmt --all
--check` passed at the time. Two plausible explanations:

1. A rustfmt version bump between then (2026-04-20) and now
   (2026-04-21) introduced new idiom preferences. Quick to
   check: `rustc --version` against what Codex used for Phase 1.
2. The claim in the Phase 1 commit was accurate for
   `mechanics-core` and `mechanics-config` *only at per-crate
   scope*, but workspace-root `cargo fmt --all --check` had
   been out of sync before Phase 1 started — Phase 1 didn't
   cause it, didn't fix it either.

Not investigating further without Yuka's go-ahead; the fix is
the same either way.

## Recommended fix

Run `cargo fmt --all` at the workspace root, verify nothing
unexpected changed, and commit the result as a cleanup pass.
Since the diffs span submodules (mechanics, mechanics-config,
mechanics-core), this is a full submodule-walking
`./scripts/commit-all.sh` — not `--parent-only`.

Suggested message: `fmt: apply cargo fmt --all to clear
pre-existing drift`.

Ordering options:

- **Fold into the orphan-file cleanup commit** (bundle
  orphan-removal + fmt-fix in one logical "pre-publish cleanup"
  commit). Simpler history.
- **Separate commits** (orphan-removal first, then fmt-fix).
  Cleaner semantic separation but more commits for minor work.

I lean toward bundling, since both are pre-publish hygiene and
fmt fixes aren't a semantic change.

## Going forward

Consider adding `cargo fmt --all --check` as a CI gate (the
conventions §CI list already mentions `cargo fmt --check` but
we can't easily tell whether per-crate CIs enforce it, given
each submodule has its own CI). A pre-push git hook at the
workspace level is another option — would catch drift before
it lands.

Yuka's read on the likely cause: the drift predates the fmt
convention itself (the pre-landing §Pre-landing checks entry
was added to `docs/design/13-conventions.md` only on 2026-04-21,
via the same session that surfaced this note). So the drift is
not a regression against the rule; it's what the rule was
written to prevent going forward.

## Outcome

Fixed in the same session. Running `cargo fmt --all` at the
workspace root touched 10 files across three submodules:

- `mechanics/src/bin/mechanics-rs.rs`
- `mechanics-config/src/{config.rs, endpoint/mod.rs,
  endpoint/validate.rs, headers.rs, lib.rs}`
- `mechanics-core/src/internal/http/{endpoint/execute.rs, mod.rs,
  tests/headers.rs, tests/query.rs, tests/response_limit.rs,
  tests/serde_config.rs, tests/template.rs}`

Pre-landing trio re-run after the fmt apply:

- `cargo fmt --all --check` ✓ clean.
- `cargo clippy --workspace --all-targets -- -D warnings` ✓ clean.
- `cargo test --workspace` ✓ clean (181 passing across the
  workspace; 0 failures; 20 ignored by `#[ignore]` or feature
  gating as designed).

To land alongside the orphan-file deletion as a single
pre-publish-cleanup commit walking all three dirty submodules
plus the parent pointer bumps.
