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
# orphan the next time the submodule is checked out. Also refuses
# the parent commit if .claude/settings.json contains any
# reference to a guarded script (`scripts/commit-all.sh` or
# `scripts/publish-crate.sh`) — a permission entry for either
# would let agents commit or publish without human approval.
#
# Usage:
#   scripts/commit-all.sh [--anonymize] [--parent-only] [message]
#
# --parent-only: skip the submodule walk. Use this when the parent
#   has its own pending work (e.g. docs, scripts) that should land
#   independently of whatever the submodules are currently doing.
#   Handy when submodules hold in-progress Codex work that shouldn't
#   be committed yet.
# Message defaults to "updates".
#
# Scope: the parent commit uses `git add -A` before `git commit`,
# so ALL dirty parent files get swept in. Pre-staging a subset
# with `git add` does NOT scope the commit — selective staging is
# meaningless here. To make a single-purpose commit when the parent
# has unrelated dirty files, clean them out of the tree first
# (move to /tmp, or commit them separately in a prior --parent-only
# invocation), then run commit-all.sh. See
# docs/design/13-conventions.md §Git workflow.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/codex-guard.sh"

parent_only=0
anonymize=0
while [ $# -gt 0 ]; do
    case "$1" in
        --parent-only) parent_only=1; shift ;;
        --anonymize) anonymize=1; shift ;;
        --help)
            echo "Usage: $0 [--anonymize] [--parent-only] [message]"
            exit
            ;;
        --)                           shift; break ;;
        -*) printf 'unknown flag: %s\n' "$1" >&2; exit 2 ;;
        *) break ;;
    esac
done

msg="${1:-updates}"

if [ "${anonymize}" -eq 1 ] ; then
    audit_flags="--anonymize"
else
    audit_flags=
fi

# Stash the message in a temp file so we don't have to escape it
# through `git submodule foreach`'s nested shell.
msgfile="$("$(dirname "$0")"/mktemp.sh commit-msg)"
trap 'rm -f "$msgfile"' EXIT INT HUP TERM
printf '%s\n\n' "$msg" > "$msgfile"
printf 'Audit-Info: %s\n' "$( "$(dirname "$0")"/print-audit-info.sh $audit_flags )" >> "$msgfile"
printf 'Code-stats: %s\n' "$( "$(dirname "$0")"/stats.sh )" >> "$msgfile"
export MSG_FILE="$msgfile"

# Signal that we call Git through a proper wrapper.
WORKSPACE_GIT_WRAPPER=1
export WORKSPACE_GIT_WRAPPER

# Commit each submodule's changes (if any), unless --parent-only.
if [ "$parent_only" -eq 0 ]; then
    git submodule foreach --quiet '
branch=$(git rev-parse --abbrev-ref HEAD)
if ! git diff --quiet || ! git diff --cached --quiet || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    if [ "$branch" = "HEAD" ]; then
        printf "%s!!! $name is in detached HEAD with uncommitted changes.%s\n" "$C_ERR" "$C_RESET" >&2
        echo "    Refusing to commit (would create an orphan)." >&2
        echo "    Checkout a branch inside the submodule and re-run." >&2
        exit 1
    fi
    printf "%s=== committing in $name (branch: $branch) ===%s\n" "$C_HEADER" "$C_RESET"
    git add -A
    # -S forces GPG/SSH signing; commit aborts here if no key.
    git commit -s -S -F "$MSG_FILE"
    # Defence in depth: verify the commit actually carries a
    # signature. %G? returns "N" for unsigned commits.
    sig=$(git log -n 1 --format=%G? HEAD)
    if [ "$sig" = "N" ]; then
        printf "%s!!! $name: HEAD $(git rev-parse --short HEAD) has no signature.%s\n" "$C_ERR" "$C_RESET" >&2
        echo "    Rolling back with git reset --soft HEAD~1." >&2
        git reset --soft HEAD~1
        exit 1
    fi
else
    printf "%s=== $name clean ===%s\n" "$C_DIM" "$C_RESET"
fi
'
else
    printf '%s=== --parent-only: skipping submodules ===%s\n' "$C_DIM" "$C_RESET"
fi

# Safety: refuse the parent commit if .claude/settings.json
# contains any reference to a guarded script. A permission entry
# for either `scripts/commit-all.sh` or `scripts/publish-crate.sh`
# (e.g. `Bash(./scripts/commit-all.sh *)`) would let an agent run
# that script without human approval, defeating the human-in-the-
# loop gate this workflow relies on — commit-all.sh grants the
# ability to produce signed commits, publish-crate.sh grants the
# ability to publish to crates.io and mint signed release tags.
# The check is a verbatim fixed-string match so any form of the
# allow entry is caught. To restore the ability to commit, remove
# the offending entry from .claude/settings.json.
settings_file=".claude/settings.json"
if [ -f "$settings_file" ]; then
    # POSIX `for` over an unquoted space-separated list —
    # consistent with `crates=$*; for c in $crates` elsewhere in
    # the scripts. No entry contains whitespace, so splitting on
    # IFS is safe.
    for guarded in scripts/commit-all.sh scripts/publish-crate.sh; do
        if grep -F -q "$guarded" "$settings_file"; then
            echo "!!! $settings_file references $guarded." >&2
            echo "    Refusing to commit the parent — a permission entry for" >&2
            echo "    this script would let agents run it without human" >&2
            echo "    approval. Remove the entry from $settings_file and" >&2
            echo "    re-run." >&2
            exit 1
        fi
    done
fi

# Commit parent's changes (including bumped submodule pointers).
if ! git diff --quiet || ! git diff --cached --quiet || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    printf '%s=== committing in parent ===%s\n' "$C_HEADER" "$C_RESET"
    git add -A
    # -S forces GPG/SSH signing; commit aborts here if no key.
    git commit -s -S -F "$msgfile"
    sig=$(git log -n 1 --format=%G? HEAD)
    if [ "$sig" = "N" ]; then
        printf '%s!!! parent: HEAD %s has no signature.%s\n' \
            "$C_ERR" "$(git rev-parse --short HEAD)" "$C_RESET" >&2
        echo "    Rolling back with git reset --soft HEAD~1." >&2
        git reset --soft HEAD~1
        exit 1
    fi
else
    printf '%s=== parent clean ===%s\n' "$C_DIM" "$C_RESET"
fi
