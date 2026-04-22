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

## Known fallout of the 2 → 3 bump (fixed)

The IPv6 leg of `print-audit-info.sh`'s IP-geolocation probe
initially regressed to "empty" after the bump. Root cause was
specific to ureq 3.x + rustls + IPv6 literals in URLs:

- `print-audit-info.sh` used to fetch
  `https://1.1.1.1/cdn-cgi/trace` (for v4) and
  `https://[2606:4700:4700::1111]/cdn-cgi/trace` (for v6, to
  force the socket family via IP literal).
- In ureq 3.x, the TLS layer calls
  `rustls_pki_types::ServerName::try_from(uri.authority().host())`
  for SNI. With `http 1.x`'s `.host()` returning the IPv6
  literal *with brackets* (`"[2606:...]"`), that string is
  neither a valid DNS name nor a parseable `IpAddr`, so the
  conversion fails with `"Rustls invalid dns name error"` and
  the v6 probe came back empty.
- ureq 2.x did not hit this (its `.host()` handling or rustls
  wiring avoided the bracketed form).

Concrete effect (pre-fix): commit `100605d`'s `Audit-Info`
trailer read `6=/` whereas prior commits read e.g.
`6=240b:10:9fc3:6e00:…/JP`. v4 was unaffected.

**Fix (Yuka's call, implemented immediately after filing this
note):** switch both probes to
`https://ipv4.icanhazip.com/cdn-cgi/trace` and
`https://ipv6.icanhazip.com/cdn-cgi/trace`. Both Cloudflare-
operated, same `cdn-cgi/trace` endpoint as `1.1.1.1` served,
same output format (ip / loc / etc). Key property:
`ipv4.icanhazip.com` has only A records and
`ipv6.icanhazip.com` has only AAAA, so DNS forces the socket
family without us having to use a bracketed IPv6 literal in
the URL. `ServerName::try_from(hostname)` succeeds as a DNS
name; happy-eyeballs can't accidentally pick the wrong family
because the other family doesn't resolve at all.

Post-fix audit trailer (this session): `4=106.72.159.195/JP
6=240b:10:9fc3:6e00:…/JP` — both fields populated again.

Approaches considered and rejected before Yuka pointed me at
icanhazip:

- **Hostname with both A + AAAA** (e.g. `one.one.one.one`):
  happy-eyeballs picks either family and we can't tell which,
  so the result collapses to duplicating the v4 probe.
- **Strip brackets before the SNI lookup**: would need a
  custom ureq `Transport` / TLS connector — internal-API
  rabbit hole for a best-effort probe.
- **Disable TLS verification for the v6 probe only**: a
  `disable_verification(true)` setting exists in ureq's
  `TlsConfig`, but it doesn't bypass the `ServerName::try_from`
  step that's actually failing here.
- **Fetch the v6 probe over plain HTTP**: Cloudflare serves
  `/cdn-cgi/trace` over HTTP to some paths but redirects
  others; adds fragility with no clear upside.

The icanhazip fix is strictly better than any of those — same
reliability, no internal-API coupling, DNS does the work.

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
