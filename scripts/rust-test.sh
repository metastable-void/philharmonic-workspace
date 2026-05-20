#!/bin/sh
# scripts/rust-test.sh — run `cargo test` across the workspace, a
# single crate, or the in-tree `xtask` dev tooling, with optional
# control over `#[ignore]`-gated tests, test-name filtering,
# feature selection, and release-mode builds.
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
# Additional flags (compose with any of the above):
#   --filter <pat>      pass <pat> to cargo test as the positional
#                       test-name filter (cargo treats it as a
#                       substring match across test function paths).
#   --features <list>   pass `--features <list>` to cargo test;
#                       requires a positional crate or --xtask
#                       (cargo's --features needs a definite package).
#   --no-default-features
#                       pass `--no-default-features` to cargo test;
#                       requires a positional crate or --xtask.
#   --all-features      pass `--all-features` to cargo test.
#   --release           pass `--release` to cargo test.
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
# Mandated for test passes in this workspace — raw `cargo test`
# is soft-banned (CLAUDE.md / AGENTS.md §"Hard rules vs. soft
# rules"). Use `--filter`, `--features`, `--release` for the
# common bespoke needs; for anything else not covered, surface
# the request as a prompt-override and extend this script.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/cargo-noise-filter.sh"

mode=default
xtask_only=0
filter_pat=
features=
no_default_features=0
all_features=0
release=0
quiet=0

while [ $# -gt 0 ]; do
    case "$1" in
        --ignored)              mode=ignored;          shift ;;
        --include-ignored)      mode=include-ignored;  shift ;;
        --xtask)                xtask_only=1;          shift ;;
        --filter)
            shift
            if [ -z "${1:-}" ]; then
                echo "--filter requires a test-name pattern" >&2; exit 2
            fi
            filter_pat=$1; shift ;;
        --features)
            shift
            if [ -z "${1:-}" ]; then
                echo "--features requires a comma-separated feature list" >&2; exit 2
            fi
            features=$1; shift ;;
        --no-default-features)  no_default_features=1; shift ;;
        --all-features)         all_features=1;        shift ;;
        --release)              release=1;             shift ;;
        --quiet)                quiet=1;               shift ;;
        --)                                            shift; break ;;
        -*)                printf 'unknown flag: %s\n' "$1" >&2; exit 2 ;;
        *)                 break ;;
    esac
done

# `--quiet` suppresses cargo's compile-progress chatter
# ("Compiling X v0.1.0" / "Finished" / "Running"). Test name
# output and failures still come through. Use this when pre-
# landing (or another wrapper) is printing its own progress.
if [ "$quiet" -eq 1 ]; then
    quiet_arg=--quiet
else
    quiet_arg=
fi

if [ "$xtask_only" -eq 1 ] && [ $# -gt 0 ]; then
    echo "Usage: $0 [--xtask | <crate-name>] [--include-ignored|--ignored] [--filter <pat>] [--features <list>] [--no-default-features] [--all-features] [--release]" >&2
    echo "       --xtask is mutually exclusive with a positional crate arg." >&2
    exit 2
fi

if [ $# -gt 1 ]; then
    echo "Usage: $0 [--xtask | <crate-name>] [--include-ignored|--ignored] [--filter <pat>] [--features <list>] [--no-default-features] [--all-features] [--release]" >&2
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

# Feature flags only apply when a specific package is targeted
# (cargo's restriction). Workspace mode with --features is a usage
# error — surface it explicitly rather than passing it to cargo
# and letting cargo emit a less obvious diagnostic.
if [ "$xtask_only" -eq 0 ] && [ -z "$crate" ]; then
    if [ -n "$features" ] || [ "$no_default_features" -eq 1 ]; then
        echo "rust-test.sh: --features / --no-default-features require a positional crate or --xtask" >&2
        exit 2
    fi
fi

# Compose cargo-side flags (added before the `--` separator).
cargo_flags=
if [ -n "$features" ]; then
    cargo_flags="$cargo_flags --features $features"
fi
if [ "$no_default_features" -eq 1 ]; then
    cargo_flags="$cargo_flags --no-default-features"
fi
if [ "$all_features" -eq 1 ]; then
    cargo_flags="$cargo_flags --all-features"
fi
if [ "$release" -eq 1 ]; then
    cargo_flags="$cargo_flags --release"
fi

# `extra` carries everything after the `--` separator (test-binary
# args) plus the cargo-test positional filter <pat> (which goes
# *before* the `--`). Keeping the filter in `extra` rather than
# `cargo_flags` matches cargo's CLI shape: `cargo test [opts]
# [filter] -- [test-binary args]`.
case "$mode" in
    default)         extra_after='' ;;
    ignored)         extra_after='-- --ignored' ;;
    include-ignored) extra_after='-- --include-ignored' ;;
esac

if [ -n "$filter_pat" ]; then
    # Filter goes before `--`. Quote it via the variable so cargo
    # treats embedded spaces as part of one substring.
    filter_arg=$filter_pat
else
    filter_arg=
fi

if [ "$xtask_only" -eq 1 ]; then
    printf '=== cargo test -p xtask%s%s %s (CARGO_TARGET_DIR=%s) ===\n' \
        "$cargo_flags" "${filter_arg:+ $filter_arg}" "$extra_after" "$CARGO_TARGET_DIR"
    # shellcheck disable=SC2086
    run_with_cargo_noise_filter cargo test $quiet_arg -p xtask $cargo_flags $filter_arg $extra_after
elif [ -n "$crate" ]; then
    printf '=== cargo test -p %s%s%s %s ===\n' \
        "$crate" "$cargo_flags" "${filter_arg:+ $filter_arg}" "$extra_after"
    # $extra_after is intentionally unquoted so `-- --ignored` word-splits
    # into two arguments (an empty value expands to no args).
    # shellcheck disable=SC2086
    run_with_cargo_noise_filter cargo test $quiet_arg -p "$crate" $cargo_flags $filter_arg $extra_after
else
    printf '=== cargo test --workspace --exclude xtask%s%s %s ===\n' \
        "$cargo_flags" "${filter_arg:+ $filter_arg}" "$extra_after"
    # shellcheck disable=SC2086
    run_with_cargo_noise_filter cargo test $quiet_arg --workspace --exclude xtask $cargo_flags $filter_arg $extra_after
fi
