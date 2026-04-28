#!/bin/sh
# scripts/rust-test.sh — run `cargo test` across the workspace or
# a single crate, with optional control over `#[ignore]`-gated
# tests.
#
# Usage:
#   ./scripts/rust-test.sh                            workspace, skip #[ignore]
#   ./scripts/rust-test.sh <crate>                    one crate, skip #[ignore]
#   ./scripts/rust-test.sh --ignored                  workspace, only #[ignore]'d
#   ./scripts/rust-test.sh --ignored <crate>          one crate, only #[ignore]'d
#   ./scripts/rust-test.sh --include-ignored          workspace, all tests
#   ./scripts/rust-test.sh --include-ignored <crate>  one crate, all tests
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
. "$(dirname -- "$0")/lib/cargo-target-dir.sh"

mode=default
while [ $# -gt 0 ]; do
    case "$1" in
        --ignored)         mode=ignored;         shift ;;
        --include-ignored) mode=include-ignored; shift ;;
        --)                                       shift; break ;;
        -*)                printf 'unknown flag: %s\n' "$1" >&2; exit 2 ;;
        *)                 break ;;
    esac
done

if [ $# -gt 1 ]; then
    echo "Usage: $0 [--include-ignored|--ignored] [<crate-name>]" >&2
    exit 2
fi

crate=${1:-}

case "$mode" in
    default)         extra='' ;;
    ignored)         extra='-- --ignored' ;;
    include-ignored) extra='-- --include-ignored' ;;
esac

if [ -n "$crate" ]; then
    printf '=== cargo test -p %s %s ===\n' "$crate" "$extra"
    # $extra is intentionally unquoted so `-- --ignored` word-splits
    # into two arguments (an empty $extra expands to no args).
    # shellcheck disable=SC2086
    cargo test -p "$crate" $extra
else
    printf '=== cargo test --workspace %s ===\n' "$extra"
    # shellcheck disable=SC2086
    cargo test --workspace $extra
fi
