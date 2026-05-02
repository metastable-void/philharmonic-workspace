#!/bin/sh
# scripts/rust-test.sh — run `cargo test` across the workspace, a
# single crate, or the in-tree `xtask` dev tooling, with optional
# control over `#[ignore]`-gated tests.
#
# Usage:
#   ./scripts/rust-test.sh                            workspace (excl. xtask), skip #[ignore]
#   ./scripts/rust-test.sh <crate>                    one crate, skip #[ignore]
#                                                     (CARGO_TARGET_DIR=target-xtask if crate is xtask)
#   ./scripts/rust-test.sh --xtask                    only xtask, skip #[ignore], target-xtask
#   ./scripts/rust-test.sh --ignored                  workspace (excl. xtask), only #[ignore]'d
#   ./scripts/rust-test.sh --ignored <crate>          one crate, only #[ignore]'d
#   ./scripts/rust-test.sh --xtask --ignored          only xtask, only #[ignore]'d, target-xtask
#   ./scripts/rust-test.sh --include-ignored          workspace (excl. xtask), all tests
#   ./scripts/rust-test.sh --include-ignored <crate>  one crate, all tests
#   ./scripts/rust-test.sh --xtask --include-ignored  only xtask, all tests, target-xtask
#
# Workspace mode excludes xtask via `cargo test --workspace
# --exclude xtask`. xtask has its own `target-xtask/` build cache
# (CONTRIBUTING.md §8.1) and is checked separately so workspace
# builds and Codex runs share `target-main/` without xtask
# compilation artifacts piling up. Pass `--xtask` (or the crate
# name `xtask` positionally) to scope to xtask alone with
# `target-xtask`.
#
# `#[ignore]` is the project convention for tests that need real
# infrastructure (testcontainers, live network, DB servers,
# external API keys, etc.). The default run skips them so the
# pre-landing loop stays fast — for a modified crate, run the
# `--ignored` variant against that crate separately to exercise
# its integration tests. See docs/design/13-conventions.md
# §Pre-landing checks.
#
# Mandated for test passes in this workspace — prefer this over
# raw `cargo test`. Bespoke invocations (e.g. running a single
# named test via `cargo test <pat>`) remain fine; the canonical
# pre-landing test flow goes through this script.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"

mode=default
xtask_only=0
while [ $# -gt 0 ]; do
    case "$1" in
        --ignored)         mode=ignored;         shift ;;
        --include-ignored) mode=include-ignored; shift ;;
        --xtask)           xtask_only=1;         shift ;;
        --)                                       shift; break ;;
        -*)                printf 'unknown flag: %s\n' "$1" >&2; exit 2 ;;
        *)                 break ;;
    esac
done

if [ "$xtask_only" -eq 1 ] && [ $# -gt 0 ]; then
    echo "Usage: $0 [--xtask | <crate-name>] [--include-ignored|--ignored]" >&2
    echo "       --xtask is mutually exclusive with a positional crate arg." >&2
    exit 2
fi

if [ $# -gt 1 ]; then
    echo "Usage: $0 [--xtask | <crate-name>] [--include-ignored|--ignored]" >&2
    exit 2
fi

# Pin target-xtask when xtask is the only thing being tested, so
# the build cache stays isolated from `target-main`. Set it
# *before* sourcing cargo-target-dir.sh (whose guard preserves any
# explicit value).
if [ "$xtask_only" -eq 1 ] || [ "${1:-}" = "xtask" ]; then
    CARGO_TARGET_DIR=target-xtask
    export CARGO_TARGET_DIR
fi

. "$(dirname -- "$0")/lib/cargo-target-dir.sh"

crate=${1:-}

case "$mode" in
    default)         extra='' ;;
    ignored)         extra='-- --ignored' ;;
    include-ignored) extra='-- --include-ignored' ;;
esac

if [ "$xtask_only" -eq 1 ]; then
    printf '=== cargo test -p xtask %s (CARGO_TARGET_DIR=%s) ===\n' "$extra" "$CARGO_TARGET_DIR"
    # shellcheck disable=SC2086
    cargo test -p xtask $extra
elif [ -n "$crate" ]; then
    printf '=== cargo test -p %s %s ===\n' "$crate" "$extra"
    # $extra is intentionally unquoted so `-- --ignored` word-splits
    # into two arguments (an empty $extra expands to no args).
    # shellcheck disable=SC2086
    cargo test -p "$crate" $extra
else
    printf '=== cargo test --workspace --exclude xtask %s ===\n' "$extra"
    # shellcheck disable=SC2086
    cargo test --workspace --exclude xtask $extra
fi
