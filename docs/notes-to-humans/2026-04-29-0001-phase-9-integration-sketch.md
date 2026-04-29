# Phase 9 integration sketch

**Date**: 2026-04-29 (昭和の日)
**Author**: Claude Code
**Context**: Based on the integration vision in HUMANS.md
(updated today) and the current crate state post-Phase 8.

---

## Where things stand

All library crates are published. The `philharmonic` meta-crate
exists as a 0.0.0 placeholder with no dependencies. No bin
targets, no WebUI, no server infrastructure exists yet.
`common-assets/d-icon.svg` is already committed.

Three connector impl crates remain at 0.0.0:
`llm-anthropic`, `llm-gemini`, `email-smtp` — deferred to
Phase 7 Tiers 2–3 (SMTP after GW, LLM providers on/after
2026-05-07). The meta-crate can feature-gate these so they're
opt-in once they ship.

## Proposed integration order

### Step 1 — Meta-crate re-exports + feature flags

Turn `philharmonic` from a placeholder into a real facade:

- Add all published library crates as dependencies.
- Re-export each at the top level (`pub use philharmonic_types;`
  etc.).
- Feature-gate each connector impl (all shipped ones default-on).
  The three unshipped impls get feature flags that are off by
  default until their 0.1.0 lands.
- `default-features = false` suppresses all connector impls for
  consumers who want to pick individually.
- Bump to `0.1.0` and publish. This unblocks downstream
  consumers who want a single dep.

### Step 2 — Shared server infrastructure module

Before writing the three bins, build the common scaffolding
they'll all share. This can live as a `server` module inside
`philharmonic`'s `src/`, or as a separate in-tree crate if it
grows too large (prefer the module first — YAGNI).

What the module needs:

- **`https` feature**: `rustls` + `tokio-rustls` for TLS
  termination, HTTP/2 via axum/hyper's existing support.
  HTTP/3 (QUIC) is a stretch goal, not a gate.
- **SIGHUP handler**: `tokio::signal::unix::signal(SIGHUP)` →
  re-read config + refresh TLS certs from disk without
  downtime.
- **Clap CLI skeleton**: `serve` (default), `version`, `help`
  subcommands shared across all three bins. The `install`
  subcommand is a separate concern (step 6).
- **TOML config loading**: primary file
  (`/etc/philharmonic/<name>.toml`) + drop-in directory
  (`/etc/philharmonic/<name>.toml.d/*.toml`) merged in
  lexicographic order. Location overridable via CLI flag.
  `serde` + `toml` — already workspace deps.

HUMANS.md says "let's extend `mechanics` HTTP server crate to
that shape first." Concretely: `mechanics` already provides an
HTTP server; the shared module should integrate smoothly with
what `mechanics` exposes, and the `mechanics-worker` bin
should be the first consumer that proves the pattern works.

### Step 3 — `mechanics-worker` binary (first, simplest)

`philharmonic/src/bin/mechanics-worker/main.rs`

- Wraps `mechanics`' HTTP server with Clap CLI + config file
  loading + optional TLS + SIGHUP.
- Config: `/etc/philharmonic/mechanics.toml`.
- This is the simplest bin (single concern: JS execution) and
  proves the shared server pattern before the more complex
  bins.

### Step 4 — `philharmonic-connector` binary

`philharmonic/src/bin/philharmonic-connector/main.rs`

- Wraps `philharmonic-connector-service` + all shipped impl
  crates.
- Feature flags mirror the meta-crate's connector features.
  `default-features = false` in `Cargo.toml` + manual feature
  selection lets operators deploy only what they need.
- Config: `/etc/philharmonic/connector.toml`.
- Run one per realm.

### Step 5 — `philharmonic-api` binary (most complex)

`philharmonic/src/bin/philharmonic-api/main.rs`

- Wires: `philharmonic-api` (the library crate), store backend
  (`philharmonic-store-sqlx-mysql`), policy, SCK + signing
  keys from config, verifying key registry.
- Config: `/etc/philharmonic/api.toml`.
- Embeds WebUI static assets (see step 7) via `include_bytes!`
  or `rust-embed`. SPA routing: any path not matching
  `/v1/...` serves `index.html`.
- Integrates the connector proxy feature (the API server is
  what faces the internet on port 443).
- HTTPS is the default for this bin.

### Step 6 — `install` subcommand

Each bin gets an `install` subcommand (requires root):

- Copies the binary to `/usr/local/bin/`.
- Writes a systemd service unit to
  `/usr/local/lib/systemd/system/<name>.service`, creating
  intermediate directories.
- Creates config directories + default config files.
- Runs `systemctl enable <name>.service` (but not start).
- Idempotent. Prints "how to configure" instructions at the end.

This is lower priority than getting the bins running.
Can ship after the core serve path works.

### Step 7 — WebUI

Redux + React + Webpack. Build artifacts committed to Git:

- `index.html`, `main.js`, `main.css`, `icon.svg`
  (copied from `common-assets/d-icon.svg`)
- No Node.js needed at Rust build time — the JS build is a
  separate, human-triggered step. The Rust binary embeds the
  committed artifacts.
- SPA routing: everything except API paths and known static
  files → `index.html`.

Minimum viable surface:

- Login (paste a `pht_` long-lived token).
- Workflow template list / create / read.
- Instance create / read / history / steps.
- Step execution.
- Audit log inspection.

The WebUI is a test/demo artifact, not the product. Keep it
minimal — its purpose is proving the API works from a browser
and exercising the ephemeral-token flow in a real frontend.

### Step 8 — End-to-end testcontainers

- MySQL via `testcontainers`.
- Spawn all three bins (in-process or as child processes).
- Test flows per ROADMAP: tenant admin → endpoint config →
  workflow template → step execution → terminal state → audit.
- Ephemeral-token flow: minting authority → instance-scoped
  token → client step execution → subject in records.

### Step 9 — musl cross-compilation

- Verify `x86_64-unknown-linux-musl` builds for all three bins.
- CI cross-compile target.
- The `embed` crate's `inline-blob` + 2.27 GB bge-m3 weights
  may need special handling for musl link (already solved for
  ELF via `.lrodata.*`, but verify the musl linker path).

### Step 10 — Docker compose (optional, post-deadline)

- Minimal Alpine images.
- Local override files for HTTPS certs, hostnames, etc.
- Not blocking for 5/2; nice-to-have for reference deployment.

---

## Realistic scope for 2026-05-02

Three working days remain (4/29 afternoon, 4/30, 5/1) plus
5/2 itself. The full integration plan above is ambitious for
that window. Suggested cut order if time pressure forces it:

**Must-have by 5/2:**
1. Meta-crate wiring (step 1) — half a day.
2. At least one bin target running with TLS + config (steps
   2–3, or jump to step 5 if the API bin is higher priority).
3. Testcontainers e2e happy path (step 8) — even if it
   exercises library crates in-process rather than the bin
   targets.

**Should-have by 5/2:**
4. All three bin targets (steps 3–5).
5. WebUI skeleton (login + one CRUD flow).

**Can slip past 5/2:**
6. `install` subcommand (step 6).
7. Full WebUI surface (step 7 beyond skeleton).
8. musl verification (step 9).
9. Docker compose (step 10).
10. Reference deployment on real infrastructure (ROADMAP
    Phase 9 task 3).

## Open questions for Yuka

1. **Bin target home**: HUMANS.md says `philharmonic`
   meta-crate. ROADMAP Phase 9 task 2 says "in-tree,
   non-published, mirrors xtask pattern." These are at odds.
   My recommendation: put the bins in `philharmonic` (the
   meta-crate) since that's what ships. The "test/demo"
   qualifier still applies — these bins are examples of one
   deployment shape, not prescriptions. Alternatively, a
   separate in-tree `philharmonic-bins/` crate (never
   published) could hold them, keeping the meta-crate as
   pure re-exports.

2. **WebUI toolchain**: HUMANS.md says Redux + React + Webpack.
   Is this firm, or would a lighter alternative (e.g. Preact +
   esbuild, or even a pure vanilla-JS SPA) be acceptable for
   the test/demo scope? The committed-artifacts pattern works
   either way.

3. **Which bin first**: HUMANS.md says "extend mechanics first."
   But the API bin is the most integration-heavy and the one
   that faces the internet. If time is tight, should we start
   with the API bin (highest integration value) or the
   mechanics-worker bin (simplest, proves the pattern)?

4. **Connector proxy in the API bin**: HUMANS.md says the API
   server "integrates the Connector proxy feature into itself."
   This means the API binary also acts as a connector router?
   Or does it forward to a separate connector router process?
   The former is simpler for single-machine deployments; the
   latter matches the crate architecture better.
