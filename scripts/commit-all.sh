#!/usr/bin/env bash
# Commit pending changes across the workspace.
#
# Walks each submodule, commits any dirty tree there, then commits
# the parent (which includes the bumped submodule pointers).
#
# Every commit is signed off (`-s`). This is the workspace
# convention; see docs/design/13-conventions.md §Git workflow.
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
if ! git diff --quiet || ! git diff --cached --quiet || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    echo "=== committing in $name ==="
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
