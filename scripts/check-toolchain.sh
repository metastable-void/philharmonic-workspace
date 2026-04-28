#!/bin/sh
# scripts/check-toolchain.sh — report the local Rust toolchain and
# (optionally) update it via rustup.
#
# Usage:
#   ./scripts/check-toolchain.sh            # print versions; if rustup
#                                           # is installed, run `rustup check`
#   ./scripts/check-toolchain.sh --update   # same, plus `rustup update`
#
# Exit code:
#   0 on success, including when rustup is missing (we only warn).
#   The underlying `rustup update` exit code is propagated on --update
#   if it fails.
#
# Why: CI/local Rust version drift has bitten us (see
# docs/notes-to-humans/2026-04-21-0004-ci-local-rust-version-drift.md).
# `pre-landing.sh` calls this script in no-flag mode, so every
# pre-landing run prints the current toolchain and surfaces pending
# updates — nudging us to `rustup update` before drift lands on CI
# as a new clippy lint.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/cargo-target-dir.sh"

do_update=0
while [ $# -gt 0 ]; do
    case "$1" in
        --update) do_update=1; shift ;;
        --)                   shift; break ;;
        -*) printf 'unknown flag: %s\n' "$1" >&2; exit 2 ;;
        *) break ;;
    esac
done

if [ $# -gt 0 ]; then
    echo "Usage: $0 [--update]" >&2
    exit 2
fi

echo '=== local Rust toolchain ==='
if command -v rustc >/dev/null 2>&1; then
    rustc --version
else
    echo '!!! rustc not on PATH' >&2
fi
if command -v cargo >/dev/null 2>&1; then
    cargo --version
else
    echo '!!! cargo not on PATH' >&2
fi

if ! command -v rustup >/dev/null 2>&1; then
    echo
    echo '!!! rustup not on PATH — skipping toolchain update check.' >&2
    echo '    Install rustup from https://rustup.rs/ to get `rustup check`' >&2
    echo '    and automated updates. CI uses a pinned stable via the' >&2
    echo '    dtolnay/rust-toolchain action, so local rustup-based updates' >&2
    echo '    are the primary way to keep local aligned with CI.' >&2
    exit 0
fi

echo
if [ "$do_update" -eq 1 ]; then
    echo '=== rustup update ==='
    rustup update
else
    echo '=== rustup check ==='
    # rustup check exits 100 when any toolchain has an update available.
    # This script's purpose is to print update status, not fail on it —
    # `|| :` swallows the non-zero so downstream pre-landing steps still
    # run. Without this, `set -e` in the caller aborts the moment a new
    # nightly ships.
    rustup check || :
fi

# Nightly + miri presence probe (scripts/miri-test.sh needs both).
# We don't auto-install here — that's setup.sh's job. Just warn if
# missing so contributors notice the drift.
echo
echo '=== nightly + miri (for scripts/miri-test.sh) ==='
if rustup toolchain list 2>/dev/null | grep -q '^nightly'; then
    nightly_version="$(rustup run nightly rustc --version 2>/dev/null || echo 'unknown')"
    printf '  nightly: %s\n' "$nightly_version"
    if rustup component list --toolchain nightly --installed 2>/dev/null | grep -q '^miri'; then
        printf '  miri: installed\n'
    else
        echo '!!! miri not installed on nightly.' >&2
        echo '    Fix: rustup +nightly component add miri' >&2
        echo '    Or:  scripts/setup.sh (installs idempotently)' >&2
    fi
else
    echo '!!! nightly toolchain not installed.' >&2
    echo '    Fix: rustup toolchain install nightly' >&2
    echo '    Or:  scripts/setup.sh (installs idempotently)' >&2
fi
