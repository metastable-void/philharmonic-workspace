# CI rust version can be ahead of local rust version

**Date:** 2026-04-21
**Surfaced by:** first CI run on commit `43fc22b`, which failed
with `clippy::while_let_loop` in `mechanics-config/src/template.rs`
even though `./scripts/pre-landing.sh` passed locally.

## The observation

- **Local `rustc`:** 1.94.1 (on this dev machine at the time of
  writing).
- **CI `rustc`:** 1.95.0 (GitHub Actions `dtolnay/rust-toolchain@stable`
  resolves to the current stable, which is ahead of local).
- **Effect:** clippy rules that tighten or become pedantic in a
  newer stable (here: `clippy::while_let_loop` for the `loop` +
  `let Some(...) = ... else { break }` pattern) can break CI
  while local pre-landing passes.

Pre-landing remains defense-in-depth; CI is authoritative.

## The fix (applied)

Rewrote the offending `loop` in `mechanics-config/src/template.rs`
as a `while let`. The rewrite is behavior-preserving and passes
both 1.94.1 and 1.95.0 clippy, so we don't lock in
version-specific behavior.

## Why this keeps happening

`cargo-semver-checks`, `cargo-audit`, `cargo` itself, and
workspace-level toolchain choices all evolve on their own
cadence. Running `dtolnay/rust-toolchain@stable` in CI picks up
whatever stable is on the day the job runs. Local dev rust
trails as long as the developer doesn't actively `rustup update`.

Three realistic stances:

1. **Pin CI to a specific rust version** (e.g.
   `dtolnay/rust-toolchain@1.94.1`). Keeps CI deterministic,
   matches local. Downside: miss security fixes in newer stables,
   and delay discovering new lint regressions.
2. **`rustup update` before every pre-landing run.** Best
   fidelity; worst ergonomics.
3. **Accept the drift** — fix CI-only failures as they happen,
   which is what we did this time.

Stance 3 is fine for now; the cost of a CI-only lint failure is
a one-commit follow-up. Revisit if the frequency gets annoying,
or consider stance 1 when we care about reproducibility for a
release.

## Related

- The commit that broke: `43fc22b`.
- The fix: to be committed in a submodule bump of
  `mechanics-config` (`src/template.rs` only).
- Workspace convention: `./scripts/pre-landing.sh` is the canonical
  local check; CI runs the same script so behavior drift comes
  from the toolchain, not the script.

## Follow-up (same session)

Added `./scripts/check-toolchain.sh [--update]` which prints the
local rustc/cargo versions and runs `rustup check` (or
`rustup update` with `--update`). Wired it as step 0 of
`pre-landing.sh`, so every pre-landing run surfaces a pending
update before the drift turns into a CI failure.

Also: by the time that script was added, local had already been
`rustup update`'d to 1.95.0 (matching CI), so this specific
instance is resolved. The note stays in the archive because the
*pattern* will recur — the script exists to make the next
occurrence noisy at contributor time instead of silent until CI.
