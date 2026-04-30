#!/bin/sh
# scripts/tokei.sh — print lines-of-code statistics.
#
# Tries, in order:
#   1. `tokei` if already on PATH
#   2. `xtask tokei-stats` (uses the tokei crate as a library;
#      works even when CARGO_HOME is unwritable, e.g. inside
#      Codex's sandbox)
#   3. `cargo install tokei` then `tokei`
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/workspace-cd.sh"

if command -v tokei >/dev/null 2>&1; then
    exec tokei
fi

if [ -f xtask/src/bin/tokei-stats.rs ]; then
    exec ./scripts/xtask.sh tokei-stats
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "Neither tokei nor cargo found" >&2
    exit 1
fi

cargo install -q tokei --features=all
exec tokei
