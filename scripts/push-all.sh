#!/bin/sh
# Push every submodule's current branch to origin, then push the
# parent. Pushes go through this script (not direct `git push`) so
# submodules are always pushed before the parent bumps their
# pointers — the common failure mode is pushing the parent while
# the referenced submodule commit only lives locally.
#
# Behavior:
# - Submodule push failures abort before the parent is pushed,
#   so we never push a parent pointer that origin can't resolve.
# - Submodules in detached HEAD are skipped with a warning. This
#   is a normal state right after `git submodule update`; the
#   real guardrail against pushing unresolvable pointers is
#   `push.recurseSubmodules=check` on the parent (see ROADMAP §2).
# - `--follow-tags` ships any annotated tags pointing at the
#   pushed commits. scripts/publish-crate.sh creates signed
#   annotated tags on release commits, so a single push-all
#   carries them along with the branch update.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

cd "$(git rev-parse --show-toplevel)"

git submodule foreach '
branch=$(git rev-parse --abbrev-ref HEAD)
if [ "$branch" = "HEAD" ]; then
    echo "!!! $name is in detached HEAD; skipping push." >&2
    echo "    If you made local commits here, checkout a branch" >&2
    echo "    and re-run this script before the parent is pushed." >&2
else
    git push --follow-tags origin "$branch"
fi
'

# Push parent. With push.recurseSubmodules=check configured,
# Git refuses this if any referenced submodule commit is not on
# origin. --follow-tags carries any parent-level release tags.
git push --follow-tags
