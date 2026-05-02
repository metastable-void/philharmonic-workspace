#!/bin/sh
# scripts/pre-landing.sh — the canonical pre-landing-check driver.
#
# Runs the mandated flow in order:
#   0. ./scripts/check-toolchain.sh             (prints rust/cargo versions;
#                                                if rustup is installed, runs
#                                                `rustup check` to surface
#                                                pending toolchain updates)
#   1. ./scripts/rust-lint.sh                   (fmt + check + clippy -D warnings + doc)
#   2. ./scripts/rust-test.sh                   (cargo test --workspace --exclude xtask, skips #[ignore])
#   3. ./scripts/rust-test.sh --ignored <X>     for each modified non-xtask crate X
#
# Step 3 exercises the `#[ignore]`-gated integration tests (the
# testcontainers / live-service ones) for crates you actually
# changed — the workspace-level run in step 2 skips them for
# speed.
#
# Auto-detects modified crates as workspace members with a dirty
# working tree (unstaged changes, staged changes, or untracked
# files). Submodule-backed and in-tree (non-submodule, e.g.
# `xtask`) members are both reported by `./scripts/show-dirty.sh`,
# but **xtask is excluded from the default --ignored loop**: in-
# tree dev-tooling changes go through the explicit `--xtask` mode
# below, not the workspace flow. Pass crate names explicitly to
# override auto-detection.
#
# xtask mode (`--xtask`):
#   xtask is the in-tree dev-tooling crate. It carries its own
#   `target-xtask/` build cache (CONTRIBUTING.md §8.1) so workspace
#   builds and Codex runs share `target-main/` without xtask
#   artifacts piling up. The default workspace flow excludes xtask
#   from `cargo check/clippy/doc/test --workspace` (via
#   `--exclude xtask`) and skips it from the `--ignored` loop.
#   Run `pre-landing.sh --xtask` explicitly when xtask itself was
#   changed; the run is scoped to xtask only and uses
#   `target-xtask` throughout. The two modes are mutually
#   exclusive: `--xtask` is incompatible with positional crate
#   names and with `--no-ignored` (xtask has no `#[ignore]`-gated
#   integration tests, so there is no step 3).
#
# Usage:
#   ./scripts/pre-landing.sh                    # workspace minus xtask, auto-detect modified crates
#   ./scripts/pre-landing.sh <crate>...         # explicit list (xtask not allowed here; use --xtask)
#   ./scripts/pre-landing.sh --no-ignored       # skip step 3 (rare; fast iteration)
#   ./scripts/pre-landing.sh --no-ignored <crate>...
#   ./scripts/pre-landing.sh --xtask            # ONLY xtask, target-xtask, no --ignored phase
#
# Run before every commit that touches Rust code. GitHub CI runs
# this same script (with a clean checkout → no dirty crates → no
# --ignored phase), so contributor and CI behavior don't drift.
# Slow-by-design (CONTRIBUTING.md §11) — run once per commit, not
# repeatedly within a single turn.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

no_ignored=0
xtask_only=0
while [ $# -gt 0 ]; do
    case "$1" in
        --no-ignored) no_ignored=1; shift ;;
        --xtask)      xtask_only=1; shift ;;
        --)                           shift; break ;;
        -*) printf 'unknown flag: %s\n' "$1" >&2; exit 2 ;;
        *) break ;;
    esac
done

if [ "$xtask_only" -eq 1 ]; then
    if [ $# -gt 0 ]; then
        echo "pre-landing.sh: --xtask is mutually exclusive with positional crate names" >&2
        exit 2
    fi
    if [ "$no_ignored" -eq 1 ]; then
        echo "pre-landing.sh: --xtask implies no --ignored phase; --no-ignored is redundant" >&2
        exit 2
    fi
    printf '%s=== pre-landing (--xtask): scope = xtask only, CARGO_TARGET_DIR=target-xtask ===%s\n' \
        "$C_HEADER" "$C_RESET"
    ./scripts/check-toolchain.sh
    ./scripts/rust-lint.sh --xtask
    ./scripts/rust-test.sh --xtask
    printf '%s=== pre-landing: xtask checks passed ===%s\n' "$C_OK" "$C_RESET"
    exit 0
fi

# Pin target-main for the default workspace path (rust-lint /
# rust-test inherit through their own cargo-target-dir.sh source).
. "$(dirname -- "$0")/lib/cargo-target-dir.sh"

if [ $# -gt 0 ]; then
    crates=$*
    # Refuse `pre-landing.sh xtask` — xtask is gated behind --xtask.
    for c in $crates; do
        if [ "$c" = "xtask" ]; then
            echo "pre-landing.sh: xtask is not allowed in the workspace flow; use --xtask instead" >&2
            exit 2
        fi
    done
    printf '%s=== pre-landing: explicit crates: %s ===%s\n' "$C_HEADER" "$crates" "$C_RESET"
else
    # Workspace members (submodule-backed or in-tree) with
    # unstaged, staged, or untracked changes count as modified.
    # `show-dirty.sh` emits one name per line. Filter xtask out so
    # the default --ignored loop never tests xtask in target-main;
    # xtask drift is handled by `--xtask` mode separately.
    crates=$(./scripts/show-dirty.sh | grep -v -x -F xtask || :)
    if [ -n "$crates" ]; then
        # Collapse newlines to spaces for the header. $crates is
        # intentionally unquoted so default-IFS word-splitting
        # flattens the newlines.
        # shellcheck disable=SC2086
        printf '%s=== pre-landing: auto-detected modified crates: %s ===%s\n' \
            "$C_HEADER" "$(printf '%s ' $crates)" "$C_RESET"
    else
        printf '%s=== pre-landing: no modified non-xtask crates detected; running workspace checks only ===%s\n' \
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
