#!/bin/sh
# scripts/rust-lint.sh — run the lint quartet (fmt-check + check +
# clippy + rustdoc missing_docs) across the workspace or a single
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
#   RUSTDOCFLAGS="-D missing_docs" cargo doc --no-deps
#
# The rustdoc step catches undocumented public items in all
# workspace crates.
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
. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/cargo-target-dir.sh"

if [ $# -gt 1 ]; then
    echo "Usage: $0 [<crate-name>]" >&2
    exit 2
fi

if [ $# -eq 1 ]; then
    crate=$1
    printf '%s=== rust-lint scope: crate %s ===%s\n' "$C_HEADER" "$crate" "$C_RESET"
    printf '%s--- cargo fmt -p %s --check ---%s\n' "$C_DIM" "$crate" "$C_RESET"
    cargo fmt -p "$crate" --check
    printf '%s--- cargo check -p %s ---%s\n' "$C_DIM" "$crate" "$C_RESET"
    cargo check -p "$crate"
    printf '%s--- cargo clippy -p %s --all-targets -- -D warnings ---%s\n' "$C_DIM" "$crate" "$C_RESET"
    cargo clippy -p "$crate" --all-targets -- -D warnings
    printf '%s--- cargo doc -p %s (missing_docs) ---%s\n' "$C_DIM" "$crate" "$C_RESET"
    RUSTDOCFLAGS="-D missing_docs" cargo doc --no-deps -p "$crate"
else
    printf '%s=== rust-lint scope: workspace ===%s\n' "$C_HEADER" "$C_RESET"
    printf '%s--- cargo fmt --all --check ---%s\n' "$C_DIM" "$C_RESET"
    cargo fmt --all --check
    printf '%s--- cargo check --workspace ---%s\n' "$C_DIM" "$C_RESET"
    cargo check --workspace
    printf '%s--- cargo clippy --workspace --all-targets -- -D warnings ---%s\n' "$C_DIM" "$C_RESET"
    cargo clippy --workspace --all-targets -- -D warnings
    printf '%s--- cargo doc --workspace (missing_docs) ---%s\n' "$C_DIM" "$C_RESET"
    RUSTDOCFLAGS="-D missing_docs" cargo doc --no-deps --workspace
fi

printf '%s=== rust-lint: clean ===%s\n' "$C_OK" "$C_RESET"
