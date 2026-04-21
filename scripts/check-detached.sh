#!/bin/sh
# scripts/check-detached.sh — report any submodule in detached HEAD
# with a non-zero exit when there is at least one. Useful pre-flight
# before `commit-all.sh`, `publish-crate.sh`, or any multi-commit
# operation — `commit-all.sh` refuses detached-HEAD submodules on
# its own, but this script checks upfront so you catch the problem
# before dirty work piles up.
#
# Usage:
#   ./scripts/check-detached.sh
#
# Exit code:
#   0 — all submodules on a branch.
#   1 — one or more in detached HEAD (reports each, one per line).
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"

# Collect rows via `$()` so SIGPIPE on a truncating consumer (e.g.
# `check-detached.sh | head -1`) doesn't abort the submodule walk.
# Same pattern as heads.sh.
output=$(
    git submodule foreach --quiet '
        branch=$(git rev-parse --abbrev-ref HEAD)
        if [ "$branch" = "HEAD" ]; then
            printf "%s (at %s)\n" "$name" "$(git rev-parse --short HEAD)"
        fi
    '
)

if [ -n "$output" ]; then
    count=$(printf '%s\n' "$output" | wc -l | tr -d ' ')
    echo '!!! detached-HEAD submodules:' >&2
    printf '%s\n' "$output" | sed 's/^/    /' >&2
    printf '!!! %s submodule(s) in detached HEAD. Check out a branch inside each before committing.\n' "$count" >&2
    exit 1
fi

echo 'all submodules on a branch.'
