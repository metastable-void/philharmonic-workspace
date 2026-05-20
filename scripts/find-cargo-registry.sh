#!/bin/sh
# scripts/find-cargo-registry.sh — locate vendored crate source
# files under `~/.cargo/registry/src/...` without retyping the
# `find` skeleton every time.
#
# When investigating upstream behaviour (e.g. checking how
# `sqlx-postgres` parses a connection URL, or whether `ring`'s
# error type implements `Debug`), agents repeatedly run patterns
# like:
#
#   find ~/.cargo/registry/src -maxdepth 4 -path '*sqlx-postgres*' -name '*.rs'
#   find ~/.cargo/registry/src -maxdepth 4 -path '*tokio-1.*' -name '*lib.rs'
#
# This script wraps that pattern so the cwd, the registry path,
# the depth, and the file glob are consistent.
#
# Usage:
#   ./scripts/find-cargo-registry.sh <crate-substring>
#       # list every .rs file under registry/src whose path
#       # contains <crate-substring>. The substring is matched
#       # against the full registry path, so a versioned name like
#       # `sqlx-postgres-0.8` or `ring-0.17.16` works the same way.
#   ./scripts/find-cargo-registry.sh <crate-substring> <name-glob>
#       # restrict by filename glob (default: `*.rs`).
#       # Example: ./scripts/find-cargo-registry.sh tokio 'lib.rs'
#   ./scripts/find-cargo-registry.sh --list
#       # list every vendored crate directory (one per line).
#   ./scripts/find-cargo-registry.sh --root
#       # print the resolved registry-src root and exit.
#
# All modes write paths relative to the user's home directory
# (prefix `~`) so output is shorter and paste-friendly.
#
# Mandated wrapper for vendored-source lookups. Raw `find
# ~/.cargo/registry/src ...` is soft-banned (CLAUDE.md / AGENTS.md
# §"Hard rules vs. soft rules") along with the rest of the
# workspace tooling. Use this wrapper or, for more involved
# inspection, copy a vendored file out and `Read` it.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

. "$(dirname -- "$0")/lib/script-help.sh"
script_help_handle "$@"

. "$(dirname -- "$0")/lib/colors.sh"

mode=search
substr=
name_glob='*.rs'

while [ $# -gt 0 ]; do
    case "$1" in
        --list)  mode=list;  shift ;;
        --root)  mode=root;  shift ;;
        -h|--help)
            printf 'Usage: find-cargo-registry.sh <crate-substring> [<name-glob>]\n'
            printf '       find-cargo-registry.sh --list | --root\n'
            exit 0 ;;
        -*)
            printf 'find-cargo-registry.sh: unknown flag: %s\n' "$1" >&2
            exit 2 ;;
        *)
            if [ -z "$substr" ]; then
                substr=$1; shift
            elif [ "$name_glob" = '*.rs' ]; then
                name_glob=$1; shift
            else
                printf 'find-cargo-registry.sh: too many positional args\n' >&2
                exit 2
            fi ;;
    esac
done

# Resolve the registry-src root. The first wildcard component is
# the registry name (typically `index.crates.io-<hash>`), which
# varies across machines — let the shell glob expand it.
home=${HOME:-}
if [ -z "$home" ]; then
    printf '%s!!! find-cargo-registry.sh: $HOME unset%s\n' \
        "$C_ERR" "$C_RESET" >&2
    exit 1
fi

src_dir=
for cand in "$home/.cargo/registry/src/"*; do
    if [ -d "$cand" ]; then
        src_dir=$cand
        break
    fi
done

if [ -z "$src_dir" ]; then
    printf '%s!!! no vendored sources under ~/.cargo/registry/src/%s\n' \
        "$C_ERR" "$C_RESET" >&2
    printf '    (cargo populates this on first build — try `./scripts/rust-lint.sh` first)\n' >&2
    exit 1
fi

# Display paths with `~` instead of the literal home dir.
prettify() {
    sed "s|^$home|~|"
}

case "$mode" in
    root)
        printf '%s\n' "$src_dir" | prettify
        ;;
    list)
        find "$src_dir" -maxdepth 1 -mindepth 1 -type d | sort | prettify
        ;;
    search)
        if [ -z "$substr" ]; then
            printf 'find-cargo-registry.sh: missing <crate-substring>\n' >&2
            printf 'Run with -h for usage.\n' >&2
            exit 2
        fi
        # `-path '*<substr>*'` matches the substring anywhere in
        # the absolute path. `-name '<glob>'` restricts to files
        # whose basename matches. `-type f` keeps directories
        # off the output.
        find "$src_dir" -type f -path "*${substr}*" -name "$name_glob" | sort | prettify
        ;;
esac
