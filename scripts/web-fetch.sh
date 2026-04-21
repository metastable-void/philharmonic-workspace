#!/bin/sh
# scripts/web-fetch.sh — thin shim calling `./scripts/xtask.sh
# web-fetch -- "$@"`. The real implementation is
# `xtask/src/bin/web-fetch.rs` — a Rust bin using `ureq` that
# doesn't depend on `curl` / `wget` / `fetch` / `ftp` being on
# PATH (those aren't shipped on stripped GNU/Linux or macOS
# baselines).
#
# Preserved as a shell entry point so shell callers (notably
# `print-audit-info.sh`, which runs inside `commit-all.sh`) keep
# working without learning about xtask. Prefer
# `./scripts/xtask.sh web-fetch -- <URL> [<outfile>]` at new
# call sites; use this shim when you're already in shell and
# want a short single-command invocation.
#
# Usage:
#   ./scripts/web-fetch.sh <URL> [<outfile>]
#
# User-Agent: `WEB_FETCH_UA` env var, default
# `philharmonic-dev-agent/1.0`. HTTP 4xx/5xx fail the fetch
# (exit 2). Use `./scripts/web-fetch.sh ... || :` at the call
# site to tolerate HTTP errors — `print-audit-info.sh` uses that
# idiom for best-effort IP geolocation.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
exec "$(dirname -- "$0")"/xtask.sh web-fetch -- "$@"
