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
    rustup check
fi
