#!/bin/sh
# scripts/pre-landing.sh — the canonical pre-landing-check driver.
#
# Runs the mandated flow in order:
#   0. ./scripts/check-toolchain.sh             (prints rust/cargo versions;
#                                                if rustup is installed, runs
#                                                `rustup check` to surface
#                                                pending toolchain updates)
#   1. ./scripts/rust-lint.sh                   (fmt + check + clippy -D warnings)
#   2. ./scripts/rust-test.sh                   (cargo test --workspace, skips #[ignore])
#   3. ./scripts/rust-test.sh --ignored <X>     for each modified crate X
#
# Step 3 exercises the `#[ignore]`-gated integration tests (the
# testcontainers / live-service ones) for crates you actually
# changed — the workspace-level run in step 2 skips them for
# speed.
#
# Auto-detects modified crates as workspace members with a dirty
# working tree (unstaged changes, staged changes, or untracked
# files). Submodule-backed and in-tree (non-submodule, e.g.
# `xtask`) members are both covered uniformly via
# `./scripts/show-dirty.sh`. Pass crate names explicitly to
# override.
#
# Usage:
#   ./scripts/pre-landing.sh                    # auto-detect modified crates
#   ./scripts/pre-landing.sh <crate>...         # explicit list
#   ./scripts/pre-landing.sh --no-ignored       # skip step 3 (rare; use for fast iteration)
#   ./scripts/pre-landing.sh --no-ignored <crate>...
#
# Run before every commit that touches Rust code. GitHub CI runs
# this same script (with a clean checkout → no dirty crates → no
# --ignored phase), so contributor and CI behavior don't drift.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

no_ignored=0
while [ $# -gt 0 ]; do
    case "$1" in
        --no-ignored) no_ignored=1; shift ;;
        --)                           shift; break ;;
        -*) printf 'unknown flag: %s\n' "$1" >&2; exit 2 ;;
        *) break ;;
    esac
done

if [ $# -gt 0 ]; then
    crates=$*
    printf '%s=== pre-landing: explicit crates: %s ===%s\n' "$C_HEADER" "$crates" "$C_RESET"
else
    # Workspace members (submodule-backed or in-tree) with
    # unstaged, staged, or untracked changes count as modified.
    # `show-dirty.sh` emits one name per line.
    crates=$(./scripts/show-dirty.sh)
    if [ -n "$crates" ]; then
        # Collapse newlines to spaces for the header. $crates is
        # intentionally unquoted so default-IFS word-splitting
        # flattens the newlines.
        # shellcheck disable=SC2086
        printf '%s=== pre-landing: auto-detected modified crates: %s ===%s\n' \
            "$C_HEADER" "$(printf '%s ' $crates)" "$C_RESET"
    else
        printf '%s=== pre-landing: no modified crates detected; running workspace checks only ===%s\n' \
            "$C_HEADER" "$C_RESET"
    fi
fi

./scripts/check-toolchain.sh
./scripts/rust-lint.sh
./scripts/rust-test.sh

if [ "$no_ignored" -eq 1 ]; then
    printf '%s=== pre-landing: --no-ignored; skipping step 3 ===%s\n' "$C_HEADER" "$C_RESET"
elif [ -n "$crates" ]; then
    # shellcheck disable=SC2086
    for c in $crates; do
        printf '%s=== pre-landing: --ignored phase for %s ===%s\n' "$C_HEADER" "$c" "$C_RESET"
        ./scripts/rust-test.sh --ignored "$c"
    done
else
    printf '%s=== pre-landing: no --ignored phase needed (no modified crates) ===%s\n' "$C_HEADER" "$C_RESET"
fi

printf '%s=== pre-landing: all checks passed ===%s\n' "$C_OK" "$C_RESET"
