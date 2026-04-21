#!/bin/sh
# scripts/rust-lint.sh — run the lint trio (fmt-check + check +
# clippy with `-D warnings`) across the workspace or a single
# crate.
#
# Usage:
#   ./scripts/rust-lint.sh               # whole workspace
#   ./scripts/rust-lint.sh <crate-name>  # single crate
#
# Runs in order:
#   cargo fmt   (--all | -p <crate>) --check
#   cargo check (--workspace | -p <crate>)
#   cargo clippy (--workspace | -p <crate>) --all-targets -- -D warnings
#
# Mandated for lint passes in this workspace — prefer this over
# raw `cargo fmt/check/clippy`. Bespoke cargo invocations remain
# fine for exceptional cases (e.g. clippy with a specific lint
# toggled), but the canonical pre-landing lint run goes through
# this script.
#
# If the fmt-check step fails, run `cargo fmt --all` (or
# `cargo fmt -p <crate>`) to apply the missing formatting, then
# re-run the script.
#
# See docs/design/13-conventions.md §Pre-landing checks.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"

if [ $# -gt 1 ]; then
    echo "Usage: $0 [<crate-name>]" >&2
    exit 2
fi

if [ $# -eq 1 ]; then
    crate=$1
    printf '=== rust-lint scope: crate %s ===\n' "$crate"
    echo '--- cargo fmt -p '"$crate"' --check ---'
    cargo fmt -p "$crate" --check
    echo '--- cargo check -p '"$crate"' ---'
    cargo check -p "$crate"
    echo '--- cargo clippy -p '"$crate"' --all-targets -- -D warnings ---'
    cargo clippy -p "$crate" --all-targets -- -D warnings
else
    echo '=== rust-lint scope: workspace ==='
    echo '--- cargo fmt --all --check ---'
    cargo fmt --all --check
    echo '--- cargo check --workspace ---'
    cargo check --workspace
    echo '--- cargo clippy --workspace --all-targets -- -D warnings ---'
    cargo clippy --workspace --all-targets -- -D warnings
fi

echo '=== rust-lint: clean ==='
