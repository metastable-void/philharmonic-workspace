#!/bin/sh
# scripts/cargo-install.sh — workspace-wide front door for
# installing cargo tooling. Wraps `cargo binstall` so workspace
# scripts get prebuilt binaries when available and fall back to
# source builds otherwise.
#
# `cargo-binstall` itself is installed on first use via
# `cargo install --locked cargo-binstall`. Subsequent runs go
# straight to `cargo binstall`. CI runners and dev boxes that
# already have `cargo-binstall` on `PATH` skip the bootstrap.
#
# Usage:
#   ./scripts/cargo-install.sh <crate>[@<version>] [<crate>...]
#       # Install one or more crates via `cargo binstall`.
#       # Anything after the script name is forwarded verbatim
#       # to `cargo binstall`, so `--locked`, `--force`, version
#       # pins, etc. work as documented by cargo-binstall. The
#       # wrapper always prepends `--no-confirm` so unattended /
#       # CI runs don't block on the interactive prompt
#       # `cargo binstall` shows by default.
#   ./scripts/cargo-install.sh --setup
#       # Bootstrap-only mode: ensure `cargo-binstall` itself
#       # is installed, then exit. Does not install any other
#       # crate. Used by `./scripts/setup.sh` to prepare a
#       # fresh clone.
#   ./scripts/cargo-install.sh -h | --help
#       # Print this usage block and exit.
#
# Read-only guard: if `CARGO_HOME` (default `$HOME/.cargo`)
# is not writable, the script prints a warning and exits 0
# without attempting an install — this lets read-only
# verification paths (e.g. CI matrix legs that just check
# script syntax) call it without blowing up.
#
# Mandated wrapper for `cargo install` of workspace tooling —
# raw `cargo install` is soft-banned in the same vein as the
# rest of the cargo-wrapper rules (CLAUDE.md / AGENTS.md
# §"Hard rules vs. soft rules"). Use this wrapper so future
# tooling-install changes (e.g., pinning a binstall version,
# adding offline-mirror support) land in one place.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

. "$(dirname -- "$0")/lib/script-help.sh"
script_help_handle "$@"

cargo_home="${CARGO_HOME:-$HOME/.cargo}"
if [ ! -w "$cargo_home" ] 2>/dev/null; then
    echo "!!! CARGO_HOME not writable, aborting install"
    exit 0
fi

skip_install=0

if [ "${1:-}" = "--setup" ] ; then
    skip_install=1
fi

if ! command -v cargo-binstall >/dev/null 2>&1; then
    cargo install --locked cargo-binstall
fi

if [ "$skip_install" -eq 1 ] ; then
    exit 0
fi

exec cargo binstall --no-confirm "$@"
