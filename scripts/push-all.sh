#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# Push each submodule's current branch
git submodule foreach '
branch=$(git rev-parse --abbrev-ref HEAD)
if [ "$branch" != "HEAD" ]; then
    git push -u origin "$branch" || true
fi
'

# Push parent
git push
