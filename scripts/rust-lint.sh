#!/bin/sh
# scripts/rust-lint.sh — run the lint quartet (fmt-check + check +
# clippy + rustdoc missing_docs) across the workspace, a single
# crate, or the in-tree `xtask` dev tooling.
#
# Usage:
#   ./scripts/rust-lint.sh               # workspace, EXCLUDING xtask
#   ./scripts/rust-lint.sh <crate-name>  # single crate (target-main, or
#                                        # target-xtask if crate is xtask)
#   ./scripts/rust-lint.sh --xtask       # ONLY xtask, target-xtask
#
# Runs in order:
#   cargo fmt   <scope> --check
#   cargo check <scope>
#   cargo clippy <scope> --all-targets -- -D warnings
#   RUSTDOCFLAGS="-D missing_docs" cargo doc --no-deps <scope>
#
# Scope mode:
#   workspace (default) — `cargo check/clippy/doc/test --workspace
#       --exclude xtask`. Workspace fmt-check enumerates every
#       non-xtask workspace member and passes one `-p <name>` flag
#       per member. xtask is the in-tree dev-tooling crate
#       (CONTRIBUTING.md §8); it has its own `target-xtask/` build
#       cache and is checked separately so workspace builds and
#       Codex runs share `target-main/` without xtask compilation
#       artifacts piling up. Run `--xtask` explicitly when xtask
#       changes (or before publishing changes that affect dev
#       tooling).
#   single crate (positional arg) — `cargo … -p <crate>`. If the
#       crate name is `xtask`, CARGO_TARGET_DIR is set to
#       `target-xtask` so the build cache stays isolated from
#       `target-main`.
#   xtask (`--xtask`) — `cargo … -p xtask`, with
#       CARGO_TARGET_DIR=target-xtask. Equivalent to passing the
#       crate name `xtask` positionally.
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
. "$(dirname -- "$0")/lib/cargo-noise-filter.sh"

# Parse --xtask early so we can pin CARGO_TARGET_DIR=target-xtask
# *before* sourcing cargo-target-dir.sh (which only sets the
# default when CARGO_TARGET_DIR is unset, preserving any explicit
# value we set here).
xtask_only=0
if [ "${1:-}" = "--xtask" ]; then
    xtask_only=1
    shift
fi

if [ "$xtask_only" -eq 1 ] && [ $# -gt 0 ]; then
    echo "Usage: $0 [--xtask | <crate-name>]" >&2
    echo "       --xtask is mutually exclusive with a positional crate arg." >&2
    exit 2
fi

if [ $# -gt 1 ]; then
    echo "Usage: $0 [--xtask | <crate-name>]" >&2
    exit 2
fi

# Pin target-xtask when xtask is the only thing being checked, so
# the build cache stays isolated from `target-main` (used by
# workspace builds, CLI cargo invocations, and Codex). This
# applies to both `--xtask` and `<crate-name>=xtask`.
if [ "$xtask_only" -eq 1 ] || [ "${1:-}" = "xtask" ]; then
    CARGO_TARGET_DIR=target-xtask
    export CARGO_TARGET_DIR
fi

. "$(dirname -- "$0")/lib/cargo-target-dir.sh"

if [ "$xtask_only" -eq 1 ] || [ "${1:-}" = "xtask" ]; then
    crate=xtask
    printf '%s=== rust-lint scope: xtask only (CARGO_TARGET_DIR=%s) ===%s\n' \
        "$C_HEADER" "$CARGO_TARGET_DIR" "$C_RESET"
    printf '%s--- cargo fmt -p xtask --check ---%s\n' "$C_DIM" "$C_RESET"
    run_with_cargo_noise_filter cargo fmt -p "$crate" --check
    printf '%s--- cargo check -p xtask ---%s\n' "$C_DIM" "$C_RESET"
    run_with_cargo_noise_filter cargo check -p "$crate"
    printf '%s--- cargo clippy -p xtask --all-targets -- -D warnings ---%s\n' "$C_DIM" "$C_RESET"
    run_with_cargo_noise_filter cargo clippy -p "$crate" --all-targets -- -D warnings
    printf '%s--- cargo doc -p xtask (missing_docs) ---%s\n' "$C_DIM" "$C_RESET"
    RUSTDOCFLAGS="-D missing_docs" run_with_cargo_noise_filter cargo doc --no-deps -p "$crate"
elif [ $# -eq 1 ]; then
    crate=$1
    printf '%s=== rust-lint scope: crate %s ===%s\n' "$C_HEADER" "$crate" "$C_RESET"
    printf '%s--- cargo fmt -p %s --check ---%s\n' "$C_DIM" "$crate" "$C_RESET"
    run_with_cargo_noise_filter cargo fmt -p "$crate" --check
    printf '%s--- cargo check -p %s ---%s\n' "$C_DIM" "$crate" "$C_RESET"
    run_with_cargo_noise_filter cargo check -p "$crate"
    printf '%s--- cargo clippy -p %s --all-targets -- -D warnings ---%s\n' "$C_DIM" "$crate" "$C_RESET"
    run_with_cargo_noise_filter cargo clippy -p "$crate" --all-targets -- -D warnings
    printf '%s--- cargo doc -p %s (missing_docs) ---%s\n' "$C_DIM" "$crate" "$C_RESET"
    RUSTDOCFLAGS="-D missing_docs" run_with_cargo_noise_filter cargo doc --no-deps -p "$crate"
else
    # Workspace mode — exclude xtask. fmt has no `--exclude` flag,
    # so enumerate non-xtask workspace members and pass one
    # `-p <name>` per member to a single fmt invocation.
    . "$(dirname -- "$0")/lib/workspace-members.sh"
    fmt_pkgs=
    for member in $workspace_members; do
        if [ -f "$member/Cargo.toml" ]; then
            name=$(sed -n 's/^name *= *"\([^"]*\)"/\1/p' "$member/Cargo.toml" | head -1)
        else
            name=$(basename "$member")
        fi
        if [ -n "$name" ] && [ "$name" != "xtask" ]; then
            fmt_pkgs="$fmt_pkgs -p $name"
        fi
    done

    printf '%s=== rust-lint scope: workspace (excluding xtask) ===%s\n' "$C_HEADER" "$C_RESET"
    printf '%s--- cargo fmt%s --check ---%s\n' "$C_DIM" "$fmt_pkgs" "$C_RESET"
    # shellcheck disable=SC2086
    run_with_cargo_noise_filter cargo fmt $fmt_pkgs --check
    printf '%s--- cargo check --workspace --exclude xtask ---%s\n' "$C_DIM" "$C_RESET"
    run_with_cargo_noise_filter cargo check --workspace --exclude xtask
    printf '%s--- cargo clippy --workspace --exclude xtask --all-targets -- -D warnings ---%s\n' \
        "$C_DIM" "$C_RESET"
    run_with_cargo_noise_filter cargo clippy --workspace --exclude xtask --all-targets -- -D warnings
    printf '%s--- cargo doc --workspace --exclude xtask (missing_docs) ---%s\n' "$C_DIM" "$C_RESET"
    RUSTDOCFLAGS="-D missing_docs" run_with_cargo_noise_filter cargo doc --no-deps --workspace --exclude xtask
fi

printf '%s=== rust-lint: clean ===%s\n' "$C_OK" "$C_RESET"
