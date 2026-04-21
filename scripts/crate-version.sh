#!/bin/sh
# scripts/crate-version.sh — print the version string of a
# workspace crate, parsed from its own `Cargo.toml`.
#
# Usage:
#   ./scripts/crate-version.sh <crate-name>
#
# Output: the version (e.g. `0.2.3`) on stdout, followed by a
# newline. Exits non-zero if the crate directory or version line
# can't be located.
#
# Intended for other scripts to consume:
#
#   version=$(./scripts/crate-version.sh mechanics-core)
#
# Standalone use is fine too.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"

if [ $# -ne 1 ]; then
    echo "Usage: $0 <crate-name>" >&2
    exit 2
fi

crate=$1

if [ ! -f "$crate/Cargo.toml" ]; then
    printf '!!! %s: not a workspace crate (no %s/Cargo.toml)\n' "$crate" "$crate" >&2
    exit 1
fi

# Awk for the first `version = "..."` line. `[package]` appears
# before `[dependencies]` in every workspace crate, so the first
# match is the crate's own version (not a dep version).
version=$(awk -F'"' '/^version *=/ { print $2; exit }' "$crate/Cargo.toml")

if [ -z "$version" ]; then
    printf '!!! %s: could not parse version from Cargo.toml\n' "$crate" >&2
    exit 1
fi

printf '%s\n' "$version"
