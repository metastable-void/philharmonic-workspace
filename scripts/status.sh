#!/usr/bin/env bash
set -euo pipefail

echo "=== parent ==="
git -C "$(git rev-parse --show-toplevel)" status -s
echo

git submodule foreach --quiet '
echo "=== $name ==="
git status -s
status=$(git status --porcelain=v2 --branch 2>/dev/null | grep "^# branch\.ab" || true)
if [ -n "$status" ]; then
    ahead=$(echo "$status" | awk "{print \$3}")
    behind=$(echo "$status" | awk "{print \$4}")
    if [ "$ahead" != "+0" ] || [ "$behind" != "-0" ]; then
        echo "  (branch: ahead=$ahead behind=$behind)"
    fi
fi
echo
'
