#!/bin/sh
# scripts/xtask.sh — invoke a Rust bin from the in-tree `xtask`
# crate. This is the canonical wrapper; prefer it over
# `cargo run -p xtask --bin <tool> --` at call sites so the
# invocation surface is consistent and there's one place to
# add pre-build caching or release-mode toggles later.
#
# Usage:
#   ./scripts/xtask.sh --list                   # list available tools
#   ./scripts/xtask.sh --help                   # this message
#   ./scripts/xtask.sh <tool>                   # run <tool> with no args
#   ./scripts/xtask.sh <tool> -- [<args>...]    # run <tool> with args
#
# The `--` separator is **required** before any argument intended
# for the bin itself. Wrapper-level flags (currently `--list`,
# `--help`) live before the bin name; everything after `--`
# passes through verbatim to the bin's `argv`. The rule exists so
# future wrapper-level flags (e.g. a `--release` toggle) can't
# collide with a bin's own flag of the same name.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"

bins_dir="xtask/src/bin"

list_bins() {
    if [ ! -d "$bins_dir" ]; then
        printf '!!! xtask.sh: %s not found\n' "$bins_dir" >&2
        exit 1
    fi
    # Enumerate `xtask/src/bin/*.rs`; the filename (without .rs)
    # is cargo's bin name.
    for f in "$bins_dir"/*.rs; do
        [ -f "$f" ] || continue
        b=${f##*/}
        b=${b%.rs}
        printf '%s\n' "$b"
    done
}

usage() {
    cat >&2 <<EOF
Usage:
  $0 --list                   # list available tools
  $0 --help                   # this message
  $0 <tool>                   # run <tool> with no args
  $0 <tool> -- [<args>...]    # run <tool> with args (note the \`--\` separator)
EOF
}

if [ $# -eq 0 ]; then
    usage
    exit 2
fi

case "$1" in
    --list)
        list_bins
        exit 0
        ;;
    --help|-h)
        usage
        echo >&2
        echo "Available tools:" >&2
        list_bins | sed 's/^/  /' >&2
        exit 0
        ;;
    --*)
        printf '!!! xtask.sh: unknown wrapper flag: %s\n' "$1" >&2
        usage
        exit 2
        ;;
esac

tool=$1
shift

# Require `--` before any bin args for unambiguous parsing.
if [ $# -gt 0 ]; then
    if [ "$1" != "--" ]; then
        printf '!!! xtask.sh: use `--` before arguments to the bin (got: %s)\n' "$1" >&2
        printf '    Example: %s gen-uuid -- --v4\n' "$0" >&2
        exit 2
    fi
    shift
fi

# Verify the bin exists before cargo-run tries to build.
if [ ! -f "$bins_dir/$tool.rs" ]; then
    printf '!!! xtask.sh: no such bin: %s\n' "$tool" >&2
    printf '    Available:\n' >&2
    list_bins | sed 's/^/      /' >&2
    exit 1
fi

exec cargo xtask "$tool" -- "$@"
