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
#   ./scripts/rust-lint.sh --phase <p>   # run only one phase
#                                        # p ∈ {fmt, check, clippy, doc}
#                                        # default: all four phases
#   ./scripts/rust-lint.sh --target <triple>
#                                        # cross-compile via cargo check
#                                        # / clippy / doc. fmt has no
#                                        # --target and is skipped under
#                                        # this flag's effects. Requires
#                                        # `rustup target add <triple>`
#                                        # for the target's stdlib.
#
# Runs in order (the "lint quartet" — gate by `--phase` if you
# only want one):
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
# Phase mode (`--phase <name>`) runs only the named phase. Useful
# when you want a fast "does it compile?" feedback loop
# (`--phase check`), a one-off clippy run after refactoring
# (`--phase clippy`), or a docs lint pass (`--phase doc`). The
# `fmt` phase still honours `--fix` vs check mode; the other
# three phases ignore fix mode where it doesn't apply (e.g.,
# `--phase check --fix` runs plain `cargo check`).
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
# Mandated for lint passes in this workspace — raw `cargo
# fmt/check/clippy/doc` is soft-banned (CLAUDE.md / AGENTS.md
# §"Hard rules vs. soft rules"). For a one-phase build-sanity
# check use `--phase check` rather than raw `cargo check`. For
# bespoke needs not covered by these flags, surface the request
# as a prompt-override and extend this script.
#
# If the fmt-check step fails in check mode, either re-run with
# `--fix` to apply formatting and autofixable clippy lints in
# place, or run the script with `--phase fmt --fix` to fix
# formatting only and re-run the full pass.
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
quiet=0
phase=all
target=
crate=

while [ $# -gt 0 ]; do
    case "$1" in
        --xtask) xtask_only=1; shift ;;
        --fix)   fix_mode=1; shift ;;
        --quiet) quiet=1; shift ;;
        --phase)
            shift
            case "${1:-}" in
                fmt|check|clippy|doc|all)
                    phase=$1; shift ;;
                '')
                    echo "--phase requires a value: fmt|check|clippy|doc|all" >&2
                    exit 2 ;;
                *)
                    echo "--phase value must be one of: fmt, check, clippy, doc, all" >&2
                    exit 2 ;;
            esac ;;
        --target)
            shift
            if [ -z "${1:-}" ]; then
                echo "--target requires a Rust target triple (e.g. x86_64-unknown-freebsd)" >&2
                exit 2
            fi
            target=$1; shift ;;
        --)      shift; break ;;
        -*) echo "unknown flag: $1" >&2; exit 2 ;;
        *)
            if [ -n "$crate" ]; then
                echo "Usage: $0 [--xtask | <crate-name>] [--fix] [--phase <name>] [--quiet] [--target <triple>]" >&2
                exit 2
            fi
            crate=$1
            shift ;;
    esac
done

# `--quiet` propagates `--quiet` to cargo check/clippy/doc (NOT
# to cargo fmt — fmt has no quiet flag and emits nothing when
# clean anyway). Errors and warnings still surface; only the
# "Compiling X v0.1.0" / "Checking X" / "Finished" / "Documenting"
# progress lines are suppressed. Use this when something else
# (pre-landing) is printing its own per-step progress.
quiet_arg=
if [ "$quiet" -eq 1 ]; then
    quiet_arg=--quiet
fi

# `--target <triple>` cross-compiles via cargo check / clippy /
# doc. fmt has no `--target` (it's source-level) and is skipped.
# Use this for cfg-gated platforms — e.g. surfacing dead-code
# warnings on `x86_64-unknown-freebsd` when most probes are
# `cfg(target_os = "linux")`-gated. Requires the target's
# stdlib to be installed via `rustup target add <triple>`.
target_arg=
if [ -n "$target" ]; then
    target_arg="--target $target"
fi

if [ "$xtask_only" -eq 1 ] && [ -n "$crate" ]; then
    echo "Usage: $0 [--xtask | <crate-name>] [--fix] [--phase <name>]" >&2
    echo "       --xtask is mutually exclusive with a positional crate arg." >&2
    exit 2
fi

# Phase gate helper. Returns 0 (true) when the named phase should
# run, given the current $phase. `all` runs everything; a specific
# phase runs only itself.
phase_runs() {
    [ "$phase" = all ] || [ "$phase" = "$1" ]
}

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
    printf '%s=== rust-lint scope: xtask only%s phase=%s (CARGO_TARGET_DIR=%s) ===%s\n' \
        "$C_HEADER" "$fix_label" "$phase" "$CARGO_TARGET_DIR" "$C_RESET"
    if phase_runs fmt; then
        printf '%s--- cargo fmt -p xtask %s ---%s\n' "$C_DIM" "$fmt_check_flag" "$C_RESET"
        # shellcheck disable=SC2086
        run_with_cargo_noise_filter cargo fmt -p "$crate" $fmt_check_flag
    fi
    if phase_runs check; then
        printf '%s--- cargo check -p xtask ---%s\n' "$C_DIM" "$C_RESET"
        # shellcheck disable=SC2086
        run_with_cargo_noise_filter cargo check $quiet_arg $target_arg -p "$crate"
    fi
    if phase_runs clippy; then
        printf '%s--- cargo clippy %s-p xtask --all-targets -- -D warnings ---%s\n' \
            "$C_DIM" "${clippy_fix_args:+$clippy_fix_args }" "$C_RESET"
        # shellcheck disable=SC2086
        run_with_cargo_noise_filter cargo clippy $quiet_arg $target_arg $clippy_fix_args -p "$crate" --all-targets -- -D warnings
    fi
    if phase_runs doc; then
        printf '%s--- cargo doc -p xtask (missing_docs) ---%s\n' "$C_DIM" "$C_RESET"
        # shellcheck disable=SC2086
        RUSTDOCFLAGS="-D missing_docs" run_with_cargo_noise_filter cargo doc $quiet_arg $target_arg --no-deps -p "$crate"
    fi
elif [ -n "$crate" ]; then
    printf '%s=== rust-lint scope: crate %s%s phase=%s ===%s\n' \
        "$C_HEADER" "$crate" "$fix_label" "$phase" "$C_RESET"
    if phase_runs fmt; then
        printf '%s--- cargo fmt -p %s %s ---%s\n' "$C_DIM" "$crate" "$fmt_check_flag" "$C_RESET"
        # shellcheck disable=SC2086
        run_with_cargo_noise_filter cargo fmt -p "$crate" $fmt_check_flag
    fi
    if phase_runs check; then
        printf '%s--- cargo check -p %s ---%s\n' "$C_DIM" "$crate" "$C_RESET"
        # shellcheck disable=SC2086
        run_with_cargo_noise_filter cargo check $quiet_arg $target_arg -p "$crate"
    fi
    if phase_runs clippy; then
        printf '%s--- cargo clippy %s-p %s --all-targets -- -D warnings ---%s\n' \
            "$C_DIM" "${clippy_fix_args:+$clippy_fix_args }" "$crate" "$C_RESET"
        # shellcheck disable=SC2086
        run_with_cargo_noise_filter cargo clippy $quiet_arg $target_arg $clippy_fix_args -p "$crate" --all-targets -- -D warnings
    fi
    if phase_runs doc; then
        printf '%s--- cargo doc -p %s (missing_docs) ---%s\n' "$C_DIM" "$crate" "$C_RESET"
        # shellcheck disable=SC2086
        RUSTDOCFLAGS="-D missing_docs" run_with_cargo_noise_filter cargo doc $quiet_arg $target_arg --no-deps -p "$crate"
    fi
else
    # Workspace mode — exclude xtask. fmt has no `--exclude` flag,
    # so enumerate non-xtask workspace members and pass one
    # `-p <name>` per member to a single fmt invocation.
    . "$(dirname -- "$0")/lib/workspace-members.sh"
    fmt_pkgs=
    if phase_runs fmt; then
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
    fi

    printf '%s=== rust-lint scope: workspace (excluding xtask)%s phase=%s ===%s\n' \
        "$C_HEADER" "$fix_label" "$phase" "$C_RESET"
    if phase_runs fmt; then
        printf '%s--- cargo fmt%s %s ---%s\n' "$C_DIM" "$fmt_pkgs" "$fmt_check_flag" "$C_RESET"
        # shellcheck disable=SC2086
        run_with_cargo_noise_filter cargo fmt $fmt_pkgs $fmt_check_flag
    fi
    if phase_runs check; then
        printf '%s--- cargo check --workspace --exclude xtask ---%s\n' "$C_DIM" "$C_RESET"
        # shellcheck disable=SC2086
        run_with_cargo_noise_filter cargo check $quiet_arg $target_arg --workspace --exclude xtask
    fi
    if phase_runs clippy; then
        printf '%s--- cargo clippy %s--workspace --exclude xtask --all-targets -- -D warnings ---%s\n' \
            "$C_DIM" "${clippy_fix_args:+$clippy_fix_args }" "$C_RESET"
        # shellcheck disable=SC2086
        run_with_cargo_noise_filter cargo clippy $quiet_arg $target_arg $clippy_fix_args --workspace --exclude xtask --all-targets -- -D warnings
    fi
    if phase_runs doc; then
        printf '%s--- cargo doc --workspace --exclude xtask (missing_docs) ---%s\n' "$C_DIM" "$C_RESET"
        # shellcheck disable=SC2086
        RUSTDOCFLAGS="-D missing_docs" run_with_cargo_noise_filter cargo doc $quiet_arg $target_arg --no-deps --workspace --exclude xtask
    fi
fi

printf '%s=== rust-lint: clean ===%s\n' "$C_OK" "$C_RESET"
