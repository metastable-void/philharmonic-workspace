#!/usr/bin/env bash
# Workspace status: parent and every submodule.
# Shows dirty working trees plus ahead/behind vs. the upstream
# branch (when on a branch).

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# The "# branch.ab +A -B" line (porcelain v2) carries ahead/behind
# counts when an upstream is set. Parse with `cut` for POSIX-sh
# portability — this script is bash, but the submodule block runs
# inside `git submodule foreach`, which uses /bin/sh.
echo "=== parent ==="
git status -s
status=$(git status --porcelain=v2 --branch 2>/dev/null | grep "^# branch\.ab" || true)
if [ -n "$status" ]; then
    ahead=$(echo "$status" | cut -d" " -f3)
    behind=$(echo "$status" | cut -d" " -f4)
    if [ "$ahead" != "+0" ] || [ "$behind" != "-0" ]; then
        echo "  (branch: ahead=$ahead behind=$behind)"
    fi
fi
echo

git submodule foreach --quiet '
echo "=== $name ==="
git status -s
status=$(git status --porcelain=v2 --branch 2>/dev/null | grep "^# branch\.ab" || true)
if [ -n "$status" ]; then
    ahead=$(echo "$status" | cut -d" " -f3)
    behind=$(echo "$status" | cut -d" " -f4)
    if [ "$ahead" != "+0" ] || [ "$behind" != "-0" ]; then
        echo "  (branch: ahead=$ahead behind=$behind)"
    fi
fi
branch=$(git rev-parse --abbrev-ref HEAD)
if [ "$branch" = "HEAD" ]; then
    echo "  (detached HEAD)"
fi
echo
'
