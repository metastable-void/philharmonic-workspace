#!/usr/bin/env bash
# Commit pending changes across the workspace.
#
# Walks each submodule, commits any dirty tree there, then commits
# the parent (which includes the bumped submodule pointers).
#
# Every commit is signed off (`-s`). This is the workspace
# convention; see docs/design/13-conventions.md §Git workflow.
#
# Safety: refuses to commit in a submodule that's in detached HEAD
# state if it has changes to commit — that commit would be an
# orphan the next time the submodule is checked out.
#
# Usage:
#   scripts/commit-all.sh [message]
# Message defaults to "updates".

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

msg="${1:-updates}"

# Stash the message in a temp file so we don't have to escape it
# through `git submodule foreach`'s nested shell.
msgfile="$(mktemp)"
trap 'rm -f "$msgfile"' EXIT
printf '%s\n' "$msg" > "$msgfile"
export MSG_FILE="$msgfile"

# Commit each submodule's changes (if any).
git submodule foreach --quiet '
branch=$(git rev-parse --abbrev-ref HEAD)
if ! git diff --quiet || ! git diff --cached --quiet || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    if [ "$branch" = "HEAD" ]; then
        echo "!!! $name is in detached HEAD with uncommitted changes." >&2
        echo "    Refusing to commit (would create an orphan)." >&2
        echo "    Checkout a branch inside the submodule and re-run." >&2
        exit 1
    fi
    echo "=== committing in $name (branch: $branch) ==="
    git add -A
    git commit -s -F "$MSG_FILE"
else
    echo "=== $name clean ==="
fi
'

# Commit parent's changes (including bumped submodule pointers).
if ! git diff --quiet || ! git diff --cached --quiet || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    echo "=== committing in parent ==="
    git add -A
    git commit -s -F "$msgfile"
else
    echo "=== parent clean ==="
fi
