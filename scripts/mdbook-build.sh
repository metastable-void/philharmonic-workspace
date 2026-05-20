#!/bin/sh
# scripts/mdbook-build.sh — build the `book/` mdbook output from
# `docs/` (mdbook config in `book.toml`).
#
# Usage:
#   ./scripts/mdbook-build.sh               # full rebuild
#   ./scripts/mdbook-build.sh --clean       # rm -rf book/ first
#   ./scripts/mdbook-build.sh --check       # mdbook test (link/anchor checks)
#
# mdbook is installed by `./scripts/setup.sh` via `cargo install
# mdbook`. The `book/` directory is committed in this workspace so
# the book builds reproducibly without forcing every clone to
# install mdbook; this script regenerates it after `docs/` edits.
#
# Mandated wrapper for `mdbook build`. Raw `mdbook build` is
# soft-banned (CLAUDE.md / AGENTS.md §"Hard rules vs. soft rules")
# along with the rest of the workspace tooling — using this
# wrapper keeps the cwd, the config path, and the
# clean / check flags consistent across agent sessions.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

clean=0
check=0
while [ $# -gt 0 ]; do
    case "$1" in
        --clean) clean=1; shift ;;
        --check) check=1; shift ;;
        -h|--help)
            printf 'Usage: mdbook-build.sh [--clean] [--check] [-h|--help]\n'
            exit 0 ;;
        *)
            printf 'mdbook-build.sh: unknown flag: %s\n' "$1" >&2
            exit 2 ;;
    esac
done

if ! command -v mdbook >/dev/null 2>&1; then
    printf '%s!!! mdbook not installed; run ./scripts/setup.sh%s\n' \
        "$C_ERR" "$C_RESET" >&2
    exit 1
fi

if [ ! -f book.toml ]; then
    printf '%s!!! no book.toml in %s; mdbook needs the workspace root%s\n' \
        "$C_ERR" "$(pwd)" "$C_RESET" >&2
    exit 1
fi

if [ "$clean" -eq 1 ]; then
    printf '%s=== mdbook clean ===%s\n' "$C_HEADER" "$C_RESET"
    rm -rf book/
fi

if [ "$check" -eq 1 ]; then
    printf '%s=== mdbook test (link / anchor checks) ===%s\n' "$C_HEADER" "$C_RESET"
    mdbook test
else
    printf '%s=== mdbook build ===%s\n' "$C_HEADER" "$C_RESET"
    mdbook build
fi

printf '%s=== mdbook-build: done ===%s\n' "$C_OK" "$C_RESET"
