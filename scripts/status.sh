#!/bin/sh
# Workspace status: parent and every submodule.
# Shows dirty working trees plus ahead/behind vs. the upstream
# branch (when on a branch).
#
# Colors: ANSI sequences are emitted only when stdout is a TTY
# and NO_COLOR is unset. Pass `--no-color` to force plain
# output. Git's own status coloring follows the same gate.
#
# POSIX sh only — see CONTRIBUTING.md §6.

set -eu

no_color=0
while [ $# -gt 0 ]; do
    case "$1" in
        --no-color) no_color=1; shift ;;
        -h|--help)
            printf 'Usage: status.sh [--no-color] [-h|--help]\n'
            exit 0 ;;
        *)
            printf 'status.sh: unknown flag: %s\n' "$1" >&2
            exit 2 ;;
    esac
done

if [ "$no_color" -eq 1 ]; then
    NO_COLOR=1
    export NO_COLOR
fi
. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

# git status coloring: `always` when our colors are on,
# `never` otherwise. C_RESET is non-empty iff colors are on.
if [ -n "$C_RESET" ]; then
    git_color=always
else
    git_color=never
fi
export git_color

printf '%s=== parent ===%s\n' "$C_HEADER" "$C_RESET"
git -c color.status="$git_color" status -s
status=$(git status --porcelain=v2 --branch 2>/dev/null | grep "^# branch\.ab" || true)
if [ -n "$status" ]; then
    ahead=$(echo "$status" | cut -d" " -f3)
    behind=$(echo "$status" | cut -d" " -f4)
    branch_name=$(git branch --show-current)
    if [ "$ahead" != "+0" ] || [ "$behind" != "-0" ]; then
        printf '  %s(branch %s: ahead=%s behind=%s)%s\n' \
            "$C_WARN" "$branch_name" "$ahead" "$behind" "$C_RESET"
    fi
fi
echo

git submodule foreach --quiet '
dirty=$(git -c color.status="$git_color" status -s)
ahead=""
behind=""
status=$(git status --porcelain=v2 --branch 2>/dev/null | grep "^# branch\.ab" || true)
if [ -n "$status" ]; then
    ahead=$(echo "$status" | cut -d" " -f3)
    behind=$(echo "$status" | cut -d" " -f4)
fi
diverged=0
if [ -n "$ahead" ] && { [ "$ahead" != "+0" ] || [ "$behind" != "-0" ]; }; then
    diverged=1
fi
branch=$(git rev-parse --abbrev-ref HEAD)
detached=0
if [ "$branch" = "HEAD" ]; then
    detached=1
fi

if [ -z "$dirty" ] && [ "$diverged" = "0" ] && [ "$detached" = "0" ]; then
    exit 0
fi

printf "%s=== %s ===%s\n" "$C_HEADER" "$name" "$C_RESET"
if [ -n "$dirty" ]; then
    printf "%s\n" "$dirty"
fi

branch_name=$(git branch --show-current)
if [ "$diverged" = "1" ]; then
    printf "  %s(branch %s: ahead=%s behind=%s)%s\n" \
        "$C_WARN" "$branch_name" "$ahead" "$behind" "$C_RESET"
fi
if [ "$detached" = "1" ]; then
    printf "  %s(detached HEAD)%s\n" "$C_ERR" "$C_RESET"
fi
echo
'
