#!/usr/bin/env bash
# Push every submodule's current branch to origin, then push the
# parent. Pushes go through this script (not direct `git push`) so
# submodules are always pushed before the parent bumps their
# pointers — the common failure mode is pushing the parent while
# the referenced submodule commit only lives locally.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# Push each submodule's current branch.
git submodule foreach '
branch=$(git rev-parse --abbrev-ref HEAD)
if [ "$branch" != "HEAD" ]; then
    git push -u origin "$branch" || true
fi
'

# Push parent.
git push
