#!/bin/sh
# Commit pending changes across the workspace.
#
# Walks each submodule, commits any dirty tree there, then commits
# the parent (which includes the bumped submodule pointers).
#
# Every commit is signed off (`-s`, DCO trailer) *and*
# cryptographically signed (`-S`, GPG or SSH). Signing is
# enforced two ways: we pass `-S` to `git commit` (so a missing
# signing key aborts the commit before it lands), and we verify
# the resulting HEAD with `git log --format=%G?` — if the commit
# somehow lacks a signature, we roll it back and fail. See
# docs/design/13-conventions.md §Git workflow.
#
# Safety: refuses to commit in a submodule that's in detached HEAD
# state if it has changes to commit — that commit would be an
# orphan the next time the submodule is checked out.
#
# Usage:
#   scripts/commit-all.sh [--parent-only] [message]
#
# --parent-only: skip the submodule walk. Use this when the parent
#   has its own pending work (e.g. docs, scripts) that should land
#   independently of whatever the submodules are currently doing.
#   Handy when submodules hold in-progress Codex work that shouldn't
#   be committed yet.
# Message defaults to "updates".
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

cd "$(git rev-parse --show-toplevel)"

parent_only=0
if [ "${1:-}" = "--parent-only" ]; then
    parent_only=1
    shift
fi

msg="${1:-updates}"

# Stash the message in a temp file so we don't have to escape it
# through `git submodule foreach`'s nested shell.
msgfile="$(mktemp)"
trap 'rm -f "$msgfile"' EXIT
printf '%s\n' "$msg" > "$msgfile"
export MSG_FILE="$msgfile"

# Commit each submodule's changes (if any), unless --parent-only.
if [ "$parent_only" -eq 0 ]; then
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
    # -S forces GPG/SSH signing; commit aborts here if no key.
    git commit -s -S -F "$MSG_FILE"
    # Defence in depth: verify the commit actually carries a
    # signature. %G? returns "N" for unsigned commits.
    sig=$(git log -n 1 --format=%G? HEAD)
    if [ "$sig" = "N" ]; then
        echo "!!! $name: HEAD $(git rev-parse --short HEAD) has no signature." >&2
        echo "    Rolling back with git reset --soft HEAD~1." >&2
        git reset --soft HEAD~1
        exit 1
    fi
else
    echo "=== $name clean ==="
fi
'
else
    echo "=== --parent-only: skipping submodules ==="
fi

# Commit parent's changes (including bumped submodule pointers).
if ! git diff --quiet || ! git diff --cached --quiet || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    echo "=== committing in parent ==="
    git add -A
    # -S forces GPG/SSH signing; commit aborts here if no key.
    git commit -s -S -F "$msgfile"
    sig=$(git log -n 1 --format=%G? HEAD)
    if [ "$sig" = "N" ]; then
        echo "!!! parent: HEAD $(git rev-parse --short HEAD) has no signature." >&2
        echo "    Rolling back with git reset --soft HEAD~1." >&2
        git reset --soft HEAD~1
        exit 1
    fi
else
    echo "=== parent clean ==="
fi
