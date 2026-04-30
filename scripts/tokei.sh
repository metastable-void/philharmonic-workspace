#!/bin/sh
# scripts/tokei.sh — print lines-of-code statistics via `tokei`.
#
# Tries, in order:
#   1. `tokei` if already on PATH
#   2. `cargo install tokei` (skipped if CARGO_HOME is unwritable,
#      e.g. inside Codex's sandbox)
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/workspace-cd.sh"

if command -v tokei >/dev/null 2>&1; then
    exec tokei
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "tokei not found and cargo not available" >&2
    exit 1
fi

cargo_home="${CARGO_HOME:-$HOME/.cargo}"
if [ ! -w "$cargo_home" ] 2>/dev/null; then
    echo "tokei not on PATH and CARGO_HOME ($cargo_home) is not writable" >&2
    echo "install tokei manually: cargo install tokei --features=all" >&2
    exit 1
fi

cargo install -q tokei --features=all
exec tokei
