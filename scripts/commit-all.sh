#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

msg="${1:-updates}"

# Commit each submodule's changes (if any)
git submodule foreach --quiet '
if ! git diff --quiet || ! git diff --cached --quiet || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    echo "=== committing in $name ==="
    git add -A
    git commit -m "'"$msg"'"
else
    echo "=== $name clean ==="
fi
'

# Commit parent's changes (including bumped submodule pointers)
if ! git diff --quiet || ! git diff --cached --quiet || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    echo "=== committing in parent ==="
    git add -A
    git commit -m "$msg"
else
    echo "=== parent clean ==="
fi
