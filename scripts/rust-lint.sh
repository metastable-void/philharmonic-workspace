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
#   ./scripts/rust-lint.sh --fix [...]   # apply fmt + clippy autofixes
#                                        # in place (dirty tree OK); still
#                                        # fails on non-fixable warnings
#                                        # via clippy `-D warnings`
#
# Runs in order:
#   cargo fmt   <scope> [--check]                                check mode
#   cargo fmt   <scope>                                          fix mode
#   cargo check <scope>
#   cargo clippy <scope> --all-targets -- -D warnings            check mode
#   cargo clippy --fix --allow-dirty --allow-staged <scope> \    fix mode
#                --all-targets -- -D warnings
#   RUSTDOCFLAGS="-D missing_docs" cargo doc --no-deps <scope>
#
# Fix mode (`--fix`) rewrites source files in place. `cargo clippy
# --fix` normally refuses to run against a dirty working tree;
# `--allow-dirty --allow-staged` is passed so the fix pass works
# against pre-landing's typical state (uncommitted Rust edits the
# author is about to land). Non-fixable warnings still fail the
# run via `-D warnings`, so this never silently weakens the gate.
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
# If the fmt-check step fails in check mode, either re-run with
# `--fix` to apply formatting and autofixable clippy lints in
# place, or run `cargo fmt --all` (or `cargo fmt -p <crate>`)
# manually and re-run the script.
#
# See docs/design/13-conventions.md §Pre-landing checks.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/cargo-noise-filter.sh"

# Parse flags in any order. We need to know `--xtask` *before*
# sourcing cargo-target-dir.sh (which only sets the default when
# CARGO_TARGET_DIR is unset, preserving any explicit value we set
# here), and we need to know `--fix` before composing the fmt /
# clippy commands.
xtask_only=0
fix_mode=0
crate=

while [ $# -gt 0 ]; do
    case "$1" in
        --xtask) xtask_only=1; shift ;;
        --fix)   fix_mode=1; shift ;;
        --)      shift; break ;;
        -*) echo "unknown flag: $1" >&2; exit 2 ;;
        *)
            if [ -n "$crate" ]; then
                echo "Usage: $0 [--xtask | <crate-name>] [--fix]" >&2
                exit 2
            fi
            crate=$1
            shift ;;
    esac
done

if [ "$xtask_only" -eq 1 ] && [ -n "$crate" ]; then
    echo "Usage: $0 [--xtask | <crate-name>] [--fix]" >&2
    echo "       --xtask is mutually exclusive with a positional crate arg." >&2
    exit 2
fi

# Pin target-xtask when xtask is the only thing being checked, so
# the build cache stays isolated from `target-main` (used by
# workspace builds, CLI cargo invocations, and Codex). This
# applies to both `--xtask` and `<crate-name>=xtask`.
if [ "$xtask_only" -eq 1 ] || [ "$crate" = "xtask" ]; then
    CARGO_TARGET_DIR=target-xtask
    export CARGO_TARGET_DIR
fi

. "$(dirname -- "$0")/lib/cargo-target-dir.sh"

# fmt_check_flag: `--check` in check mode, empty in fix mode (so
#                 cargo fmt rewrites in place).
# clippy_fix_args: empty in check mode; `--fix --allow-dirty
#                  --allow-staged` in fix mode. `--allow-dirty
#                  --allow-staged` lets clippy rewrite source even
#                  when the working tree has uncommitted edits,
#                  which is the normal pre-landing state.
if [ "$fix_mode" -eq 1 ]; then
    fmt_check_flag=
    clippy_fix_args='--fix --allow-dirty --allow-staged'
    fix_label=' (fix mode)'
else
    fmt_check_flag=--check
    clippy_fix_args=
    fix_label=
fi

if [ "$xtask_only" -eq 1 ] || [ "$crate" = "xtask" ]; then
    crate=xtask
    printf '%s=== rust-lint scope: xtask only%s (CARGO_TARGET_DIR=%s) ===%s\n' \
        "$C_HEADER" "$fix_label" "$CARGO_TARGET_DIR" "$C_RESET"
    printf '%s--- cargo fmt -p xtask %s ---%s\n' "$C_DIM" "$fmt_check_flag" "$C_RESET"
    # shellcheck disable=SC2086
    run_with_cargo_noise_filter cargo fmt -p "$crate" $fmt_check_flag
    printf '%s--- cargo check -p xtask ---%s\n' "$C_DIM" "$C_RESET"
    run_with_cargo_noise_filter cargo check -p "$crate"
    printf '%s--- cargo clippy %s-p xtask --all-targets -- -D warnings ---%s\n' \
        "$C_DIM" "${clippy_fix_args:+$clippy_fix_args }" "$C_RESET"
    # shellcheck disable=SC2086
    run_with_cargo_noise_filter cargo clippy $clippy_fix_args -p "$crate" --all-targets -- -D warnings
    printf '%s--- cargo doc -p xtask (missing_docs) ---%s\n' "$C_DIM" "$C_RESET"
    RUSTDOCFLAGS="-D missing_docs" run_with_cargo_noise_filter cargo doc --no-deps -p "$crate"
elif [ -n "$crate" ]; then
    printf '%s=== rust-lint scope: crate %s%s ===%s\n' "$C_HEADER" "$crate" "$fix_label" "$C_RESET"
    printf '%s--- cargo fmt -p %s %s ---%s\n' "$C_DIM" "$crate" "$fmt_check_flag" "$C_RESET"
    # shellcheck disable=SC2086
    run_with_cargo_noise_filter cargo fmt -p "$crate" $fmt_check_flag
    printf '%s--- cargo check -p %s ---%s\n' "$C_DIM" "$crate" "$C_RESET"
    run_with_cargo_noise_filter cargo check -p "$crate"
    printf '%s--- cargo clippy %s-p %s --all-targets -- -D warnings ---%s\n' \
        "$C_DIM" "${clippy_fix_args:+$clippy_fix_args }" "$crate" "$C_RESET"
    # shellcheck disable=SC2086
    run_with_cargo_noise_filter cargo clippy $clippy_fix_args -p "$crate" --all-targets -- -D warnings
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

    printf '%s=== rust-lint scope: workspace (excluding xtask)%s ===%s\n' \
        "$C_HEADER" "$fix_label" "$C_RESET"
    printf '%s--- cargo fmt%s %s ---%s\n' "$C_DIM" "$fmt_pkgs" "$fmt_check_flag" "$C_RESET"
    # shellcheck disable=SC2086
    run_with_cargo_noise_filter cargo fmt $fmt_pkgs $fmt_check_flag
    printf '%s--- cargo check --workspace --exclude xtask ---%s\n' "$C_DIM" "$C_RESET"
    run_with_cargo_noise_filter cargo check --workspace --exclude xtask
    printf '%s--- cargo clippy %s--workspace --exclude xtask --all-targets -- -D warnings ---%s\n' \
        "$C_DIM" "${clippy_fix_args:+$clippy_fix_args }" "$C_RESET"
    # shellcheck disable=SC2086
    run_with_cargo_noise_filter cargo clippy $clippy_fix_args --workspace --exclude xtask --all-targets -- -D warnings
    printf '%s--- cargo doc --workspace --exclude xtask (missing_docs) ---%s\n' "$C_DIM" "$C_RESET"
    RUSTDOCFLAGS="-D missing_docs" run_with_cargo_noise_filter cargo doc --no-deps --workspace --exclude xtask
fi

printf '%s=== rust-lint: clean ===%s\n' "$C_OK" "$C_RESET"
