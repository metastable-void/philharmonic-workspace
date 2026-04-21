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
# Auto-detects modified crates as submodules with a dirty working
# tree (unstaged changes, staged changes, or untracked files).
# Pass crate names explicitly to override.
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
    printf '=== pre-landing: explicit crates: %s ===\n' "$crates"
else
    # Submodules with unstaged, staged, or untracked changes count
    # as modified. `show-dirty.sh` emits one name per line.
    crates=$(./scripts/show-dirty.sh)
    if [ -n "$crates" ]; then
        # Collapse newlines to spaces for the header. $crates is
        # intentionally unquoted so default-IFS word-splitting
        # flattens the newlines.
        # shellcheck disable=SC2086
        printf '=== pre-landing: auto-detected modified crates: %s ===\n' "$(printf '%s ' $crates)"
    else
        echo '=== pre-landing: no modified crates detected; running workspace checks only ==='
    fi
fi

./scripts/check-toolchain.sh
./scripts/rust-lint.sh
./scripts/rust-test.sh

if [ "$no_ignored" -eq 1 ]; then
    echo '=== pre-landing: --no-ignored; skipping step 3 ==='
elif [ -n "$crates" ]; then
    # shellcheck disable=SC2086
    for c in $crates; do
        printf '=== pre-landing: --ignored phase for %s ===\n' "$c"
        ./scripts/rust-test.sh --ignored "$c"
    done
else
    echo '=== pre-landing: no --ignored phase needed (no modified crates) ==='
fi

echo '=== pre-landing: all checks passed ==='
