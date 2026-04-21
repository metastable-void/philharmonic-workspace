#!/bin/sh
# scripts/show-dirty.sh — print the names of submodules whose
# working tree is dirty (unstaged changes, staged changes, or
# untracked files not covered by .gitignore), one name per line.
#
# Machine-readable — `pre-landing.sh` uses this to compute modified
# crates, and other scripts can consume it the same way. Also
# usable standalone to inspect dirty submodules without the noise
# of `status.sh`.
#
# Usage:
#   ./scripts/show-dirty.sh
#
# Output: one crate name per line. Empty output if nothing is
# dirty. Always exits 0 (the absence of dirty submodules isn't a
# failure).
#
# Does not report the parent's own dirtiness — use `status.sh` for
# a full picture.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"

# Buffer via `$()` so SIGPIPE from a truncating consumer doesn't
# abort the foreach walk.
output=$(
    git submodule foreach --quiet '
        if ! git diff --quiet \
            || ! git diff --cached --quiet \
            || [ -n "$(git ls-files --others --exclude-standard)" ]; then
            echo "$name"
        fi
    '
)

if [ -n "$output" ]; then
    printf '%s\n' "$output"
fi
