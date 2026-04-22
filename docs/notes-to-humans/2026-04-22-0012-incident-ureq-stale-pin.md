# Incident: `ureq` pinned at 2.x while 3.x is current upstream

**Date filed:** 2026-04-22
**Severity:** low (tooling only; not on any published crate's critical path)
**Discovered by:** Yuka, while reviewing dep pins
**Fixed by:** commit landing alongside this note

## What happened

`xtask/Cargo.toml` pinned `ureq = "2"` and had been using 2.x's
API (`RequestBuilder::set`, `ureq::Error::Status(code, resp)`,
`response.into_string()`, `response.into_reader()`) in two bins:

- `xtask/src/bin/crates-io-versions.rs` — the very tool the
  workspace mandates for crate-version lookups.
- `xtask/src/bin/web-fetch.rs` — the Rust-bin replacement for
  the curl / wget wrapper.

crates.io has had `ureq 3.x` for some time (latest at time of
discovery: `3.3.0`). The 2.x line is effectively in maintenance.
We should have been on 3.x already.

## Why it slipped through

The xtask bins were written / extended at various points this
week. At each of those points I pinned ureq by writing `ureq = "2"`
without running
`./scripts/xtask.sh crates-io-versions -- ureq` to confirm that
was actually the current major. I was going from memory /
training-data intuition rather than the live index — which is
exactly the anti-pattern the workspace rule at
[CLAUDE.md](../../CLAUDE.md) §"Never recall a Rust crate's
published version from memory" is meant to prevent.

The rule is there for crate-version pins across the whole
workspace, not just for published-crate dependencies. xtask is
`publish = false`, so the pin doesn't ship anywhere public, but
the principle — reach for the live index, not memory — applies
regardless of where the Cargo.toml lives.

## Fix

- Bumped `xtask/Cargo.toml`: `ureq = "2"` → `ureq = "3"`.
- Migrated both bins to ureq 3.x's API:
  - `.set(key, val)` → `.header(key, val)`.
  - `ureq::Error::Status(code, resp)` →
    `ureq::Error::StatusCode(code)`. The response is no longer
    attached on the error path, so the 2.x-style
    `resp.status_text()` fallback is gone; error messages now
    carry only the numeric code. This is a minor loss of
    diagnostic detail but consistent with how most HTTP clients
    model errors.
  - `response.into_string()` →
    `response.into_body().read_to_string()`.
  - `response.into_reader()` →
    `response.into_body().into_reader()`.
- Smoke-tested both bins against real endpoints (crates.io
  sparse index for `thiserror`, crates.io JSON API for both a
  real crate and a known-missing crate) — 200 and 404 paths
  both behave correctly.
- `./scripts/rust-lint.sh xtask` clean.

## What this incident should change going forward

**Not a new rule.** The existing rule already covers this —
[CLAUDE.md](../../CLAUDE.md) and
[docs/design/13-conventions.md §Crate version lookup](../design/13-conventions.md)
both say the version lookup goes through
`./scripts/xtask.sh crates-io-versions -- <crate>`, no
exceptions, not even for internal tooling and not even for
"well-known" crates.

The correction for me is **mechanical consistency**: every time
I write or touch a `Cargo.toml` line that pins a version, run
the lookup, even if the crate feels stable or the xtask is
internal. This incident is the second time a stale version has
leaked in this week (the first was `coset = "0.3"` vs `"0.4"`
during Phase 3 dispatch, caught by Yuka before Codex ran; the
third was `Sha256`'s CBOR shape mismatch in the Wave A
proposal, caught while drafting the reference generator). All
three had the same root cause: I was going from memory rather
than the live index.

Going forward: treat "does this version match live crates.io?"
as a required pre-commit step on any Cargo.toml change. The
wrapper is one command; the cost of skipping it is commits like
this one.

## Why file this as an incident at all

The scope here is genuinely narrow — xtask tooling, not a
crypto path, not a published crate. But the pattern that caused
it (skip-the-lookup) is exactly the kind of habit erosion that
compounds: today a stale tooling pin, next week a stale
dependency pin on a published crate, next month a stale
crypto-library pin that lands in a Gate-1 proposal. Documenting
it explicitly is cheaper than waiting for the compounding one.

It also serves as a fixed reference to cite later if the same
pattern recurs — "this is the third time; either the rule isn't
landing or the lookup step needs to be automated."
