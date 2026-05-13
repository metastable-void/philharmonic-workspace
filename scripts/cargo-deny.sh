#!/bin/sh
# scripts/cargo-deny.sh — run `cargo deny check bans` against the
# workspace, auto-installing `cargo-deny` via `cargo install --locked`
# on first use.
#
# The workspace's `deny.toml` configures the full set of cargo-deny
# checks (advisories, licenses, bans, sources). This wrapper only
# runs the **bans** check by default — the project relies on
# `cargo-audit.sh` for advisory scanning, treats licenses as a
# release-time concern rather than a per-commit gate, and keeps the
# sources check off the pre-landing path entirely. Pass through
# extra args to run something else (e.g. `./scripts/cargo-deny.sh
# check all` for a full local audit).
#
# Usage:
#   ./scripts/cargo-deny.sh                          # cargo deny check bans
#   ./scripts/cargo-deny.sh check all                # full cargo-deny pass
#   ./scripts/cargo-deny.sh check licenses           # ad-hoc license review
#
# When to run:
#   - Automatically: every `./scripts/pre-landing.sh` invocation.
#   - Manually: before bumping a workspace dep version, to verify the
#     new transitive set doesn't pull in a banned crate.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"

if ! command -v cargo-deny >/dev/null 2>&1; then
    echo '=== installing cargo-deny ==='
    cargo install --locked cargo-deny
fi

if [ $# -eq 0 ]; then
    set -- check bans
fi

printf '=== cargo deny %s ===\n' "$*"
cargo deny "$@"
