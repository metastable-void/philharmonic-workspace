#!/bin/sh
# scripts/crate-version.sh — print the version string of a
# workspace crate, parsed from its own `Cargo.toml`.
#
# Usage:
#   ./scripts/crate-version.sh <crate-name>
#   ./scripts/crate-version.sh --all
#
# Without `--all`: prints the version (e.g. `0.2.3`) of the named
# crate on stdout, followed by a newline. Exits non-zero if the
# crate directory or version line can't be located. Intended for
# other scripts to consume:
#
#   version=$(./scripts/crate-version.sh mechanics-core)
#
# Standalone use is fine too.
#
# With `--all`: walks every workspace submodule and prints one
# aligned `<crate-name>  <version>` line per submodule that has a
# `Cargo.toml` with a parseable `version = "..."`. Useful for a
# quick at-a-glance view of the whole family's versions (e.g. when
# preparing a multi-crate release). Submodules without a top-level
# `Cargo.toml` or without a parseable version are skipped silently;
# empty output means nothing matched.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"

usage() {
    # $1 — exit code (0 for --help, 2 for invocation errors).
    cat >&2 <<'EOF'
Usage:
  crate-version.sh <crate-name>   # print <crate>/Cargo.toml version
  crate-version.sh --all          # print every submodule crate's version
EOF
    exit "${1:-2}"
}

# Awk for the first `version = "..."` line. `[package]` appears
# before `[dependencies]` in every workspace crate, so the first
# match is the crate's own version (not a dep version).
parse_version() {
    awk -F'"' '/^version *=/ { print $2; exit }' "$1"
}

if [ $# -ne 1 ]; then
    usage
fi

case $1 in
    -h|--help)
        usage 0
        ;;
    --all)
        # Buffer via `$()` so SIGPIPE from a truncating consumer
        # (e.g. `./scripts/crate-version.sh --all | head -3`) doesn't
        # abort the submodule walk mid-way.
        output=$(
            git submodule foreach --quiet 'printf "%s\n" "$sm_path"' \
            | while read -r path; do
                [ -f "$path/Cargo.toml" ] || continue
                ver=$(parse_version "$path/Cargo.toml")
                [ -n "$ver" ] || continue
                printf '%-48s %s\n' "$path" "$ver"
            done
        )
        if [ -n "$output" ]; then
            printf '%s\n' "$output"
        fi
        ;;
    -*)
        printf '!!! unknown flag: %s\n' "$1" >&2
        usage
        ;;
    *)
        crate=$1
        if [ ! -f "$crate/Cargo.toml" ]; then
            printf '!!! %s: not a workspace crate (no %s/Cargo.toml)\n' "$crate" "$crate" >&2
            exit 1
        fi
        version=$(parse_version "$crate/Cargo.toml")
        if [ -z "$version" ]; then
            printf '!!! %s: could not parse version from Cargo.toml\n' "$crate" >&2
            exit 1
        fi
        printf '%s\n' "$version"
        ;;
esac
